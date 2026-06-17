//! OpenAI 兼容引擎(spec 04 §3)。适用 LM Studio / Ollama / DeepSeek / OpenAI。

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{lang_display_name, render_system_prompt, send_json, Translator};
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

pub struct OpenAiTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl OpenAiTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        OpenAiTranslator { cfg, client }
    }

    fn timeout(&self) -> Duration {
        Duration::from_millis(self.cfg.timeout_ms.max(1) as u64)
    }

    /// 单条对话补全,返回 `choices[0].message.content`(spec 04 §3.1)。
    async fn chat_once(&self, system: &str, user: &str) -> Result<String, LenzError> {
        let mut body = json!({
            "model": self.cfg.model.clone().unwrap_or_default(),
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "temperature": self.cfg.temperature.unwrap_or(0.1),
            "stream": false,
        });
        // 可选:限制最大输出 token 数(留空则交给服务端默认)。
        if let Some(max) = self.cfg.max_tokens {
            body["max_tokens"] = json!(max);
        }

        let mut rb = self
            .client
            .post(chat_endpoint(&self.cfg.api_url))
            .timeout(self.timeout())
            .json(&body);
        // 本地 LLM 常无需鉴权:apiKey 为空则不带 Authorization 头。
        if !self.cfg.api_key.is_empty() {
            rb = rb.bearer_auth(&self.cfg.api_key);
        }

        let resp: ChatResponse = send_json(rb).await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| LenzError::TranslateFailed("empty_response".into()))
    }

    /// 批量:一次请求让模型返回 JSON 数组;解析失败 / 长度不符则逐条兜底
    /// (spec 04 §3.2)。
    async fn translate_many(
        &self,
        system: &str,
        texts: &[String],
        source: Option<&str>,
        target: &str,
    ) -> Result<Vec<String>, LenzError> {
        let src = source
            .map(lang_display_name)
            .unwrap_or_else(|| "the source language".to_string());
        let tgt = lang_display_name(target);
        let array = serde_json::to_string(texts)
            .map_err(|e| LenzError::TranslateFailed(format!("encode: {e}")))?;
        let user = format!(
            "Translate each item in the JSON array below from {src} to {tgt}.\n\
             Return ONLY a JSON array of translated strings, same length, same order.\n\n{array}"
        );

        if let Ok(content) = self.chat_once(system, &user).await {
            let cleaned = extract_json_array(&content);
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&cleaned) {
                if parsed.len() == texts.len() {
                    return Ok(parsed);
                }
                log::warn!(
                    "openai batch length mismatch ({} vs {}), falling back to sequential",
                    parsed.len(),
                    texts.len()
                );
            } else {
                log::warn!("openai batch parse failed, falling back to sequential");
            }
        }

        // 兜底:逐条串行(同一引擎、同一 system prompt)。
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.chat_once(system, t).await?);
        }
        Ok(out)
    }
}

#[async_trait]
impl Translator for OpenAiTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Openai
    }
    fn supports_batch(&self) -> bool {
        true
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();
        let source = req.source_lang.as_deref();
        let system =
            render_system_prompt(self.cfg.system_prompt.as_deref(), source, &req.target_lang);

        let translations = if req.texts.len() == 1 {
            vec![self.chat_once(&system, &req.texts[0]).await?]
        } else {
            self.translate_many(&system, &req.texts, source, &req.target_lang)
                .await?
        };

        Ok(TranslateResponse {
            translations,
            detected_source_lang: None,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// 组装 chat completions 端点,容忍 `apiUrl` 的多种形态:
/// - 已含 `/chat/completions` → 原样;
/// - 已含版本段(`/v1`、`/api/paas/v4` 等)→ 仅补 `/chat/completions`;
/// - 裸 host(`http://localhost:1234`)→ 补 `/v1/chat/completions`。
fn chat_endpoint(api_url: &str) -> String {
    let base = api_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return "https://api.openai.com/v1/chat/completions".to_string();
    }
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else if has_version_segment(base) {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// 路径里是否已含形如 `vN` 的版本段(如 `/v1`、`/paas/v4`)。
fn has_version_segment(url: &str) -> bool {
    url.split('/').any(|seg| {
        seg.len() >= 2 && seg.starts_with('v') && seg[1..].bytes().all(|b| b.is_ascii_digit())
    })
}

/// 去掉模型偶尔包裹的 ```json ... ``` 代码围栏。
fn extract_json_array(content: &str) -> String {
    let t = content.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```JSON"))
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_handles_v1_variants() {
        assert_eq!(
            chat_endpoint("http://localhost:1234"),
            "http://localhost:1234/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("http://localhost:1234/v1"),
            "http://localhost:1234/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("http://localhost:1234/v1/"),
            "http://localhost:1234/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("https://x/v1/chat/completions"),
            "https://x/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint(""),
            "https://api.openai.com/v1/chat/completions"
        );
        // 智谱 /api/paas/v4:已含版本段,只补 /chat/completions。
        assert_eq!(
            chat_endpoint("https://open.bigmodel.cn/api/paas/v4"),
            "https://open.bigmodel.cn/api/paas/v4/chat/completions"
        );
        // 硅基流动 /v1。
        assert_eq!(
            chat_endpoint("https://api.siliconflow.cn/v1"),
            "https://api.siliconflow.cn/v1/chat/completions"
        );
        // 裸 host 名含 v 但非版本段,不应误判。
        assert_eq!(
            chat_endpoint("https://api.openai.com"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn extract_strips_code_fences() {
        assert_eq!(extract_json_array("```json\n[\"a\"]\n```"), "[\"a\"]");
        assert_eq!(extract_json_array("[\"a\",\"b\"]"), "[\"a\",\"b\"]");
        assert_eq!(extract_json_array("```\n[\"x\"]```"), "[\"x\"]");
    }
}
