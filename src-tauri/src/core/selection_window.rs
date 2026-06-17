//! 选区蒙层窗口的运行时创建 / 销毁(spec 02 §3 / spec 09 §4)。
//!
//! **每块显示器**各创建一个透明全屏窗口 `selection-{i}`,精确铺满该屏;前端在其上
//! 用 canvas 绘制遮罩 + 框选(见 `src/windows/selection`)。框选完成 / 取消由前端
//! emit `capture://region-selected` / `capture://region-cancelled`,Rust 侧据此销毁
//! 全部蒙层窗口并接续 F2/F3 流程。
//!
//! ⚠️ 为何「每屏一窗」而非单个跨虚拟屏的窗口:macOS「调度中心 → 显示器各自拥有
//! 独立 Space」(系统默认开启)会把横跨多屏的窗口强制夹到单块屏幕上,导致蒙层只
//! 出现在某一屏、其余屏框选不到。按屏拆窗可规避该限制,在 Windows 上也同样成立。
//!
//! macOS 要求窗口创建在主线程,故 `open` 把构建动作 dispatch 到主线程执行。

use tauri::{AppHandle, Manager, Monitor, WebviewUrl, WebviewWindowBuilder};

use crate::error::LenzError;
use crate::types::TriggerKind;

/// selection 窗口 label 前缀;每块显示器一个窗口,label 为 `selection-{i}`。
pub const SELECTION_LABEL_PREFIX: &str = "selection";

/// 为每块显示器打开一个选区蒙层窗口;已存在则先销毁再重建(spec 02 §4)。
///
/// 构建被 dispatch 到主线程(macOS 约束),故本函数立即返回,真正失败仅记日志。
pub fn open(app: &AppHandle, trigger: TriggerKind) -> Result<(), LenzError> {
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        if let Err(e) = open_inner(&app2, trigger) {
            log::error!("open selection window failed: {e}");
        }
    })
    .map_err(|e| LenzError::internal(format!("dispatch open selection: {e}")))
}

/// 主线程内实际构建窗口:遍历显示器,每块各建一个铺满该屏的蒙层窗口。
fn open_inner(app: &AppHandle, trigger: TriggerKind) -> Result<(), LenzError> {
    // 旧窗口残留则同步销毁,避免 label 冲突。
    close(app)?;

    let trig = match trigger {
        TriggerKind::Overlay => "overlay",
        _ => "screenshot",
    };
    let url = format!("index.html?window=selection&trigger={trig}");

    // 每块显示器一窗:用物理坐标精确铺满该屏(规避 macOS 独立 Space 对跨屏窗口的夹取)。
    let monitors = query_monitors(app)?;
    for (i, m) in monitors.iter().enumerate() {
        let label = format!("{SELECTION_LABEL_PREFIX}-{i}");
        let win = WebviewWindowBuilder::new(app, label.as_str(), WebviewUrl::App(url.clone().into()))
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            // 仅首个窗口取焦(承载键盘 ESC);其余屏在点击时自然获焦。
            .focused(i == 0)
            // 先隐藏构建,定位/定尺后再 show,避免默认位置闪现。
            .visible(false)
            .build()
            .map_err(|e| LenzError::internal(format!("build {label}: {e}")))?;

        win.set_position(*m.position())
            .map_err(|e| LenzError::internal(format!("position {label}: {e}")))?;
        win.set_size(*m.size())
            .map_err(|e| LenzError::internal(format!("size {label}: {e}")))?;
        win.show()
            .map_err(|e| LenzError::internal(format!("show {label}: {e}")))?;
        if i == 0 {
            let _ = win.set_focus();
        }
    }
    Ok(())
}

/// 销毁全部选区蒙层窗口(若存在)。`destroy` 同步移除,可跨线程调用。
pub fn close(app: &AppHandle) -> Result<(), LenzError> {
    let prefix = format!("{SELECTION_LABEL_PREFIX}-");
    for (label, w) in app.webview_windows() {
        if label == SELECTION_LABEL_PREFIX || label.starts_with(&prefix) {
            w.destroy()
                .map_err(|e| LenzError::internal(format!("destroy {label}: {e}")))?;
        }
    }
    Ok(())
}

/// 查询全部显示器(走任一已存在窗口,避免依赖 AppHandle 上不稳定的 monitor API)。
fn query_monitors(app: &AppHandle) -> Result<Vec<Monitor>, LenzError> {
    let base = app
        .get_webview_window("popup")
        .or_else(|| app.get_webview_window("main"))
        .ok_or_else(|| LenzError::internal("no base window for monitor query"))?;
    let monitors = base
        .available_monitors()
        .map_err(|e| LenzError::internal(format!("available_monitors: {e}")))?;
    if monitors.is_empty() {
        return Err(LenzError::internal("no monitor found"));
    }
    Ok(monitors)
}

/// 列出各显示器的逻辑边界 `(x, y, w, h)`。
///
/// 各屏以自身 `scale_factor` 把物理坐标换算为逻辑坐标——单屏 / 同 DPR 多屏精确;
/// 混合 DPR 为近似(spec 02 §9 开放问题)。供 popup 定位与夹紧复用(见 screenshot.rs)。
pub fn logical_monitor_bounds(app: &AppHandle) -> Result<Vec<(f64, f64, f64, f64)>, LenzError> {
    Ok(query_monitors(app)?
        .iter()
        .map(|m| {
            let sf = m.scale_factor();
            let p = m.position();
            let s = m.size();
            (
                p.x as f64 / sf,
                p.y as f64 / sf,
                s.width as f64 / sf,
                s.height as f64 / sf,
            )
        })
        .collect())
}
