//! Google Cloud Translation v2(Basic)引擎(spec 04 §4)。

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;

use super::{send_json, Translator};
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

const DEFAULT_ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";

pub struct GoogleTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl GoogleTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        GoogleTranslator { cfg, client }
    }
}

#[async_trait]
impl Translator for GoogleTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Google
    }
    fn supports_batch(&self) -> bool {
        true
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();
        let endpoint = if self.cfg.api_url.trim().is_empty() {
            DEFAULT_ENDPOINT
        } else {
            self.cfg.api_url.trim()
        };

        // 一次请求传多个 q;form 编码自动重复 key(spec 04 §4)。
        let mut params: Vec<(&str, String)> = Vec::with_capacity(req.texts.len() + 4);
        for t in &req.texts {
            params.push(("q", t.clone()));
        }
        params.push(("target", google_lang_code(&req.target_lang)));
        if let Some(src) = req.source_lang.as_deref() {
            params.push(("source", google_lang_code(src)));
        }
        params.push(("format", "text".to_string()));
        params.push(("key", self.cfg.api_key.clone()));

        let rb = self
            .client
            .post(endpoint)
            .timeout(Duration::from_millis(self.cfg.timeout_ms.max(1) as u64))
            .form(&params);

        let resp: GoogleResp = send_json(rb).await?;
        if resp.data.translations.len() != req.texts.len() {
            return Err(LenzError::TranslateFailed("length_mismatch".into()));
        }

        let detected = resp
            .data
            .translations
            .first()
            .and_then(|t| t.detected_source_language.clone());
        let translations = resp
            .data
            .translations
            .into_iter()
            .map(|t| html_unescape(&t.translated_text))
            .collect();

        Ok(TranslateResponse {
            translations,
            detected_source_lang: detected,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// BCP-47 → Google 语言代码。中文细分单独处理,其余取主子标签。
fn google_lang_code(code: &str) -> String {
    let c = code.trim().to_ascii_lowercase();
    match c.as_str() {
        "zh" | "zh-cn" | "zh-hans" => "zh-CN".to_string(),
        "zh-tw" | "zh-hant" | "zh-hk" => "zh-TW".to_string(),
        _ => c.split('-').next().unwrap_or(&c).to_string(),
    }
}

/// Google v2 即便 `format=text` 也会 HTML 转义少数实体,这里还原常见几个。
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[derive(Deserialize)]
struct GoogleResp {
    data: GoogleData,
}

#[derive(Deserialize)]
struct GoogleData {
    translations: Vec<GoogleTranslation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleTranslation {
    translated_text: String,
    detected_source_language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_codes() {
        assert_eq!(google_lang_code("zh-CN"), "zh-CN");
        assert_eq!(google_lang_code("zh-Hant"), "zh-TW");
        assert_eq!(google_lang_code("en-US"), "en");
        assert_eq!(google_lang_code("ja"), "ja");
    }

    #[test]
    fn unescape_entities() {
        assert_eq!(html_unescape("a &amp; b &#39;c&#39;"), "a & b 'c'");
    }
}
