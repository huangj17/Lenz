//! 截图翻译 F2 流程编排(spec 06)。
//!
//! 由 `setup.rs` 订阅 `capture://region-selected` 触发:销毁选区窗口后,在一个
//! async task 中串行执行 截图 → OCR → 翻译,并在关键节点 emit
//! `screenshot://progress` / `screenshot://result` / `screenshot://error`,popup 窗口订阅渲染。
//!
//! 取消:所有 popup 请求共享 watch 会话;新请求/ESC 使旧 future 立即释放。
//! 原生阻塞任务可能继续完成,但不能投递过期事件。

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

use crate::config::{Config, ConfigStore};
use crate::core::ocr::{
    recognize_with_timeout, resolve_engine, OcrBlock, OcrEngine, OcrInput, OcrOptions, OcrPurpose,
};
use crate::core::popup_position::{below_selection, ScreenRect, SelectionPlacement};
use crate::core::selection_text::{foreground_id, SelectedText};
use crate::core::translator::{Translator, TranslatorManager};
use crate::core::{capture, keystroke, selection_window};
use crate::error::LenzError;
use crate::types::{
    Rect, RegionSelectedPayload, ScreenshotCost, ScreenshotError, ScreenshotProgress,
    ScreenshotResult, ScreenshotTranslating, TranslateRequest, TriggerKind,
};

const POPUP_LABEL: &str = "popup";
/// popup 窗口默认逻辑宽度(用户未拖拽调宽时;可被 `ui.popup.width` 覆盖)。
pub const POPUP_WIDTH: f64 = 360.0;
/// 用户可拖拽调节的宽度下/上限(逻辑像素)。
pub const POPUP_MIN_WIDTH: f64 = 300.0;
pub const POPUP_MAX_WIDTH: f64 = 720.0;

/// 当前生效的 popup 宽度:读 `ui.popup.width` 并夹紧到 [MIN, MAX];读配置失败回退默认。
pub fn popup_width(app: &AppHandle) -> f64 {
    app.state::<ConfigStore>()
        .get()
        .ok()
        .map(|c| (c.ui.popup.width as f64).clamp(POPUP_MIN_WIDTH, POPUP_MAX_WIDTH))
        .unwrap_or(POPUP_WIDTH)
}
/// loading 态初始高度(前端渲染后会 setSize 精修)。
const POPUP_INIT_HEIGHT: f64 = 150.0;
/// 选区与 popup 的垂直间距(spec 06 §2.6)。
const POPUP_GAP: f64 = 12.0;

#[derive(Default)]
pub struct SelectionPopupPosition(Mutex<Option<SelectionPlacement>>);
/// 抓屏前等待选区蒙层从屏幕消失的时延:覆盖「事件线程→主线程」派发 + 合成器出帧,
/// 避免截到遮罩 / 销毁残帧导致 OCR 空结果(spec 02 §7)。
const OVERLAY_CLEAR_DELAY_MS: u64 = 150;

const EVT_PROGRESS: &str = "screenshot://progress";
const EVT_TRANSLATING: &str = "screenshot://translating";
const EVT_RESULT: &str = "screenshot://result";
const EVT_ERROR: &str = "screenshot://error";

/// popup webview 挂载完监听后 emit 的就绪信号(前端 `PopupWindow` → 后端)。
pub const EVENT_POPUP_READY: &str = "popup://ready";

/// popup 就绪信号最长等待;超时仍继续投递(宁可丢首帧也不卡死)。
const POPUP_READY_TIMEOUT: Duration = Duration::from_secs(3);
const POPUP_READY_POLL: Duration = Duration::from_millis(20);

/// All popup requests share one cancellation scope. Dropping the losing future
/// cancels reqwest work immediately instead of merely suppressing its result.
pub struct ScreenshotSession {
    generation: tokio::sync::watch::Sender<u32>,
    requests: Mutex<std::collections::HashMap<(u32, String), tokio::sync::watch::Sender<bool>>>,
}
impl Default for ScreenshotSession {
    fn default() -> Self {
        Self {
            generation: tokio::sync::watch::channel(0).0,
            requests: Mutex::new(Default::default()),
        }
    }
}
impl ScreenshotSession {
    pub fn begin(&self) -> u32 {
        // Serialize generation changes with child registration, so a new child
        // cannot be removed by the previous generation's cleanup.
        let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = 0;
        self.generation.send_modify(|value| {
            *value += 1;
            next = *value;
        });
        requests.clear();
        next
    }
    pub fn is_current(&self, gen: u32) -> bool {
        *self.generation.borrow() == gen
    }
    pub fn cancel(&self) {
        self.begin();
    }
    pub async fn run<T>(&self, gen: u32, work: impl std::future::Future<Output = T>) -> Option<T> {
        let mut changes = self.generation.subscribe();
        tokio::select! {
            biased;
            _ = changes.wait_for(|current| *current != gen) => None,
            value = work => Some(value),
        }
    }

    pub fn cancel_request(&self, gen: u32, request_id: &str) {
        let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        if !self.is_current(gen) {
            return;
        }
        // Retain a cancelled value even if cancellation arrives before invoke
        // starts polling; watch::wait_for checks the initial value as well.
        requests
            .entry((gen, request_id.to_string()))
            .or_insert_with(|| tokio::sync::watch::channel(false).0)
            .send_replace(true);
    }

    pub async fn run_request<T>(
        &self,
        gen: u32,
        request_id: &str,
        work: impl std::future::Future<Output = T>,
    ) -> Option<T> {
        let key = (gen, request_id.to_string());
        let mut cancelled = {
            let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
            if !self.is_current(gen) {
                return None;
            }
            requests
                .entry(key.clone())
                .or_insert_with(|| tokio::sync::watch::channel(false).0)
                .subscribe()
        };
        let result = self
            .run(gen, async {
                tokio::select! {
                    biased;
                    _ = cancelled.wait_for(|value| *value) => None,
                    value = work => Some(value),
                }
            })
            .await
            .flatten();
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
        result
    }
}

fn popup_engine_ids(config: &Config) -> Vec<&str> {
    let mut ids: Vec<_> = config
        .translator
        .engines
        .iter()
        .filter(|engine| {
            engine.enabled
                && !config
                    .ui
                    .popup
                    .services
                    .get(&engine.id)
                    .is_some_and(|policy| policy.skip_initial_translation())
        })
        .map(|engine| engine.id.as_str())
        .collect();
    if let Some(index) = ids
        .iter()
        .position(|id| *id == config.translator.default_engine_id)
    {
        let default_id = ids.remove(index);
        ids.insert(0, default_id);
    }
    ids
}

