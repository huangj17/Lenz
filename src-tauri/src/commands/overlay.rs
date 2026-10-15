//! 原图覆盖层与划词监听相关 Tauri commands(spec 07 / spec 05 / spec 10)。

use std::time::Duration;

use tauri::Manager;
use tauri::{AppHandle, State};

use crate::config::ConfigStore;
use crate::error::LenzError;

/// 显示原图覆盖层。
///
/// 运行时创建可交互蒙层;前端就绪后显示并聚焦。
#[tauri::command]
#[specta::specta]
pub async fn show_overlay(
    app: AppHandle,
    payload: crate::core::overlay::OverlayPayload,
) -> Result<(), LenzError> {
    crate::core::overlay::close(&app)?;
    let mut payload = payload;
    payload.generation = app
        .state::<crate::core::overlay::OverlayState>()
        .session
        .begin();
    payload.viewport = crate::core::overlay::viewport_for_region(&app, payload.region)?;
    *app.state::<crate::core::overlay::OverlayState>()
        .payload
        .lock()
        .map_err(|_| LenzError::internal("overlay lock poisoned"))? = Some(payload.clone());
    crate::core::overlay::display(&app, &payload)
}

/// 关闭原图覆盖层。
#[tauri::command]
#[specta::specta]
pub async fn close_overlay(app: AppHandle) -> Result<(), LenzError> {
    crate::core::overlay::close(&app)
}

#[tauri::command]
#[specta::specta]
pub fn get_overlay_payload(app: AppHandle) -> Option<crate::core::overlay::OverlayPayload> {
    app.state::<crate::core::overlay::OverlayState>()
        .payload
        .lock()
        .ok()
        .and_then(|s| s.clone())
}
#[tauri::command]
#[specta::specta]
pub fn overlay_ready(app: AppHandle, generation: u32) -> Result<(), LenzError> {
    crate::core::overlay::ready(&app, generation)
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
/// 优先取用图标浮出瞬间(源应用仍在前台)AX 预读的选中文本缓存,直接翻译——规避「点击
/// 图标夺焦后再读系统级焦点元素会读到错内容」。缓存缺失(AX 不支持,如部分 Chrome/
/// Electron)时才退回实时兜底:隐藏图标 → 等焦点回切 → 合成 Cmd+C → 轮询剪贴板 → 翻译。
#[tauri::command]
#[specta::specta]
pub async fn trigger_selection_translate(app: AppHandle) -> Result<(), LenzError> {
    // 先取缓存(hide 不动缓存,但先取更稳妥),再隐藏图标。
    let cached = crate::core::indicator::take_captured();
    crate::core::indicator::hide(&app);
    #[cfg(target_os = "windows")]
    crate::core::indicator::restore_foreground();
    if let Some(text) = cached {
        crate::core::screenshot::translate_with_text(&app, text);
        return Ok(());
    }
    // 无预读缓存:退回实时兜底。让焦点回到用户原应用(点击图标可能短暂夺焦),
    // 合成的 Cmd+C 才会复制其选区。
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

/// 启停系统登录自启动。先修改操作系统注册项,成功后再写配置;写配置失败时回滚注册项,
/// 避免 UI 偏好与真实系统状态分叉。
#[tauri::command]
#[specta::specta]
pub async fn set_auto_start(
    state: State<'_, ConfigStore>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), LenzError> {
    let was_enabled = auto_start_enabled(&app)?;
    apply_auto_start(&app, enabled)?;
    let patch = serde_json::json!({ "app": { "autoStart": enabled } });
    if let Err(e) = state.set(&app, patch) {
        if let Err(rollback) = apply_auto_start(&app, was_enabled) {
            log::error!("rollback autostart after config failure failed: {rollback}");
        }
        return Err(e);
    }
    Ok(())
}

/// 让系统注册状态与给定偏好一致。供启动恢复与配置热更新复用。
pub(crate) fn apply_auto_start(app: &AppHandle, enabled: bool) -> Result<(), LenzError> {
    let current = auto_start_enabled(app)?;
    // 禁用且本就未注册时无需操作;启用时始终重写当前可执行文件路径,以修复应用升级/
    // 移动后遗留的旧 LaunchAgent 或注册表值。
    if !enabled && !current {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    return apply_macos_auto_start(app, enabled);
    #[cfg(target_os = "windows")]
    return if crate::core::windows_package::full_name()?.is_some() {
        crate::core::windows_package::set_auto_start(enabled)
    } else {
        apply_windows_auto_start(enabled)
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Err(LenzError::not_implemented())
}

fn auto_start_enabled(app: &AppHandle) -> Result<bool, LenzError> {
    #[cfg(target_os = "macos")]
    return Ok(macos_launch_agent_path(app)?.exists());
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        if crate::core::windows_package::full_name()?.is_some() {
            return crate::core::windows_package::auto_start_enabled();
        }
        let status = std::process::Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "Lenz",
            ])
            .status()
            .map_err(|e| LenzError::internal(format!("query autostart registry: {e}")))?;
        Ok(status.success())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
        Ok(false)
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_path(app: &AppHandle) -> Result<std::path::PathBuf, LenzError> {
    let home = app
        .path()
        .home_dir()
        .map_err(|e| LenzError::internal(format!("resolve home directory: {e}")))?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join("app.lenz.desktop.plist"))
}

#[cfg(target_os = "macos")]
fn apply_macos_auto_start(app: &AppHandle, enabled: bool) -> Result<(), LenzError> {
    let path = macos_launch_agent_path(app)?;
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let executable = std::env::current_exe()
        .map_err(|e| LenzError::internal(format!("resolve current executable: {e}")))?;
    let escaped = xml_escape(&executable.to_string_lossy());
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\"><dict>\n\
  <key>Label</key><string>app.lenz.desktop</string>\n\
  <key>ProgramArguments</key><array><string>{escaped}</string></array>\n\
  <key>RunAtLoad</key><true/>\n\
</dict></plist>\n"
    );
    let parent = path
        .parent()
        .ok_or_else(|| LenzError::internal("invalid LaunchAgents path"))?;
    std::fs::create_dir_all(parent)?;
    let tmp = path.with_extension("plist.tmp");
    std::fs::write(&tmp, plist)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "windows")]
fn apply_windows_auto_start(enabled: bool) -> Result<(), LenzError> {
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let status = if enabled {
        let executable = std::env::current_exe()
            .map_err(|e| LenzError::internal(format!("resolve current executable: {e}")))?;
        let value = format!("\"{}\"", executable.display());
        std::process::Command::new("reg")
            .args([
                "add", RUN_KEY, "/v", "Lenz", "/t", "REG_SZ", "/d", &value, "/f",
            ])
            .status()
    } else {
        std::process::Command::new("reg")
            .args(["delete", RUN_KEY, "/v", "Lenz", "/f"])
            .status()
    }
    .map_err(|e| LenzError::internal(format!("update autostart registry: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(LenzError::internal(format!(
            "update autostart registry exited with {status}"
        )))
    }
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

/// 兼容旧命令的手动检查入口;返回详细结果的入口为 check_for_update。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn check_update(app: AppHandle) -> Result<(), LenzError> {
    crate::core::system::check_update(&app).await?;
    Ok(())
}
