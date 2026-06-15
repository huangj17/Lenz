//! Windows.Media.Ocr 实现(spec 03 §4)。
// TODO(M2): 接入 windows crate(Graphics.Imaging / Media.Ocr / Storage.Streams):
//   PNG → SoftwareBitmap → OcrEngine::TryCreateFromLanguage(+UserProfile fallback)
//   → RecognizeAsync → 遍历 Lines(bbox = Word.BoundingRect 并集,置信度填 0.9)。
//   无语言包 → emit ocr://language-missing。
//   在 Windows 机器上验证(本地 macOS 无法编译/运行 windows target)。

use async_trait::async_trait;

use super::{OcrEngine, OcrInput, OcrOptions, OcrResult};
use crate::error::LenzError;

pub struct WinMediaOcr;

impl WinMediaOcr {
    pub fn new() -> Self {
        WinMediaOcr
    }
}

#[async_trait]
impl OcrEngine for WinMediaOcr {
    fn name(&self) -> &'static str {
        "win-media-ocr"
    }

    fn supported_languages(&self) -> Vec<String> {
        Vec::new()
    }

    async fn recognize(
        &self,
        _input: OcrInput,
        _opts: OcrOptions,
    ) -> Result<OcrResult, LenzError> {
        Err(LenzError::OcrUnavailable("win-media-ocr not implemented yet".into()))
    }
}