fn popup_primary(app: &AppHandle, config: &Config) -> Option<Arc<dyn Translator>> {
    let manager = app.state::<TranslatorManager>();
    popup_engine_ids(config)
        .into_iter()
        .find_map(|id| manager.get(Some(id)))
}

/// 最近一次成功的翻译结果(F1/F2 通用)。作为 managed state 注入。
///
/// 持久化到磁盘(`last-result.json`),仅在重新读取的选区原文、目标语言和引擎匹配时复用。
/// 取词失败不能据此回显无关旧内容。
#[derive(Default)]
pub struct LastResult(Mutex<Option<ScreenshotResult>>);

/// 持久化文件名(位于 `app_config_dir()`,与 `config.json` 同目录)。
const LAST_RESULT_FILE: &str = "last-result.json";

impl LastResult {
    fn set(&self, r: &ScreenshotResult) {
        if let Ok(mut g) = self.0.lock() {
            *g = Some(r.clone());
        }
    }
    fn get(&self) -> Option<ScreenshotResult> {
        self.0.lock().ok().and_then(|g| g.clone())
    }

    /// 把当前内存中的结果写盘(best-effort,失败仅记日志,不影响主流程)。
    fn persist(&self, app: &AppHandle) {
        let Some(result) = self.get() else { return };
        let Ok(dir) = app.path().app_config_dir() else {
            return;
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("persist last-result: create dir failed: {e}");
            return;
        }
        match serde_json::to_string(&result) {
            Ok(json) => {
                if let Err(e) = std::fs::write(dir.join(LAST_RESULT_FILE), json) {
                    log::warn!("persist last-result: write failed: {e}");
                }
            }
            Err(e) => log::warn!("persist last-result: serialize failed: {e}"),
        }
    }

    /// 启动时从磁盘加载上次结果到内存(缺失/损坏则静默留空)。
    pub fn hydrate(&self, app: &AppHandle) {
        let Ok(dir) = app.path().app_config_dir() else {
            return;
        };
        let path = dir.join(LAST_RESULT_FILE);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return; // 首次运行无文件,正常。
        };
        match serde_json::from_str::<ScreenshotResult>(&raw) {
            Ok(r) => {
                if let Ok(mut g) = self.0.lock() {
                    *g = Some(r);
                }
                log::info!("已加载上次翻译结果缓存");
            }
            Err(e) => log::warn!("hydrate last-result: 解析失败,忽略: {e}"),
        }
    }
}

/// popup webview 是否至少就绪过一次(其 `screenshot://*` 监听已挂上)。作为 managed state 注入。
///
/// 关闭「冷启动首帧竞态」:第一次划词/截图时,popup webview 可能尚未订阅
/// `screenshot://result`,后端一旦 emit 就丢帧(Tauri 不向迟到订阅者重放),表现为
/// 首次触发弹窗不出内容。popup 挂载完成后 emit `popup://ready`;后端在首帧投递前
/// 用 `wait_popup_ready` 短暂等待此信号(已就绪则零成本)。
#[derive(Default)]
pub struct PopupReady(std::sync::atomic::AtomicBool);

