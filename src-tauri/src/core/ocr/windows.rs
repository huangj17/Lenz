//! Windows.Media.Ocr 实现(spec 03 §4)。
//!
//! 流程:PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `SoftwareBitmap`
//! → `OcrEngine`(按语言尝试 + 用户配置回退)→ `RecognizeAsync` → 遍历 Lines。
//!
//! WinRT 异步操作在 `spawn_blocking` 内用 `.join()` 阻塞等待;所有 COM 对象的
//! 生命周期都局限在闭包内,只返回纯 Rust `Vec<OcrBlock>`。
//!
//! 此模块已通过 Windows GNU 目标交叉编译;语言包和 WinRT 服务仍需 Windows 实机验收。

use async_trait::async_trait;

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat};
use windows::Media::Ocr::OcrEngine as WinOcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

use super::{map_pixel_bbox, OcrBlock, OcrEngine, OcrInput, OcrOptions, OcrResult};
use crate::error::LenzError;
use crate::types::Rect;
use std::time::Instant;

/// Win OCR 不返回置信度,统一填该常量(spec 03 §4)。
const WIN_OCR_CONFIDENCE: f32 = 0.9;

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
        if crate::core::windows_package::require_identity().is_err() {
            return Vec::new();
        }
        let _apartment = initialize_apartment().ok();
        // OcrEngine::AvailableRecognizerLanguages() 返回系统已装识别语言。
        match available_languages() {
            Ok(langs) if !langs.is_empty() => langs,
            _ => Vec::new(),
        }
    }

    async fn recognize(&self, input: OcrInput, opts: OcrOptions) -> Result<OcrResult, LenzError> {
        let started = Instant::now();
        let pixel_rect = input.pixel_rect;
        let scale = input.scale_factor;
        let png = input.png;

        let blocks =
            tokio::task::spawn_blocking(move || run_win_ocr(&png, &opts, &pixel_rect, scale))
                .await
                .map_err(|e| LenzError::OcrFailed(format!("spawn_blocking join error: {e}")))??;

        Ok(OcrResult {
            blocks,
            detected_language: None,
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

fn win_err(ctx: &str, e: windows::core::Error) -> LenzError {
    LenzError::OcrFailed(format!("{ctx}: {e}"))
}

/// 在阻塞线程内执行一次 Windows OCR。所有 WinRT 对象局限本函数。
fn run_win_ocr(
    png: &[u8],
    opts: &OcrOptions,
    pixel_rect: &Rect,
    scale: f32,
) -> Result<Vec<OcrBlock>, LenzError> {
    crate::core::windows_package::require_identity()?;
    let _apartment = initialize_apartment()?;
    // 1) PNG 字节 → 内存随机访问流。
    let stream = InMemoryRandomAccessStream::new().map_err(|e| win_err("create stream", e))?;
    let writer = DataWriter::CreateDataWriter(&stream).map_err(|e| win_err("create writer", e))?;
    writer
        .WriteBytes(png)
        .map_err(|e| win_err("write bytes", e))?;
    writer
        .StoreAsync()
        .map_err(|e| win_err("store async", e))?
        .join()
        .map_err(|e| win_err("store", e))?;
    writer
        .FlushAsync()
        .map_err(|e| win_err("flush async", e))?
        .join()
        .map_err(|e| win_err("flush", e))?;
    // 解除 writer 对 stream 的所有权,避免其析构关闭流;并回到流头部。
    let _ = writer.DetachStream();
    stream.Seek(0).map_err(|e| win_err("seek", e))?;

    // 2) 解码为 SoftwareBitmap。
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| win_err("decoder async", e))?
        .join()
        .map_err(|e| win_err("decoder", e))?;
    let bitmap = decoder
        .GetSoftwareBitmapConvertedAsync(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Ignore)
        .map_err(|e| win_err("bitmap async", e))?
        .join()
        .map_err(|e| win_err("bitmap", e))?;

    // 3) 创建 OCR 引擎(按 preferred_languages 依序尝试 → 用户配置回退)。
    let engine = create_engine(opts)?;

    // 4) 识别。
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| win_err("recognize async", e))?
        .join()
        .map_err(|e| win_err("recognize", e))?;

    // 5) 遍历 Lines;bbox = 行内各 Word.BoundingRect 的并集。
    let mut blocks = Vec::new();
    let lines = result.Lines().map_err(|e| win_err("lines", e))?;
    let line_count = lines.Size().map_err(|e| win_err("lines size", e))?;
    for i in 0..line_count {
        let line = lines.GetAt(i).map_err(|e| win_err("line", e))?;
        let text = line
            .Text()
            .map_err(|e| win_err("line text", e))?
            .to_string();

        let words = line.Words().map_err(|e| win_err("words", e))?;
        let word_count = words.Size().map_err(|e| win_err("words size", e))?;
        // 并集:min(x,y) / max(x+w, y+h)。
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for j in 0..word_count {
            let w = words.GetAt(j).map_err(|e| win_err("word", e))?;
            let r = w.BoundingRect().map_err(|e| win_err("bounding rect", e))?;
            let (x0, y0) = (r.X as f64, r.Y as f64);
            let (x1, y1) = (x0 + r.Width as f64, y0 + r.Height as f64);
            bounds = Some(match bounds {
                None => (x0, y0, x1, y1),
                Some((ax0, ay0, ax1, ay1)) => (ax0.min(x0), ay0.min(y0), ax1.max(x1), ay1.max(y1)),
            });
        }

        let Some((x0, y0, x1, y1)) = bounds else {
            continue; // 无 word 边框,跳过该行。
        };
        let bbox = map_pixel_bbox(x0, y0, x1 - x0, y1 - y0, pixel_rect, scale);
        blocks.push(OcrBlock {
            text,
            bbox,
            confidence: WIN_OCR_CONFIDENCE,
        });
    }

    Ok(blocks)
}

/// 按 `preferred_languages` 依序 `TryCreateFromLanguage`,失败回退到
/// `TryCreateFromUserProfileLanguages`;仍不可用 → `OcrUnavailable`(spec 03 §4.1)。
fn create_engine(opts: &OcrOptions) -> Result<WinOcrEngine, LenzError> {
    for tag in &opts.preferred_languages {
        let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) else {
            continue;
        };
        // TryCreateFromLanguage 在缺语言包时可能返回 Err 或 null;两种都视为不可用,继续尝试下一个。
        if let Ok(engine) = WinOcrEngine::TryCreateFromLanguage(&language) {
            return Ok(engine);
        }
    }
    // 回退:用户配置语言。
    if !opts.auto_detect_language {
        return Err(LenzError::OcrUnavailable(
            "no installed preferred OCR language pack".into(),
        ));
    }
    WinOcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| {
        // 上层据此 emit `ocr://language-missing` + 系统设置引导。
        LenzError::OcrUnavailable("no installed OCR language pack".into())
    })
}

