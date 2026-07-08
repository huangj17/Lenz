//! 划词指示图标窗口(spec 05 §6 / 设计调整 2026-06-18)。
//!
//! 监听线程检测到划词/双击手势后(`selection://detected`),由 setup 直接调 [`show_at`]
//! 在光标处浮出一个 Lens 小图标(静态窗口 `indicator`,运行时定位 + show/hide 复用);
//! **不读剪贴板**,以免每次手势都合成 Cmd+C 干扰用户剪贴板。用户 hover/点击图标 → 前端
//! invoke `trigger_selection_translate` → 合成 Cmd+C 复制选区 → 翻译并弹框。
//!
//! 维护一份「当前是否可见 + 图标屏幕矩形」的全局态:监听线程在每次 mouseDown 时据此
//! 判断——点击落在图标外即 emit `selection://dismiss` 让图标**立刻**消失(失去选区即隐藏),
//! 落在图标内则放行点击(触发翻译)。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager};

use crate::core::selection_window;

const LABEL: &str = "indicator";
/// 图标窗口边长(逻辑像素),与 tauri.conf.json 一致。
const SIZE: f64 = 36.0;
/// 相对光标(选区松开点)的偏移:略靠右下,避免盖住刚选中的文字。
const OFFSET_X: f64 = 6.0;
const OFFSET_Y: f64 = 16.0;
/// 无操作自动隐藏时延(毫秒;主要消失路径是「点别处即隐藏」,此为兜底)。
const AUTO_HIDE_MS: u64 = 2500;
/// 检测去抖时延下限(毫秒):双击/三击等连续点击会逐下触发检测,等这段时间内无新点击
/// 才真正浮出图标 + 预读,避免用「中间一下」的半截选区(如三击第 2 下的单词)翻译。
/// 实际时延取 `max(此下限, config.selection.popupDelayMs)`(spec 05 §4「去抖 popupDelayMs」),
/// 下限保证略大于 macOS 常见多击间隔,即便用户把 popupDelayMs 调得很小也能合并多击。
const DETECT_DEBOUNCE_FLOOR_MS: u64 = 220;

/// 检测去抖代号:每次划词手势自增;去抖任务据此判断期间是否又来了新点击。
static DETECT_GEN: AtomicU64 = AtomicU64::new(0);

/// 图标当前是否可见(供监听线程在 mouseDown 时快速判断)。
static VISIBLE: AtomicBool = AtomicBool::new(false);
/// 图标当前屏幕矩形 `(x, y, w, h)`(逻辑坐标);命中测试用。
static RECT: Mutex<(f64, f64, f64, f64)> = Mutex::new((0.0, 0.0, 0.0, 0.0));
/// 图标浮出瞬间(源应用仍在前台)AX 直读到的选中文本缓存。
///
/// 点击图标会让 Lenz 夺焦,之后再读系统级焦点元素的 `AXSelectedText` 就会读到错的内容
/// (读到 Lenz 自己的窗口)。故在检测到划词手势、图标浮出时就把选中文本抓下来存这里,
/// 点击时优先取用;取不到再退回实时兜底。AX 直读不碰剪贴板,不违反「不在每次手势合成
/// Cmd+C」的约束(spec 05 §6)。
static CAPTURED: Mutex<Option<String>> = Mutex::new(None);

/// 图标显示会话代号:每次 `show_at` 自增,自动隐藏任务据此判断是否仍当前
/// (重新定位 / 用户触发 / 主动隐藏都会令旧的隐藏任务失效)。作为 managed state 注入。
#[derive(Default)]
pub struct IndicatorSession {
    generation: AtomicU64,
}

impl IndicatorSession {
    fn begin(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }
    fn is_current(&self, gen: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == gen
    }
}

/// 图标当前是否可见。
pub fn is_visible() -> bool {
    VISIBLE.load(Ordering::SeqCst)
}