impl PopupReady {
    /// 标记 popup 已就绪(收到 `popup://ready` 时调用)。幂等。
    pub fn mark(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn is_ready(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// 在首帧 emit 前等待 popup webview 就绪(见 [`PopupReady`])。已就绪立即返回;
/// 每 `POPUP_READY_POLL` 轮询一次,最多等 `POPUP_READY_TIMEOUT` 后放弃继续投递。
async fn wait_popup_ready(app: &AppHandle) {
    if app.state::<PopupReady>().is_ready() {
        return;
    }
    let deadline = Instant::now() + POPUP_READY_TIMEOUT;
    loop {
        if app.state::<PopupReady>().is_ready() {
            return;
        }
        if Instant::now() >= deadline {
            log::warn!("popup 就绪信号 {POPUP_READY_TIMEOUT:?} 内未到,继续投递(首帧可能丢失)");
            return;
        }
        tokio::time::sleep(POPUP_READY_POLL).await;
    }
}

/// 处理 `capture://region-selected`:销毁选区窗口,按 trigger 分流。
pub fn on_region_selected(app: &AppHandle, payload: &str) {
    let parsed: RegionSelectedPayload = match serde_json::from_str(payload) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("region-selected payload parse failed: {e}");
            return;
        }
    };

    // 先隐藏(立即从屏幕移除,无白底残帧),再销毁释放。
    selection_window::hide_all(app);
    if let Err(e) = selection_window::close(app) {
        log::warn!("close selection window after select: {e}");
    }

    let region = Rect {
        x: parsed.x,
        y: parsed.y,
        w: parsed.w,
        h: parsed.h,
    };

    match parsed.trigger {
        TriggerKind::Screenshot => start_f2(app.clone(), region),
        // 原图覆盖采用独立会话,保留各 OCR block 的位置。
        TriggerKind::Overlay => super::overlay::start(app.clone(), region),
        TriggerKind::Selection => {}
    }
}

/// 在进入选区或抓屏前隐藏所有翻译浮层。
pub fn hide_for_capture(app: &AppHandle) {
    let _ = super::overlay::close(app);
    app.state::<ScreenshotSession>().cancel();
    crate::core::indicator::hide(app);
    if let Some(popup) = app.get_webview_window(POPUP_LABEL) {
        let _ = popup.hide();
    }
}

/// 启动 F2:校验选区 → 隐藏浮层 → 抓屏 → 显示 popup 并继续 OCR/翻译。
fn start_f2(app: AppHandle, region: Rect) {
    // w<4 || h<4 视为点击取消(spec 06 §2.2)。
    if region.w < 4 || region.h < 4 {
        log::debug!(
            "selection too small ({}x{}), treated as cancel",
            region.w,
            region.h
        );
        return;
    }

    hide_for_capture(&app);
    let gen = app.state::<ScreenshotSession>().begin();

    tauri::async_runtime::spawn(async move {
        app.state::<ScreenshotSession>()
            .run(gen, run_pipeline(&app, region, gen))
            .await;
    });
}

/// 截图 → OCR → 翻译,逐阶段 emit 进度 / 结果 / 错误。
async fn run_pipeline(app: &AppHandle, region: Rect, gen: u32) {
    let cfg = match app.state::<ConfigStore>().get() {
        Ok(c) => c,
        Err(e) => return emit_error(app, gen, "capture", "failed", &e.to_string(), None),
    };

    // ── 截图 ──

    // 等待选区蒙层真正从屏幕消失再抓屏:close/hide 是投递到主线程的异步操作,
    // 立即抓屏会截到遮罩或销毁残帧,导致 OCR「未识别到文字」(spec 02 §7)。
    tokio::time::sleep(Duration::from_millis(OVERLAY_CLEAR_DELAY_MS)).await;
    let t0 = Instant::now();
    let cap = capture::capture_region_with_timeout(region).await;
    let cost_capture = t0.elapsed().as_millis() as u32;
    if !is_current(app, gen) {
        return;
    }
    let scale = cap.as_ref().ok().map(|c| c.scale_factor as f64);
    position_and_show_popup(app, &region, scale);
    wait_popup_ready(app).await;
    if !is_current(app, gen) {
        return;
    }
    let cap = match cap {
        Ok(c) => c,
        Err(LenzError::Timeout(m)) => return emit_error(app, gen, "capture", "timeout", &m, None),
        Err(LenzError::Permission(m)) => {
            return emit_error(app, gen, "capture", "permission", &m, None)
        }
        Err(e) => return emit_error(app, gen, "capture", "failed", &e.to_string(), None),
    };

    // ── OCR ──
    emit_progress(app, gen, "ocr");
    let t1 = Instant::now();
    let engine = match resolve_engine(
        &cfg.ocr,
        app.state::<Arc<dyn OcrEngine>>().inner().clone(),
        OcrPurpose::Screenshot,
    ) {
        Ok(engine) => engine,
        Err(error) => return emit_error(app, gen, "ocr", "failed", &error.to_string(), None),
    };
    let opts = OcrOptions {
        preferred_languages: cfg.ocr.preferred_languages.clone(),
        fast_mode: cfg.ocr.fast_mode,
        auto_detect_language: cfg.ocr.auto_detect_language,
        min_confidence: cfg.ocr.min_confidence,
    };
    let input = OcrInput {
        png: cap.png,
        pixel_rect: cap.pixel_rect,
        scale_factor: cap.scale_factor,
    };
    let ocr = recognize_with_timeout(engine.as_ref(), input, opts).await;
    if !is_current(app, gen) {
        return;
    }
    let ocr = match ocr {
        Ok(r) => r,
        Err(LenzError::Timeout(m)) => return emit_error(app, gen, "ocr", "timeout", &m, None),
        Err(LenzError::Permission(m)) => {
            return emit_error(app, gen, "ocr", "permission", &m, None)
        }
        Err(LenzError::OcrUnavailable(message)) => {
            let _ = app.emit("ocr://language-missing", ());
            return emit_error(app, gen, "ocr", "language_missing", &message, None);
        }
        Err(LenzError::OcrIdentityRequired(message)) => {
            let _ = app.emit("ocr://package-required", ());
            return emit_error(app, gen, "ocr", "package_required", &message, None);
        }
        Err(e) => return emit_error(app, gen, "ocr", "failed", &e.to_string(), None),
    };
    let original = ocr.text.unwrap_or_else(|| join_blocks(&ocr.blocks));
    if original.trim().is_empty() {
        return emit_error(app, gen, "ocr", "no_text", "no text recognized", None);
    }
    let cost_ocr = t1.elapsed().as_millis() as u32;

    // ── 翻译 ──
    let translator = popup_primary(app, &cfg);
    let request_id = translator
        .as_ref()
        .map(|_| uuid::Uuid::new_v4().to_string());
    emit_translating(
        app,
        gen,
        &original,
        ocr.detected_language.clone(),
        &cfg.translator.target_lang,
        translator.as_ref().map(|engine| engine.id()).unwrap_or(""),
        request_id.clone(),
    );
    let t2 = Instant::now();
    let (Some(translator), Some(request_id)) = (translator, request_id) else {
        return;
    };
    let req = TranslateRequest {
        texts: vec![original.clone()],
        source_lang: ocr.detected_language.clone(),
        target_lang: cfg.translator.target_lang.clone(),
        engine_id: None,
    };
    let resp = app
        .state::<ScreenshotSession>()
        .run_request(gen, &request_id, translator.translate(req))
        .await;
    let Some(resp) = resp else { return };
    if !is_current(app, gen) {
        return;
    }
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return emit_error(
                app,
                gen,
                "translate",
                if matches!(e, LenzError::Timeout(_)) {
                    "timeout"
                } else {
                    "failed"
                },
                &e.to_string(),
                Some(original),
            )
        }
    };
    let cost_translate = t2.elapsed().as_millis() as u32;

    let translation = resp.translations.into_iter().next().unwrap_or_default();
    emit_result(
        app,
        ScreenshotResult {
            session_id: gen,
            original,
            translation,
            source_lang: resp.detected_source_lang.or(ocr.detected_language),
            target_lang: cfg.translator.target_lang.clone(),
            engine_id: resp.engine_id,
            region,
            cost: ScreenshotCost {
                capture: cost_capture,
                ocr: cost_ocr,
                translate: cost_translate,
            },
        },
    );
}

/// 把 OCR 文本块按阅读顺序拼接,尽量保留原排版(spec 06 §2.4):同一行的块用空格
/// 相连;相邻行用 `\n`;当行间垂直空隙明显大于一整行时,判为段落分隔并保留一个空行
/// (`\n\n`),使原文的分段结构在弹框与译文中都得到保留。每块先 `trim` 去首尾空白。
fn join_blocks(blocks: &[OcrBlock]) -> String {
    let mut out = String::new();
    let mut prev_mid_y: Option<i32> = None;
    // 「当前行」的底边与行高(同行多块取并集),换行时据此估算行间空隙判断是否分段。
    let mut line_bottom = 0;
    let mut line_h = 0;
    for b in blocks {
        let text = b.text.trim();
        if text.is_empty() {
            continue;
        }
        let mid_y = b.bbox.y + b.bbox.h / 2;
        let h = b.bbox.h.max(1);
        // 行高的一半作为同行判定阈值(小字号下至少 6px)。
        let same_line = matches!(prev_mid_y, Some(py) if (mid_y - py).abs() <= (h / 2).max(6));
        if prev_mid_y.is_some() {
            out.push_str(if same_line {
                " "
            } else if b.bbox.y - line_bottom > line_h.max(h) {
                // 行间空隙超过一整行高 → 视为段落分隔,保留一个空行。
                "\n\n"
            } else {
                "\n"
            });
        }
        out.push_str(text);
        // 更新「当前行」几何:同行并入其 bbox,换行则重置为本块。
        if same_line {
            line_bottom = line_bottom.max(b.bbox.y + h);
            line_h = line_h.max(h);
        } else {
            line_bottom = b.bbox.y + h;
            line_h = h;
        }
        prev_mid_y = Some(mid_y);
    }
    out
}

