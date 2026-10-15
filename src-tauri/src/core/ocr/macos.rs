//! macOS Vision OCR 实现(spec 03 §3)。
//!
//! 用 objc2-vision 调 `VNRecognizeTextRequest`,在 `spawn_blocking` 中同步执行。
//!
//! objc2 的 ObjC 对象(`Retained<T>`)不是 `Send`,因此整个 Vision 调用都被
//! 关进 `tokio::task::spawn_blocking` 的闭包里:闭包只捕获纯 Rust 数据
//! (`Vec<u8>` 与 `OcrOptions`),在内部创建/使用/销毁全部 ObjC 对象,只把
//! 纯 Rust 的 `Vec<OcrBlock>` 返回出来。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use objc2::rc::{autoreleasepool, Retained};
use objc2::{AnyThread, ClassType}; // 提供 alloc() / class()。
use objc2_core_foundation::CFData;
use objc2_core_graphics::{CGColorRenderingIntent, CGDataProvider, CGImage};
use objc2_core_ml::MLCPUComputeDevice;
use objc2_foundation::{NSArray, NSDictionary, NSObjectProtocol, NSString};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation, VNRequest,
    VNRequestTextRecognitionLevel,
};

use super::{
    map_normalized_bbox, OcrBlock, OcrEngine, OcrInput, OcrOptions, OcrResult,
    OCR_INITIALIZATION_TIMEOUT, OCR_TIMEOUT,
};
use crate::error::LenzError;

