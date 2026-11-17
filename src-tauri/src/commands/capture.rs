//! 截图采集相关 Tauri commands(spec 02)。

use tauri::{AppHandle, WebviewWindow};

use crate::core::capture::{capture_region_with_timeout, CaptureResult};
use crate::core::selection_window;
use crate::error::LenzError;
use crate::types::{Rect, TriggerKind};

/// 采集指定屏幕区域(逻辑坐标),返回 PNG + 物理像素信息。
#[tauri::command]
#[specta::specta]
pub async fn capture_screen_region(rect: Rect) -> Result<CaptureResult, LenzError> {
    capture_region_with_timeout(rect).await
}

/// 进入选区模式:定格整屏后为每块显示器各创建一个 selection 蒙层窗口(spec 02 §4)。
#[tauri::command]
#[specta::specta]
pub async fn start_selection(trigger: TriggerKind, app: AppHandle) -> Result<(), LenzError> {
    selection_window::open(&app, trigger)
}

/// 取消选区模式:销毁 selection 窗口并丢弃定格图(spec 02 §4)。
#[tauri::command]
#[specta::specta]
pub async fn cancel_selection(app: AppHandle) -> Result<(), LenzError> {
    selection_window::cancel(&app)
}

/// 选区窗口已画好定格背景:显示调用方窗口(spec 02 §3.1)。
#[tauri::command]
#[specta::specta]
pub fn selection_ready(window: WebviewWindow) {
    selection_window::reveal(&window);
}
