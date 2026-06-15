//! OCR 抽象层(spec 03)。M1 仅 trait 骨架。
//!
//! 平台实现:macOS Vision(`VNRecognizeTextRequest`)、Windows.Media.Ocr。
//! `default_engine()` 工厂在启动阶段注入 `State<Arc<dyn OcrEngine>>`。

use crate::error::LenzError;

/// OCR 引擎抽象(spec 03 §2)。
// TODO(M2): 补全 OcrInput / OcrOptions / OcrResult / OcrBlock 共享类型与方法签名。
pub trait OcrEngine: Send + Sync {
    /// 引擎支持的识别语言(BCP-47)。
    fn supported_languages(&self) -> Result<Vec<String>, LenzError>;
}

/// 返回当前平台默认 OCR 引擎。
// TODO(M2): 按 target_os 返回 Vision / WinOCR 实现;预留 Tesseract 兜底位。
pub fn default_engine() -> Result<(), LenzError> {
    Err(LenzError::OcrUnavailable("not_implemented".into()))
}
