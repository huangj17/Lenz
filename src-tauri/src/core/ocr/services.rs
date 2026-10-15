//! 从每次请求的配置快照解析 OCR 服务,切换无需重启,系统 Vision 实例始终复用。

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::remote::{BaiduOcr, VisionOcr};
use super::tencent::TencentOcr;
use super::{recognize_with_timeout, OcrEngine, OcrInput, OcrOptions};
use crate::config::schema::{OcrConfig, OcrService, OcrServiceKind};
use crate::error::LenzError;
use crate::types::Rect;

#[derive(Clone, Copy)]
pub enum OcrPurpose {
    Screenshot,
    Overlay,
}

pub fn resolve_engine(
    config: &OcrConfig,
    system: Arc<dyn OcrEngine>,
    purpose: OcrPurpose,
) -> Result<Arc<dyn OcrEngine>, LenzError> {
    let service = config
        .services
        .iter()
        .find(|s| s.id == config.default_service_id)
        .ok_or_else(|| LenzError::Config("ocr_service_not_found".into()))?;
    // 视觉模型只输出文字,覆盖翻译明确回退本机系统 OCR,不上传图片给模型。
    if matches!(purpose, OcrPurpose::Overlay) && service.kind == OcrServiceKind::Vision {
        return Ok(system);
    }
    build_service(service, system)
}

fn build_service(
    service: &OcrService,
    system: Arc<dyn OcrEngine>,
) -> Result<Arc<dyn OcrEngine>, LenzError> {
    if !(1_000..=300_000).contains(&service.timeout_ms) {
        return Err(LenzError::Config("ocr_invalid_timeout".into()));
    }
    match service.kind {
        OcrServiceKind::System => Ok(system),
        OcrServiceKind::Baidu => Ok(Arc::new(BaiduOcr::new(service.clone())?)),
        OcrServiceKind::Tencent => Ok(Arc::new(TencentOcr::new(service.clone())?)),
        OcrServiceKind::Vision => Ok(Arc::new(VisionOcr::new(service.clone())?)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TestOcrResult {
    pub success: bool,
    pub text: String,
    pub cost_ms: u32,
    pub error: Option<String>,
}

/// 识别随应用附带的固定样例,无需截图权限,不读取用户屏幕。
pub async fn test_service(
    service: &OcrService,
    config: &OcrConfig,
    system: Arc<dyn OcrEngine>,
) -> TestOcrResult {
    let started = Instant::now();
    let result = async {
        let engine = build_service(service, system)?;
        recognize_with_timeout(
            engine.as_ref(),
            OcrInput {
                png: include_bytes!("../../../resources/OcrSmoke.png").to_vec(),
                pixel_rect: Rect {
                    x: 0,
                    y: 0,
                    w: 1024,
                    h: 220,
                },
                scale_factor: 1.0,
            },
            OcrOptions {
                preferred_languages: config.preferred_languages.clone(),
                fast_mode: config.fast_mode,
                auto_detect_language: config.auto_detect_language,
                min_confidence: config.min_confidence,
            },
        )
        .await
    }
    .await;
    let (success, text, error) = match result {
        Ok(result) => {
            let text = result.text.unwrap_or_else(|| {
                result
                    .blocks
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            let compact: String = text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_uppercase();
            let success = compact.contains("LENZ") && compact.contains("12345");
            (
                success,
                text,
                (!success).then(|| "ocr_test_mismatch".into()),
            )
        }
        Err(LenzError::Config(m) | LenzError::OcrFailed(m)) => (false, String::new(), Some(m)),
        Err(LenzError::Timeout(_)) => (false, String::new(), Some("ocr_timeout".into())),
        Err(LenzError::Network(_)) => (false, String::new(), Some("ocr_network".into())),
        Err(LenzError::OcrIdentityRequired(_)) => {
            (false, String::new(), Some("ocr_package_required".into()))
        }
        Err(LenzError::OcrUnavailable(_)) => (false, String::new(), Some("ocr_unavailable".into())),
        Err(_) => (false, String::new(), Some("ocr_test_failed".into())),
    };
    TestOcrResult {
        success,
        text,
        cost_ms: started.elapsed().as_millis() as u32,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{schema::validate_ocr_config, Config};

    #[test]
    fn tencent_is_used_for_both_purposes_and_never_falls_back_on_invalid_credentials() {
        let mut config = Config::default().ocr;
        let mut service = config.services[0].clone();
        service.id = "tencent".into();
        service.kind = OcrServiceKind::Tencent;
        service.api_key = "fixture-only".into();
        service.secret_key = "fixture-secret".into();
        config.services.push(service);
        config.default_service_id = "tencent".into();
        let system = super::super::default_engine();
        for purpose in [OcrPurpose::Screenshot, OcrPurpose::Overlay] {
            let engine = resolve_engine(&config, system.clone(), purpose).unwrap();
            assert_eq!(engine.name(), "tencent-ocr");
            assert!(!Arc::ptr_eq(&engine, &system));
            assert_eq!(
                engine.recognition_timeout(&OcrOptions {
                    preferred_languages: Vec::new(),
                    fast_mode: false,
                    auto_detect_language: true,
                    min_confidence: 0.3,
                }),
                std::time::Duration::from_secs(30)
            );
        }
        config.services[1].secret_key.clear();
        assert!(resolve_engine(&config, system, OcrPurpose::Overlay).is_err());
    }

    #[tokio::test]
    async fn vision_is_only_used_for_screenshots_and_never_for_overlay() {
        let mut config = Config::default().ocr;
        let mut vision = config.services[0].clone();
        vision.id = "vision".into();
        vision.kind = OcrServiceKind::Vision;
        vision.api_url = "http://localhost:1234/v1".into();
        vision.model = "fixture-vision".into();
        config.services.push(vision);
        config.default_service_id = "vision".into();
        let system = super::super::default_engine();
        let overlay = resolve_engine(&config, system.clone(), OcrPurpose::Overlay).unwrap();
        assert!(Arc::ptr_eq(&system, &overlay));
        assert_eq!(
            resolve_engine(&config, system.clone(), OcrPurpose::Screenshot)
                .unwrap()
                .name(),
            "vision-model"
        );
        config.default_service_id = "system".into();
        assert!(Arc::ptr_eq(
            &system,
            &resolve_engine(&config, system.clone(), OcrPurpose::Screenshot).unwrap()
        ));
        config.default_service_id = "deleted".into();
        assert!(resolve_engine(&config, system, OcrPurpose::Screenshot).is_err());
    }

    #[test]
    fn config_rejects_duplicate_missing_or_spoofed_system_services() {
        let original = Config::default().ocr;
        validate_ocr_config(&original).unwrap();
        let mut duplicate = original.clone();
        duplicate.services.push(duplicate.services[0].clone());
        assert!(validate_ocr_config(&duplicate).is_err());
        let mut missing = original.clone();
        missing.services.clear();
        assert!(validate_ocr_config(&missing).is_err());
        let mut spoof = original;
        spoof.services[0].kind = OcrServiceKind::Vision;
        assert!(validate_ocr_config(&spoof).is_err());
    }
}
