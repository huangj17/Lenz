//! 配置相关 Tauri commands(spec 01 §5)。

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::config::{Config, ConfigStore, TranslatorEngine};
use crate::error::LenzError;
use crate::tray;

/// 读取当前配置。
#[tauri::command]
#[specta::specta]
pub async fn get_config(state: State<'_, ConfigStore>) -> Result<Config, LenzError> {
    state.get()
}

/// 应用 RFC 7396 JSON Merge Patch 并持久化(spec 01 §5)。
///
/// 变更涉及快捷键 / 划词开关时,顺带重建托盘菜单(spec 10 §1.3)。
#[tauri::command]
#[specta::specta]
pub async fn set_config(
    state: State<'_, ConfigStore>,
    app: AppHandle,
    patch: String,
) -> Result<Config, LenzError> {
    // 线缆层用 JSON 字符串承载任意 merge patch:RFC 7396 允许 null 删字段、任意深度,
    // 强类型表达不了;且 specta(rc)对 serde_json::Value 的导出会无限递归爆栈,
    // 故前端 JSON.stringify、后端在此 parse。
    let patch: Value = serde_json::from_str(&patch)
        .map_err(|e| LenzError::Config(format!("invalid merge patch json: {e}")))?;
    let config = state.set(&app, patch)?;
    // 配置可能影响托盘显示(快捷键 / 划词勾选),重建菜单。
    if let Err(e) = tray::rebuild_tray_menu(&app) {
        log::warn!("rebuild tray menu after set_config failed: {e}");
    }
    Ok(config)
}

/// 重置为默认配置。
#[tauri::command]
#[specta::specta]
pub async fn reset_config(
    state: State<'_, ConfigStore>,
    app: AppHandle,
) -> Result<Config, LenzError> {
    let config = state.reset(&app)?;
    if let Err(e) = tray::rebuild_tray_menu(&app) {
        log::warn!("rebuild tray menu after reset_config failed: {e}");
    }
    Ok(config)
}

/// 列出已配置的翻译引擎(spec 01 §5)。
#[tauri::command]
#[specta::specta]
pub async fn list_engines(
    state: State<'_, ConfigStore>,
) -> Result<Vec<TranslatorEngine>, LenzError> {
    Ok(state.get()?.translator.engines)
}