/// 显示并聚焦 popup(F1/F2 各定位路径的统一收尾)。
///
/// popup 需要 `set_focus` 才能接收 ESC、并在失焦时自动隐藏(spec 09 §3.4)。但 macOS 上
/// `set_focus` 会 `activateIgnoringOtherApps` 激活整个 app,把**此前仍可见、只是被切到
/// 后台的主窗口**也一并拽回前台(用户反馈:开着主界面切到别的 app 后按快捷键翻译,主
/// 窗口被带出来)。故聚焦 popup 前先判断:主窗口可见但当前并非聚焦态(用户已切走)→ 先
/// 隐藏它,免得被 app 激活拽回;主窗口正聚焦(用户在设置页操作,如从主界面按 Alt+S)则
/// 不动它。popup 自身是 `alwaysOnTop` 浮层,隐藏主窗口不影响其置顶显示。
fn show_and_focus_popup(app: &AppHandle, popup: &tauri::WebviewWindow) {
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    #[cfg(target_os = "macos")]
    if let Some(main) = app.get_webview_window("main") {
        if main.is_visible().unwrap_or(false) && !main.is_focused().unwrap_or(false) {
            let _ = main.hide();
        }
    }
    let _ = popup.show();
    let _ = popup.set_focus();
}

/// 把 popup 定位到选区正下方(水平居中),夹紧到所在显示器,然后显示并聚焦。
fn position_and_show_popup(app: &AppHandle, region: &Rect, scale: Option<f64>) {
    *app.state::<SelectionPopupPosition>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    #[cfg(target_os = "windows")]
    {
        let scale = scale
            .or_else(|| {
                crate::core::cursor::current_physical().map(crate::core::cursor::point_scale)
            })
            .unwrap_or(1.0);
        let cx = (region.x as f64 + region.w as f64 / 2.0) * scale;
        let cy = (region.y as f64 + region.h as f64 / 2.0) * scale;
        position_windows_popup(
            app,
            cx - popup_width(app) * scale / 2.0,
            (region.y as f64 + region.h as f64 + POPUP_GAP) * scale,
            cx,
            cy,
            scale,
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = scale;
        let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
            log::warn!("popup window not found; cannot show F2 result");
            return;
        };

        let w = popup_width(app);
        let mut x = region.x as f64 + region.w as f64 / 2.0 - w / 2.0;
        let mut y = region.y as f64 + region.h as f64 + POPUP_GAP;

        // 夹紧到包含选区中心的显示器工作区;放不下则改置选区上方。
        if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
            let cx = region.x as f64 + region.w as f64 / 2.0;
            let cy = region.y as f64 + region.h as f64 / 2.0;
            if let Some((lx, ly, lw, lh)) = bounds
                .into_iter()
                .find(|(mx, my, mw, mh)| cx >= *mx && cx < mx + mw && cy >= *my && cy < my + mh)
            {
                x = x.clamp(lx, (lx + lw - w).max(lx));
                if y + POPUP_INIT_HEIGHT > ly + lh {
                    y = (region.y as f64 - POPUP_INIT_HEIGHT - POPUP_GAP).max(ly);
                }
            }
        }

        let _ = popup.set_position(LogicalPosition::new(x, y));
        let _ = popup.set_size(LogicalSize::new(w, POPUP_INIT_HEIGHT));
        show_and_focus_popup(app, &popup);
    }
}

/// 内容渲染后 `resize_popup` 调用:按新高度把 popup 重新夹紧到所在显示器内。
///
/// 初次定位只按 `POPUP_INIT_HEIGHT` 摆放,长译文撑高后(最高 480)若沿用旧的左上角,
/// 底部会溢出屏幕、内容被截断(spec 06 §2.6)。这里读当前位置 + 新高度,把窗口夹回
/// 显示器矩形内(放不下时顶边贴齐显示器顶部),优先保证内容完整可见。
pub fn clamp_popup_into_monitor(
    app: &AppHandle,
    height: f64,
    before_resize: Option<tauri::PhysicalPosition<i32>>,
) {
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        return;
    };
    let scale = popup.scale_factor().unwrap_or(1.0);
    let Ok(phys) = popup.outer_position() else {
        return;
    };
    {
        let state = app.state::<SelectionPopupPosition>();
        let mut anchor = state.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(placement) = anchor.as_mut() {
            // Native resizing may move the top edge. Only movement before the resize
            // is evidence that the user dragged the popup.
            let before = before_resize.unwrap_or(phys);
            #[cfg(target_os = "windows")]
            let (current, size_scale) = ((before.x as f64, before.y as f64), scale);
            #[cfg(not(target_os = "windows"))]
            let (current, size_scale) = ((before.x as f64 / scale, before.y as f64 / scale), 1.0);
            let width = popup
                .outer_size()
                .map(|s| s.width as f64 / scale * size_scale)
                .unwrap_or_else(|_| popup_width(app) * size_scale);
            if let Some((x, y)) = placement.resize(current, width, height * size_scale) {
                #[cfg(target_os = "windows")]
                let _ = popup.set_position(tauri::PhysicalPosition::new(
                    x.round() as i32,
                    y.round() as i32,
                ));
                #[cfg(not(target_os = "windows"))]
                let _ = popup.set_position(LogicalPosition::new(x, y));
                return;
            }
            // User dragged the popup: respect its new position from now on.
            *anchor = None;
        }
    }
    let pos = phys.to_logical::<f64>(scale);
    // 用窗口当前实际宽度(尊重用户拖拽后的宽度),而非固定常量。
    let w = popup
        .outer_size()
        .map(|s| s.to_logical::<f64>(scale).width)
        .unwrap_or_else(|_| popup_width(app));

    let Ok(bounds) = selection_window::logical_monitor_bounds(app) else {
        return;
    };
    // 选包含弹框中心的显示器;落在缝隙里则兜底用第一块。
    let cx = pos.x + w / 2.0;
    let cy = pos.y + height / 2.0;
    let Some((mx, my, mw, mh)) = bounds
        .iter()
        .find(|(mx, my, mw, mh)| cx >= *mx && cx < mx + mw && cy >= *my && cy < my + mh)
        .or_else(|| bounds.first())
        .copied()
    else {
        return;
    };

    let x = pos.x.clamp(mx, (mx + mw - w).max(mx));
    let y = pos.y.clamp(my, (my + mh - height).max(my));
    if (x - pos.x).abs() > 0.5 || (y - pos.y).abs() > 0.5 {
        let _ = popup.set_position(LogicalPosition::new(x, y));
    }
}

