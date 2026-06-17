//! 截图采集相关 Tauri commands(spec 02)。

use tauri::AppHandle;

use crate::core::capture::{capture_region, CaptureResult};
use crate::core::selection_window;
use crate::error::LenzError;
use crate::types::{Rect, TriggerKind};

/// 采集指定屏幕区域(逻辑坐标),返回 PNG + 物理像素信息。
#[tauri::command]
#[specta::specta]
pub async fn capture_screen_region(rect: Rect) -> Result<CaptureResult, LenzError> {
    // xcap 为阻塞调用,移出异步 runtime。
    tokio::task::spawn_blocking(move || capture_region(rect))
        .await
        .map_err(|e| LenzError::Internal(format!("capture join: {e}")))?
}

/// 进入选区模式:为每块显示器各创建一个 selection 蒙层窗口(spec 02 §4)。
#[tauri::command]
#[specta::specta]
pub async fn start_selection(trigger: TriggerKind, app: AppHandle) -> Result<(), LenzError> {
    selection_window::open(&app, trigger)
}

/// 取消选区模式:销毁 selection 窗口(spec 02 §4)。
#[tauri::command]
#[specta::specta]
pub async fn cancel_selection(app: AppHandle) -> Result<(), LenzError> {
    selection_window::close(&app)
}
