//! DeepL API(Free / Pro)引擎(spec 04 §5)。

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;

use super::{send_json, Translator};
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

const FREE_ENDPOINT: &str = "https://api-free.deepl.com/v2/translate";
const PRO_ENDPOINT: &str = "https://api.deepl.com/v2/translate";

pub struct DeepLTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl DeepLTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        DeepLTranslator { cfg, client }
    }

    /// 端点解析:显式 apiUrl 优先;否则按 key 后缀 `:fx`(Free 标识)选择。
    fn endpoint(&self) -> &str {
        let url = self.cfg.api_url.trim();
        if !url.is_empty() {
            url
        } else if self.cfg.api_key.ends_with(":fx") {
            FREE_ENDPOINT
        } else {
            PRO_ENDPOINT
        }
    }
}

#[async_trait]
impl Translator for DeepLTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Deepl
    }
    fn supports_batch(&self) -> bool {
        true
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();

        // 多个 text 字段一次请求(spec 04 §5)。
        let mut params: Vec<(&str, String)> = Vec::with_capacity(req.texts.len() + 2);
        for t in &req.texts {
            params.push(("text", t.clone()));
        }
        params.push(("target_lang", deepl_lang_code(&req.target_lang)));
        if let Some(src) = req.source_lang.as_deref() {
            params.push(("source_lang", deepl_lang_code(src)));
        }

        let rb = self
            .client
            .post(self.endpoint())
            .timeout(Duration::from_millis(self.cfg.timeout_ms.max(1) as u64))
            .header(
                "Authorization",
                format!("DeepL-Auth-Key {}", self.cfg.api_key),
            )
            .form(&params);

        let resp: DeepLResp = send_json(rb).await?;
        if resp.translations.len() != req.texts.len() {
            return Err(LenzError::TranslateFailed("length_mismatch".into()));
        }

        let detected = resp
            .translations
            .first()
            .and_then(|t| t.detected_source_language.clone());
        let translations = resp.translations.into_iter().map(|t| t.text).collect();

        Ok(TranslateResponse {
            translations,
            detected_source_lang: detected,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// BCP-47 → DeepL 语言代码(大写;中文统一 `ZH`,葡语区分变体)。
fn deepl_lang_code(code: &str) -> String {
    let c = code.trim().to_ascii_lowercase();
    match c.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-tw" | "zh-hant" => "ZH".to_string(),
        "pt-br" => "PT-BR".to_string(),
        "pt" | "pt-pt" => "PT-PT".to_string(),
        _ => c.split('-').next().unwrap_or(&c).to_ascii_uppercase(),
    }
}

#[derive(Deserialize)]
struct DeepLResp {
    translations: Vec<DeepLTranslation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeepLTranslation {
    text: String,
    detected_source_language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepl_codes() {
        assert_eq!(deepl_lang_code("zh-CN"), "ZH");
        assert_eq!(deepl_lang_code("en-US"), "EN");
        assert_eq!(deepl_lang_code("pt-BR"), "PT-BR");
        assert_eq!(deepl_lang_code("de"), "DE");
    }
}
