//! Windows.Media.Ocr 实现(spec 03 §4)。
//!
//! 流程:PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `SoftwareBitmap`
//! → `OcrEngine`(按语言尝试 + 用户配置回退)→ `RecognizeAsync` → 遍历 Lines。
//!
//! WinRT 异步操作在 `spawn_blocking` 内用 `.get()` 阻塞等待;所有 COM 对象的
//! 生命周期都局限在闭包内,只返回纯 Rust `Vec<OcrBlock>`。
//!
//! ⚠️ 本文件仅在 `target_os = "windows"` 编译,无法在 macOS 开发机本地验证,
//!    需在 Windows 机 / CI(windows-latest)上 `cargo check`/`cargo test` 校验。
//!    重点核对项:`TryCreateFromLanguage` 在缺语言包时的返回语义(Err vs null)、
//!    `IVectorView` 迭代、`IAsyncOperation::get()` 是否需要额外 feature。

use async_trait::async_trait;

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
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
    // 1) PNG 字节 → 内存随机访问流。
    let stream = InMemoryRandomAccessStream::new().map_err(|e| win_err("create stream", e))?;
    let writer = DataWriter::CreateDataWriter(&stream).map_err(|e| win_err("create writer", e))?;
    writer
        .WriteBytes(png)
        .map_err(|e| win_err("write bytes", e))?;
    writer
        .StoreAsync()
        .map_err(|e| win_err("store async", e))?
        .get()
        .map_err(|e| win_err("store", e))?;
    writer
        .FlushAsync()
        .map_err(|e| win_err("flush async", e))?
        .get()
        .map_err(|e| win_err("flush", e))?;
    // 解除 writer 对 stream 的所有权,避免其析构关闭流;并回到流头部。
    let _ = writer.DetachStream();
    stream.Seek(0).map_err(|e| win_err("seek", e))?;

    // 2) 解码为 SoftwareBitmap。
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| win_err("decoder async", e))?
        .get()
        .map_err(|e| win_err("decoder", e))?;
    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .map_err(|e| win_err("bitmap async", e))?
        .get()
        .map_err(|e| win_err("bitmap", e))?;

    // 3) 创建 OCR 引擎(按 preferred_languages 依序尝试 → 用户配置回退)。
    let engine = create_engine(opts)?;

    // 4) 识别。
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| win_err("recognize async", e))?
        .get()
        .map_err(|e| win_err("recognize", e))?;

    // 5) 遍历 Lines;bbox = 行内各 Word.BoundingRect 的并集。
    let mut blocks = Vec::new();
    if WIN_OCR_CONFIDENCE < opts.min_confidence {
        // 置信度常量低于阈值 → 全部过滤,直接返回空。
        return Ok(blocks);
    }
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
    WinOcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| {
        // TODO(M4/M7): 上层据此 emit `ocr://language-missing` + 系统设置引导。
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
