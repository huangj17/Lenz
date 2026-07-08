//! 截图翻译 F2 流程编排(spec 06)。
//!
//! 由 `setup.rs` 订阅 `capture://region-selected` 触发:销毁选区窗口后,在一个
//! async task 中串行执行 截图 → OCR → 翻译,并在关键节点 emit
//! `screenshot://progress` / `screenshot://result` / `screenshot://error`,popup 窗口订阅渲染。
//!
//! 取消(spec 06 §7):用自增**会话代号**实现——新触发 / 用户 ESC 都会 `begin` 一个
//! 更大的代号,运行中的旧流程在每个 await 点检查 `is_current`,不再当前即静默放弃。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::config::ConfigStore;
use crate::error::LenzError;
use crate::core::ocr::{OcrBlock, OcrEngine, OcrInput, OcrOptions};
use crate::core::translator::TranslatorManager;
use crate::core::{capture, keystroke, selection_window};
use crate::types::{
    RegionSelectedPayload, Rect, ScreenshotCost, ScreenshotError, ScreenshotProgress,
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
/// 整体超时(spec 06 §4)。
const TOTAL_TIMEOUT: Duration = Duration::from_millis(10_000);
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

/// F2 会话代号(spec 06 §7)。作为 Tauri managed state 注入。
#[derive(Default)]
pub struct ScreenshotSession {
    generation: AtomicU64,
}

impl ScreenshotSession {
    /// 开启新会话并返回其代号(令所有旧会话失效)。
    pub fn begin(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }
    /// 该代号是否仍是当前会话。
    pub fn is_current(&self, gen: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == gen
    }
    /// 取消当前会话(令运行中的流程在下个检查点放弃)。
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// 最近一次成功的翻译结果(F1/F2 通用)。作为 managed state 注入。
///
/// 用于划词快捷键的「无新选区则回显上次内容」(见 `on_selection_shortcut`)。内存态之外
/// 还持久化到磁盘(`last-result.json`),使**应用重启后**按 Alt+D(无选区/剪贴板空)仍能
/// 回显上次结果。
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
                log::info!("已加载上次翻译结果(供 Alt+D 回显)");
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
        // 原图覆盖在 M6 接续;此处仅关窗。
        TriggerKind::Overlay => log::debug!("overlay region selected (pipeline: M6)"),
        TriggerKind::Selection => {}
    }
}

/// 启动 F2:校验选区 → 立即显示 popup(loading)→ spawn 流程(带总超时)。
fn start_f2(app: AppHandle, region: Rect) {
    // w<4 || h<4 视为点击取消(spec 06 §2.2)。
    if region.w < 4 || region.h < 4 {
        log::debug!("selection too small ({}x{}), treated as cancel", region.w, region.h);
        return;
    }

    let gen = app.state::<ScreenshotSession>().begin();
    position_and_show_popup(&app, &region);

    tauri::async_runtime::spawn(async move {
        match tokio::time::timeout(TOTAL_TIMEOUT, run_pipeline(&app, region, gen)).await {
            Ok(()) => {}
            Err(_) => {
                if app.state::<ScreenshotSession>().is_current(gen) {
                    emit_error(&app, "timeout", "timeout", "response timed out", None);
                }
            }
        }
    });
}