/// 点 `(x, y)`(逻辑坐标)是否落在当前图标矩形内(不可见时恒 false)。
pub fn hit_test(x: f64, y: f64) -> bool {
    if !VISIBLE.load(Ordering::SeqCst) {
        return false;
    }
    let (rx, ry, rw, rh) = *RECT.lock().unwrap();
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

/// 取走并清空缓存的选中文本(点击图标触发翻译时消费)。
pub fn take_captured() -> Option<String> {
    CAPTURED.lock().unwrap().take()
}

/// 去抖版的图标显示:检测到划词手势后调用。等 `DETECT_DEBOUNCE_MS` 内无新点击,才真正
/// 浮出图标并预读选区——把双击/三击等连续点击合并成一次,只用最终选区(避免用三击第
/// 2 下的半截单词翻译)。位置用最后一次手势的落点。
pub fn show_debounced(app: &AppHandle, x: f64, y: f64) {
    let my = DETECT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    // 去抖时延 = max(下限, 配置的 popupDelayMs);读配置失败则用下限兜底。
    let delay = app
        .state::<crate::config::ConfigStore>()
        .get()
        .map(|c| (c.selection.popup_delay_ms as u64).max(DETECT_DEBOUNCE_FLOOR_MS))
        .unwrap_or(DETECT_DEBOUNCE_FLOOR_MS);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(delay)).await;
        // 期间又来了新点击(多击未结束)→ 作废本次,交由更晚的手势处理。
        if DETECT_GEN.load(Ordering::SeqCst) != my {
            return;
        }
        let gen = show_at(&app, x, y);
        capture_selection_async(&app, gen);
    });
}

/// 图标浮出后异步 AX 直读当前选中文本并缓存(源应用此刻仍在前台)。
///
/// AX 调用可能对无响应目标应用同步阻塞,放 `spawn_blocking`。仅当仍是 `gen` 对应的
/// 图标会话时才写入,避免旧会话的迟到结果污染新会话(如快速连续两次划词)。
pub fn capture_selection_async(app: &AppHandle, gen: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let text = tokio::task::spawn_blocking(crate::core::selection_text::read_selected_text)
            .await
            .ok()
            .flatten();
        if let Some(t) = text {
            if app.state::<IndicatorSession>().is_current(gen) {
                log::info!("划词图标:浮出时 AX 预读命中 {} 字", t.chars().count());
                *CAPTURED.lock().unwrap() = Some(t);
            }
        }
    });
}

/// 在 `(x, y)`(全局逻辑坐标,选区松开点)附近显示划词指示图标;`AUTO_HIDE_MS`
/// 内无操作则自动隐藏。返回本次显示的会话代号(供 `capture_selection_async` 门控)。
pub fn show_at(app: &AppHandle, x: f64, y: f64) -> u64 {
    // 新会话:清空上次遗留的预读缓存,避免跨会话误用。
    *CAPTURED.lock().unwrap() = None;
    let Some(win) = app.get_webview_window(LABEL) else {
        log::warn!("indicator window not found");
        return 0;
    };

    let mut px = x + OFFSET_X;
    let mut py = y + OFFSET_Y;
    // 夹紧到光标所在显示器工作区内,避免图标越出屏幕。
    if let Ok(bounds) = selection_window::logical_monitor_bounds(app) {
        if let Some((lx, ly, lw, lh)) = bounds
            .into_iter()
            .find(|(mx, my, mw, mh)| x >= *mx && x < mx + mw && y >= *my && y < my + mh)
        {
            px = px.clamp(lx, (lx + lw - SIZE).max(lx));
            py = py.clamp(ly, (ly + lh - SIZE).max(ly));
        }
    }

    let _ = win.set_size(LogicalSize::new(SIZE, SIZE));
    let _ = win.set_position(LogicalPosition::new(px, py));
    let _ = win.show();
    // 注意:不调用 set_focus —— 保持用户原应用为前台,不打断用户。
    *RECT.lock().unwrap() = (px, py, SIZE, SIZE);
    VISIBLE.store(true, Ordering::SeqCst);

    // 自动隐藏(带会话代号:重定位/触发/主动隐藏会令此任务失效)。
    let gen = app.state::<IndicatorSession>().begin();
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(AUTO_HIDE_MS)).await;
        if app2.state::<IndicatorSession>().is_current(gen) {
            hide(&app2);
        }
    });
    gen
}

/// 隐藏图标并令自动隐藏计时失效(点别处 / 用户触发翻译 / 自动到时调用)。
pub fn hide(app: &AppHandle) {
    app.state::<IndicatorSession>().begin();
    VISIBLE.store(false, Ordering::SeqCst);
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}
