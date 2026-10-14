//! 翻译抽象层(spec 04)。
//!
//! `Translator` trait 抽象单/批量翻译;各引擎实现见同目录子模块
//! (`openai` / `google` / `deepl` / `youdao`)。`TranslatorManager` 维护
//! 「id → 引擎」表与默认引擎 id,共享一个 `reqwest::Client`,
//! 支持订阅 `config://changed` 后 `reload` 热更新(不重启进程)。

mod apple;
pub use apple::available as apple_available;
mod baidu;
mod deepl;
mod google;
mod languages;
mod microsoft;
mod openai;
mod youdao;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

/// 翻译器抽象(spec 04 §1)。所有引擎统一接收 `TranslateRequest`(单条即
/// `texts.len()==1`),返回与 `texts` 等长的译文。
// Limit the exception to async_trait's generated boxed-Future signatures.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait Translator: Send + Sync {
    /// 引擎 id(uuid)。
    fn id(&self) -> &str;
    /// 用户自定义名称。
    fn name(&self) -> &str;
    /// 引擎种类。
    fn kind(&self) -> EngineKind;
    /// 是否原生支持单请求批量(否则 manager / 引擎内部逐条串行)。
    fn supports_batch(&self) -> bool;
    /// 执行一次(批量)翻译。
    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError>;
}

/// 多引擎管理器(spec 04 §2)。作为 Tauri managed state 注入。
pub struct TranslatorManager {
    client: reqwest::Client,
    engines: RwLock<HashMap<String, Arc<dyn Translator>>>,
    default_id: RwLock<String>,
}

impl TranslatorManager {
    /// 空管理器(启动后由 `reload` 注入引擎)。
    pub fn new() -> Self {
        TranslatorManager {
            client: build_http_client(),
            engines: RwLock::new(HashMap::new()),
            default_id: RwLock::new(String::new()),
        }
    }

    /// 按当前配置构建并注入引擎(spec 04 §2)。覆盖式重建,可重复调用以热更新。
    ///
    /// - 仅纳入 `enabled` 的引擎;
    /// - 所有引擎共享同一个 `reqwest::Client`;
    /// - 跳过缺少必需凭据或尚未实现的引擎;
    /// - 解析默认引擎:配置指定且可用则用之,否则按配置顺序取首个可用引擎。
    pub fn reload(&self, engines: &[TranslatorEngine], default_id: &str) {
        let client = self.client.clone();

        let mut map: HashMap<String, Arc<dyn Translator>> = HashMap::new();
        for cfg in engines {
            if !cfg.enabled {
                continue;
            }
            if let Some(reason) = engine_unavailable_reason(cfg) {
                log::warn!("translator engine '{}' skipped: {reason}", cfg.id);
                continue;
            }
            map.insert(cfg.id.clone(), build_engine(cfg.clone(), client.clone()));
        }

        let resolved_default = if !default_id.is_empty() && map.contains_key(default_id) {
            default_id.to_string()
        } else {
            engines
                .iter()
                .find(|cfg| map.contains_key(&cfg.id))
                .map(|cfg| cfg.id.clone())
                .unwrap_or_default()
        };

        match self.engines.write() {
            Ok(mut g) => *g = map,
            Err(e) => log::error!("translator engines lock poisoned: {e}"),
        }
        match self.default_id.write() {
            Ok(mut g) => *g = resolved_default,
            Err(e) => log::error!("translator default_id lock poisoned: {e}"),
        }
        log::info!(
            "translator reloaded: {} engine(s), default={}",
            map_len(&self.engines),
            self.default_id
                .read()
                .map(|g| g.clone())
                .unwrap_or_default()
        );
    }

    /// 取指定引擎;`None` / 空串取默认引擎(spec 04 §2)。
    pub fn get(&self, id: Option<&str>) -> Option<Arc<dyn Translator>> {
        let engines = self.engines.read().ok()?;
        let key = match id {
            Some(i) if !i.is_empty() => i.to_string(),
            _ => self.default_id.read().ok()?.clone(),
        };
        engines.get(&key).cloned()
    }