/// 系统已安装的识别语言(BCP-47)。
fn available_languages() -> Result<Vec<String>, LenzError> {
    let langs = WinOcrEngine::AvailableRecognizerLanguages()
        .map_err(|e| win_err("available languages", e))?;
    let count = langs.Size().map_err(|e| win_err("languages size", e))?;
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let lang = langs.GetAt(i).map_err(|e| win_err("language", e))?;
        if let Ok(tag) = lang.LanguageTag() {
            out.push(tag.to_string());
        }
    }
    Ok(out)
}

struct Apartment(bool);
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
}
fn initialize_apartment() -> Result<Apartment, LenzError> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if result.is_ok() {
        Ok(Apartment(true))
    } else if result.0 == 0x80010106u32 as i32 {
        Ok(Apartment(false))
    }
    // existing STA
    else {
        result
            .ok()
            .map(|_| Apartment(false))
            .map_err(|e| win_err("initialize COM", e))
    }
}

/// Runs the production OCR path from the installed package before any GUI/config setup.
/// The fixed synthetic fixture never captures the user's screen or reads credentials.
pub fn write_install_diagnostic(output: &std::path::Path) -> Result<bool, LenzError> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Report {
        ok: bool,
        package_identity: Option<String>,
        available_languages: Vec<String>,
        recognized_text: String,
        blocks: Vec<OcrBlock>,
        error: Option<String>,
    }
    let identity = crate::core::windows_package::full_name().ok().flatten();
    let engine = WinMediaOcr::new();
    let languages = engine.supported_languages();
    let options = OcrOptions {
        preferred_languages: vec!["en-US".into()],
        fast_mode: false,
        auto_detect_language: true,
        min_confidence: 0.3,
    };
    let blocks = run_win_ocr(
        include_bytes!("../../../resources/OcrSmoke.png"),
        &options,
        &Rect {
            x: 0,
            y: 0,
            w: 1024,
            h: 220,
        },
        1.0,
    );
    let (blocks, mut error) = match blocks {
        Ok(blocks) => (blocks, None),
        Err(error) => (Vec::new(), Some(error.to_string())),
    };
    let text = blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let ok = identity.is_some()
        && error.is_none()
        && text.contains("LENZ")
        && text.contains("12345")
        && !blocks.is_empty();
    if !ok && error.is_none() {
        error = Some("The installed recognizer did not read the synthetic LENZ 12345 image. Check the OCR language capabilities (English is recommended for this diagnostic).".into());
    }
    let report = Report {
        ok,
        package_identity: identity,
        available_languages: languages,
        recognized_text: text,
        blocks,
        error,
    };
    let temp = output.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temp, serde_json::to_vec_pretty(&report)?)?;
    std::fs::rename(&temp, output)?;
    Ok(ok)
}
