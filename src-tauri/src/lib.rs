//! Lenz —— 跨平台截图翻译桌面工具。Tauri 应用装配入口。

mod commands;
mod config;
mod core;
mod error;
mod setup;
mod tray;
mod types;

#[cfg(target_os = "windows")]
pub fn diagnose_windows_ocr(output: &std::path::Path) -> Result<bool, String> {
    core::ocr::write_install_diagnostic(output).map_err(|error| error.to_string())
}

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::{collect_commands, Builder as SpectaBuilder};

/// 构建 tauri-specta Builder(集中注册所有 command 与共享类型)。
///
/// 同时被运行时(`invoke_handler`)与 debug 构建的类型导出复用。
fn specta_builder() -> SpectaBuilder<tauri::Wry> {
    SpectaBuilder::<tauri::Wry>::new()
        .commands(collect_commands![
            // config
            commands::config::get_config,
            commands::config::set_config,
            commands::config::reset_config,
            commands::config::list_engines,
            // translate
            commands::translate::translate_text,
            commands::translate::begin_popup_session,
            commands::translate::translate_popup,
            commands::translate::cancel_popup_translation,
            commands::translate::translate_batch,
            commands::translate::test_engine,
            // ocr
            commands::ocr::run_ocr,
            commands::ocr::test_ocr_service,
            // capture
            commands::capture::capture_screen_region,
            commands::capture::start_selection,
            commands::capture::cancel_selection,
            commands::capture::selection_ready,
            // screenshot (F2)
            commands::screenshot::cancel_screenshot,
            commands::screenshot::open_screen_settings,
            commands::screenshot::resize_popup,
            commands::screenshot::resize_popup_width,
            // overlay / selection / system
            commands::overlay::show_overlay,
            commands::overlay::close_overlay,
            commands::overlay::get_overlay_payload,
            commands::overlay::overlay_ready,
            commands::overlay::toggle_selection_listener,
            commands::overlay::trigger_selection_translate,
            commands::overlay::open_accessibility_settings,
            commands::overlay::set_auto_start,
            commands::overlay::check_update,
            commands::system::get_system_info,
            commands::system::open_system_resource,
            commands::system::check_for_update,
            commands::system::get_update_state,
            commands::system::get_license_notices,
        ])
        // 事件载荷不会从 command 签名自动收集,显式导出以免前端手写契约漂移。
        .typ::<types::ConfigChangedPayload>()
        .typ::<types::StartupWarning>()
        .typ::<types::RegionSelectedPayload>()
        .typ::<types::ScreenshotCost>()
        .typ::<types::ScreenshotResult>()
        .typ::<types::ScreenshotTranslating>()
        .typ::<types::ScreenshotError>()
        .typ::<types::ScreenshotProgress>()
        .typ::<types::SelectionDetected>()
        .typ::<types::SelectionInput>()
        .typ::<types::PermissionRequired>()
        .typ::<core::overlay::OverlayClosed>()
        .typ::<core::overlay::OverlayUpdated>()
}

#[cfg(any(debug_assertions, test))]
fn export_typescript(specta: &SpectaBuilder<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let path = "../src/lib/bindings.ts";
    specta.export(specta_typescript::Typescript::default(), path)?;
    // Normalize the generator's trailing spaces on both dev startup and test export.
    let source = std::fs::read_to_string(path)?;
    let formatted = source
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(path, formatted)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = specta_builder();

    // 仅 debug 构建导出 TS bindings 到前端 src/lib/bindings.ts。
    #[cfg(debug_assertions)]
    {
        if let Err(e) = export_typescript(&specta) {
            eprintln!("failed to export specta typescript bindings: {e}");
        }
    }

    tauri::Builder::default()
        // 必须最先注册:重复启动的进程在初始化任何状态前退出,把请求转交给已运行实例。
        // 否则会出现两个托盘图标、快捷键注册冲突,且两份 ConfigStore 互相覆盖配置。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 再次启动(开始菜单 `--show-main`、双击程序)视为「打开 Lenz」:显示设置窗口。
            let handle = app.clone();
            if let Err(e) = app.run_on_main_thread(move || crate::tray::show_main_window(&handle)) {
                log::warn!("show main window for second instance failed: {e}");
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("lenz-startup")
                .setup(|app, _| setup::prepare(app))
                .build(),
        )
        .manage(crate::core::ocr::default_engine())
        .manage(crate::core::freeze::FreezeState::default())
        .register_asynchronous_uri_scheme_protocol(
            crate::core::freeze::FRAME_SCHEME,
            |ctx, request, responder| {
                let app = ctx.app_handle().clone();
                std::thread::spawn(move || {
                    responder.respond(crate::core::freeze::serve(&app, &request));
                });
            },
        )
        .invoke_handler(specta.invoke_handler())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                handle_close_requested(window, api);
            }
        })
        .setup(|app| setup::setup(app))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 原生关闭请求(红灯、Cmd+W、Alt+F4)的处理方式。