    /// 路由到目标引擎执行翻译(spec 04 §2)。
    pub async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let engine = self
            .get(req.engine_id.as_deref())
            .ok_or_else(|| LenzError::Config("engine_not_found".into()))?;
        // 空输入直接返回空结果,避免向引擎发空请求。
        if req.texts.is_empty() {
            return Ok(TranslateResponse {
                translations: Vec::new(),
                detected_source_lang: None,
                engine_id: engine.id().to_string(),
                cost_ms: 0,
            });
        }
        engine.translate(req).await
    }
}

fn map_len(engines: &RwLock<HashMap<String, Arc<dyn Translator>>>) -> usize {
    engines.read().map(|map| map.len()).unwrap_or_default()
}

/// 返回引擎不能安全装载的原因。设置 UI 可以暂存不完整草稿,运行态不能把这些草稿
/// 当成候选默认引擎,否则会把每次翻译路由到必然失败的请求。
fn engine_unavailable_reason(cfg: &TranslatorEngine) -> Option<&'static str> {
    let key = cfg.api_key.trim();
    match cfg.kind {
        EngineKind::Apple if !apple::available() => Some("Apple Translation requires macOS 15+"),
        EngineKind::Google if key.is_empty() => Some("Google API key is required"),
        EngineKind::Deepl if key.is_empty() => Some("DeepL API key is required"),
        EngineKind::Microsoft if key.is_empty() => Some("Microsoft API key is required"),
        EngineKind::Youdao | EngineKind::Baidu => {
            let valid_pair = key
                .split_once(':')
                .map(|(id, secret)| !id.trim().is_empty() && !secret.trim().is_empty())
                .unwrap_or(false);
            (!valid_pair).then_some("credential must contain a non-empty id and secret")
        }
        EngineKind::Openai
            if matches!(cfg.id.as_str(), "builtin-zhipu" | "builtin-siliconflow")
                && key.is_empty() =>
        {
            Some("API key is required")
        }
        _ => None,
    }
}

impl Default for TranslatorManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 为 `test_engine` 命令构造一次性引擎(不入 manager,可测草稿引擎)。
pub fn build_test_engine(cfg: TranslatorEngine) -> Arc<dyn Translator> {
    build_engine(cfg, build_http_client())
}

/// 工厂:据 `kind` 构造对应引擎实现(spec 04 §1)。
fn build_engine(cfg: TranslatorEngine, client: reqwest::Client) -> Arc<dyn Translator> {
    match cfg.kind {
        EngineKind::Openai => Arc::new(openai::OpenAiTranslator::new(cfg, client)),
        EngineKind::Google => Arc::new(google::GoogleTranslator::new(cfg, client)),
        EngineKind::Deepl => Arc::new(deepl::DeepLTranslator::new(cfg, client)),
        EngineKind::Youdao => Arc::new(youdao::YoudaoTranslator::new(cfg, client)),
        EngineKind::Baidu => Arc::new(baidu::BaiduTranslator::new(cfg, client)),
        EngineKind::Microsoft => Arc::new(microsoft::MicrosoftTranslator::new(cfg, client)),
        EngineKind::Apple => Arc::new(apple::AppleTranslator::new(cfg, client)),
    }
}

// ──────────────────────────────────────────────────────────────────────────
// HTTP client 构建(spec 04 §7)
// ──────────────────────────────────────────────────────────────────────────

/// 构建共享 `reqwest::Client`(30s 超时);构建失败回退默认 client。
///
/// 不做任何应用内代理配置:沿用 reqwest 默认行为(尊重系统环境变量代理)。
pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|e| {
            log::warn!("build http client failed: {e}; using default");
            reqwest::Client::new()
        })
}

// ──────────────────────────────────────────────────────────────────────────
// 共享 HTTP / 语言 / Prompt helper(供子模块 `super::` 复用)
// ──────────────────────────────────────────────────────────────────────────

/// 默认 system prompt(spec 04 §3.1)。
pub(crate) const DEFAULT_SYSTEM_PROMPT: &str = "You are a professional translator.\n\
Translate the following text from {sourceLang} to {targetLang}.\n\
Output only the translation, no explanation, no extra content.";