// ──────────────────────────────────────────────────────────────────────────
// F1 划词翻译 —— 快捷键模式(spec 05 §3)
// ──────────────────────────────────────────────────────────────────────────

/// 合成 Cmd+C 前等待触发快捷键的修饰键松开的轮询间隔与上限(spec 05 §3)。
const MOD_WAIT_INTERVAL: Duration = Duration::from_millis(10);
const MOD_WAIT_TIMEOUT: Duration = Duration::from_millis(400);

/// 划词翻译(快捷键模式):自动复制前台选中文本 → 翻译 → popup。
///
/// 流程(spec 05 §3):
/// 优先读取前台应用的系统选区,失败才等待修饰键松开并复制当前选区。
/// 选区和触发位置在显示 popup 前捕获;取词失败显示原因,不回显无关旧结果。
pub fn on_selection_shortcut(app: &AppHandle) {
    let gen = app.state::<ScreenshotSession>().begin();
    let anchor = selection_cursor_anchor();
    let source = foreground_id();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        handle_selection_shortcut(app, gen, anchor, source).await;
    });
}

/// 直接用已拿到的选中文本翻译(划词图标点击时,复用图标浮出瞬间 AX 预读的缓存)。
/// 跳过实时读取,规避「点击图标夺焦后读到错内容」。
pub fn translate_with_text(app: &AppHandle, text: SelectedText) {
    let gen = app.state::<ScreenshotSession>().begin();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        translate_selection(&app, text, gen).await;
    });
}

async fn handle_selection_shortcut(
    app: AppHandle,
    gen: u32,
    anchor: Option<ScreenRect>,
    source: Option<u64>,
) {
    if !crate::core::selection::permission_granted() {
        return selection_error(&app, gen, anchor, "selection_permission").await;
    }
    let Some(source_id) = source else {
        return selection_error(&app, gen, anchor, "selection_empty").await;
    };
    // ① 首选 Accessibility API 直读选中文本:不碰剪贴板、不合成按键,规避修饰键污染 /
    //    同文字重复 / 焦点时序 / 剪贴板被覆盖等一系列问题(spec 05 §3)。AX 调用可能对
    //    无响应目标应用造成同步 IPC 阻塞,放到 spawn_blocking 上跑,避免堵塞 async 运行时。
    let ax = tokio::task::spawn_blocking(move || {
        (foreground_id() == source)
            .then(crate::core::selection_text::read_selected_text)
            .flatten()
    })
    .await
    .ok()
    .flatten();
    if !is_current(&app, gen) {
        return;
    }
    if foreground_id() != source {
        return;
    }
    if let Some(mut selected) = ax {
        log::info!("划词:AX 直读命中 {} 字", selected.text.chars().count());
        selected.bounds = selected.bounds.or(anchor);
        return translate_selection(&app, selected, gen).await;
    }

    // ② 兜底(AX 取不到,如部分 Chrome/Electron):合成 Cmd+C 读剪贴板。
    //    Alt+D 触发时 Option 等修饰键往往仍按着,会并入合成的 Cmd+C(→ ⌘⌥C,前台不复制),
    //    先等其松开;原生剪贴板版本号可识别重复复制的相同文字。
    if !wait_modifiers_released().await {
        return selection_error(&app, gen, anchor, "selection_modifiers").await;
    }
    if !is_current(&app, gen) || foreground_id() != source {
        return;
    }
    let copied = copy_selection_via_clipboard(source_id).await;
    if !is_current(&app, gen) {
        return;
    }
    if foreground_id() != source {
        return;
    }
    if let Some(text) = copied {
        log::info!("划词:剪贴板兜底命中 {} 字", text.chars().count());
        return translate_selection(
            &app,
            SelectedText {
                text,
                bounds: anchor,
            },
            gen,
        )
        .await;
    }
    selection_error(&app, gen, anchor, "selection_empty").await;
}

/// 拿到选中文本后的统一收尾(F1 各路径——AX 直读 / 剪贴板兜底 / 图标预读——的唯一汇聚点)。
///
/// spec 05 §4 的「长度过滤 + 去重」在此集中落地:
/// - **长度过滤**:先 `trim`,空串直接忽略(下界 ≥1);再截断到 5000 字(上界)。
/// - **去重**:原文与「上次翻译结果」完全相同 → 直接回显缓存,不再重复请求引擎
///   (满足验收「同段不重复触发」,且省一次 LLM 调用)。
///
/// 通过过滤后:使用本次捕获的选区位置弹框并翻译。
async fn translate_selection(app: &AppHandle, selected: SelectedText, gen: u32) {
    let SelectedText {
        text,
        bounds: anchor,
    } = selected;
    if !is_current(app, gen) {
        return;
    }
    let text: String = text.trim().chars().take(5000).collect();
    if text.is_empty() {
        return;
    }
    // 去重键必须包含原文、目标语言和实际默认引擎。仅按原文缓存会在用户切换语言或
    // 默认引擎后回显旧译文。
    if let Some(last) = app.state::<LastResult>().get() {
        if let Ok(cfg) = app.state::<ConfigStore>().get() {
            let current_engine = popup_primary(app, &cfg).map(|engine| engine.id().to_string());
            if cached_result_matches(
                &last,
                &text,
                &cfg.translator.target_lang,
                current_engine.as_deref(),
            ) {
                log::info!("划词:原文/目标语言/引擎均相同,回显缓存结果");
                return reshow_result(app, last, gen, anchor).await;
            }
        }
    }
    if !is_current(app, gen) {
        return;
    }
    position_selection_popup(app, anchor);
    // 首帧投递前等待 popup webview 就绪,避免冷启动首次触发丢掉 progress/result(方案 A)。
    wait_popup_ready(app).await;
    app.state::<ScreenshotSession>()
        .run(gen, run_text_pipeline(app, text, gen))
        .await;
}

