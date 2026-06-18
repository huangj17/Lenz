//! 当前鼠标光标位置查询(spec 05 §7:划词弹框定位锚点)。
//!
//! 返回全局显示「逻辑坐标」(top-left 原点,单位 point),与 Tauri 的逻辑坐标系
//! 一致,可直接用于 `LogicalPosition` 摆放窗口。多屏时为虚拟屏坐标。
//!
//! - macOS:构造一个无源 `CGEvent` 取其 `location`(即当前光标位置,points)。
//! - 其它平台:暂未实现,返回 `None`,调用方回退到居中定位。

/// 当前光标的全局逻辑坐标 `(x, y)`;取不到时返回 `None`。
#[cfg(target_os = "macos")]
pub fn current_logical() -> Option<(f64, f64)> {
    use objc2_core_graphics::CGEvent;
    // 无事件源构造的 CGEvent 会携带「当前光标位置」。
    let event = CGEvent::new(None)?;
    let p = CGEvent::location(Some(&event));
    Some((p.x, p.y))
}

/// 非 macOS:暂不支持,返回 `None`(调用方回退居中)。
// TODO(M5 Windows): GetCursorPos + 按所在显示器 DPI 折算为逻辑坐标。
#[cfg(not(target_os = "macos"))]
pub fn current_logical() -> Option<(f64, f64)> {
    None
}