pub fn auto_detect_supported() -> bool {
    objc2_foundation::NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion
        >= 13
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct RecognitionProfile {
    languages: Vec<String>,
    fast: bool,
    auto_detect: bool,
}

impl From<&OcrOptions> for RecognitionProfile {
    fn from(opts: &OcrOptions) -> Self {
        Self {
            languages: opts.preferred_languages.clone(),
            fast: opts.fast_mode,
            auto_detect: opts.auto_detect_language,
        }
    }
}

pub struct VisionOcr {
    initialized: Arc<Mutex<HashSet<RecognitionProfile>>>,
    serial: Arc<tokio::sync::Semaphore>,
}

impl VisionOcr {
    pub fn new() -> Self {
        Self {
            initialized: Arc::default(),
            serial: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    async fn run_native<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, LenzError> + Send + 'static,
    ) -> Result<T, LenzError> {
        let permit = self
            .serial
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| LenzError::internal(format!("Vision queue: {e}")))?;
        tokio::task::spawn_blocking(move || {
            // 原生调用启动后无法强制中止;即使外层 future 被取消,仍持有并发槽到结束。
            let _permit = permit;
            work()
        })
        .await
        .map_err(|e| LenzError::OcrFailed(format!("spawn_blocking join error: {e}")))?
    }
}

#[async_trait]
impl OcrEngine for VisionOcr {
    fn name(&self) -> &'static str {
        "macos-vision"
    }

    fn recognition_timeout(&self, opts: &OcrOptions) -> Duration {
        if self
            .initialized
            .lock()
            .map(|profiles| profiles.contains(&RecognitionProfile::from(opts)))
            .unwrap_or(false)
        {
            OCR_TIMEOUT
        } else {
            OCR_INITIALIZATION_TIMEOUT
        }
    }

    fn supported_languages(&self) -> Vec<String> {
        // 调用 Vision 取当前支持的识别语言;失败则回退到常见 BCP-47 列表。
        // 注意:该调用同样涉及非 Send 的 ObjC 对象,但 `supported_languages`
        // 是同步签名,这里直接在当前线程构造并销毁对象即可(无跨线程传递)。
        match vision_supported_languages() {
            Ok(langs) if !langs.is_empty() => langs,
            // TODO(M2): 某些系统/版本下取语言可能失败,先用常见语言兜底。
            _ => vec![
                "en-US".into(),
                "zh-Hans".into(),
                "ja-JP".into(),
                "ko-KR".into(),
            ],
        }
    }

    async fn recognize(&self, input: OcrInput, opts: OcrOptions) -> Result<OcrResult, LenzError> {
        let started = Instant::now();

        // 截图物理像素尺寸:直接用 pixel_rect 的宽高(spec 提示:它就是截图
        // 的物理像素尺寸),无需再解码 PNG 拿尺寸。
        let img_w = input.pixel_rect.w as f64;
        let img_h = input.pixel_rect.h as f64;
        let pixel_rect = input.pixel_rect;
        let scale = input.scale_factor;
        let png = input.png;

        // 闭包只捕获纯 Rust 数据(png / opts / 尺寸 / 回映射参数),
        // 内部完成全部 ObjC 交互,返回纯 Rust 的 Vec<OcrBlock>。
        let profile = RecognitionProfile::from(&opts);
        let initialized = self.initialized.clone();
        let blocks = self
            .run_native(move || {
                let native_started = Instant::now();
                // Tokio 阻塞线程没有 Cocoa run loop 自动建立的 autorelease pool。Vision
                // 内部会生成 autoreleased 对象,必须在同一线程及时回收。
                let result =
                    autoreleasepool(|_| run_vision(&png, &opts, img_w, img_h, &pixel_rect, scale));
                if result.is_ok() {
                    if let Ok(mut profiles) = initialized.lock() {
                        profiles.insert(profile);
                    }
                }
                log::info!(
                    "Vision native finished: elapsed_ms={} success={}",
                    native_started.elapsed().as_millis(),
                    result.is_ok()
                );
                result
            })
            .await?;

        Ok(OcrResult {
            blocks,
            // Vision 不直接暴露逐块语言,这里先不填(M2 可选增强)。
            detected_language: None,
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// 在阻塞线程内执行一次完整的 Vision 文本识别。
///
/// 所有 ObjC 对象(`Retained<T>`)的生命周期都局限在本函数内,不跨线程传递。
fn run_vision(
    png: &[u8],
    opts: &OcrOptions,
    img_w: f64,
    img_h: f64,
    pixel_rect: &crate::types::Rect,
    scale: f32,
) -> Result<Vec<OcrBlock>, LenzError> {
    // 1) 先用 CoreGraphics 明确解码 PNG。直接把 NSData 交给 Vision 在部分 macOS
    // 版本会让 performRequests 返回 false 且不提供 NSError。
    let data = unsafe { CFData::new(None, png.as_ptr(), png.len() as isize) }
        .ok_or_else(|| LenzError::OcrFailed("create CFData failed".into()))?;
    let provider = CGDataProvider::with_cf_data(Some(&data))
        .ok_or_else(|| LenzError::OcrFailed("create CGDataProvider failed".into()))?;
    let image = unsafe {
        CGImage::with_png_data_provider(
            Some(&provider),
            std::ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
    .ok_or_else(|| LenzError::OcrFailed("decode PNG failed".into()))?;

    // 2) 创建图像请求句柄:initWithCGImage:options:,options 传一个空字典。
    //    NSDictionary::new() 返回类型化的空字典,泛型由目标方法签名推断
    //    (VNImageOption -> AnyObject)。
    let options: Retained<NSDictionary<_, _>> = NSDictionary::new();
    let handler = unsafe {
        VNImageRequestHandler::initWithCGImage_options(
            VNImageRequestHandler::alloc(),
            &image,
            &options,
        )
    };

    // 3) 创建并配置 VNRecognizeTextRequest。
    let request: Retained<VNRecognizeTextRequest> = VNRecognizeTextRequest::new();

    // 识别精度:fast_mode 用 .Fast,否则 .Accurate(默认)。
    let level = if opts.fast_mode {
        VNRequestTextRecognitionLevel::Fast
    } else {
        VNRequestTextRecognitionLevel::Accurate
    };
    request.setRecognitionLevel(level);
    // 开启语言纠错(spec 要求恒为 true)。
    request.setUsesLanguageCorrection(true);
    if auto_detect_supported() {
        request.setAutomaticallyDetectsLanguage(opts.auto_detect_language);
    }

    // 若指定了优先语言,设置识别语言数组(BCP-47/ISO 语言码)。
    if !opts.preferred_languages.is_empty() {
        let langs: Vec<Retained<NSString>> = opts
            .preferred_languages
            .iter()
            .map(|s| NSString::from_str(s))
            .collect();
        let lang_array = NSArray::from_retained_slice(&langs);
        request.setRecognitionLanguages(&lang_array);
    }

    configure_compute_device(&request)?;

    // 4) 执行请求:把 request 作为 VNRequest 放进 NSArray<VNRequest>。
    //    VNRecognizeTextRequest 是 VNRequest 的子类,这里向上转型为 VNRequest。
    let request_as_vn: &VNRequest = &request;
    let requests = NSArray::from_slice(&[request_as_vn]);
    handler
        .performRequests_error(&requests)
        .map_err(|e| LenzError::OcrFailed(format!("performRequests failed: {e}")))?;

    // 5) 遍历识别结果(VNRecognizedTextObservation 列表)。
    let results: Retained<NSArray<VNRecognizedTextObservation>> = match request.results() {
        Some(r) => r,
        // 无 results 视为「识别到 0 块文本」,返回空。
        None => return Ok(Vec::new()),
    };

    let mut blocks: Vec<OcrBlock> = Vec::with_capacity(results.len());
    for observation in results.iter() {
        // 取置信度最高的候选(topCandidates(1))。
        let candidates = observation.topCandidates(1);
        let Some(best) = candidates.iter().next() else {
            continue;
        };

        let confidence = best.confidence();
        // 过滤低置信度文本块。
        if confidence < opts.min_confidence {
            continue;
        }

        let text = best.string().to_string();

        // 归一化 bbox(原点左下);用 mod.rs 提供的 helper 回映射。
        // SAFETY: observation 在本次遍历期间存活;boundingBox 只读取属性。
        let rect = unsafe { observation.boundingBox() };
        let bbox = map_normalized_bbox(
            (
                rect.origin.x,
                rect.origin.y,
                rect.size.width,
                rect.size.height,
            ),
            (img_w, img_h),
            pixel_rect,
            scale,
        );

        blocks.push(OcrBlock {
            text,
            bbox,
            confidence,
        });
    }

    Ok(blocks)
}

/// GUI 进程中自动选择的加速设备可能卡在 TextRecognition 的同步等待。
/// 为支持的阶段显式选 CPU,避免不可取消的硬件推理阻塞整个 OCR 队列。
fn configure_compute_device(request: &VNRecognizeTextRequest) -> Result<(), LenzError> {
    let major = objc2_foundation::NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion;
    if major >= 14 {
        // SAFETY: macOS 14+ 提供此 API;设备来自该请求自己的支持列表。
        let stages = unsafe { request.supportedComputeStageDevicesAndReturnError() }
            .map_err(|e| LenzError::OcrFailed(format!("Vision compute devices: {e}")))?;
        let mut assigned = 0;
        for stage in stages.allKeys() {
            if let Some(devices) = stages.objectForKey(&stage) {
                if let Some(cpu) = devices
                    .iter()
                    .find(|device| device.isKindOfClass(MLCPUComputeDevice::class()))
                {
                    unsafe {
                        request.setComputeDevice_forComputeStage(Some(&cpu), &stage);
                    }
                    assigned += 1;
                }
            }
        }
        if assigned == 0 {
            return Err(LenzError::OcrFailed(
                "Vision CPU compute device is unavailable".into(),
            ));
        }
        log::info!("Vision compute device: CPU stages={assigned}");
    } else {
        // macOS 12/13 尚无逐阶段设备 API。
        #[allow(deprecated)]
        unsafe {
            request.setUsesCPUOnly(true);
        }
    }
    Ok(())
}

/// 取 Vision 当前支持的识别语言列表(BCP-47/ISO)。
fn vision_supported_languages() -> Result<Vec<String>, LenzError> {
    let request = VNRecognizeTextRequest::new();
    // SAFETY: request 是刚创建的有效对象;该方法只读取其支持的语言集合,
    // 失败时返回 NSError(已转为 Result)。
    let langs = unsafe { request.supportedRecognitionLanguagesAndReturnError() }
        .map_err(|e| LenzError::OcrUnavailable(format!("supportedRecognitionLanguages: {e}")))?;
    Ok(langs.iter().map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ocr::{OcrInput, OcrOptions};
    use crate::types::Rect;

    fn test_options() -> OcrOptions {
        OcrOptions {
            preferred_languages: vec!["en-US".into(), "zh-Hans".into()],
            fast_mode: false,
            auto_detect_language: true,
            min_confidence: 0.3,
        }
    }

    #[test]
    fn language_and_mode_changes_receive_their_own_initialization_budget() {
        let engine = VisionOcr::new();
        let mut opts = test_options();
        assert_eq!(
            engine.recognition_timeout(&opts),
            OCR_INITIALIZATION_TIMEOUT
        );
        engine
            .initialized
            .lock()
            .unwrap()
            .insert(RecognitionProfile::from(&opts));
        assert_eq!(engine.recognition_timeout(&opts), OCR_TIMEOUT);
        opts.min_confidence = 0.8; // 置信度只在识别后过滤,不重新加载模型。
        assert_eq!(engine.recognition_timeout(&opts), OCR_TIMEOUT);
        opts.fast_mode = true;
        assert_eq!(
            engine.recognition_timeout(&opts),
            OCR_INITIALIZATION_TIMEOUT
        );
        opts.fast_mode = false;
        opts.preferred_languages.push("ja-JP".into());
        assert_eq!(
            engine.recognition_timeout(&opts),
            OCR_INITIALIZATION_TIMEOUT
        );
        opts.preferred_languages.pop();
        opts.auto_detect_language = false;
        assert_eq!(
            engine.recognition_timeout(&opts),
            OCR_INITIALIZATION_TIMEOUT
        );
    }

    #[tokio::test]
    async fn cancelling_native_waiter_keeps_vision_serial_until_native_work_finishes() {
        let engine = Arc::new(VisionOcr::new());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let first = {
            let engine = engine.clone();
            tokio::spawn(async move {
                engine
                    .run_native(move || {
                        let _ = started_tx.send(());
                        let _ = finish_rx.recv();
                        Ok(())
                    })
                    .await
            })
        };
        started_rx.await.unwrap();
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        assert_eq!(engine.serial.available_permits(), 0);
        let second = {
            let engine = engine.clone();
            tokio::spawn(async move { engine.run_native(|| Ok(42)).await })
        };
        finish_tx.send(()).unwrap();
        assert_eq!(second.await.unwrap().unwrap(), 42);
        assert_eq!(engine.serial.available_permits(), 1);
    }

    /// 候选系统字体路径(挑第一个存在的)。
    const FONT_CANDIDATES: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
    ];

    fn load_font_bytes() -> Option<Vec<u8>> {
        for path in FONT_CANDIDATES {
            if let Ok(bytes) = std::fs::read(path) {
                eprintln!("[vision-test] using font: {path}");
                return Some(bytes);
            }
        }
        None
    }

    /// 在白底图上渲染高对比度黑字,编码为 PNG 字节。
    fn render_text_png(text: &str, w: u32, h: u32, font_size: f32) -> Option<Vec<u8>> {
        use ab_glyph::FontRef;
        use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
        use imageproc::drawing::draw_text_mut;

        let font_bytes = load_font_bytes()?;
        let font = FontRef::try_from_slice(&font_bytes).ok()?;

        // 白底。
        let mut img: RgbImage = RgbImage::from_pixel(w, h, Rgb([255, 255, 255]));
        let scale = ab_glyph::PxScale::from(font_size);
        // 黑字,留出左/上边距。
        draw_text_mut(&mut img, Rgb([0, 0, 0]), 24, 60, scale, &font, text);

        let mut png: Vec<u8> = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(img.as_raw(), w, h, ExtendedColorType::Rgb8)
            .ok()?;
        Some(png)
    }

    #[test]
    #[ignore = "requires macOS Vision system service; run outside a process sandbox"]
    fn vision_recognizes_rendered_text() {
        let (w, h) = (640u32, 220u32);
        let text = "Hello Vision 2026";

        let png = match render_text_png(text, w, h, 64.0) {
            Some(p) => p,
            None => {
                eprintln!(
                    "[vision-test] no usable system font found; skipping assertions (no panic)"
                );
                return;
            }
        };

        let input = OcrInput {
            png,
            pixel_rect: Rect {
                x: 0,
                y: 0,
                w: w as i32,
                h: h as i32,
            },
            scale_factor: 1.0,
        };
        let opts = test_options();

        // recognize 是 async;在测试里用一个临时 tokio 运行时跑它。
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let engine = VisionOcr::new();
        assert_eq!(
            engine.recognition_timeout(&opts),
            OCR_INITIALIZATION_TIMEOUT
        );
        let result = rt
            .block_on(super::super::recognize_with_timeout(
                &engine,
                input.clone(),
                opts.clone(),
            ))
            .expect("vision recognize should succeed");

        eprintln!(
            "[vision-test] cost_ms={} blocks={}",
            result.cost_ms,
            result.blocks.len()
        );
        for (i, b) in result.blocks.iter().enumerate() {
            eprintln!(
                "[vision-test] block[{i}] text={:?} bbox=({},{},{},{}) conf={:.4}",
                b.text, b.bbox.x, b.bbox.y, b.bbox.w, b.bbox.h, b.confidence
            );
        }

        // 拼接全部 block 文本,归一化(去空格 + 小写)后断言。
        let joined: String = result
            .blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        eprintln!("[vision-test] normalized joined text = {joined:?}");

        assert!(
            joined.contains("hello"),
            "recognized text should contain 'hello', got: {joined:?}"
        );
        assert!(
            joined.contains("vision"),
            "recognized text should contain 'vision', got: {joined:?}"
        );
        assert_eq!(engine.recognition_timeout(&opts), OCR_TIMEOUT);
        let warm = rt
            .block_on(super::super::recognize_with_timeout(&engine, input, opts))
            .expect("warm Vision recognize should succeed within the normal budget");
        assert!(warm
            .blocks
            .iter()
            .any(|block| block.text.to_lowercase().contains("hello")));
        eprintln!("[vision-test] warm_cost_ms={}", warm.cost_ms);
    }
}