/// 截图 → OCR → 翻译,逐阶段 emit 进度 / 结果 / 错误。
async fn run_pipeline(app: &AppHandle, region: Rect, gen: u64) {
    // 首帧投递前等待 popup webview 就绪,避免冷启动首次截图丢掉 progress/result(方案 A)。
    // popup 已在 `on_region_selected` 同步 show;此处仅门控随后的 emit。
    wait_popup_ready(app).await;

    let cfg = match app.state::<ConfigStore>().get() {
        Ok(c) => c,
        Err(e) => return emit_error(app, "capture", "failed", &e.to_string(), None),
    };

    // ── 截图 ──
    emit_progress(app, "capture");
    // 等待选区蒙层真正从屏幕消失再抓屏:close/hide 是投递到主线程的异步操作,
    // 立即抓屏会截到遮罩或销毁残帧,导致 OCR「未识别到文字」(spec 02 §7)。
    tokio::time::sleep(Duration::from_millis(OVERLAY_CLEAR_DELAY_MS)).await;
    let t0 = Instant::now();
    let cap = tokio::task::spawn_blocking(move || capture::capture_region(region)).await;
    if !is_current(app, gen) {
        return;
    }
    let cap = match cap {
        Ok(Ok(c)) => c,
        Ok(Err(LenzError::Permission(m))) => {
            return emit_error(app, "capture", "permission", &m, None)
        }
        Ok(Err(e)) => return emit_error(app, "capture", "failed", &e.to_string(), None),
        Err(e) => return emit_error(app, "capture", "failed", &format!("join: {e}"), None),
    };
    let cost_capture = t0.elapsed().as_millis() as u32;

    // ── OCR ──
    emit_progress(app, "ocr");
    let t1 = Instant::now();
    let engine = app.state::<Arc<dyn OcrEngine>>().inner().clone();
    let opts = OcrOptions {
        preferred_languages: cfg.ocr.preferred_languages.clone(),
        fast_mode: false,
        min_confidence: cfg.ocr.min_confidence,
    };
    let input = OcrInput {
        png: cap.png,
        pixel_rect: cap.pixel_rect,
        scale_factor: cap.scale_factor,
    };
    let ocr = engine.recognize(input, opts).await;
    if !is_current(app, gen) {
        return;
    }
    let ocr = match ocr {
        Ok(r) => r,
        Err(LenzError::Permission(m)) => return emit_error(app, "ocr", "permission", &m, None),
        Err(e) => return emit_error(app, "ocr", "failed", &e.to_string(), None),
    };
    let original = join_blocks(&ocr.blocks);
    if original.trim().is_empty() {
        return emit_error(app, "ocr", "no_text", "no text recognized", None);
    }
    let cost_ocr = t1.elapsed().as_millis() as u32;

    // ── 翻译 ──
    // 原文(OCR 结果)已知:发 translating,让 popup 显示原文 + 引擎行转圈,而非整块骨架。
    emit_translating(
        app,
        &original,
        ocr.detected_language.clone(),
        &cfg.translator.target_lang,
        &cfg.translator.default_engine_id,
    );
    let t2 = Instant::now();
    let translator = {
        let mgr = app.state::<TranslatorManager>();
        mgr.get(None)
    };
    let Some(translator) = translator else {
        return emit_error(app, "translate", "failed", "no engine configured", Some(original));
    };
    let req = TranslateRequest {
        texts: vec![original.clone()],
        source_lang: ocr.detected_language.clone(),
        target_lang: cfg.translator.target_lang.clone(),
        engine_id: None,
    };
    let resp = translator.translate(req).await;
    if !is_current(app, gen) {
        return;
    }
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return emit_error(app, "translate", "failed", &e.to_string(), Some(original))
        }
    };
    let cost_translate = t2.elapsed().as_millis() as u32;

    let translation = resp.translations.into_iter().next().unwrap_or_default();
    emit_result(
        app,
        ScreenshotResult {
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

/// 把 OCR 文本块按阅读顺序拼接(同一行用空格,换行用 `\n`)(spec 06 §2.4)。
fn join_blocks(blocks: &[OcrBlock]) -> String {
    let mut out = String::new();
    let mut prev_mid_y: Option<i32> = None;
    for b in blocks {
        let text = b.text.trim();
        if text.is_empty() {
            continue;
        }
        let mid_y = b.bbox.y + b.bbox.h / 2;
        // 行高的一半作为同行判定阈值。
        let same_line = match prev_mid_y {
            Some(py) => (mid_y - py).abs() <= (b.bbox.h / 2).max(6),
            None => false,
        };
        if !out.is_empty() {
            out.push_str(if same_line { " " } else { "\n" });
        }
        out.push_str(text);
        prev_mid_y = Some(mid_y);
    }
    out
}

/// 把 popup 定位到选区正下方(水平居中),夹紧到所在显示器,然后显示并聚焦。
fn position_and_show_popup(app: &AppHandle, region: &Rect) {
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
    let _ = popup.show();
    let _ = popup.set_focus();
}

/// 内容渲染后 `resize_popup` 调用:按新高度把 popup 重新夹紧到所在显示器内。
///
/// 初次定位只按 `POPUP_INIT_HEIGHT` 摆放,长译文撑高后(最高 480)若沿用旧的左上角,
/// 底部会溢出屏幕、内容被截断(spec 06 §2.6)。这里读当前位置 + 新高度,把窗口夹回
/// 显示器矩形内(放不下时顶边贴齐显示器顶部),优先保证内容完整可见。
pub fn clamp_popup_into_monitor(app: &AppHandle, height: f64) {
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        return;
    };
    let scale = popup.scale_factor().unwrap_or(1.0);
    let Ok(phys) = popup.outer_position() else {
        return;
    };
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

/// 选中文本送入剪贴板后,等待系统写入完成的轮询间隔与上限。
const COPY_POLL_INTERVAL: Duration = Duration::from_millis(20);
const COPY_POLL_TIMEOUT: Duration = Duration::from_millis(180);

/// 合成 Cmd+C 前等待触发快捷键的修饰键松开的轮询间隔与上限(spec 05 §3)。
const MOD_WAIT_INTERVAL: Duration = Duration::from_millis(10);
const MOD_WAIT_TIMEOUT: Duration = Duration::from_millis(400);

/// 划词翻译(快捷键模式):自动复制前台选中文本 → 翻译 → popup。
///
/// 流程(spec 05 §3):
/// 1. 记录复制前剪贴板;向前台应用合成 `Cmd/Ctrl+C` 复制当前选区;
/// 2. 轮询等待剪贴板变化,取得「选中文本」;
/// 3. 有新选区 → 居中显示 popup + 翻译;
/// 4. 无新选区(未选中 / 无权限)→ 回显上次结果;都没有则回退到当前剪贴板内容。
pub fn on_selection_shortcut(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        handle_selection_shortcut(app).await;
    });
}

