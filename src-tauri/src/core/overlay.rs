//! 原图覆盖翻译:共用截图与 OCR 坐标契约,独立取消作用域。
use super::{
    capture,
    ocr::{OcrEngine, OcrInput, OcrOptions},
    screenshot::ScreenshotSession,
    translator::TranslatorManager,
};
use crate::{
    config::ConfigStore,
    error::LenzError,
    types::{Rect, TranslateRequest},
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(not(target_os = "windows"))]
use tauri::{LogicalPosition, LogicalSize};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayBlock {
    pub bbox: Rect,
    pub translation: String,
    pub bg_color: [u8; 3],
    pub fg_color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OverlayClosed {
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPayload {
    pub generation: u32,
    pub scale_factor: f64,
    pub region: Rect,
    pub blocks: Vec<OverlayBlock>,
}
#[derive(Default)]
pub struct OverlayState {
    pub session: ScreenshotSession,
    pub payload: Mutex<Option<OverlayPayload>>,
    escape_owned: AtomicBool,
}

pub fn close<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), LenzError> {
    let state = app.state::<OverlayState>();
    state.session.cancel();
    let existed = state
        .payload
        .lock()
        .map_err(|_| LenzError::internal("overlay lock poisoned"))?
        .take()
        .is_some();
    if let Some(window) = app.get_webview_window("overlay") {
        window.destroy()?;
    }
    if state.escape_owned.swap(false, Ordering::SeqCst) {
        app.global_shortcut()
            .unregister("Escape")
            .map_err(|e| LenzError::internal(e.to_string()))?;
    }
    let _ = crate::tray::rebuild_tray_menu(app);
    if existed {
        app.emit(
            "overlay://closed",
            OverlayClosed {
                id: "overlay".into(),
            },
        )?;
    }
    Ok(())
}

pub fn start(app: AppHandle, region: Rect) {
    if region.w < 4 || region.h < 4 {
        return;
    }
    super::screenshot::hide_for_capture(&app);
    let gen = app.state::<OverlayState>().session.begin();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<OverlayState>();
        let result = state.session.run(gen, pipeline(&app, region, gen)).await;
        if let Some(Err(error)) = result {
            if state.session.is_current(gen) {
                if matches!(error, LenzError::OcrUnavailable(_)) {
                    let _ = app.emit("ocr://language-missing", ());
                }
                if matches!(error, LenzError::OcrIdentityRequired(_)) {
                    let _ = app.emit("ocr://package-required", ());
                }
                let _ = app.emit("app://operation-error", error.to_string());
                crate::tray::show_main_window(&app);
            }
        }
    });
}

