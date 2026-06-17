//! 微软 Azure Translator(Text API v3.0)引擎(spec 04 §6.1)。
//!
//! 认证:`Ocp-Apim-Subscription-Key` + `Ocp-Apim-Subscription-Region`(区域资源必填)。
//! 支持单请求批量(JSON 数组)。

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{send_json, Translator};
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

const DEFAULT_HOST: &str = "https://api.cognitive.microsofttranslator.com";

pub struct MicrosoftTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl MicrosoftTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        MicrosoftTranslator { cfg, client }
    }
}

#[async_trait]
impl Translator for MicrosoftTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Microsoft
    }
    fn supports_batch(&self) -> bool {
        true
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();

        // body:[{ "Text": "..." }, ...]
        let body: Vec<_> = req.texts.iter().map(|t| json!({ "Text": t })).collect();

        let mut query: Vec<(&str, String)> =
            vec![("api-version", "3.0".into()), ("to", ms_lang_code(&req.target_lang))];
        if let Some(src) = req.source_lang.as_deref() {
            query.push(("from", ms_lang_code(src)));
        }

        let mut rb = self
            .client
            .post(translate_endpoint(&self.cfg.api_url))
            .timeout(Duration::from_millis(self.cfg.timeout_ms.max(1) as u64))
            .query(&query)
            .header("Ocp-Apim-Subscription-Key", &self.cfg.api_key)
            .json(&body);
        if let Some(region) = self.cfg.region.as_deref().filter(|r| !r.is_empty()) {
            rb = rb.header("Ocp-Apim-Subscription-Region", region);
        }

        let resp: Vec<MsResult> = send_json(rb).await?;
        if resp.len() != req.texts.len() {
            return Err(LenzError::TranslateFailed("length_mismatch".into()));
        }

        let detected = resp
            .first()
            .and_then(|r| r.detected_language.as_ref())
            .map(|d| d.language.clone());
        let translations = resp
            .into_iter()
            .map(|r| {
                r.translations
                    .into_iter()
                    .next()
                    .map(|t| t.text)
                    .unwrap_or_default()
            })
            .collect();

        Ok(TranslateResponse {
            translations,
            detected_source_lang: detected,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// 组装 `/translate` 端点,容忍 apiUrl 是否已含路径。
fn translate_endpoint(api_url: &str) -> String {
    let base = api_url.trim().trim_end_matches('/');
    let base = if base.is_empty() { DEFAULT_HOST } else { base };
    if base.ends_with("/translate") {
        base.to_string()
    } else {
        format!("{base}/translate")
    }
}

/// BCP-47 → Azure 语言代码(中文用 `zh-Hans` / `zh-Hant`,其余取主子标签)。
fn ms_lang_code(code: &str) -> String {
    let c = code.trim().to_ascii_lowercase();
    match c.as_str() {
        "zh" | "zh-cn" | "zh-hans" => "zh-Hans".to_string(),
        "zh-tw" | "zh-hant" | "zh-hk" => "zh-Hant".to_string(),
        _ => c.split('-').next().unwrap_or(&c).to_string(),
    }
}

#[derive(Deserialize)]
struct MsResult {
    translations: Vec<MsTranslation>,
    #[serde(rename = "detectedLanguage")]
    detected_language: Option<MsDetected>,
}

#[derive(Deserialize)]
struct MsTranslation {
    text: String,
}

#[derive(Deserialize)]
struct MsDetected {
    language: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_defaults_and_appends() {
        assert_eq!(
            translate_endpoint(""),
            "https://api.cognitive.microsofttranslator.com/translate"
        );
        assert_eq!(translate_endpoint("https://x.com"), "https://x.com/translate");
        assert_eq!(
            translate_endpoint("https://x.com/translate"),
            "https://x.com/translate"
        );
    }

    #[test]
    fn ms_codes() {
        assert_eq!(ms_lang_code("zh-CN"), "zh-Hans");
        assert_eq!(ms_lang_code("zh-Hant"), "zh-Hant");
        assert_eq!(ms_lang_code("en-US"), "en");
        assert_eq!(ms_lang_code("ja"), "ja");
    }
}