/// 直接用已拿到的选中文本翻译(划词图标点击时,复用图标浮出瞬间 AX 预读的缓存)。
/// 跳过实时读取,规避「点击图标夺焦后读到错内容」。
pub fn translate_with_text(app: &AppHandle, text: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        translate_selection(&app, text).await;
    });
}

async fn handle_selection_shortcut(app: AppHandle) {
    // ① 首选 Accessibility API 直读选中文本:不碰剪贴板、不合成按键,规避修饰键污染 /
    //    同文字重复 / 焦点时序 / 剪贴板被覆盖等一系列问题(spec 05 §3)。AX 调用可能对
    //    无响应目标应用造成同步 IPC 阻塞,放到 spawn_blocking 上跑,避免堵塞 async 运行时。
    let ax = tokio::task::spawn_blocking(crate::core::selection_text::read_selected_text)
        .await
        .ok()
        .flatten();
    if let Some(text) = ax {
        log::info!("划词:AX 直读命中 {} 字", text.chars().count());
        return translate_selection(&app, text).await;
    }

    // ② 兜底(AX 取不到,如部分 Chrome/Electron):合成 Cmd+C 读剪贴板。
    //    Alt+D 触发时 Option 等修饰键往往仍按着,会并入合成的 Cmd+C(→ ⌘⌥C,前台不复制),
    //    先等其松开;再用哨兵串法判定是否真复制到选区(不靠与旧剪贴板差异,规避同文字重复)。
    wait_modifiers_released().await;
    if let Some(text) = copy_selection_via_clipboard(&app).await {
        log::info!("划词:剪贴板兜底命中 {} 字", text.chars().count());
        return translate_selection(&app, text).await;
    }

    // ③ AX 与剪贴板都拿不到选区:回显上次结果(spec 05 §3 —— 再次按键复现上次内容)。
    if let Some(last) = app.state::<LastResult>().get() {
        log::info!("划词:无新选区,回显上次结果");
        app.state::<ScreenshotSession>().begin();
        position_popup_at_cursor(&app);
        wait_popup_ready(&app).await;
        let _ = app.emit(EVT_RESULT, last);
    } else {
        log::info!("划词:AX 空 + 剪贴板无选区 + 无历史;忽略(未选中文本或无权限)");
    }
}

/// 拿到选中文本后的统一收尾:截断 5000 字(spec 05 §4)→ 新会话 + 光标处弹框 + 翻译。
async fn translate_selection(app: &AppHandle, text: String) {
    let text: String = text.chars().take(5000).collect();
    let gen = app.state::<ScreenshotSession>().begin();
    position_popup_at_cursor(app);
    // 首帧投递前等待 popup webview 就绪,避免冷启动首次触发丢掉 progress/result(方案 A)。
    wait_popup_ready(app).await;
    let _ = tokio::time::timeout(TOTAL_TIMEOUT, run_text_pipeline(app, text, gen)).await;
}

