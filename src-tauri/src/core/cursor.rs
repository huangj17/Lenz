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
#[cfg(target_os = "windows")]
pub fn current_logical() -> Option<(f64, f64)> {
    let mut point = windows::Win32::Foundation::POINT::default();
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).ok()?;
    }
    Some(physical_to_logical(point))
}
#[cfg(target_os = "windows")]
pub fn physical_to_logical(point: windows::Win32::Foundation::POINT) -> (f64, f64) {
    let scale = point_scale(point);
    (point.x as f64 / scale, point.y as f64 / scale)
}
#[cfg(target_os = "windows")]
pub fn point_scale(point: windows::Win32::Foundation::POINT) -> f64 {
    use windows::Win32::{
        Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST},
        UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    };
    let mut x = 96;
    let mut y = 96;
    unsafe {
        let _ = GetDpiForMonitor(
            MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST),
            MDT_EFFECTIVE_DPI,
            &mut x,
            &mut y,
        );
    }
    x.max(1) as f64 / 96.0
}
#[cfg(target_os = "windows")]
pub fn current_physical() -> Option<windows::Win32::Foundation::POINT> {
    let mut point = windows::Win32::Foundation::POINT::default();
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).ok()?;
    }
    Some(point)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn current_logical() -> Option<(f64, f64)> {
    None
}
