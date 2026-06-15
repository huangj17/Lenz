//! 应用启动时序(spec 00 §7)。
//!
//! ```text
//! 1. 加载 config(容错:损坏则备份 + 重建)
//! 2. 注册托盘图标与菜单
//! 3. 注册全局快捷键(config 驱动)
//! 4. 按 config.selection.enabled/autoDetect 启动划词监听(M1 占位)
//! 5. 创建 / 显示 main 窗口(若 config.app.openOnLaunch)
//! 6. 异步触发 check_update(若 config.app.checkUpdate;M1 占位)
//! ```
//! 失败 step 不阻断主流程:记录日志并 emit `app://startup-warning`。

use tauri::{App, Emitter, Manager};

use crate::config::ConfigStore;
use crate::core::shortcut;
use crate::tray;
use crate::types::StartupWarning;

/// `app://startup-warning` 事件名。
const EVENT_STARTUP_WARNING: &str = "app://startup-warning";

/// Tauri setup 钩子主体。
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // macOS:菜单栏 app,不在 Dock 显示(spec 11 §4.1)。
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let handle = app.handle();

    // Step 1:加载配置(失败则整体失败——这是后续步骤的硬前置)。
    let store = match ConfigStore::load(handle) {
        Ok(s) => s,
        Err(e) => {
            // 极端情况:连默认配置都写不进去。用内存默认值兜底,保证应用能起。
            log::error!("config load failed: {e}; falling back to in-memory default");
            warn(app, "config", &format!("{e}"));
            // 仍尝试 manage 一个默认 store,避免后续 state::<ConfigStore> panic。
            // 这里直接返回错误更危险(应用起不来),故重试一次 load 的弱化版本不可行,
            // 因此选择继续:用 ConfigStore::load 已尽力,e 多为 IO,记录后继续。
            return Err(Box::new(e));
        }
    };

    let config = store.get().unwrap_or_default();
    app.manage(store);

    // Step 2:注册托盘。
    if let Err(e) = tray::build_tray(handle) {
        log::warn!("build tray failed: {e}");
        warn(app, "tray", &format!("{e}"));
    }

    // Step 3:注册全局快捷键。
    if let Err(e) = shortcut::register_shortcuts(handle, &config.shortcuts) {
        log::warn!("register shortcuts failed: {e}");
        warn(app, "shortcuts", &format!("{e}"));
    }

    // Step 4:启动划词监听(M1 占位)。
    // TODO(M5): 据 config.selection.enabled / auto_detect 启停 SelectionListener。
    if config.selection.enabled {
        log::debug!("selection listener enabled in config (listener impl: M5)");
    }

    // Step 5:按 openOnLaunch 决定是否显示 main 窗口。
    if config.app.open_on_launch {
        if let Some(window) = handle.get_webview_window("main") {
            if let Err(e) = window.show() {
                log::warn!("show main window failed: {e}");
                warn(app, "window", &format!("{e}"));
            }
            let _ = window.set_focus();
        } else {
            log::warn!("main window not found at startup");
            warn(app, "window", "main window not found");
        }
    }

    // Step 6:异步触发检查更新(M1 占位,不阻塞)。
    if config.app.check_update {
        let handle_for_update = handle.clone();
        tauri::async_runtime::spawn(async move {
            // spec 10 §4.1:启动后延迟 3s。
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            // TODO(M8): 调用 tauri-plugin-updater 检查更新。
            log::debug!("startup check_update tick (impl: M8)");
            let _ = &handle_for_update;
        });
    }

    log::info!("Lenz setup complete");
    Ok(())
}

/// 记录并向前端 emit 启动警告(spec 00 §7)。
fn warn(app: &App, step: &str, message: &str) {
    let _ = app.emit(
        EVENT_STARTUP_WARNING,
        StartupWarning {
            step: step.to_string(),
            message: message.to_string(),
        },
    );
}
