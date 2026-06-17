//! 选区蒙层窗口的运行时创建 / 销毁(spec 02 §3 / spec 09 §4)。
//!
//! 运行时创建一个覆盖**所有显示器并集**的透明全屏窗口 `selection`,前端在其上
//! 用 canvas 绘制遮罩 + 框选(见 `src/windows/selection`)。框选完成 / 取消由前端
//! emit `capture://region-selected` / `capture://region-cancelled`,Rust 侧据此销毁
//! 本窗口并接续 F2/F3 流程。
//!
//! macOS 要求窗口创建在主线程,故 `open` 把构建动作 dispatch 到主线程执行。

use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::error::LenzError;
use crate::types::TriggerKind;

/// selection 窗口 label。
pub const SELECTION_LABEL: &str = "selection";

/// 打开覆盖全部显示器的选区蒙层窗口;已存在则先销毁再重建(spec 02 §4)。
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

/// 主线程内实际构建窗口。
fn open_inner(app: &AppHandle, trigger: TriggerKind) -> Result<(), LenzError> {
    // 旧窗口残留则同步销毁,避免 label 冲突。
    close(app)?;

    let (origin, size) = virtual_desktop_rect(app)?;

    let trig = match trigger {
        TriggerKind::Overlay => "overlay",
        _ => "screenshot",
    };
    let url = format!("index.html?window=selection&trigger={trig}");

    let win = WebviewWindowBuilder::new(app, SELECTION_LABEL, WebviewUrl::App(url.into()))
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(true)
        // 先隐藏构建,定位/定尺后再 show,避免默认位置闪现。
        .visible(false)
        .build()
        .map_err(|e| LenzError::internal(format!("build selection window: {e}")))?;

    win.set_position(origin)
        .map_err(|e| LenzError::internal(format!("position selection: {e}")))?;
    win.set_size(size)
        .map_err(|e| LenzError::internal(format!("size selection: {e}")))?;
    win.show()
        .map_err(|e| LenzError::internal(format!("show selection: {e}")))?;
    let _ = win.set_focus();
    Ok(())
}

/// 销毁选区窗口(若存在)。`destroy` 同步移除,可跨线程调用。
pub fn close(app: &AppHandle) -> Result<(), LenzError> {
    if let Some(w) = app.get_webview_window(SELECTION_LABEL) {
        w.destroy()
            .map_err(|e| LenzError::internal(format!("destroy selection: {e}")))?;
    }
    Ok(())
}

/// 计算所有显示器并集的逻辑矩形(原点 + 尺寸)。
///
/// 各屏以自身 `scale_factor` 把物理坐标换算为逻辑坐标后取并集——单屏 / 同 DPR 多屏
/// 精确;混合 DPR 为近似(spec 02 §9 开放问题)。
fn virtual_desktop_rect(
    app: &AppHandle,
) -> Result<(LogicalPosition<f64>, LogicalSize<f64>), LenzError> {
    let bounds = logical_monitor_bounds(app)?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (lx, ly, lw, lh) in &bounds {
        min_x = min_x.min(*lx);
        min_y = min_y.min(*ly);
        max_x = max_x.max(lx + lw);
        max_y = max_y.max(ly + lh);
    }
    Ok((
        LogicalPosition::new(min_x, min_y),
        LogicalSize::new(max_x - min_x, max_y - min_y),
    ))
}

/// 列出各显示器的逻辑边界 `(x, y, w, h)`。
///
/// monitor 查询走任一已存在窗口(popup 常驻 / main),避免依赖 AppHandle 上不稳定的
/// monitor API。供选区定位与 popup 夹紧复用。
pub fn logical_monitor_bounds(app: &AppHandle) -> Result<Vec<(f64, f64, f64, f64)>, LenzError> {
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
    Ok(monitors
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
