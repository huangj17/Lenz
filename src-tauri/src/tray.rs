//! 系统托盘图标与菜单(spec 10 §1)。
//!
//! 菜单结构(macOS 默认快捷键示例):
//! ```text
//! 打开主窗口
//! ───────────
//! 划词翻译        ⌥D
//! 截图翻译        ⌥S
//! 原图覆盖翻译    ⌥F
//! ───────────
//! [✓] 划词监听
//! ───────────
//! 设置...        ⌘, (macOS)
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
const ID_CLOSE_OVERLAY: &str = "close_overlay";
const ID_SETTINGS: &str = "settings";
const ID_CHECK_UPDATE: &str = "check_update";
const ID_ABOUT: &str = "about";
const ID_QUIT: &str = "quit";

/// 构建托盘菜单,菜单上的快捷键提示从当前 config 读取。
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, LenzError> {
    let store = app.state::<ConfigStore>();
    let config = store.get()?;
    let sc = &config.shortcuts;
    let zh = crate::core::system::is_chinese(app);
    let label = |zh_text, en_text| if zh { zh_text } else { en_text };

    // 原生 accelerator 由系统绘制在右侧快捷键列,无配置则不显示。
    let item_with_acc = |id: &str, name: &str, acc: &Option<String>| {
        let mut item = MenuItemBuilder::with_id(id, name);
        if let Some(acc) = acc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            item = item.accelerator(acc);
        }
        item.build(app)
    };

    let open_main = MenuItemBuilder::with_id(ID_OPEN_MAIN, label("打开主窗口", "Open Main Window"))
        .build(app)?;
    let selection = item_with_acc(
        ID_SELECTION,
        label("划词翻译", "Selection Translation"),
        &sc.selection,
    )?;
    let screenshot = item_with_acc(
        ID_SCREENSHOT,
        label("截图翻译", "Screenshot Translation"),
        &sc.screenshot,
    )?;
    let overlay = item_with_acc(
        ID_OVERLAY,
        label("原图覆盖翻译", "Overlay Translation"),
        &sc.overlay,
    )?;
    let close_overlay =
        MenuItemBuilder::with_id(ID_CLOSE_OVERLAY, label("关闭覆盖层", "Close Overlay"))
            .enabled(app.get_webview_window("overlay").is_some())
            .build(app)?;
    let listen =
        CheckMenuItemBuilder::with_id(ID_TOGGLE_LISTEN, label("划词监听", "Selection Listener"))
            .checked(config.selection.enabled)
            .build(app)?;
    let settings = MenuItemBuilder::with_id(ID_SETTINGS, label("设置...", "Settings..."));
    #[cfg(target_os = "macos")]
    let settings = settings.accelerator("Cmd+,");
    #[cfg(target_os = "windows")]
    let settings = settings.accelerator("Ctrl+,");
    let settings = settings.build(app)?;
    let check_update =
        MenuItemBuilder::with_id(ID_CHECK_UPDATE, label("检查更新", "Check for Updates"))
            .build(app)?;
    let about = MenuItemBuilder::with_id(ID_ABOUT, label("关于 Lenz", "About Lenz")).build(app)?;
    let quit = MenuItemBuilder::with_id(ID_QUIT, label("退出", "Quit")).build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_main)
        .separator()
        .item(&selection)
        .item(&screenshot)
        .item(&overlay)
        .item(&close_overlay)
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
    // macOS 仅给托盘菜单绑定 accelerator,菜单关闭时无法从应用窗口触发。
    // 同一个设置项也插入默认应用菜单的「关于 / 分隔线」之后,保留原生编辑菜单。
    #[cfg(target_os = "macos")]
    {
        let app_menu = app
            .menu()
            .ok_or_else(|| LenzError::internal("default app menu not found"))?;
        let app_submenu = app_menu
            .items()?
            .into_iter()
            .next()
            .and_then(|item| item.as_submenu().cloned())
            .ok_or_else(|| LenzError::internal("default app submenu not found"))?;
        let settings = menu
            .get(ID_SETTINGS)
            .ok_or_else(|| LenzError::internal("settings menu item not found"))?;
        app_submenu.insert(&settings, 2)?;
        app_submenu.insert(&tauri::menu::PredefinedMenuItem::separator(app)?, 3)?;
    }
    // macOS 使用单色模板由系统适配深浅色;Windows 直接绘制位图,需使用彩色图标。
    let icon_bytes: &[u8] = if cfg!(target_os = "macos") {
        include_bytes!("../icons/tray.png")
    } else {
        include_bytes!("../icons/32x32.png")
    };
    let icon = tauri::image::Image::from_bytes(icon_bytes)
        .map_err(|e| LenzError::internal(format!("load tray icon: {e}")))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
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
    #[cfg(target_os = "macos")]
    if let Some(item) = app.menu().and_then(|menu| menu.get(ID_SETTINGS)) {
        if let Some(item) = item.as_menuitem() {
            item.set_text(if crate::core::system::is_chinese(app) {
                "设置..."
            } else {
                "Settings..."
            })?;
        }
    }
    tray.set_menu(Some(menu))?;
    Ok(())
}

/// 处理托盘与应用菜单事件(包括 macOS 的 Cmd+,)。
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        ID_OPEN_MAIN | ID_SETTINGS => show_main_window(app),
        ID_SELECTION => emit_shortcut(app, "shortcut://selection"),
        ID_SCREENSHOT => emit_shortcut(app, "shortcut://screenshot"),
        ID_OVERLAY => emit_shortcut(app, "shortcut://overlay"),
        ID_CLOSE_OVERLAY => {
            let _ = crate::core::overlay::close(app);
        }
        ID_TOGGLE_LISTEN => toggle_listen(app),
        ID_CHECK_UPDATE => emit_shortcut(app, "tray://check-update"),
        ID_ABOUT => show_about(app),
        ID_QUIT => app.exit(0),
        other => log::debug!("unhandled tray menu event: {other}"),
    }
}

/// 显示并聚焦 main 窗口(spec 10 §1.3)。
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    // 翻译蒙层置顶且接收键盘。切换到设置前先关闭,避免主窗口在蒙层后夺走焦点。
    if app.get_webview_window("overlay").is_some() {
        if let Err(error) = crate::core::overlay::close(app) {
            log::warn!("close overlay before showing main window failed: {error}");
            return;
        }
    }
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
    // config://changed 订阅者据 enabled 启停 SelectionListener。
}

/// emit 一个无载荷事件(用于将托盘点击复用为快捷键触发路径)。
fn emit_shortcut<R: Runtime>(app: &AppHandle<R>, event: &str) {
    if let Err(e) = app.emit(event, ()) {
        log::warn!("emit {event} from tray failed: {e}");
    }
}