/// 渲染 system prompt:替换 `{sourceLang}` / `{targetLang}`(spec 04 §3.1)。
pub(crate) fn render_system_prompt(
    template: Option<&str>,
    source: Option<&str>,
    target: &str,
) -> String {
    let tmpl = template
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SYSTEM_PROMPT);
    let src = source
        .map(lang_display_name)
        .unwrap_or_else(|| "the source language".to_string());
    let tgt = lang_display_name(target);
    tmpl.replace("{sourceLang}", &src)
        .replace("{targetLang}", &tgt)
}

/// BCP-47 → 英文全名(用于 LLM prompt),未知则原样返回。
pub(crate) fn lang_display_name(code: &str) -> String {
    let c = code.trim().to_ascii_lowercase();
    let name = match c.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-hans-cn" => "Chinese (Simplified)",
        "zh-tw" | "zh-hant" | "zh-hk" => "Chinese (Traditional)",
        "en" | "en-us" | "en-gb" => "English",
        "ja" | "ja-jp" => "Japanese",
        "ko" | "ko-kr" => "Korean",
        "fr" | "fr-fr" => "French",
        "de" | "de-de" => "German",
        "es" | "es-es" => "Spanish",
        "it" | "it-it" => "Italian",
        "pt" | "pt-pt" => "Portuguese",
        "pt-br" => "Portuguese (Brazil)",
        "ru" | "ru-ru" => "Russian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "th" => "Thai",
        "vi" => "Vietnamese",
        _ => return code.trim().to_string(),
    };
    name.to_string()
}

/// 把 `reqwest::Error` 映射为 `LenzError`(spec 04 §9)。
pub(crate) fn map_reqwest_err(e: reqwest::Error) -> LenzError {
    if e.is_timeout() {
        LenzError::Timeout("response timed out".into())
    } else if e.is_connect() {
        LenzError::Network(format!("connect: {e}"))
    } else {
        LenzError::Network(format!("{e}"))
    }
}

/// HTTP 非 2xx → `LenzError`(spec 04 §9)。`body` 截断后并入 message 便于排查。
pub(crate) fn status_to_error(status: StatusCode, body: &str) -> LenzError {
    let snippet: String = body.chars().take(200).collect();
    match status.as_u16() {
        401 | 403 => LenzError::TranslateFailed("auth".into()),
        408 => LenzError::Timeout("response timed out".into()),
        429 => LenzError::TranslateFailed("rate_limit".into()),
        400..=499 => {
            LenzError::TranslateFailed(format!("client_error({}): {snippet}", status.as_u16()))
        }
        500..=599 => LenzError::TranslateFailed(format!("server_error: {snippet}")),
        other => LenzError::TranslateFailed(format!("http_{other}: {snippet}")),
    }
}