fn cached_result_matches(
    result: &ScreenshotResult,
    original: &str,
    target_lang: &str,
    engine_id: Option<&str>,
) -> bool {
    result.original == original
        && result.target_lang == target_lang
        && engine_id == Some(result.engine_id.as_str())
}

/// 当前选区与缓存匹配时,在本次选区位置回显已有结果。
async fn reshow_result(
    app: &AppHandle,
    mut result: ScreenshotResult,
    gen: u32,
    anchor: Option<ScreenRect>,
) {
    if !is_current(app, gen) {
        return;
    }
    result.session_id = gen;
    position_selection_popup(app, anchor);
    wait_popup_ready(app).await;
    if is_current(app, result.session_id) {
        let _ = app.emit(EVT_RESULT, result);
    }
}

/// 合成 Cmd+C 前等待触发快捷键的修饰键松开,避免被仍按住的 Option/Cmd 等并入合成事件
/// (spec 05 §3)。超时返回失败,不能向仍按住快捷键的源应用发送被污染的复制组合。
async fn wait_modifiers_released() -> bool {
    let deadline = Instant::now() + MOD_WAIT_TIMEOUT;
    while keystroke::modifiers_held() {
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(MOD_WAIT_INTERVAL).await;
    }
    true
}

async fn selection_error(app: &AppHandle, gen: u32, anchor: Option<ScreenRect>, code: &str) {
    if !is_current(app, gen) {
        return;
    }
    position_selection_popup(app, anchor);
    wait_popup_ready(app).await;
    emit_error(app, gen, "selection", code, code, None);
}

/// 原生剪贴板事务必须跑完恢复阶段,不能被翻译会话取消中断。
async fn copy_selection_via_clipboard(source: u64) -> Option<String> {
    tokio::task::spawn_blocking(move || crate::core::clipboard_selection::read(source))
        .await
        .ok()
        .flatten()
}

/// 纯文本翻译 pipeline(F1 快捷键复用 F2 的翻译 + 事件,无截图 / OCR 阶段)。
async fn run_text_pipeline(app: &AppHandle, original: String, gen: u32) {
    let cfg = match app.state::<ConfigStore>().get() {
        Ok(c) => c,
        Err(e) => {
            return emit_error(
                app,
                gen,
                "translate",
                if matches!(e, LenzError::Timeout(_)) {
                    "timeout"
                } else {
                    "failed"
                },
                &e.to_string(),
                Some(original),
            )
        }
    };
    let translator = popup_primary(app, &cfg);
    let request_id = translator
        .as_ref()
        .map(|_| uuid::Uuid::new_v4().to_string());
    emit_translating(
        app,
        gen,
        &original,
        None,
        &cfg.translator.target_lang,
        translator.as_ref().map(|engine| engine.id()).unwrap_or(""),
        request_id.clone(),
    );
    let t = Instant::now();
    let (Some(translator), Some(request_id)) = (translator, request_id) else {
        return;
    };
    let req = TranslateRequest {
        texts: vec![original.clone()],
        source_lang: None,
        target_lang: cfg.translator.target_lang.clone(),
        engine_id: None,
    };
    let resp = app
        .state::<ScreenshotSession>()
        .run_request(gen, &request_id, translator.translate(req))
        .await;
    let Some(resp) = resp else { return };
    if !is_current(app, gen) {
        return;
    }
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return emit_error(
                app,
                gen,
                "translate",
                if matches!(e, LenzError::Timeout(_)) {
                    "timeout"
                } else {
                    "failed"
                },
                &e.to_string(),
                Some(original),
            )
        }
    };
    let translation = resp.translations.into_iter().next().unwrap_or_default();
    emit_result(
        app,
        ScreenshotResult {
            session_id: gen,
            original,
            translation,
            source_lang: resp.detected_source_lang,
            target_lang: cfg.translator.target_lang.clone(),
            engine_id: resp.engine_id,
            region: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            cost: ScreenshotCost {
                capture: 0,
                ocr: 0,
                translate: t.elapsed().as_millis() as u32,
            },
        },
    );
}

/// Capture once at trigger time, before any AX IPC, copy delay or translation.
pub fn selection_cursor_anchor() -> Option<ScreenRect> {
    #[cfg(target_os = "windows")]
    {
        crate::core::cursor::current_physical().map(|p| ScreenRect::point(p.x as f64, p.y as f64))
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::core::cursor::current_logical().map(|(x, y)| ScreenRect::point(x, y))
    }
}

fn position_selection_popup(app: &AppHandle, anchor: Option<ScreenRect>) {
    let Some(selection) = anchor else {
        return position_popup_centered(app);
    };
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        return;
    };
    let cx = selection.x + selection.w / 2.0;
    let cy = selection.y + selection.h / 2.0;
    let monitors = popup.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| {
        #[cfg(target_os = "windows")]
        let scale = 1.0;
        #[cfg(not(target_os = "windows"))]
        let scale = m.scale_factor();
        ScreenRect::new(
            m.position().x as f64 / scale,
            m.position().y as f64 / scale,
            m.size().width as f64 / scale,
            m.size().height as f64 / scale,
        )
        .is_some_and(|r| r.contains(cx, cy))
    });
    let Some(monitor) = monitor else {
        return position_popup_centered(app);
    };
    #[cfg(target_os = "windows")]
    let (coordinate_scale, size_scale) = (1.0, monitor.scale_factor());
    #[cfg(not(target_os = "windows"))]
    let (coordinate_scale, size_scale) = (monitor.scale_factor(), 1.0);
    let work = monitor.work_area();
    let Some(work) = ScreenRect::new(
        work.position.x as f64 / coordinate_scale,
        work.position.y as f64 / coordinate_scale,
        work.size.width as f64 / coordinate_scale,
        work.size.height as f64 / coordinate_scale,
    ) else {
        return;
    };
    let gap = POPUP_GAP * size_scale;
    let position = below_selection(
        selection,
        work,
        popup_width(app) * size_scale,
        POPUP_INIT_HEIGHT * size_scale,
        gap,
    );
    *app.state::<SelectionPopupPosition>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(SelectionPlacement {
        selection,
        monitor: work,
        gap,
        position,
    });
    #[cfg(target_os = "windows")]
    {
        let _ = popup.set_position(tauri::PhysicalPosition::new(
            position.0.round() as i32,
            position.1.round() as i32,
        ));
        let _ = popup.set_size(tauri::PhysicalSize::new(
            (popup_width(app) * size_scale).round() as u32,
            (POPUP_INIT_HEIGHT * size_scale).round() as u32,
        ));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = popup.set_size(LogicalSize::new(popup_width(app), POPUP_INIT_HEIGHT));
        let _ = popup.set_position(LogicalPosition::new(position.0, position.1));
    }
    show_and_focus_popup(app, &popup);
}

