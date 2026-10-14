//! OCR 相关 Tauri commands(spec 03)。

use std::sync::Arc;

use tauri::State;

use crate::config::ConfigStore;
use crate::core::capture::capture_region;
use crate::core::ocr::{OcrEngine, OcrInput, OcrOptions, OcrResult};
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
    let opts = OcrOptions {
        preferred_languages: cfg.ocr.preferred_languages.clone(),
        fast_mode: cfg.ocr.fast_mode,
        auto_detect_language: cfg.ocr.auto_detect_language,
        min_confidence: cfg.ocr.min_confidence,
    };

    // 截图(阻塞)移出异步 runtime。
    let capture = tokio::task::spawn_blocking(move || capture_region(rect))
        .await
        .map_err(|e| LenzError::Internal(format!("capture join: {e}")))??;

    let input = OcrInput {
        png: capture.png,
        pixel_rect: capture.pixel_rect,
        scale_factor: capture.scale_factor,
    };
    ocr.recognize(input, opts).await
}
