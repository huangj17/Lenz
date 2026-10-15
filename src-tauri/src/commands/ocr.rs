//! OCR 相关 Tauri commands(spec 03)。

use std::sync::Arc;

use tauri::State;

use crate::config::schema::OcrService;
use crate::config::ConfigStore;
use crate::core::capture::capture_region_with_timeout;
use crate::core::ocr::{
    recognize_with_timeout, resolve_engine, test_service, OcrEngine, OcrInput, OcrOptions,
    OcrPurpose, OcrResult, TestOcrResult,
};
use crate::error::LenzError;
use crate::types::Rect;

/// 对屏幕区域执行 OCR:截图 → 识别。
///
/// 供设置页「测试 OCR」与 F2/F3 链路使用;识别选项取自 `config.ocr`。
#[tauri::command]
#[specta::specta]
pub async fn run_ocr(
    rect: Rect,
    config: State<'_, ConfigStore>,
    ocr: State<'_, Arc<dyn OcrEngine>>,
) -> Result<OcrResult, LenzError> {
    let cfg = config.get()?;
    let engine = resolve_engine(&cfg.ocr, ocr.inner().clone(), OcrPurpose::Screenshot)?;
    let opts = OcrOptions {
        preferred_languages: cfg.ocr.preferred_languages.clone(),
        fast_mode: cfg.ocr.fast_mode,
        auto_detect_language: cfg.ocr.auto_detect_language,
        min_confidence: cfg.ocr.min_confidence,
    };

    let capture = capture_region_with_timeout(rect).await?;

    let input = OcrInput {
        png: capture.png,
        pixel_rect: capture.pixel_rect,
        scale_factor: capture.scale_factor,
    };
    recognize_with_timeout(engine.as_ref(), input, opts).await
}

/// 测试传入的服务草稿,只识别内置样例,不会切换默认服务。
#[tauri::command]
#[specta::specta]
pub async fn test_ocr_service(
    service: OcrService,
    config: State<'_, ConfigStore>,
    ocr: State<'_, Arc<dyn OcrEngine>>,
) -> Result<TestOcrResult, LenzError> {
    Ok(test_service(&service, &config.get()?.ocr, ocr.inner().clone()).await)
}