async fn pipeline(app: &AppHandle, region: Rect, gen: u32) -> Result<(), LenzError> {
    let cfg = app.state::<ConfigStore>().get()?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let (cap, ocr) = tokio::time::timeout(Duration::from_secs(10), async {
        let cap = tokio::task::spawn_blocking(move || capture::capture_region(region))
            .await
            .map_err(|e| LenzError::internal(e.to_string()))??;
        let engine = app.state::<Arc<dyn OcrEngine>>().inner().clone();
        let ocr = engine
            .recognize(
                OcrInput {
                    png: cap.png.clone(),
                    pixel_rect: cap.pixel_rect,
                    scale_factor: cap.scale_factor,
                },
                OcrOptions {
                    preferred_languages: cfg.ocr.preferred_languages.clone(),
                    fast_mode: cfg.ocr.fast_mode,
                    auto_detect_language: cfg.ocr.auto_detect_language,
                    min_confidence: cfg.ocr.min_confidence,
                },
            )
            .await?;
        Ok::<_, LenzError>((cap, ocr))
    })
    .await
    .map_err(|_| LenzError::Timeout("capture/OCR timed out".into()))??;
    if ocr.blocks.is_empty() {
        return Err(LenzError::OcrFailed("no text recognized".into()));
    }
    let translator = app
        .state::<TranslatorManager>()
        .get(None)
        .ok_or_else(|| LenzError::TranslateFailed("no enabled translation engine".into()))?;
    let make_request = |texts| TranslateRequest {
        texts,
        source_lang: ocr.detected_language.clone(),
        target_lang: cfg.translator.target_lang.clone(),
        engine_id: Some(translator.id().into()),
    };
    let texts: Vec<String> = ocr.blocks.iter().map(|b| b.text.clone()).collect();
    let batch = if translator.supports_batch() {
        translator.translate(make_request(texts)).await.ok()
    } else {
        None
    };
    let mut translations = Vec::new();
    let mut last_error = None;
    for (i, block) in ocr.blocks.iter().enumerate() {
        let translated = batch.as_ref().and_then(|r| r.translations.get(i)).cloned();
        if let Some(text) = translated {
            translations.push(text);
            continue;
        }
        match translator
            .translate(make_request(vec![block.text.clone()]))
            .await
        {
            Ok(r) => translations.push(r.translations.into_iter().next().unwrap_or_default()),
            Err(error) => {
                last_error = Some(error);
                translations.push(String::new());
            }
        }
    }
    if translations.iter().all(|t| t.trim().is_empty()) {
        return Err(
            last_error.unwrap_or_else(|| LenzError::TranslateFailed("empty translation".into()))
        );
    }
    let scale = cap.scale_factor as f64;
    let clipped = Rect {
        x: (cap.pixel_rect.x as f64 / scale).round() as i32,
        y: (cap.pixel_rect.y as f64 / scale).round() as i32,
        w: (cap.pixel_rect.w as f64 / scale).round() as i32,
        h: (cap.pixel_rect.h as f64 / scale).round() as i32,
    };
    let image = image::load_from_memory(&cap.png)
        .map_err(|e| LenzError::internal(e.to_string()))?
        .to_rgb8();
    let blocks = ocr
        .blocks
        .iter()
        .zip(translations)
        .filter_map(|(block, translation)| {
            if translation.trim().is_empty() {
                return None;
            }
            let bbox = relative_bbox(block.bbox, clipped)?;
            let color = sample_color(&image, bbox, scale);
            let light =
                (299 * color[0] as u32 + 587 * color[1] as u32 + 114 * color[2] as u32) > 128_000;
            Some(OverlayBlock {
                bbox,
                translation,
                bg_color: color,
                fg_color: if light { "#000000" } else { "#ffffff" }.into(),
            })
        })
        .collect();
    let payload = OverlayPayload {
        generation: gen,
        scale_factor: scale,
        region: clipped,
        blocks,
    };
    if !app.state::<OverlayState>().session.is_current(gen) {
        return Ok(());
    }
    *app.state::<OverlayState>()
        .payload
        .lock()
        .map_err(|_| LenzError::internal("overlay lock poisoned"))? = Some(payload.clone());
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if handle.state::<OverlayState>().session.is_current(gen) {
            if let Err(error) = display(&handle, &payload) {
                let _ = handle.emit("app://operation-error", error.to_string());
                crate::tray::show_main_window(&handle);
            }
        }
    })?;
    Ok(())
}