/// 发送请求并把成功响应反序列化为 `T`;非 2xx / 解析失败映射为 `LenzError`。
pub(crate) async fn send_json<T: DeserializeOwned>(
    req: reqwest::RequestBuilder,
) -> Result<T, LenzError> {
    let resp = req.send().await.map_err(map_reqwest_err)?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(status_to_error(status, &body));
    }
    let text = resp.text().await.map_err(map_reqwest_err)?;
    serde_json::from_str::<T>(&text)
        .map_err(|e| LenzError::TranslateFailed(format!("parse_error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_prompt_substitutes_lang_names() {
        let out = render_system_prompt(None, Some("en-US"), "zh-CN");
        assert!(out.contains("English"));
        assert!(out.contains("Chinese (Simplified)"));
        assert!(!out.contains("{sourceLang}"));
        assert!(!out.contains("{targetLang}"));
    }

    #[test]
    fn render_prompt_none_source_is_generic() {
        let out = render_system_prompt(None, None, "ja");
        assert!(out.contains("the source language"));
        assert!(out.contains("Japanese"));
    }

    #[test]
    fn render_prompt_custom_template_keeps_text() {
        let out = render_system_prompt(Some("译成{targetLang}"), None, "zh-CN");
        assert_eq!(out, "译成Chinese (Simplified)");
    }

    #[test]
    fn render_prompt_blank_template_falls_back_to_default() {
        let out = render_system_prompt(Some("   "), Some("en"), "zh-CN");
        assert!(out.contains("professional translator"));
    }

    #[test]
    fn unknown_lang_passthrough() {
        assert_eq!(lang_display_name("xx-YY"), "xx-YY");
    }

    #[test]
    fn status_mapping() {
        assert!(matches!(
            status_to_error(StatusCode::UNAUTHORIZED, ""),
            LenzError::TranslateFailed(m) if m == "auth"
        ));
        assert!(matches!(
            status_to_error(StatusCode::TOO_MANY_REQUESTS, ""),
            LenzError::TranslateFailed(m) if m == "rate_limit"
        ));
        assert!(matches!(
            status_to_error(StatusCode::INTERNAL_SERVER_ERROR, "boom"),
            LenzError::TranslateFailed(m) if m.starts_with("server_error")
        ));
    }

    /// 空管理器:无引擎时 translate 返回 engine_not_found。
    #[tokio::test]
    async fn empty_manager_returns_not_found() {
        let mgr = TranslatorManager::new();
        let req = TranslateRequest {
            texts: vec!["hello".into()],
            source_lang: None,
            target_lang: "zh-CN".into(),
            engine_id: None,
        };
        let err = mgr.translate(req).await.unwrap_err();
        assert!(matches!(err, LenzError::Config(m) if m == "engine_not_found"));
    }

    fn test_engine(id: &str, kind: EngineKind, enabled: bool, api_key: &str) -> TranslatorEngine {
        TranslatorEngine {
            id: id.into(),
            name: id.into(),
            kind,
            enabled,
            api_url: String::new(),
            api_key: api_key.into(),
            model: None,
            system_prompt: None,
            timeout_ms: 1_000,
            region: None,
            max_tokens: None,
            temperature: None,
        }
    }

    #[test]
    fn reload_skips_unconfigured_engine_and_falls_back_in_config_order() {
        let mgr = TranslatorManager::new();
        let engines = vec![
            test_engine("google", EngineKind::Google, true, ""),
            test_engine("second", EngineKind::Openai, true, ""),
            test_engine("third", EngineKind::Openai, true, ""),
        ];
        mgr.reload(&engines, "google");
        assert!(mgr.get(Some("google")).is_none());
        assert_eq!(mgr.get(None).unwrap().id(), "second");
    }
}

/// M3 §3.7 运行态验收(对**真实本地端点**)。默认 `#[ignore]`,按需运行:
///
/// ```sh
/// cargo test --manifest-path src-tauri/Cargo.toml acceptance \
///   -- --ignored --nocapture --test-threads=1
/// ```
///
/// 前置:本机 Ollama(`:11434`,模型 `qwen2.5:1.5b`)与 LM Studio(`:1234`)在线。
/// 把 spec 04 §10 的人工手测固化为可复现测试;不依赖 GUI,直接驱动引擎代码路径。
#[cfg(test)]
mod acceptance {
    use super::*;
    use std::time::Instant;

    const OLLAMA_URL: &str = "http://localhost:11434/v1";
    const OLLAMA_MODEL: &str = "qwen2.5:1.5b";
    const LMSTUDIO_URL: &str = "http://localhost:1234/v1";
    const LMSTUDIO_MODEL: &str = "qwen/qwen3-4b-2507";

    /// 是否含中日韩统一表意文字(粗判译文已转中文)。
    fn has_cjk(s: &str) -> bool {
        s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    }

    /// 构造一个 OpenAI 兼容引擎配置。
    fn engine(id: &str, url: &str, model: &str) -> TranslatorEngine {
        TranslatorEngine {
            id: id.into(),
            name: id.into(),
            kind: EngineKind::Openai,
            enabled: true,
            api_url: url.into(),
            api_key: String::new(),
            model: Some(model.into()),
            system_prompt: None,
            timeout_ms: 60_000,
            region: None,
            max_tokens: None,
            // 翻译验收要可复现:温度归零。
            temperature: Some(0.0),
        }
    }

    fn en_to_zh(texts: &[&str]) -> TranslateRequest {
        TranslateRequest {
            texts: texts.iter().map(|s| s.to_string()).collect(),
            source_lang: Some("en-US".into()),
            target_lang: "zh-CN".into(),
            engine_id: None,
        }
    }

    // ── §3.7 点 1:本地端点端到端,英文 → 中文 ───────────────────────────────

    #[tokio::test]
    #[ignore = "requires local Ollama (:11434, qwen2.5:1.5b)"]
    async fn acc_ollama_end_to_end() {
        let t = build_test_engine(engine("acc-ollama", OLLAMA_URL, OLLAMA_MODEL));
        let resp = t
            .translate(en_to_zh(&[
                "Hello, world. This is an end-to-end translation test.",
            ]))
            .await
            .expect("ollama translate should succeed");
        let out = &resp.translations[0];
        println!("[ollama] {}ms -> {out}", resp.cost_ms);
        assert_eq!(resp.translations.len(), 1);
        assert!(!out.trim().is_empty(), "translation must not be empty");
        assert!(has_cjk(out), "expected Chinese output, got: {out}");
    }

    #[tokio::test]
    #[ignore = "requires local LM Studio (:1234)"]
    async fn acc_lmstudio_end_to_end() {
        let t = build_test_engine(engine("acc-lmstudio", LMSTUDIO_URL, LMSTUDIO_MODEL));
        let resp = t
            .translate(en_to_zh(&["Good morning, have a nice day."]))
            .await
            .expect("lmstudio translate should succeed");
        let out = &resp.translations[0];
        println!("[lmstudio] {}ms -> {out}", resp.cost_ms);
        assert!(!out.trim().is_empty(), "translation must not be empty");
        assert!(has_cjk(out), "expected Chinese output, got: {out}");
    }

    // ── §3.7 点 2:批量 10 条单次请求,长度一致 ──────────────────────────────

    #[tokio::test]
    #[ignore = "requires local Ollama (:11434)"]
    async fn acc_batch_ten_same_length() {
        let inputs = [
            "apple", "banana", "cat", "dog", "house", "river", "mountain", "book", "music",
            "friend",
        ];
        let t = build_test_engine(engine("acc-batch", OLLAMA_URL, OLLAMA_MODEL));
        let resp = t
            .translate(en_to_zh(&inputs))
            .await
            .expect("batch translate should succeed");
        println!(
            "[batch] {} items -> {:?}",
            resp.translations.len(),
            resp.translations
        );
        assert_eq!(
            resp.translations.len(),
            inputs.len(),
            "batch output length must equal input length"
        );
        assert!(
            resp.translations.iter().all(|s| !s.trim().is_empty()),
            "no slot may be empty"
        );
    }

    // ── §3.7 点 3:test_engine 路径 ≤ 3s ─────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires local Ollama (:11434)"]
    async fn acc_test_engine_under_3s() {
        let cfg = engine("acc-latency", OLLAMA_URL, OLLAMA_MODEL);
        // 预热:把模型加载成本排除在计时外(按钮场景下模型通常已驻留)。
        let warm = build_test_engine(cfg.clone());
        let _ = warm.translate(en_to_zh(&["warm up"])).await;
        // 正式计时:复刻 `test_engine` 命令路径(build_test_engine + translate "Hello")。
        let t = build_test_engine(cfg);
        let started = Instant::now();
        let resp = t
            .translate(en_to_zh(&["Hello"]))
            .await
            .expect("translate should succeed");
        let ms = started.elapsed().as_millis();
        println!("[latency] {ms}ms -> {}", resp.translations[0]);
        assert!(ms <= 3000, "test_engine must return within 3s, took {ms}ms");
    }

    // ── §3.7 点 5:引擎配置热更新后,下一次翻译用新配置(不重启 manager)──────

    #[tokio::test]
    #[ignore = "requires local Ollama (:11434)"]
    async fn acc_hot_reload_routing() {
        let mgr = TranslatorManager::new();
        let a = engine("hot-a", OLLAMA_URL, OLLAMA_MODEL);
        let b = engine("hot-b", OLLAMA_URL, OLLAMA_MODEL);

        // 初始:默认引擎 hot-a。
        mgr.reload(&[a.clone(), b.clone()], "hot-a");
        let r1 = mgr.translate(en_to_zh(&["Hello"])).await.expect("t1");
        assert_eq!(r1.engine_id, "hot-a", "should route to initial default");

        // 热更新:仅切默认引擎,manager 实例不变、进程不重启。
        mgr.reload(&[a, b], "hot-b");
        let r2 = mgr.translate(en_to_zh(&["Hello"])).await.expect("t2");
        assert_eq!(
            r2.engine_id, "hot-b",
            "next translate must use reloaded config"
        );
    }
}
