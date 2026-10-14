use super::Translator;
use crate::{
    config::schema::{EngineKind, TranslatorEngine},
    error::LenzError,
    types::{TranslateRequest, TranslateResponse},
};
use async_trait::async_trait;

pub struct AppleTranslator {
    cfg: TranslatorEngine,
}
impl AppleTranslator {
    pub fn new(cfg: TranslatorEngine, _: reqwest::Client) -> Self {
        Self { cfg }
    }
}
pub fn available() -> bool {
    #[cfg(target_os = "macos")]
    {
        static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *AVAILABLE.get_or_init(|| {
            std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().split('.').next()?.parse::<u32>().ok())
                .is_some_and(|v| v >= 15)
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
#[async_trait]
impl Translator for AppleTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Apple
    }
    fn supports_batch(&self) -> bool {
        true
    }
    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let mut result = native_translate(req, self.cfg.timeout_ms).await?;
        result.engine_id = self.cfg.id.clone();
        Ok(result)
    }
}
async fn native_translate(
    req: TranslateRequest,
    timeout_ms: u32,
) -> Result<TranslateResponse, LenzError> {
    #[cfg(target_os = "macos")]
    {
        use std::{os::unix::fs::PermissionsExt, process::Stdio};
        use tokio::io::AsyncWriteExt;
        if !available() {
            return Err(LenzError::TranslateFailed(
                "Apple Translation requires macOS 15+".into(),
            ));
        }
        let started = std::time::Instant::now();
        let dir = std::env::temp_dir().join("Lenz-native-translation");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("translator-{}", env!("CARGO_PKG_VERSION")));
        let binary = include_bytes!(concat!(env!("OUT_DIR"), "/lenz-apple-translate"));
        if std::fs::read(&path).ok().as_deref() != Some(binary.as_slice()) {
            let temp = dir.join(uuid::Uuid::new_v4().to_string());
            std::fs::write(&temp, binary)?;
            std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o700))?;
            std::fs::rename(temp, &path)?;
        }
        let mut child = tokio::process::Command::new(&path)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| LenzError::TranslateFailed(e.to_string()))?;
        let input = serde_json::to_vec(&serde_json::json!({
            "texts": req.texts, "sourceLang": req.source_lang, "targetLang": req.target_lang
        }))?;
        child
            .stdin
            .take()
            .ok_or_else(|| LenzError::internal("translation stdin missing"))?
            .write_all(&input)
            .await
            .map_err(|e| LenzError::TranslateFailed(e.to_string()))?;
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms.max(1) as u64),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| LenzError::Timeout("Apple translation timed out".into()))?
        .map_err(|e| LenzError::TranslateFailed(e.to_string()))?;
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            translations: Vec<String>,
            detected_language: Option<String>,
            error: Option<String>,
        }
        let result: Response = serde_json::from_slice(&output.stdout)
            .map_err(|_| LenzError::TranslateFailed("Apple translation bridge failed".into()))?;
        if let Some(error) = result.error {
            return Err(LenzError::TranslateFailed(error));
        }
        if result.translations.len() != req.texts.len() {
            return Err(LenzError::TranslateFailed(
                "Apple translation returned an invalid batch".into(),
            ));
        }
        Ok(TranslateResponse {
            translations: result.translations,
            detected_source_lang: result.detected_language,
            engine_id: String::new(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (req, timeout_ms);
        Err(LenzError::TranslateFailed(
            "Apple Translation is available on macOS only".into(),
        ))
    }
}
