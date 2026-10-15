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
            commands::translate::translate_batch,
            commands::translate::test_engine,
            // ocr
            commands::ocr::run_ocr,
            // capture
            commands::capture::capture_screen_region,
            commands::capture::start_selection,
            commands::capture::cancel_selection,
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
        .manage(crate::core::ocr::default_engine())
        .invoke_handler(specta.invoke_handler())
        // 主窗口关闭(红灯 / Cmd+W)时隐藏而非销毁,保证托盘「打开主窗口 / 设置」
        // 仍能找到并重新显示它(spec 10 §1.3)。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                } else if window.label() == "overlay" {
                    api.prevent_close();
                    let app = window.app_handle().clone();
                    // 原生关闭请求也统一清理会话,避免在窗口事件内递归销毁。
                    tauri::async_runtime::spawn(async move {
                        let _ = core::overlay::close(&app);
                    });
                }
            }
        })
        .setup(|app| setup::setup(app))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{export_typescript, specta_builder};

    /// 回归:specta 类型收集 + TS 导出不应爆栈;顺带生成正式 bindings.ts。
    #[test]
    fn export_bindings() {
        export_typescript(&specta_builder()).expect("specta export should not fail/overflow");
    }
}