/// 合成 Cmd+C 前等待触发快捷键的修饰键松开,避免被仍按住的 Option/Cmd 等并入合成事件
/// (spec 05 §3)。无修饰键按下时立即返回;最多等 `MOD_WAIT_TIMEOUT`,超时则尽力继续。
async fn wait_modifiers_released() {
    let deadline = Instant::now() + MOD_WAIT_TIMEOUT;
    while keystroke::modifiers_held() {
        if Instant::now() >= deadline {
            log::debug!("selection shortcut: 修饰键 {MOD_WAIT_TIMEOUT:?} 内未松开,继续合成 Cmd+C");
            break;
        }
        tokio::time::sleep(MOD_WAIT_INTERVAL).await;
    }
}

/// 兜底:合成 Cmd+C 取前台选区(AX 读不到时用)。先写入唯一**哨兵串**占位,再合成
/// Cmd+C;若随后剪贴板 != 哨兵,说明前台确实复制了选区(即便它恰好等于用户原剪贴板,
/// 也不会像「与旧值比对」那样把重复同字误判为无选区);读毕**还原原剪贴板**(不留痕)。
/// 返回选中文本;没复制到内容(无选区 / 无权限 / 修饰键仍污染)则返回 `None`。
async fn copy_selection_via_clipboard(app: &AppHandle) -> Option<String> {
    // 原剪贴板(可能非文本 → None,则无法精确还原,属少见边角)。
    let original = app.clipboard().read_text().ok();
    // 不可见字符包裹的哨兵,极不可能与真实选区相同。
    const SENTINEL: &str = "\u{2063}lenz-selection-probe\u{2063}";
    if app.clipboard().write_text(SENTINEL.to_string()).is_err() {
        return None;
    }

    if let Err(e) = keystroke::copy_selection() {
        log::warn!("synthesize copy keystroke failed: {e}");
    }

    // 轮询等前台把选区写入剪贴板(写入是异步的);超时仍是哨兵 → 没复制到东西。
    let deadline = Instant::now() + COPY_POLL_TIMEOUT;
    let copied = loop {
        let cur = app.clipboard().read_text().unwrap_or_default();
        if cur != SENTINEL {
            break Some(cur);
        }
        if Instant::now() >= deadline {
            break None;
        }
        tokio::time::sleep(COPY_POLL_INTERVAL).await;
    };

    // 还原原剪贴板(贯彻「不主动改剪贴板」)。原剪贴板非文本(None)时无法精确还原,
    // 但若此刻剪贴板仍是我们写入的哨兵(说明没复制到选区),必须清掉,避免把哨兵留给用户。
    match original {
        Some(s) => {
            let _ = app.clipboard().write_text(s);
        }
        None => {
            if app.clipboard().read_text().ok().as_deref() == Some(SENTINEL) {
                let _ = app.clipboard().write_text(String::new());
            }
        }
    }

    let text = copied?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 纯文本翻译 pipeline(F1 快捷键复用 F2 的翻译 + 事件,无截图 / OCR 阶段)。
async fn run_text_pipeline(app: &AppHandle, original: String, gen: u64) {
    let cfg = match app.state::<ConfigStore>().get() {
        Ok(c) => c,
        Err(e) => return emit_error(app, "translate", "failed", &e.to_string(), Some(original)),
    };
    // 原文已知:发 translating,让 popup 立即显示原文 + 引擎行转圈(F1 无截图/OCR 阶段)。
    emit_translating(
        app,
        &original,
        None,
        &cfg.translator.target_lang,
        &cfg.translator.default_engine_id,
    );
    let t = Instant::now();
    let translator = {
        let mgr = app.state::<TranslatorManager>();
        mgr.get(None)
    };
    let Some(translator) = translator else {
        return emit_error(app, "translate", "failed", "no engine configured", Some(original));
    };
    let req = TranslateRequest {
        texts: vec![original.clone()],
        source_lang: None,
        target_lang: cfg.translator.target_lang.clone(),
        engine_id: None,
    };
    let resp = translator.translate(req).await;
    if !is_current(app, gen) {
        return;
    }
    let resp = match resp {
        Ok(r) => r,
        Err(e) => return emit_error(app, "translate", "failed", &e.to_string(), Some(original)),
    };
    let translation = resp.translations.into_iter().next().unwrap_or_default();
    emit_result(
        app,
        ScreenshotResult {
            original,
            translation,
            source_lang: resp.detected_source_lang,
            target_lang: cfg.translator.target_lang.clone(),
            engine_id: resp.engine_id,
            region: Rect { x: 0, y: 0, w: 0, h: 0 },
            cost: ScreenshotCost {
                capture: 0,
                ocr: 0,
                translate: t.elapsed().as_millis() as u32,
            },
        },
    );
}

/// 划词弹框定位:锚定到当前光标右下方(+16,+16),按 popup 估算尺寸夹紧到所在
/// 显示器工作区(spec 05 §7)。取不到光标位置(非 macOS / 失败)时回退居中。
fn position_popup_at_cursor(app: &AppHandle) {
    let Some((cx, cy)) = crate::core::cursor::current_logical() else {
        return position_popup_centered(app);
    };
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        log::warn!("popup window not found; cannot show selection result");
        return;
    };
    let w = popup_width(app);
    let mut x = cx + 16.0;
    let mut y = cy + 16.0;
    if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
        if let Some((lx, ly, lw, lh)) = bounds
            .into_iter()
            .find(|(mx, my, mw, mh)| cx >= *mx && cx < mx + mw && cy >= *my && cy < my + mh)
        {
            x = x.clamp(lx, (lx + lw - w).max(lx));
            y = y.clamp(ly, (ly + lh - POPUP_INIT_HEIGHT).max(ly));
        }
    }
    let _ = popup.set_position(LogicalPosition::new(x, y));
    let _ = popup.set_size(LogicalSize::new(w, POPUP_INIT_HEIGHT));
    let _ = popup.show();
    let _ = popup.set_focus();
}

