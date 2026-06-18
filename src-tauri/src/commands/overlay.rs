//! 原图覆盖层与划词监听相关 Tauri commands(spec 07 / spec 05 / spec 10)。

use std::time::Duration;

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
/// 写配置 → emit `config://changed` → setup 订阅者据 `enabled && autoDetect` 启停
/// `SelectionListener`(见 `core::selection::sync`);此处只负责配置与托盘勾选同步。
#[tauri::command]
#[specta::specta]
pub async fn toggle_selection_listener(
    state: State<'_, ConfigStore>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), LenzError> {
    // 写入配置(触发 config://changed,各订阅方据此联动:监听器启停 + 菜单勾选)。
    let patch = serde_json::json!({ "selection": { "enabled": enabled } });
    state.set(&app, patch)?;
    // 重建托盘菜单以同步勾选状态。
    if let Err(e) = crate::tray::rebuild_tray_menu(&app) {
        log::warn!("rebuild tray menu after toggle_selection_listener failed: {e}");
    }
    // 启用监听但缺少辅助功能权限 → 直接引导用户到系统设置(spec 05 §2.1)。
    if enabled && !crate::core::selection::permission_granted() {
        open_accessibility_pane();
    }
    Ok(())
}

/// 划词指示图标被 hover/点击时触发翻译(spec 05 §6)。
///
/// 先隐藏图标 → 等焦点回到用户原应用 → 复用快捷键划词编排(合成 Cmd+C → 轮询剪贴板
/// → 翻译 → 光标处弹框)。复制只在用户真正 engage 图标时发生,故不会在每次划词手势
/// 干扰用户剪贴板。
#[tauri::command]
#[specta::specta]
pub async fn trigger_selection_translate(app: AppHandle) -> Result<(), LenzError> {
    crate::core::indicator::hide(&app);
    // 让焦点回到用户原应用(点击图标可能短暂夺焦),合成的 Cmd+C 才会复制其选区。
    tokio::time::sleep(Duration::from_millis(120)).await;
    crate::core::screenshot::on_selection_shortcut(&app);
    Ok(())
}

/// 打开系统「辅助功能」隐私设置,引导用户授权划词监听(spec 05 §2.1)。
#[tauri::command]
#[specta::specta]
pub async fn open_accessibility_settings() -> Result<(), LenzError> {
    open_accessibility_pane();
    Ok(())
}

/// 打开「辅助功能」设置面板(macOS 深链;其余平台 no-op)。
fn open_accessibility_pane() {
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
        {
            log::warn!("open accessibility settings failed: {e}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        log::debug!("open_accessibility_pane is a no-op on this platform");
    }
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