/// 把 popup 居中显示在主显示器上 1/3 处(取不到光标位置时的回退定位)。
fn position_popup_centered(app: &AppHandle) {
    *app.state::<SelectionPopupPosition>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        log::warn!("popup window not found; cannot show selection result");
        return;
    };
    let w = popup_width(app);
    let mut x = 200.0;
    let mut y = 200.0;
    if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
        if let Some((lx, ly, lw, lh)) = bounds.into_iter().next() {
            x = lx + (lw - w) / 2.0;
            y = ly + lh / 3.0;
        }
    }
    let _ = popup.set_position(LogicalPosition::new(x, y));
    let _ = popup.set_size(LogicalSize::new(w, POPUP_INIT_HEIGHT));
    show_and_focus_popup(app, &popup);
}

fn is_current(app: &AppHandle, gen: u32) -> bool {
    app.state::<ScreenshotSession>().is_current(gen)
}

/// emit `screenshot://result` 并缓存为「上次结果」(供划词快捷键回显)。内存 + 磁盘各存一份。
fn emit_result(app: &AppHandle, result: ScreenshotResult) {
    if !is_current(app, result.session_id) {
        return;
    }
    let last = app.state::<LastResult>();
    last.set(&result);
    last.persist(app);
    if is_current(app, result.session_id) {
        let _ = app.emit(EVT_RESULT, result);
    }
}

fn emit_progress(app: &AppHandle, gen: u32, stage: &str) {
    if !is_current(app, gen) {
        return;
    }
    let _ = app.emit(
        EVT_PROGRESS,
        ScreenshotProgress {
            session_id: gen,
            stage: stage.to_string(),
        },
    );
}

/// 进入翻译阶段:原文/目标语言/引擎已知,先发一份让 popup 渲染完整布局 + 引擎行转圈。
fn emit_translating(
    app: &AppHandle,
    gen: u32,
    original: &str,
    source_lang: Option<String>,
    target_lang: &str,
    engine_id: &str,
    request_id: Option<String>,
) {
    if !is_current(app, gen) {
        return;
    }
    let _ = app.emit(
        EVT_TRANSLATING,
        ScreenshotTranslating {
            session_id: gen,
            original: original.to_string(),
            source_lang,
            target_lang: target_lang.to_string(),
            engine_id: engine_id.to_string(),
            request_id,
        },
    );
}

fn emit_error(
    app: &AppHandle,
    gen: u32,
    stage: &str,
    code: &str,
    message: &str,
    original: Option<String>,
) {
    if !is_current(app, gen) {
        return;
    }
    let _ = app.emit(
        EVT_ERROR,
        ScreenshotError {
            session_id: gen,
            stage: stage.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            original,
        },
    );
}

#[cfg(target_os = "windows")]
fn position_windows_popup(
    app: &AppHandle,
    mut x: f64,
    mut y: f64,
    anchor_x: f64,
    anchor_y: f64,
    scale: f64,
) {
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        return;
    };
    let width = popup_width(app) * scale;
    let height = POPUP_INIT_HEIGHT * scale;
    if let Ok(monitors) = popup.available_monitors() {
        if let Some(m) = monitors.iter().find(|m| {
            anchor_x >= m.position().x as f64
                && anchor_y >= m.position().y as f64
                && anchor_x < m.position().x as f64 + m.size().width as f64
                && anchor_y < m.position().y as f64 + m.size().height as f64
        }) {
            x = x.clamp(
                m.position().x as f64,
                (m.position().x as f64 + m.size().width as f64 - width).max(m.position().x as f64),
            );
            y = y.clamp(
                m.position().y as f64,
                (m.position().y as f64 + m.size().height as f64 - height)
                    .max(m.position().y as f64),
            );
        }
    }
    let _ = popup.set_position(tauri::PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    ));
    let _ = popup.set_size(tauri::PhysicalSize::new(
        width.round() as u32,
        height.round() as u32,
    ));
    show_and_focus_popup(app, &popup);
}

#[cfg(test)]
mod tests {
    use super::{cached_result_matches, join_blocks};
    use crate::core::ocr::OcrBlock;
    use crate::types::{Rect, ScreenshotCost, ScreenshotResult};

