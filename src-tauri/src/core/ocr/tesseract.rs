//! Tesseract OCR 预留实现位(spec 03 §10 / PRD §7;TODO M2 §2.6 兜底)。
//!
//! 仅当启用 `ocr-tesseract` feature 时编译;默认不构建、不引入 tesseract 依赖。
//! 目的是保留一个与 `OcrEngine` 同构的切换点,便于未来在系统 OCR 不可用
//! (如 Windows 缺语言包)时切换到自带 Tesseract。
#![allow(dead_code)]

use async_trait::async_trait;

use super::{OcrEngine, OcrInput, OcrOptions, OcrResult};
use crate::error::LenzError;

pub struct TesseractOcr;

impl TesseractOcr {
    pub fn new() -> Self {
        TesseractOcr
    }
}

#[async_trait]
impl OcrEngine for TesseractOcr {
    fn name(&self) -> &'static str {
        "tesseract"
    }

    fn supported_languages(&self) -> Vec<String> {
        Vec::new()
    }

    async fn recognize(&self, _input: OcrInput, _opts: OcrOptions) -> Result<OcrResult, LenzError> {
        // TODO(P2): 接入 tesseract / leptonica;按 traineddata 语言包识别。
        Err(LenzError::OcrUnavailable(
            "tesseract engine not built".into(),
        ))
    }
}
