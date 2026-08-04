//! 系统托盘图标与菜单(spec 10 §1)。
//!
//! 菜单结构:
//! ```text
//! 打开主窗口
//! ───────────
//! 划词翻译       Alt+D
//! 截图翻译       Alt+S
//! 原图覆盖翻译   Alt+F
//! ───────────
//! [✓] 划词监听
//! ───────────
//! 设置...
//! ───────────
//! 检查更新
//! 关于 Lenz
//! ───────────
//! 退出
//! ```
//! 左键点击托盘显示 main 窗口;配置变更后用 `rebuild_tray_menu` 重建菜单。

use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::config::ConfigStore;
use crate::error::LenzError;

/// 托盘图标唯一 id,用于 `tray_by_id` 重建菜单。
pub const TRAY_ID: &str = "lenz-tray";

// 菜单项 id。
const ID_OPEN_MAIN: &str = "open_main";
const ID_SELECTION: &str = "trigger_selection";
const ID_SCREENSHOT: &str = "trigger_screenshot";
const ID_OVERLAY: &str = "trigger_overlay";
const ID_TOGGLE_LISTEN: &str = "toggle_listen";
const ID_SETTINGS: &str = "settings";
const ID_CHECK_UPDATE: &str = "check_update";
const ID_ABOUT: &str = "about";
const ID_QUIT: &str = "quit";

/// 构建托盘菜单,菜单上的快捷键提示从当前 config 读取。
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, LenzError> {
    let store = app.state::<ConfigStore>();
    let config = store.get()?;
    let sc = &config.shortcuts;

    // 快捷键提示文案:有值则附加,无值则仅显示功能名。
    let label_with_acc = |name: &str, acc: &Option<String>| match acc {
        Some(a) if !a.trim().is_empty() => format!("{name}    {a}"),
        _ => name.to_string(),
    };

    let open_main = MenuItemBuilder::with_id(ID_OPEN_MAIN, "打开主窗口").build(app)?;
    let selection =
        MenuItemBuilder::with_id(ID_SELECTION, label_with_acc("划词翻译", &sc.selection))
            .build(app)?;
    let screenshot =
        MenuItemBuilder::with_id(ID_SCREENSHOT, label_with_acc("截图翻译", &sc.screenshot))
            .build(app)?;
    let overlay = MenuItemBuilder::with_id(ID_OVERLAY, label_with_acc("原图覆盖翻译", &sc.overlay))
        .build(app)?;
    let listen = CheckMenuItemBuilder::with_id(ID_TOGGLE_LISTEN, "划词监听")
        .checked(config.selection.enabled)
        .build(app)?;
    let settings = MenuItemBuilder::with_id(ID_SETTINGS, "设置...").build(app)?;
    let check_update = MenuItemBuilder::with_id(ID_CHECK_UPDATE, "检查更新").build(app)?;
    let about = MenuItemBuilder::with_id(ID_ABOUT, "关于 Lenz").build(app)?;
    let quit = MenuItemBuilder::with_id(ID_QUIT, "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_main)
        .separator()
        .item(&selection)
        .item(&screenshot)
        .item(&overlay)
        .separator()
        .item(&listen)
        .separator()
        .item(&settings)
        .separator()
        .item(&check_update)
        .item(&about)
        .separator()
        .item(&quit)
        .build()?;

    Ok(menu)
}

/// 在启动阶段创建托盘图标(spec 10 §1.3)。
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), LenzError> {
    let menu = build_menu(app)?;
    // 专用单色模板图标(icons/tray.svg → icons/tray.png,见 scripts/gen-icons.mjs),
    // 编译期内嵌;不再复用应用窗口图标。
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))
        .map_err(|e| LenzError::internal(format!("load tray icon: {e}")))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true) // macOS:黑白模板图标,系统自动适配深浅色。
        .tooltip("Lenz")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// 配置变更后重建托盘菜单(快捷键提示 / 划词勾选状态同步,spec 10 §1.3)。
pub fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), LenzError> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        // 托盘尚未创建(如启动早期),静默跳过。
        return Ok(());
    };
    let menu = build_menu(app)?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

/// 处理托盘菜单点击。
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        ID_OPEN_MAIN | ID_SETTINGS => show_main_window(app),
        ID_SELECTION => emit_shortcut(app, "shortcut://selection"),
        ID_SCREENSHOT => emit_shortcut(app, "shortcut://screenshot"),
        ID_OVERLAY => emit_shortcut(app, "shortcut://overlay"),
        ID_TOGGLE_LISTEN => toggle_listen(app),
        ID_CHECK_UPDATE => emit_shortcut(app, "tray://check-update"),
        ID_ABOUT => show_about(app),
        ID_QUIT => app.exit(0),
        other => log::debug!("unhandled tray menu event: {other}"),
    }
}

/// 显示并聚焦 main 窗口(spec 10 §1.3)。
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        log::warn!("main window not found when trying to show");
    }
}

/// 显示 main 窗口并提示前端切到关于页(前端监听该事件路由)。
fn show_about<R: Runtime>(app: &AppHandle<R>) {
    show_main_window(app);
    let _ = app.emit("tray://open-about", ());
}

/// 切换划词监听(同步 config 与勾选状态)。
fn toggle_listen<R: Runtime>(app: &AppHandle<R>) {
    let store = app.state::<ConfigStore>();
    let enabled = match store.get() {
        Ok(cfg) => !cfg.selection.enabled,
        Err(e) => {
            log::warn!("toggle_listen read config failed: {e}");
            return;
        }
    };
    let patch = serde_json::json!({ "selection": { "enabled": enabled } });
    if let Err(e) = store.set(app, patch) {
        log::warn!("toggle_listen set config failed: {e}");
        return;
    }
    if let Err(e) = rebuild_tray_menu(app) {
        log::warn!("toggle_listen rebuild menu failed: {e}");
    }
    // TODO(M5): 据 enabled 启停 SelectionListener。
}

/// emit 一个无载荷事件(用于将托盘点击复用为快捷键触发路径)。
fn emit_shortcut<R: Runtime>(app: &AppHandle<R>, event: &str) {
    if let Err(e) = app.emit(event, ()) {
        log::warn!("emit {event} from tray failed: {e}");
    }
}