#[derive(Debug, PartialEq, Eq)]
enum CloseAction {
    /// 静态窗口:只隐藏。销毁后 `get_webview_window` 再也拿不到它,对应功能需重启才恢复。
    Hide,
    /// popup:取消进行中的会话后隐藏(同 ESC / `cancel_screenshot`)。
    CancelPopup,
    /// 覆盖层:走统一关闭,清理会话与载荷。
    CloseOverlay,
    /// 选区蒙层:整组取消。只关一块屏会把其它屏的置顶蒙层留在屏幕上。
    CancelSelection,
    /// 其它窗口按默认行为关闭。
    Default,
}

fn close_action(label: &str) -> CloseAction {
    match label {
        // 主窗口隐藏而非销毁,保证托盘「打开主窗口 / 设置」仍能重新显示(spec 10 §1.3)。
        "main" | "indicator" => CloseAction::Hide,
        "popup" => CloseAction::CancelPopup,
        "overlay" => CloseAction::CloseOverlay,
        l if l == core::selection_window::SELECTION_LABEL_PREFIX
            || l.strip_prefix(core::selection_window::SELECTION_LABEL_PREFIX)
                .is_some_and(|rest| rest.starts_with('-')) =>
        {
            CloseAction::CancelSelection
        }
        _ => CloseAction::Default,
    }
}

fn handle_close_requested(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    let action = close_action(window.label());
    if action == CloseAction::Default {
        return;
    }
    api.prevent_close();
    let app = window.app_handle().clone();
    match action {
        CloseAction::Hide => {
            let _ = window.hide();
        }
        CloseAction::CancelPopup => {
            app.state::<core::screenshot::ScreenshotSession>().cancel();
            let _ = window.hide();
        }
        // 销毁放到窗口事件之外执行,避免在窗口事件内递归销毁。
        CloseAction::CloseOverlay => {
            tauri::async_runtime::spawn(async move {
                let _ = core::overlay::close(&app);
            });
        }
        CloseAction::CancelSelection => {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = core::selection_window::cancel(&app) {
                    log::warn!("cancel selection on close request failed: {e}");
                }
            });
        }
        CloseAction::Default => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{close_action, export_typescript, specta_builder, CloseAction};

    /// 回归:popup / 划词图标是静态窗口,Alt+F4 销毁后功能会失效到重启;选区蒙层
    /// 必须整组取消。
    #[test]
    fn close_requests_never_destroy_static_windows() {
        assert_eq!(close_action("main"), CloseAction::Hide);
        assert_eq!(close_action("indicator"), CloseAction::Hide);
        assert_eq!(close_action("popup"), CloseAction::CancelPopup);
        assert_eq!(close_action("overlay"), CloseAction::CloseOverlay);
        assert_eq!(close_action("selection"), CloseAction::CancelSelection);
        assert_eq!(close_action("selection-0"), CloseAction::CancelSelection);
        assert_eq!(close_action("selection-12"), CloseAction::CancelSelection);
        assert_eq!(close_action("selectionx"), CloseAction::Default);
        assert_eq!(close_action("other"), CloseAction::Default);
    }

    /// 回归:specta 类型收集 + TS 导出不应爆栈;顺带生成正式 bindings.ts。
    #[test]
    fn export_bindings() {
        export_typescript(&specta_builder()).expect("specta export should not fail/overflow");
    }
}
