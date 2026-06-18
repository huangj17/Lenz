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

/// 图标当前是否可见(供监听线程在 mouseDown 时快速判断)。
static VISIBLE: AtomicBool = AtomicBool::new(false);
/// 图标当前屏幕矩形 `(x, y, w, h)`(逻辑坐标);命中测试用。
static RECT: Mutex<(f64, f64, f64, f64)> = Mutex::new((0.0, 0.0, 0.0, 0.0));

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

/// 在 `(x, y)`(全局逻辑坐标,选区松开点)附近显示划词指示图标;`AUTO_HIDE_MS`
/// 内无操作则自动隐藏。
pub fn show_at(app: &AppHandle, x: f64, y: f64) {
    let Some(win) = app.get_webview_window(LABEL) else {
        log::warn!("indicator window not found");
        return;
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
}

/// 隐藏图标并令自动隐藏计时失效(点别处 / 用户触发翻译 / 自动到时调用)。
pub fn hide(app: &AppHandle) {
    app.state::<IndicatorSession>().begin();
    VISIBLE.store(false, Ordering::SeqCst);
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}
