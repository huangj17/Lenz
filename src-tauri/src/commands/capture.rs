//! 截图采集相关 Tauri commands(spec 02)。

use crate::core::capture::{capture_region, CaptureResult};
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

/// 进入选区模式(创建 selection 蒙层窗口)。
// TODO(M4): 运行时创建覆盖全虚拟屏的 selection 窗口。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn start_selection(trigger: TriggerKind) -> Result<(), LenzError> {
    Err(LenzError::not_implemented())
}

/// 取消选区模式。
// TODO(M4): 销毁 selection 窗口。
#[tauri::command]
#[specta::specta]
pub async fn cancel_selection() -> Result<(), LenzError> {
    Err(LenzError::not_implemented())
}