/// 把 popup 居中显示在主显示器上 1/3 处(取不到光标位置时的回退定位)。
fn position_popup_centered(app: &AppHandle) {
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
    let _ = popup.show();
    let _ = popup.set_focus();
}

fn is_current(app: &AppHandle, gen: u64) -> bool {
    app.state::<ScreenshotSession>().is_current(gen)
}

/// emit `screenshot://result` 并缓存为「上次结果」(供划词快捷键回显)。内存 + 磁盘各存一份。
fn emit_result(app: &AppHandle, result: ScreenshotResult) {
    let last = app.state::<LastResult>();
    last.set(&result);
    last.persist(app);
    let _ = app.emit(EVT_RESULT, result);
}

fn emit_progress(app: &AppHandle, stage: &str) {
    let _ = app.emit(
        EVT_PROGRESS,
        ScreenshotProgress {
            stage: stage.to_string(),
        },
    );
}

/// 进入翻译阶段:原文/目标语言/引擎已知,先发一份让 popup 渲染完整布局 + 引擎行转圈。
fn emit_translating(
    app: &AppHandle,
    original: &str,
    source_lang: Option<String>,
    target_lang: &str,
    engine_id: &str,
) {
    let _ = app.emit(
        EVT_TRANSLATING,
        ScreenshotTranslating {
            original: original.to_string(),
            source_lang,
            target_lang: target_lang.to_string(),
            engine_id: engine_id.to_string(),
        },
    );
}

fn emit_error(app: &AppHandle, stage: &str, code: &str, message: &str, original: Option<String>) {
    let _ = app.emit(
        EVT_ERROR,
        ScreenshotError {
            stage: stage.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            original,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::join_blocks;
    use crate::core::ocr::OcrBlock;
    use crate::types::Rect;

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
        let blocks = vec![block("Hello", 0, 100, 50, 20), block("World", 60, 102, 50, 20)];
        assert_eq!(join_blocks(&blocks), "Hello World");
    }

    #[test]
    fn joins_next_line_with_newline() {
        // 第二块明显下移 → 换行相连。
        let blocks = vec![block("Line1", 0, 100, 50, 20), block("Line2", 0, 140, 50, 20)];
        assert_eq!(join_blocks(&blocks), "Line1\nLine2");
    }

    #[test]
    fn skips_empty_blocks() {
        let blocks = vec![block("A", 0, 0, 10, 10), block("   ", 20, 0, 10, 10)];
        assert_eq!(join_blocks(&blocks), "A");
    }
}
