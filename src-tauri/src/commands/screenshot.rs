//! 截图翻译 F2 的前端可调用命令(spec 06)。
//!
//! F2 主流程由 Rust 内部编排(见 `core/screenshot`),前端只需:
//! - `cancel_screenshot`:popup 上 ESC / 关闭时取消进行中的会话并隐藏 popup;
//! - `open_screen_settings`:权限缺失时跳转系统「屏幕录制」设置(spec 06 §4);
//! - `resize_popup`:内容渲染后按实际高度调整 popup 窗口。

use tauri::{AppHandle, LogicalSize, Manager, State};

use crate::core::screenshot::{clamp_popup_into_monitor, ScreenshotSession, POPUP_WIDTH};
use crate::error::LenzError;

/// 取消进行中的 F2 会话并隐藏 popup(spec 06 §6/§7)。
#[tauri::command]
#[specta::specta]
pub async fn cancel_screenshot(
    app: AppHandle,
    session: State<'_, ScreenshotSession>,
) -> Result<(), LenzError> {
    session.cancel();
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
    Ok(())
}

/// 打开系统「屏幕录制」隐私设置,引导用户授权(spec 06 §4)。
///
/// macOS 走 `x-apple.systempreferences` 深链;其余平台暂为 no-op。
#[tauri::command]
#[specta::specta]
pub async fn open_screen_settings() -> Result<(), LenzError> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn()
            .map_err(|e| LenzError::internal(format!("open screen settings: {e}")))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        log::debug!("open_screen_settings is a no-op on this platform");
    }
    Ok(())
}

/// 内容渲染后,按实际高度调整 popup 窗口(宽度固定,spec 06 §2.6 / spec 09 §3.1)。
///
/// 调整高度后把窗口夹回所在显示器内,避免长译文撑高后底部溢出屏幕、内容被截断。
///
/// macOS:透明窗口在缩放后会残留旧尺寸的窗口阴影(表现为弹框旁边的灰块),
/// 故每次 resize 后强制 `invalidateShadow` 让窗口阴影重算,消除残影。
#[tauri::command]
#[specta::specta]
pub async fn resize_popup(height: f64, app: AppHandle) -> Result<(), LenzError> {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.set_size(LogicalSize::new(POPUP_WIDTH, height));
        clamp_popup_into_monitor(&app, height);
    }
    #[cfg(target_os = "macos")]
    invalidate_popup_shadow(&app);
    Ok(())
}

/// 在主线程上对 popup 的 `NSWindow` 调 `invalidateShadow`,清除透明窗 resize 残影。
///
/// `set_size` 与本调用都被投递到主线程事件循环,按序执行 → 阴影按新尺寸重算。
#[cfg(target_os = "macos")]
fn invalidate_popup_shadow(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let Some(popup) = app.get_webview_window("popup") else {
            return;
        };
        let Ok(ptr) = popup.ns_window() else {
            return;
        };
        if ptr.is_null() {
            return;
        }
        // ptr 即该窗口的 NSWindow*;在主线程上请求重算其阴影。
        let ns_window = ptr as *mut objc2::runtime::AnyObject;
        unsafe {
            let _: () = objc2::msg_send![&*ns_window, invalidateShadow];
        }
    });
}
