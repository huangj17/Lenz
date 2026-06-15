//! OCR 抽象层(spec 03)。
//!
//! 把一张位图识别为「文本块 + bbox + 置信度」列表;抽象为 trait,平台实现:
//! macOS Vision(`VNRecognizeTextRequest`)、Windows.Media.Ocr。
//! `default_engine()` 工厂在启动阶段注入 `State<Arc<dyn OcrEngine>>`。

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::LenzError;
use crate::types::Rect;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// OCR 输入:PNG 字节 + 截图在屏幕上的物理像素位置 + 缩放因子(用于坐标回映射)。
#[derive(Debug, Clone)]
pub struct OcrInput {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,
    pub scale_factor: f32,
}

/// OCR 选项(spec 03 §2)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrOptions {
    /// 按优先级排序的识别语言(BCP-47)。
    pub preferred_languages: Vec<String>,
    /// 速度优先(对应 Vision `.fast` / WinOCR 低精度)。
    pub fast_mode: bool,
    /// 过滤低置信度文本块(0..1)。
    pub min_confidence: f32,
}

/// OCR 结果(spec 03 §2)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub blocks: Vec<OcrBlock>,
    pub detected_language: Option<String>,
    pub cost_ms: u32,
}

/// 单个识别文本块。bbox 已回映射为屏幕逻辑坐标。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrBlock {
    pub text: String,
    pub bbox: Rect,
    pub confidence: f32,
}

/// OCR 引擎抽象(spec 03 §2)。
#[async_trait]
pub trait OcrEngine: Send + Sync {
    /// 引擎标识(如 `macos-vision` / `win-media-ocr`)。
    fn name(&self) -> &'static str;
    /// 支持的识别语言(BCP-47)。
    fn supported_languages(&self) -> Vec<String>;
    /// 识别一张位图。
    async fn recognize(
        &self,
        input: OcrInput,
        opts: OcrOptions,
    ) -> Result<OcrResult, LenzError>;
}

/// 返回当前平台默认 OCR 引擎(spec 03 §2.1)。启动阶段构造一次。
pub fn default_engine() -> Arc<dyn OcrEngine> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::VisionOcr::new())
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WinMediaOcr::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Arc::new(UnsupportedOcr)
    }
}

/// 把 Vision 归一化 bbox(原点左下,0..1)回映射为屏幕逻辑坐标(spec 03 §5)。
///
/// - `(nx, ny, nw, nh)`:归一化 bbox;
/// - `(img_w, img_h)`:截图物理像素尺寸;
/// - `pixel_rect`:截图在虚拟屏的物理像素矩形;
/// - `scale`:缩放因子。
pub fn map_normalized_bbox(
    nx: f64,
    ny: f64,
    nw: f64,
    nh: f64,
    img_w: f64,
    img_h: f64,
    pixel_rect: &Rect,
    scale: f32,
) -> Rect {
    // Vision 原点在左下,翻转 Y 到左上原点。
    let flipped_y = 1.0 - ny - nh;
    let phys_x = pixel_rect.x as f64 + nx * img_w;
    let phys_y = pixel_rect.y as f64 + flipped_y * img_h;
    let s = scale as f64;
    Rect {
        x: (phys_x / s).round() as i32,
        y: (phys_y / s).round() as i32,
        w: (nw * img_w / s).round() as i32,
        h: (nh * img_h / s).round() as i32,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
struct UnsupportedOcr;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[async_trait]
impl OcrEngine for UnsupportedOcr {
    fn name(&self) -> &'static str {
        "unsupported"
    }
    fn supported_languages(&self) -> Vec<String> {
        Vec::new()
    }
    async fn recognize(
        &self,
        _input: OcrInput,
        _opts: OcrOptions,
    ) -> Result<OcrResult, LenzError> {
        Err(LenzError::OcrUnavailable("ocr unsupported on this platform".into()))
    }
}
