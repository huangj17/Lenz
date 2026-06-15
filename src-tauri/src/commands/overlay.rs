//! 原图覆盖层与划词监听相关 Tauri commands(spec 07 / spec 05 / spec 10)。M1 仅占位。

use tauri::{AppHandle, State};

use crate::config::ConfigStore;
use crate::error::LenzError;

/// 显示原图覆盖层。
///
/// M1 占位:M6 落地 overlay 数据结构后,补回入参(译文 block + 坐标 + 颜色)。
// TODO(M6): 运行时创建透明、鼠标穿透的 overlay 窗口并渲染译文 block。
#[tauri::command]
#[specta::specta]
pub async fn show_overlay() -> Result<(), LenzError> {
    Err(LenzError::not_implemented())
}

/// 关闭原图覆盖层。
// TODO(M6): 销毁所有 overlay 窗口并解绑临时 ESC 快捷键。
#[tauri::command]
#[specta::specta]
pub async fn close_overlay() -> Result<(), LenzError> {
    Err(LenzError::not_implemented())
}

/// 启停划词监听,并同步写入 `config.selection.enabled`(spec 05 §8 / spec 10 §1.2)。
///
/// M1:实现配置写入与托盘勾选同步;实际监听线程的启停留待 M5。
#[tauri::command]
#[specta::specta]
pub async fn toggle_selection_listener(
    state: State<'_, ConfigStore>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), LenzError> {
    // 写入配置(触发 config://changed,各订阅方据此联动)。
    let patch = serde_json::json!({ "selection": { "enabled": enabled } });
    state.set(&app, patch)?;
    // 重建托盘菜单以同步勾选状态。
    if let Err(e) = crate::tray::rebuild_tray_menu(&app) {
        log::warn!("rebuild tray menu after toggle_selection_listener failed: {e}");
    }
    // TODO(M5): 据 enabled 启停 SelectionListener 线程。
    Ok(())
}

/// 手动 / 启动后触发检查更新(spec 10 §4)。M1 仅占位。
// TODO(M8): 接入 tauri-plugin-updater,检查并 emit app://update-available。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn check_update(app: AppHandle) -> Result<(), LenzError> {
    // M1 占位:不报错,静默返回,避免阻断启动流程。
    log::debug!("check_update invoked (not implemented until M8)");
    Ok(())
}
