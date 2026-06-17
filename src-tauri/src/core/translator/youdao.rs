//! 有道智云翻译引擎(spec 04 §6)。
//!
//! 不支持批量,`supports_batch = false`,内部逐条串行。
//! 凭据在配置里以 `appKey:appSecret` 形式存于 `apiKey`。
//!
//! 注:spec 写「HMAC-SHA256」,但有道智云实际签名为对
//! `appKey + input + salt + curtime + appSecret` 做 **SHA256 摘要**(signType=v3,
//! 非 HMAC),此处按真实接口实现。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::Translator;
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

const DEFAULT_ENDPOINT: &str = "https://openapi.youdao.com/api";

pub struct YoudaoTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl YoudaoTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        YoudaoTranslator { cfg, client }
    }

    /// 拆分 `appKey:appSecret`。
    fn credentials(&self) -> Result<(&str, &str), LenzError> {
        self.cfg
            .api_key
            .split_once(':')
            .filter(|(k, s)| !k.is_empty() && !s.is_empty())
            .ok_or_else(|| LenzError::Config("youdao_credentials".into()))
    }

    async fn translate_one(
        &self,
        text: &str,
        app_key: &str,
        app_secret: &str,
        from: &str,
        to: &str,
    ) -> Result<YoudaoResp, LenzError> {
        let salt = uuid::Uuid::new_v4().to_string();
        let curtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .to_string();
        let sign = sign_v3(app_key, text, &salt, &curtime, app_secret);

        let endpoint = if self.cfg.api_url.trim().is_empty() {
            DEFAULT_ENDPOINT
        } else {
            self.cfg.api_url.trim()
        };

        let params = [
            ("q", text),
            ("from", from),
            ("to", to),
            ("appKey", app_key),
            ("salt", salt.as_str()),
            ("sign", sign.as_str()),
            ("signType", "v3"),
            ("curtime", curtime.as_str()),
        ];

        let rb = self
            .client
            .post(endpoint)
            .timeout(Duration::from_millis(self.cfg.timeout_ms.max(1) as u64))
            .form(&params);

        super::send_json(rb).await
    }
}

#[async_trait]
impl Translator for YoudaoTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Youdao
    }
    fn supports_batch(&self) -> bool {
        false
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();
        let (app_key, app_secret) = self.credentials()?;
        let from = req.source_lang.as_deref().map(youdao_lang_code).unwrap_or("auto");
        let to = youdao_lang_code(&req.target_lang);

        let mut translations = Vec::with_capacity(req.texts.len());
        let mut detected: Option<String> = None;
        for text in &req.texts {
            let resp = self
                .translate_one(text, app_key, app_secret, from, to)
                .await?;
            if resp.error_code != "0" {
                return Err(LenzError::TranslateFailed(format!(
                    "youdao_{}",
                    resp.error_code
                )));
            }
            if detected.is_none() {
                // 形如 "en2zh-CHS",取源语言段。
                detected = resp
                    .l
                    .as_deref()
                    .and_then(|l| l.split('2').next())
                    .map(|s| s.to_string());
            }
            let translated = resp
                .translation
                .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) })
                .ok_or_else(|| LenzError::TranslateFailed("empty_response".into()))?;
            translations.push(translated);
        }

        Ok(TranslateResponse {
            translations,
            detected_source_lang: detected,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// signType=v3 签名:`sha256(appKey + truncate(q) + salt + curtime + appSecret)`。
fn sign_v3(app_key: &str, q: &str, salt: &str, curtime: &str, app_secret: &str) -> String {
    let input = truncate_q(q);
    let raw = format!("{app_key}{input}{salt}{curtime}{app_secret}");
    let digest = Sha256::digest(raw.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// 有道 input 截断规则:`len(q)<=20` 取原文,否则 `前10 + len + 后10`(按字符)。
fn truncate_q(q: &str) -> String {
    let chars: Vec<char> = q.chars().collect();
    let len = chars.len();
    if len <= 20 {
        q.to_string()
    } else {
        let head: String = chars[..10].iter().collect();
        let tail: String = chars[len - 10..].iter().collect();
        format!("{head}{len}{tail}")
    }
}

/// BCP-47 → 有道语言代码(简/繁中文用 `zh-CHS` / `zh-CHT`)。
fn youdao_lang_code(code: &str) -> &'static str {
    match code.trim().to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "zh-hans" => "zh-CHS",
        "zh-tw" | "zh-hant" | "zh-hk" => "zh-CHT",
        "ja" | "ja-jp" => "ja",
        "ko" | "ko-kr" => "ko",
        "fr" | "fr-fr" => "fr",
        "de" | "de-de" => "de",
        "es" | "es-es" => "es",
        "ru" | "ru-ru" => "ru",
        _ => "en",
    }
}

#[derive(Deserialize)]
struct YoudaoResp {
    #[serde(rename = "errorCode")]
    error_code: String,
    translation: Option<Vec<String>>,
    /// 形如 "en2zh-CHS" 的语言方向。
    l: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_keeps_original() {
        assert_eq!(truncate_q("hello"), "hello");
        // 恰好 20 字符不截断。
        let twenty = "a".repeat(20);
        assert_eq!(truncate_q(&twenty), twenty);
    }

    #[test]
    fn truncate_long_uses_head_len_tail() {
        let s = "abcdefghijklmnopqrstuvwxyz"; // 26 chars
        // 前10 "abcdefghij" + 26 + 后10 "qrstuvwxyz"
        assert_eq!(truncate_q(s), "abcdefghij26qrstuvwxyz");
    }

    #[test]
    fn truncate_counts_unicode_chars() {
        let s = "一二三四五六七八九十一二三四五六七八九十一"; // 21 chars
        let out = truncate_q(s);
        assert!(out.contains("21"));
    }

    #[test]
    fn sign_is_stable_hex() {
        let sign = sign_v3("key", "hello", "salt", "1700000000", "secret");
        assert_eq!(sign.len(), 64);
        assert!(sign.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn youdao_codes() {
        assert_eq!(youdao_lang_code("zh-CN"), "zh-CHS");
        assert_eq!(youdao_lang_code("zh-Hant"), "zh-CHT");
        assert_eq!(youdao_lang_code("en-US"), "en");
    }
}
