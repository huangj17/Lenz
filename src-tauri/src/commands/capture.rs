//! 截图采集相关 Tauri commands(spec 02)。M1 仅占位。

use crate::error::LenzError;
use crate::types::{Rect, TriggerKind};

/// 采集指定屏幕区域,返回截图与像素信息。
///
/// M1 占位:返回类型暂为 `()`,M2 落地 `CaptureResult` 后替换。
// TODO(M2): 用 xcap 抓屏 + 逻辑坐标→物理像素裁剪 + 跨屏拼接。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn capture_screen_region(rect: Rect) -> Result<(), LenzError> {
    Err(LenzError::not_implemented())
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