    #[tokio::test]
    async fn superseding_request_drops_in_flight_work() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let session = Arc::new(super::ScreenshotSession::default());
        let gen = session.begin();
        let dropped = Arc::new(AtomicBool::new(false));
        struct PendingGuard(Arc<AtomicBool>);
        impl Drop for PendingGuard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let (started, ready) = tokio::sync::oneshot::channel();
        let worker = session.clone();
        let flag = dropped.clone();
        let task = tokio::spawn(async move {
            worker
                .run(gen, async move {
                    let _guard = PendingGuard(flag);
                    let _ = started.send(());
                    std::future::pending::<()>().await;
                })
                .await
        });
        ready.await.unwrap();
        session.begin();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap(),
            None
        );
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancelled_before_polling_never_starts_work() {
        let session = super::ScreenshotSession::default();
        let gen = session.begin();
        session.cancel();
        assert_eq!(
            session
                .run(gen, async { panic!("stale HTTP request started") })
                .await,
            None::<()>
        );
    }

    #[tokio::test]
    async fn popup_cancel_drops_every_concurrent_service_request() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let session = Arc::new(super::ScreenshotSession::default());
        let gen = session.begin();
        let dropped = Arc::new(AtomicUsize::new(0));
        struct PendingGuard(Arc<AtomicUsize>);
        impl Drop for PendingGuard {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let mut tasks = Vec::new();
        let mut ready = Vec::new();
        for _ in 0..3 {
            let session = session.clone();
            let dropped = dropped.clone();
            let (started, receiver) = tokio::sync::oneshot::channel();
            ready.push(receiver);
            tasks.push(tokio::spawn(async move {
                session
                    .run(gen, async move {
                        let _guard = PendingGuard(dropped);
                        let _ = started.send(());
                        std::future::pending::<()>().await;
                    })
                    .await
            }));
        }
        for receiver in ready {
            receiver.await.unwrap();
        }
        session.cancel();
        for task in tasks {
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(1), task)
                    .await
                    .unwrap()
                    .unwrap(),
                None
            );
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn folded_request_cancelled_before_registration_never_starts() {
        let session = super::ScreenshotSession::default();
        let gen = session.begin();
        session.cancel_request(gen, "folded");
        assert_eq!(
            session
                .run_request(gen, "folded", async {
                    panic!("folded request must not start")
                })
                .await,
            None::<()>
        );
        // A new request for the same service has a different identity.
        assert_eq!(
            session.run_request(gen, "expanded", async { 42 }).await,
            Some(42)
        );
        assert!(session.requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn folding_drops_only_its_request_and_parent_cancel_drops_siblings() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let session = Arc::new(super::ScreenshotSession::default());
        let gen = session.begin();
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for id in ["a", "b"] {
            let worker = session.clone();
            let flag = dropped.clone();
            let (started, ready) = tokio::sync::oneshot::channel();
            tasks.push(tokio::spawn(async move {
                worker
                    .run_request(gen, id, async {
                        let _guard = Guard(flag);
                        let _ = started.send(());
                        std::future::pending::<()>().await;
                    })
                    .await
            }));
            ready.await.unwrap();
        }
        session.cancel_request(gen, "a");
        let a = tasks.remove(0);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), a)
                .await
                .unwrap()
                .unwrap(),
            None
        );
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(!tasks[0].is_finished(), "another service must keep running");
        session.cancel();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), tasks.remove(0))
                .await
                .unwrap()
                .unwrap(),
            None
        );
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        assert!(session.requests.lock().unwrap().is_empty());
        let new_gen = session.begin();
        session.cancel_request(gen, "b");
        assert_eq!(
            session.run_request(new_gen, "b", async { 7 }).await,
            Some(7)
        );
    }

    #[test]
    fn native_primary_respects_per_service_folding_without_changing_default() {
        use crate::config::schema::PopupServiceUiConfig;
        let mut config = crate::config::Config::default();
        for engine in &mut config.translator.engines {
            engine.enabled = false;
        }
        let a = config.translator.engines[0].id.clone();
        let b = config.translator.engines[1].id.clone();
        config.translator.engines[0].enabled = true;
        config.translator.engines[1].enabled = true;
        config.translator.default_engine_id = b.clone();
        assert_eq!(super::popup_engine_ids(&config), [b.as_str(), a.as_str()]);
        config.ui.popup.services.insert(
            b.clone(),
            PopupServiceUiConfig {
                collapsed: true,
                remember_collapse: true,
                skip_when_collapsed: true,
            },
        );
        assert_eq!(super::popup_engine_ids(&config), [a.as_str()]);
        assert_eq!(
            config.translator.default_engine_id, b,
            "overlay default stays configured"
        );
        config
            .ui
            .popup
            .services
            .insert(a.clone(), config.ui.popup.services[&b].clone());
        assert!(super::popup_engine_ids(&config).is_empty());
        config
            .ui
            .popup
            .services
            .get_mut(&b)
            .unwrap()
            .remember_collapse = false;
        assert_eq!(super::popup_engine_ids(&config), [b.as_str()]);
        config
            .ui
            .popup
            .services
            .get_mut(&a)
            .unwrap()
            .skip_when_collapsed = false;
        assert_eq!(super::popup_engine_ids(&config), [b.as_str(), a.as_str()]);
    }

    fn block(text: &str, x: i32, y: i32, w: i32, h: i32) -> OcrBlock {
        OcrBlock {
            text: text.into(),
            bbox: Rect { x, y, w, h },
            confidence: 1.0,
        }
    }

    #[test]
    fn joins_same_line_with_space() {
        // 两块大致同一基线 → 空格相连。
        let blocks = vec![
            block("Hello", 0, 100, 50, 20),
            block("World", 60, 102, 50, 20),
        ];
        assert_eq!(join_blocks(&blocks), "Hello World");
    }

    #[test]
    fn joins_next_line_with_newline() {
        // 第二块明显下移 → 换行相连。
        let blocks = vec![
            block("Line1", 0, 100, 50, 20),
            block("Line2", 0, 140, 50, 20),
        ];
        assert_eq!(join_blocks(&blocks), "Line1\nLine2");
    }

    #[test]
    fn keeps_blank_line_between_paragraphs() {
        // 行间空隙(top2-bottom1)超过一整行高 → 段落间保留空行。
        // 行高 20:紧邻行 gap=20(=行高)仍是单换行;此处 gap=40>20 → 空行。
        let blocks = vec![
            block("Para1", 0, 100, 50, 20),
            block("Para2", 0, 160, 50, 20),
        ];
        assert_eq!(join_blocks(&blocks), "Para1\n\nPara2");
    }

    #[test]
    fn skips_empty_blocks() {
        let blocks = vec![block("A", 0, 0, 10, 10), block("   ", 20, 0, 10, 10)];
        assert_eq!(join_blocks(&blocks), "A");
    }

    #[test]
    fn cache_key_includes_target_language_and_engine() {
        let result = ScreenshotResult {
            session_id: 0,
            original: "hello".into(),
            translation: "你好".into(),
            source_lang: Some("en".into()),
            target_lang: "zh-CN".into(),
            engine_id: "engine-a".into(),
            region: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            cost: ScreenshotCost {
                capture: 0,
                ocr: 0,
                translate: 1,
            },
        };
        assert!(cached_result_matches(
            &result,
            "hello",
            "zh-CN",
            Some("engine-a")
        ));
        assert!(!cached_result_matches(
            &result,
            "hello",
            "ja",
            Some("engine-a")
        ));
        assert!(!cached_result_matches(
            &result,
            "hello",
            "zh-CN",
            Some("engine-b")
        ));
        assert!(!cached_result_matches(
            &result,
            "different",
            "zh-CN",
            Some("engine-a")
        ));
    }
}
