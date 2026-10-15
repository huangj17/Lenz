//! OCR 抽象层(spec 03)。
//!
//! 把一张位图识别为「文本块 + bbox + 置信度」列表;抽象为 trait,平台实现:
//! macOS Vision(`VNRecognizeTextRequest`)、Windows.Media.Ocr。
//! `default_engine()` 工厂在启动阶段注入 `State<Arc<dyn OcrEngine>>`。

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::LenzError;
use crate::types::Rect;

mod remote;
mod services;
mod tencent;
pub use services::{resolve_engine, test_service, OcrPurpose, TestOcrResult};

pub const OCR_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(any(target_os = "macos", test))]
const OCR_INITIALIZATION_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::auto_detect_supported;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::write_install_diagnostic;
// Tesseract 预留实现位(spec 03 §10 / 2.6 兜底):默认不编译、不引入依赖,
// 通过 `--features ocr-tesseract` 启用。当前 Vision/WinOCR 已满足 MVP。
#[cfg(feature = "ocr-tesseract")]
mod tesseract;

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
    /// 速度优先(对应 Vision `.fast`;Windows 系统 OCR 不支持切换精度)。
    pub fast_mode: bool,
    pub auto_detect_language: bool,
    /// 过滤低置信度文本块(0..1);对返回真实置信度的系统/云端 OCR 生效。
    pub min_confidence: f32,
}

/// OCR 结果(spec 03 §2)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub blocks: Vec<OcrBlock>,
    /// 无坐标的视觉模型直接返回原文,不伪造文本块位置或置信度。
    pub text: Option<String>,
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
// async_trait adds #[must_use] to its generated Future, which Clippy 1.99 flags twice.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait OcrEngine: Send + Sync {
    /// 引擎标识(如 `macos-vision` / `win-media-ocr`)。
    fn name(&self) -> &'static str;
    /// 支持的识别语言(BCP-47)。
    fn supported_languages(&self) -> Vec<String>;
    /// Vision 首次加载一组语言/识别模式时需要更长的初始化预算。
    fn recognition_timeout(&self, _opts: &OcrOptions) -> Duration {
        OCR_TIMEOUT
    }
    /// 识别一张位图。
    async fn recognize(&self, input: OcrInput, opts: OcrOptions) -> Result<OcrResult, LenzError>;
}