pub fn display(app: &AppHandle, payload: &OverlayPayload) -> Result<(), LenzError> {
    if payload.region.w < 1 || payload.region.h < 1 {
        return Err(LenzError::config("invalid overlay region"));
    }
    let window = WebviewWindowBuilder::new(
        app,
        "overlay",
        WebviewUrl::App("index.html?window=overlay".into()),
    )
    .title("Lenz Overlay")
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .visible(false)
    .build()?;
    #[cfg(not(target_os = "windows"))]
    {
        window.set_position(LogicalPosition::new(
            payload.region.x as f64,
            payload.region.y as f64,
        ))?;
        window.set_size(LogicalSize::new(
            payload.region.w as f64,
            payload.region.h as f64,
        ))?;
    }
    #[cfg(target_os = "windows")]
    {
        let scale = payload.scale_factor;
        if !scale.is_finite() || !(0.5..=8.0).contains(&scale) {
            return Err(LenzError::config("invalid overlay scale"));
        }
        window.set_position(tauri::PhysicalPosition::new(
            (payload.region.x as f64 * scale).round() as i32,
            (payload.region.y as f64 * scale).round() as i32,
        ))?;
        window.set_size(tauri::PhysicalSize::new(
            (payload.region.w as f64 * scale).round() as u32,
            (payload.region.h as f64 * scale).round() as u32,
        ))?;
    }
    window.set_ignore_cursor_events(true)?;
    let state = app.state::<OverlayState>();
    if !app.global_shortcut().is_registered("Escape") {
        let result = app
            .global_shortcut()
            .on_shortcut("Escape", |app, _, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = close(app);
                }
            });
        if result.is_ok() {
            state.escape_owned.store(true, Ordering::SeqCst);
        } else {
            log::warn!("overlay Escape unavailable; use the tray close action");
        }
    }
    crate::tray::rebuild_tray_menu(app)?;
    Ok(())
}

pub fn ready(app: &AppHandle, generation: u32) -> Result<(), LenzError> {
    if app.state::<OverlayState>().session.is_current(generation) {
        if let Some(window) = app.get_webview_window("overlay") {
            window.show()?;
        }
    }
    Ok(())
}

fn relative_bbox(bbox: Rect, region: Rect) -> Option<Rect> {
    let x = (bbox.x - region.x).max(0);
    let y = (bbox.y - region.y).max(0);
    let right = (bbox.x - region.x + bbox.w).min(region.w);
    let bottom = (bbox.y - region.y + bbox.h).min(region.h);
    (right > x && bottom > y).then_some(Rect {
        x,
        y,
        w: right - x,
        h: bottom - y,
    })
}
fn sample_color(image: &image::RgbImage, bbox: Rect, scale: f64) -> [u8; 3] {
    let mut left = ((bbox.x as f64 * scale) as u32).min(image.width().saturating_sub(1));
    let mut top = ((bbox.y as f64 * scale) as u32).min(image.height().saturating_sub(1));
    let mut right = (((bbox.x + bbox.w) as f64 * scale) as u32)
        .saturating_sub(1)
        .min(image.width().saturating_sub(1));
    let mut bottom = (((bbox.y + bbox.h) as f64 * scale) as u32)
        .saturating_sub(1)
        .min(image.height().saturating_sub(1));
    let inset = 2u32
        .min(right.saturating_sub(left) / 2)
        .min(bottom.saturating_sub(top) / 2);
    left += inset;
    right -= inset;
    top += inset;
    bottom -= inset;
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for (x, y) in (left..=right)
        .flat_map(|x| [(x, top), (x, bottom)])
        .chain((top..=bottom).flat_map(|y| [(left, y), (right, y)]))
    {
        let pixel = image.get_pixel(x, y).0;
        for i in 0..3 {
            channels[i].push(pixel[i]);
        }
    }
    std::array::from_fn(|i| {
        channels[i].sort_unstable();
        channels[i]
            .get(channels[i].len() / 2)
            .copied()
            .unwrap_or(255)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boxes_clip_negative_screen_origins_and_colors_use_capture_scale() {
        let region = Rect {
            x: -200,
            y: 40,
            w: 100,
            h: 100,
        };
        let bbox = relative_bbox(
            Rect {
                x: -210,
                y: 50,
                w: 40,
                h: 20,
            },
            region,
        )
        .unwrap();
        assert_eq!((bbox.x, bbox.y, bbox.w, bbox.h), (0, 10, 30, 20));
        let image = image::RgbImage::from_pixel(200, 200, image::Rgb([24, 48, 72]));
        assert_eq!(
            sample_color(
                &image,
                Rect {
                    x: 90,
                    y: 90,
                    w: 30,
                    h: 30
                },
                2.0
            ),
            [24, 48, 72]
        );
    }
}
