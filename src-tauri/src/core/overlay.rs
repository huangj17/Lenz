//! 原图覆盖翻译:共用截图与 OCR 坐标契约,独立取消作用域。
use super::{
    capture,
    ocr::{recognize_with_timeout, resolve_engine, OcrEngine, OcrInput, OcrOptions, OcrPurpose},
    screenshot::ScreenshotSession,
    translator::TranslatorManager,
};
use crate::{
    config::ConfigStore,
    error::LenzError,
    types::{Rect, TranslateRequest},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(not(target_os = "windows"))]
use tauri::{LogicalPosition, LogicalSize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPhase {
    Ocr,
    Translate,
    Complete,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayProgress {
    pub revision: u32,
    pub phase: OverlayPhase,
    pub blocks: Vec<OverlayBlock>,
    pub error: Option<LenzError>,
}
impl OverlayProgress {
    fn recognizing() -> Self {
        Self {
            revision: 0,
            phase: OverlayPhase::Ocr,
            blocks: Vec::new(),
            error: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayUpdated {
    pub generation: u32,
    pub progress: OverlayProgress,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPayload {
    pub generation: u32,
    pub scale_factor: f64,
    pub region: Rect,
    pub viewport: Rect,
    /// 仅包含用户选区的截图,随蒙层关闭释放,不写入磁盘。
    pub image_data_url: String,
    pub progress: OverlayProgress,
}
#[derive(Default)]
pub struct OverlayState {
    pub session: ScreenshotSession,
    pub payload: Mutex<Option<OverlayPayload>>,
}
impl OverlayState {
    /// 更新与关闭共用载荷锁。取消后不能写回载荷或重新显示旧窗口。
    fn update(
        &self,
        generation: u32,
        phase: OverlayPhase,
        blocks: Vec<OverlayBlock>,
        error: Option<LenzError>,
    ) -> Result<Option<OverlayUpdated>, LenzError> {
        let mut payload = self
            .payload
            .lock()
            .map_err(|_| LenzError::internal("overlay lock poisoned"))?;
        let Some(payload) = payload
            .as_mut()
            .filter(|p| p.generation == generation && self.session.is_current(generation))
        else {
            return Ok(None);
        };
        payload.progress = OverlayProgress {
            revision: payload.progress.revision + 1,
            phase,
            blocks,
            error,
        };
        Ok(Some(OverlayUpdated {
            generation,
            progress: payload.progress.clone(),
        }))
    }
}

fn publish(
    app: &AppHandle,
    generation: u32,
    phase: OverlayPhase,
    blocks: Vec<OverlayBlock>,
    error: Option<LenzError>,
) -> Result<bool, LenzError> {
    let Some(update) = app
        .state::<OverlayState>()
        .update(generation, phase, blocks, error)?
    else {
        return Ok(false);
    };
    // 不重复传输截图;窗口尚未挂载时由 get_overlay_payload 读取最新进度。
    app.emit_to("overlay", "overlay://updated", update)?;
    Ok(true)
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
                // 截图后的错误留在当前蒙层,关闭入口始终可用。
                if !publish(
                    &app,
                    gen,
                    OverlayPhase::Failed,
                    Vec::new(),
                    Some(error.clone()),
                )
                .unwrap_or(false)
                    && state.session.is_current(gen)
                {
                    let _ = app.emit("app://operation-error", error.to_string());
                    crate::tray::show_main_window(&app);
                }
            }
        }
    });
}

async fn pipeline(app: &AppHandle, region: Rect, gen: u32) -> Result<(), LenzError> {
    let cfg = app.state::<ConfigStore>().get()?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let cap = capture::capture_region_with_timeout(region).await?;
    let scale = cap.scale_factor as f64;
    let clipped = Rect {
        x: (cap.pixel_rect.x as f64 / scale).round() as i32,
        y: (cap.pixel_rect.y as f64 / scale).round() as i32,
        w: (cap.pixel_rect.w as f64 / scale).round() as i32,
        h: (cap.pixel_rect.h as f64 / scale).round() as i32,
    };
    let payload = OverlayPayload {
        generation: gen,
        scale_factor: scale,
        region: clipped,
        viewport: viewport_for_region(app, clipped)?,
        image_data_url: format!("data:image/png;base64,{}", STANDARD.encode(&cap.png)),
        progress: OverlayProgress::recognizing(),
    };
    {
        let state = app.state::<OverlayState>();
        let mut saved = state
            .payload
            .lock()
            .map_err(|_| LenzError::internal("overlay lock poisoned"))?;
        if !state.session.is_current(gen) {
            return Ok(());
        }
        *saved = Some(payload.clone());
    }
    // 必须先抓屏再显示蒙层,避免把 loading/工具条截图进识别输入。
    // 创建窗口与后续 OCR 并行,不再等待远端翻译。
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if handle.state::<OverlayState>().session.is_current(gen) {
            if let Err(error) = display(&handle, &payload) {
                let _ = handle.emit("app://operation-error", error.to_string());
                crate::tray::show_main_window(&handle);
            }
        }
    })?;
    let engine = resolve_engine(
        &cfg.ocr,
        app.state::<Arc<dyn OcrEngine>>().inner().clone(),
        OcrPurpose::Overlay,
    )?;
    let ocr = recognize_with_timeout(
        engine.as_ref(),
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
    if ocr.blocks.is_empty() {
        return Err(LenzError::OcrFailed("no text recognized".into()));
    }
    if !publish(app, gen, OverlayPhase::Translate, Vec::new(), None)? {
        return Ok(());
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
    publish(app, gen, OverlayPhase::Complete, blocks, None)?;
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
    .focusable(true)
    .accept_first_mouse(true)
    .resizable(false)
    .visible(false)
    .build()?;
    #[cfg(not(target_os = "windows"))]
    {
        // AppKit resizing can shift a window's origin; place it after sizing.
        window.set_size(LogicalSize::new(
            payload.viewport.w as f64,
            payload.viewport.h as f64,
        ))?;
        window.set_position(LogicalPosition::new(
            payload.viewport.x as f64,
            payload.viewport.y as f64,
        ))?;
    }
    #[cfg(target_os = "windows")]
    {
        let scale = payload.scale_factor;
        if !scale.is_finite() || !(0.5..=8.0).contains(&scale) {
            return Err(LenzError::config("invalid overlay scale"));
        }
        window.set_size(tauri::PhysicalSize::new(
            (payload.viewport.w as f64 * scale).round() as u32,
            (payload.viewport.h as f64 * scale).round() as u32,
        ))?;
        window.set_position(tauri::PhysicalPosition::new(
            (payload.viewport.x as f64 * scale).round() as i32,
            (payload.viewport.y as f64 * scale).round() as i32,
        ))?;
    }
    crate::tray::rebuild_tray_menu(app)?;
    log::info!(
        "overlay window created: generation={} blocks={} logical_size={}x{}",
        payload.generation,
        payload.progress.blocks.len(),
        payload.region.w,
        payload.region.h
    );
    Ok(())
}

pub fn ready(app: &AppHandle, generation: u32) -> Result<(), LenzError> {
    if app.state::<OverlayState>().session.is_current(generation) {
        if let Some(window) = app.get_webview_window("overlay") {
            window.show()?;
            window.set_focus()?;
            log::info!("overlay window shown: generation={generation}");
        }
    }
    Ok(())
}

pub fn viewport_for_region(app: &AppHandle, region: Rect) -> Result<Rect, LenzError> {
    let bounds = super::selection_window::logical_monitor_bounds(app)?;
    choose_viewport(region, &bounds).ok_or_else(|| LenzError::internal("no overlay monitor"))
}

/// 与截图相同,选择与选区重叠最多的屏幕。保留全局负坐标,前端再减去原点。
fn choose_viewport(region: Rect, bounds: &[(f64, f64, f64, f64)]) -> Option<Rect> {
    bounds
        .iter()
        .filter(|(_, _, w, h)| *w > 0.0 && *h > 0.0)
        .map(|&(x, y, w, h)| Rect {
            x: x.round() as i32,
            y: y.round() as i32,
            w: w.round() as i32,
            h: h.round() as i32,
        })
        .max_by_key(|screen| {
            let left = i64::from(region.x).max(i64::from(screen.x));
            let top = i64::from(region.y).max(i64::from(screen.y));
            let right = (i64::from(region.x) + i64::from(region.w))
                .min(i64::from(screen.x) + i64::from(screen.w));
            let bottom = (i64::from(region.y) + i64::from(region.h))
                .min(i64::from(screen.y) + i64::from(screen.h));
            (right - left).max(0) * (bottom - top).max(0)
        })
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

    fn pending_state() -> (Arc<OverlayState>, u32) {
        let state = Arc::new(OverlayState::default());
        let generation = state.session.begin();
        *state.payload.lock().unwrap() = Some(OverlayPayload {
            generation,
            scale_factor: 1.0,
            region: Rect {
                x: 0,
                y: 0,
                w: 700,
                h: 390,
            },
            viewport: Rect {
                x: 0,
                y: 0,
                w: 1512,
                h: 982,
            },
            image_data_url: "data:image/png;base64,test".into(),
            progress: OverlayProgress::recognizing(),
        });
        (state, generation)
    }

    #[tokio::test]
    async fn slow_translation_keeps_the_captured_image_and_loading_state() {
        let (state, generation) = pending_state();
        assert_eq!(
            state
                .payload
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .progress
                .phase,
            OverlayPhase::Ocr
        );
        let (finish, response) = tokio::sync::oneshot::channel();
        let (started, waiting) = tokio::sync::oneshot::channel();
        let worker = state.clone();
        let work = tokio::spawn(async move {
            worker
                .session
                .run(generation, async {
                    worker
                        .update(generation, OverlayPhase::Translate, Vec::new(), None)
                        .unwrap();
                    started.send(()).unwrap();
                    response.await.unwrap();
                    worker
                        .update(generation, OverlayPhase::Complete, Vec::new(), None)
                        .unwrap();
                })
                .await
        });
        waiting.await.unwrap();
        {
            let payload = state.payload.lock().unwrap();
            let payload = payload.as_ref().unwrap();
            assert_eq!(payload.progress.phase, OverlayPhase::Translate);
            assert_eq!(payload.image_data_url, "data:image/png;base64,test");
            assert_eq!(payload.progress.revision, 1);
        }
        finish.send(()).unwrap();
        assert!(work.await.unwrap().is_some());
        assert_eq!(
            state
                .payload
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .progress
                .phase,
            OverlayPhase::Complete
        );
    }

    #[tokio::test]
    async fn closing_during_loading_cancels_work_and_rejects_late_results() {
        let (state, generation) = pending_state();
        let (started, waiting) = tokio::sync::oneshot::channel();
        let worker = state.clone();
        let work = tokio::spawn(async move {
            worker
                .session
                .run(generation, async {
                    started.send(()).unwrap();
                    std::future::pending::<()>().await;
                })
                .await
        });
        waiting.await.unwrap();
        state.session.cancel();
        state.payload.lock().unwrap().take();
        assert!(work.await.unwrap().is_none());
        assert!(state
            .update(generation, OverlayPhase::Complete, Vec::new(), None)
            .unwrap()
            .is_none());
        assert!(state.payload.lock().unwrap().is_none());
    }

    #[test]
    fn old_progress_cannot_change_a_new_selection_or_its_error() {
        let (state, old_generation) = pending_state();
        let generation = state.session.begin();
        state.payload.lock().unwrap().as_mut().unwrap().generation = generation;
        assert!(state
            .update(old_generation, OverlayPhase::Complete, Vec::new(), None)
            .unwrap()
            .is_none());
        let update = state
            .update(
                generation,
                OverlayPhase::Failed,
                Vec::new(),
                Some(LenzError::Timeout("ocr timed out".into())),
            )
            .unwrap()
            .unwrap();
        assert_eq!(update.progress.phase, OverlayPhase::Failed);
        assert_eq!(update.progress.revision, 1);
        assert!(matches!(update.progress.error, Some(LenzError::Timeout(_))));
    }

    #[test]
    fn overlay_uses_the_monitor_with_the_largest_intersection() {
        let screens = [(-2560.0, -171.0, 2560.0, 1440.0), (0.0, 0.0, 1512.0, 982.0)];
        let region = Rect {
            x: -1610,
            y: 129,
            w: 700,
            h: 390,
        };
        let viewport = choose_viewport(region, &screens).unwrap();
        assert_eq!(
            (viewport.x, viewport.y, viewport.w, viewport.h),
            (-2560, -171, 2560, 1440)
        );
        // 选区横跨边界时按面积选屏,不单凭左上角。
        let viewport = choose_viewport(
            Rect {
                x: -40,
                y: 100,
                w: 400,
                h: 300,
            },
            &screens,
        )
        .unwrap();
        assert_eq!(
            (viewport.x, viewport.y, viewport.w, viewport.h),
            (0, 0, 1512, 982)
        );
        assert!(choose_viewport(region, &[]).is_none());
    }
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