/// F2/F3 与命令入口共用阶段超时;截图和 popup 握手不扣减 OCR 预算。
pub async fn recognize_with_timeout(
    engine: &dyn OcrEngine,
    input: OcrInput,
    opts: OcrOptions,
) -> Result<OcrResult, LenzError> {
    let budget = engine.recognition_timeout(&opts);
    let started = Instant::now();
    log::info!(
        "OCR started: engine={} pixel_size={}x{} fast={} auto_detect={} languages={} timeout_ms={}",
        engine.name(),
        input.pixel_rect.w,
        input.pixel_rect.h,
        opts.fast_mode,
        opts.auto_detect_language,
        opts.preferred_languages.len(),
        budget.as_millis()
    );
    let result = tokio::time::timeout(budget, engine.recognize(input, opts))
        .await
        .unwrap_or_else(|_| {
            Err(LenzError::Timeout(format!(
                "OCR timed out after {}s ({})",
                budget.as_secs(),
                engine.name()
            )))
        });
    match &result {
        Ok(ocr) => log::info!(
            "OCR finished: engine={} elapsed_ms={} blocks={}",
            engine.name(),
            started.elapsed().as_millis(),
            ocr.blocks.len()
        ),
        Err(error) => log::warn!(
            "OCR failed: engine={} elapsed_ms={} error={error}",
            engine.name(),
            started.elapsed().as_millis()
        ),
    }
    result
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
/// - `normalized`:归一化 bbox `(x, y, width, height)`;
/// - `image_size`:截图物理像素尺寸 `(width, height)`;
/// - `pixel_rect`:截图在虚拟屏的物理像素矩形;
/// - `scale`:缩放因子。
pub fn map_normalized_bbox(
    normalized: (f64, f64, f64, f64),
    image_size: (f64, f64),
    pixel_rect: &Rect,
    scale: f32,
) -> Rect {
    let (nx, ny, nw, nh) = normalized;
    let (img_w, img_h) = image_size;
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

/// 把绝对像素 bbox(原点左上,相对截图图像)回映射为屏幕逻辑坐标。
///
/// 用于 Windows.Media.Ocr —— 它返回的 `BoundingRect` 已是图像物理像素(无需翻转 Y)。
/// `raw_*` 为图像内像素;`pixel_rect` 为截图在虚拟屏的物理像素矩形;`scale` 为缩放因子。
pub fn map_pixel_bbox(
    raw_x: f64,
    raw_y: f64,
    raw_w: f64,
    raw_h: f64,
    pixel_rect: &Rect,
    scale: f32,
) -> Rect {
    let s = scale as f64;
    Rect {
        x: ((pixel_rect.x as f64 + raw_x) / s).round() as i32,
        y: ((pixel_rect.y as f64 + raw_y) / s).round() as i32,
        w: (raw_w / s).round() as i32,
        h: (raw_h / s).round() as i32,
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
    async fn recognize(&self, _input: OcrInput, _opts: OcrOptions) -> Result<OcrResult, LenzError> {
        Err(LenzError::OcrUnavailable(
            "ocr unsupported on this platform".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SimulatedOcr {
        ready: AtomicBool,
        never_finishes: bool,
    }

    #[async_trait]
    impl OcrEngine for SimulatedOcr {
        fn name(&self) -> &'static str {
            "simulated-ocr"
        }
        fn supported_languages(&self) -> Vec<String> {
            Vec::new()
        }
        fn recognition_timeout(&self, _opts: &OcrOptions) -> Duration {
            if self.ready.load(Ordering::SeqCst) {
                OCR_TIMEOUT
            } else {
                OCR_INITIALIZATION_TIMEOUT
            }
        }
        async fn recognize(
            &self,
            _input: OcrInput,
            _opts: OcrOptions,
        ) -> Result<OcrResult, LenzError> {
            if self.never_finishes {
                return std::future::pending().await;
            }
            let delay = if self.ready.load(Ordering::SeqCst) {
                Duration::from_millis(500)
            } else {
                Duration::from_secs(26)
            };
            tokio::time::sleep(delay).await;
            self.ready.store(true, Ordering::SeqCst);
            Ok(OcrResult {
                blocks: Vec::new(),
                text: None,
                detected_language: None,
                cost_ms: delay.as_millis() as u32,
            })
        }
    }

    fn test_input() -> (OcrInput, OcrOptions) {
        (
            OcrInput {
                png: Vec::new(),
                pixel_rect: Rect {
                    x: 0,
                    y: 0,
                    w: 640,
                    h: 220,
                },
                scale_factor: 1.0,
            },
            OcrOptions {
                preferred_languages: vec!["en-US".into()],
                fast_mode: false,
                auto_detect_language: true,
                min_confidence: 0.3,
            },
        )
    }

    #[tokio::test(start_paused = true)]
    async fn cold_ocr_can_finish_after_old_shared_deadline_then_uses_warm_budget() {
        let engine = SimulatedOcr {
            ready: AtomicBool::new(false),
            never_finishes: false,
        };
        // 模拟截图及 popup 握手已用去 9 秒,它们不能扣减识别预算。
        tokio::time::sleep(Duration::from_secs(9)).await;
        let (input, opts) = test_input();
        let result = recognize_with_timeout(&engine, input, opts).await.unwrap();
        assert_eq!(result.cost_ms, 26_000);
        let (input, opts) = test_input();
        assert_eq!(engine.recognition_timeout(&opts), OCR_TIMEOUT);
        assert_eq!(
            recognize_with_timeout(&engine, input, opts)
                .await
                .unwrap()
                .cost_ms,
            500
        );
    }

    #[tokio::test(start_paused = true)]
    async fn hung_ocr_still_times_out_with_its_stage_and_budget() {
        for ready in [false, true] {
            let engine = SimulatedOcr {
                ready: AtomicBool::new(ready),
                never_finishes: true,
            };
            let (input, opts) = test_input();
            let budget = engine.recognition_timeout(&opts);
            let started = tokio::time::Instant::now();
            let error = recognize_with_timeout(&engine, input, opts)
                .await
                .unwrap_err();
            assert_eq!(started.elapsed(), budget);
            assert!(
                matches!(error, LenzError::Timeout(message) if message.contains("OCR timed out"))
            );
        }
    }

    /// 整图 + 2× 缩放:物理 200×100 → 逻辑 100×50,原点不变。
    #[test]
    fn maps_full_image_with_scale() {
        let r = map_normalized_bbox(
            (0.0, 0.0, 1.0, 1.0),
            (200.0, 100.0),
            &Rect {
                x: 0,
                y: 0,
                w: 200,
                h: 100,
            },
            2.0,
        );
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 100, 50));
    }

    /// Vision 原点在左下:靠近顶部的框(ny=0.8)回映射后 y 应较小(屏幕上方)。
    #[test]
    fn flips_y_origin_bottom_left() {
        let r = map_normalized_bbox(
            (0.1, 0.8, 0.2, 0.1),
            (1000.0, 500.0),
            &Rect {
                x: 0,
                y: 0,
                w: 1000,
                h: 500,
            },
            1.0,
        );
        assert_eq!((r.x, r.y, r.w, r.h), (100, 50, 200, 50));
    }

    /// pixel_rect 物理偏移 (300,600) + 3× → 逻辑原点 (100,200);整图 300×300 → 100×100。
    #[test]
    fn applies_pixel_rect_offset_and_scale_3x() {
        let r = map_normalized_bbox(
            (0.0, 0.0, 1.0, 1.0),
            (300.0, 300.0),
            &Rect {
                x: 300,
                y: 600,
                w: 300,
                h: 300,
            },
            3.0,
        );
        assert_eq!((r.x, r.y, r.w, r.h), (100, 200, 100, 100));
    }

    /// Windows 绝对像素 bbox(左上原点,无翻转):pixel_rect 偏移 + 2× 缩放。
    #[test]
    fn maps_absolute_pixel_bbox() {
        let r = map_pixel_bbox(
            10.0,
            20.0,
            100.0,
            40.0,
            &Rect {
                x: 200,
                y: 0,
                w: 800,
                h: 600,
            },
            2.0,
        );
        // (200+10)/2=105, (0+20)/2=10, 100/2=50, 40/2=20
        assert_eq!((r.x, r.y, r.w, r.h), (105, 10, 50, 20));
    }
}
