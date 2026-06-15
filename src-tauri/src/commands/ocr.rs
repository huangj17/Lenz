//! OCR 相关 Tauri commands(spec 03)。M1 仅占位。

use crate::error::LenzError;
use crate::types::Rect;

/// 对给定屏幕区域执行 OCR。
///
/// M1 占位:入参 / 返回暂为 `Rect` / `()`,M2 落地 `OcrInput` / `OcrResult`
/// 等共享类型后替换。
// TODO(M2): 接入 OcrEngine trait(macOS Vision / Windows.Media.Ocr)。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn run_ocr(rect: Rect) -> Result<(), LenzError> {
    Err(LenzError::OcrUnavailable("not_implemented".into()))
}
