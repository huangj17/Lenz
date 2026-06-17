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
    ScreenshotResult, TranslateRequest, TriggerKind,
};

const POPUP_LABEL: &str = "popup";
/// popup 窗口逻辑宽度(前端 `resize_popup` 命令复用,见 `commands::screenshot`)。
pub const POPUP_WIDTH: f64 = 360.0;
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
const EVT_RESULT: &str = "screenshot://result";
const EVT_ERROR: &str = "screenshot://error";

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
/// 用于划词快捷键的「无新选区则回显上次内容」(见 `on_selection_shortcut`)。
#[derive(Default)]
pub struct LastResult(Mutex<Option<ScreenshotResult>>);

impl LastResult {
    fn set(&self, r: &ScreenshotResult) {
        if let Ok(mut g) = self.0.lock() {
            *g = Some(r.clone());
        }
    }
    fn get(&self) -> Option<ScreenshotResult> {
        self.0.lock().ok().and_then(|g| g.clone())
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
    emit_progress(app, "translate");
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

    let mut x = region.x as f64 + region.w as f64 / 2.0 - POPUP_WIDTH / 2.0;
    let mut y = region.y as f64 + region.h as f64 + POPUP_GAP;

    // 夹紧到包含选区中心的显示器工作区;放不下则改置选区上方。
    if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
        let cx = region.x as f64 + region.w as f64 / 2.0;
        let cy = region.y as f64 + region.h as f64 / 2.0;
        if let Some((lx, ly, lw, lh)) = bounds
            .into_iter()
            .find(|(mx, my, mw, mh)| cx >= *mx && cx < mx + mw && cy >= *my && cy < my + mh)
        {
            x = x.clamp(lx, (lx + lw - POPUP_WIDTH).max(lx));
            if y + POPUP_INIT_HEIGHT > ly + lh {
                y = (region.y as f64 - POPUP_INIT_HEIGHT - POPUP_GAP).max(ly);
            }
        }
    }

    let _ = popup.set_position(LogicalPosition::new(x, y));
    let _ = popup.set_size(LogicalSize::new(POPUP_WIDTH, POPUP_INIT_HEIGHT));
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

    let Ok(bounds) = selection_window::logical_monitor_bounds(app) else {
        return;
    };
    // 选包含弹框中心的显示器;落在缝隙里则兜底用第一块。
    let cx = pos.x + POPUP_WIDTH / 2.0;
    let cy = pos.y + height / 2.0;
    let Some((mx, my, mw, mh)) = bounds
        .iter()
        .find(|(mx, my, mw, mh)| cx >= *mx && cx < mx + mw && cy >= *my && cy < my + mh)
        .or_else(|| bounds.first())
        .copied()
    else {
        return;
    };

    let x = pos.x.clamp(mx, (mx + mw - POPUP_WIDTH).max(mx));
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

async fn handle_selection_shortcut(app: AppHandle) {
    let before = app.clipboard().read_text().unwrap_or_default();

    // 合成复制按键(macOS 需「辅助功能」权限;失败仅记日志,后续靠剪贴板比较退化)。
    if let Err(e) = keystroke::copy_selection() {
        log::warn!("synthesize copy keystroke failed: {e}");
    }

    // 轮询等待系统把选中文本写入剪贴板(写入是异步的)。
    let after = wait_clipboard_change(&app, &before).await;
    let selection = after.trim();

    // 有新选区:翻译之(长度上限 5000,spec 05 §4)。
    if !selection.is_empty() && selection != before.trim() {
        let text: String = selection.chars().take(5000).collect();
        let gen = app.state::<ScreenshotSession>().begin();
        position_popup_centered(&app);
        let _ = tokio::time::timeout(TOTAL_TIMEOUT, run_text_pipeline(&app, text, gen)).await;
        return;
    }

    // 无新选区:回显上次结果(spec 05 §3 —— 再次按键复现上次内容)。
    if let Some(last) = app.state::<LastResult>().get() {
        app.state::<ScreenshotSession>().begin();
        position_popup_centered(&app);
        let _ = app.emit(EVT_RESULT, last);
        return;
    }

    // 既无新选区也无历史:回退翻译当前剪贴板内容(首次可用)。
    if !selection.is_empty() {
        let text: String = selection.chars().take(5000).collect();
        let gen = app.state::<ScreenshotSession>().begin();
        position_popup_centered(&app);
        let _ = tokio::time::timeout(TOTAL_TIMEOUT, run_text_pipeline(&app, text, gen)).await;
    } else {
        log::debug!("selection shortcut: no selection, no clipboard, no history; ignored");
    }
}

/// 轮询剪贴板直到内容相对 `before` 变化或超时;返回最终读到的内容。
async fn wait_clipboard_change(app: &AppHandle, before: &str) -> String {
    let deadline = Instant::now() + COPY_POLL_TIMEOUT;
    loop {
        let cur = app.clipboard().read_text().unwrap_or_default();
        if cur != before && !cur.trim().is_empty() {
            return cur;
        }
        if Instant::now() >= deadline {
            return cur;
        }
        tokio::time::sleep(COPY_POLL_INTERVAL).await;
    }
}

/// 纯文本翻译 pipeline(F1 快捷键复用 F2 的翻译 + 事件,无截图 / OCR 阶段)。
async fn run_text_pipeline(app: &AppHandle, original: String, gen: u64) {
    let cfg = match app.state::<ConfigStore>().get() {
        Ok(c) => c,
        Err(e) => return emit_error(app, "translate", "failed", &e.to_string(), Some(original)),
    };
    emit_progress(app, "translate");
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

/// 把 popup 居中显示在主显示器上 1/3 处(划词快捷键模式的 MVP 定位;
/// 光标锚定见 M5 §7)。
fn position_popup_centered(app: &AppHandle) {
    let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
        log::warn!("popup window not found; cannot show selection result");
        return;
    };
    let mut x = 200.0;
    let mut y = 200.0;
    if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
        if let Some((lx, ly, lw, lh)) = bounds.into_iter().next() {
            x = lx + (lw - POPUP_WIDTH) / 2.0;
            y = ly + lh / 3.0;
        }
    }
    let _ = popup.set_position(LogicalPosition::new(x, y));
    let _ = popup.set_size(LogicalSize::new(POPUP_WIDTH, POPUP_INIT_HEIGHT));
    let _ = popup.show();
    let _ = popup.set_focus();
}

fn is_current(app: &AppHandle, gen: u64) -> bool {
    app.state::<ScreenshotSession>().is_current(gen)
}

/// emit `screenshot://result` 并缓存为「上次结果」(供划词快捷键回显)。
fn emit_result(app: &AppHandle, result: ScreenshotResult) {
    app.state::<LastResult>().set(&result);
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
