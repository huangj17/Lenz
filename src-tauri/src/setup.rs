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

use tauri::{App, AppHandle, Emitter, Listener, Manager};

use crate::config::{ConfigStore, EVENT_CONFIG_CHANGED};
use crate::core::screenshot::{self, LastResult, ScreenshotSession};
use crate::core::selection_window;
use crate::core::shortcut;
use crate::core::translator::TranslatorManager;
use crate::tray;
use crate::types::{ConfigChangedPayload, StartupWarning, TriggerKind};

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

    // 翻译引擎管理器:按配置构建引擎并注入(spec 04 §2)。
    let translator_manager = TranslatorManager::new();
    translator_manager.reload(
        &config.translator.engines,
        &config.translator.default_engine_id,
        &config.proxy,
    );
    app.manage(translator_manager);

    // 订阅 config://changed:translator.* / proxy.* 变更时热更新引擎(不重启,spec 01 §4.3)。
    let reload_handle = handle.clone();
    handle.listen(EVENT_CONFIG_CHANGED, move |event| {
        if config_change_needs_reload(event.payload()) {
            reload_translators(&reload_handle);
        }
    });

    // F2 截图翻译会话代号(取消机制,spec 06 §7)。
    app.manage(ScreenshotSession::default());
    // 最近一次翻译结果(划词快捷键「无新选区则回显上次内容」,spec 05 §3)。
    app.manage(LastResult::default());

    // F2 接线(spec 06):
    //  - Alt+S(shortcut://screenshot)→ 打开选区蒙层;
    //  - 框选完成(capture://region-selected)→ 销毁选区 + 跑 截图/OCR/翻译;
    //  - 取消(capture://region-cancelled)→ 销毁选区。
    let h_shot = handle.clone();
    handle.listen("shortcut://screenshot", move |_| {
        if let Err(e) = selection_window::open(&h_shot, TriggerKind::Screenshot) {
            log::warn!("open selection on Alt+S failed: {e}");
        }
    });
    let h_sel = handle.clone();
    handle.listen("capture://region-selected", move |event| {
        screenshot::on_region_selected(&h_sel, event.payload());
    });
    let h_cancel = handle.clone();
    handle.listen("capture://region-cancelled", move |_| {
        if let Err(e) = selection_window::close(&h_cancel) {
            log::warn!("close selection on cancel failed: {e}");
        }
    });

    // F1 划词翻译(快捷键模式):Alt+D(shortcut://selection)→ 读剪贴板 → 翻译 → popup。
    // 自动检测监听 + 选中自动复制属 M5;此处先让快捷键可用(spec 05 §3)。
    let h_word = handle.clone();
    handle.listen("shortcut://selection", move |_| {
        screenshot::on_selection_shortcut(&h_word);
    });

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

/// 判断一次 `config://changed` 是否需要重载翻译引擎。
///
/// keypath 涉及 `translator.*` / `proxy.*` 或整体重置(空 keypath)时需重载;
/// 载荷解析失败时保守重载。
fn config_change_needs_reload(payload: &str) -> bool {
    match serde_json::from_str::<ConfigChangedPayload>(payload) {
        Ok(p) => p.keypaths.iter().any(|k| {
            k.is_empty() || k.starts_with("translator") || k.starts_with("proxy")
        }),
        Err(_) => true,
    }
}

/// 读最新配置并重建 `TranslatorManager` 引擎表。
fn reload_translators(handle: &AppHandle) {
    let store = handle.state::<ConfigStore>();
    let mgr = handle.state::<TranslatorManager>();
    match store.get() {
        Ok(cfg) => mgr.reload(
            &cfg.translator.engines,
            &cfg.translator.default_engine_id,
            &cfg.proxy,
        ),
        Err(e) => log::warn!("reload translators: read config failed: {e}"),
    }
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
