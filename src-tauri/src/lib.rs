//! Lenz —— 跨平台截图翻译桌面工具。Tauri 应用装配入口。

mod commands;
mod config;
mod core;
mod error;
mod setup;
mod tray;
mod types;

use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::{collect_commands, Builder as SpectaBuilder};

/// 构建 tauri-specta Builder(集中注册所有 command 与共享类型)。
///
/// 同时被运行时(`invoke_handler`)与 debug 构建的类型导出复用。
fn specta_builder() -> SpectaBuilder<tauri::Wry> {
    SpectaBuilder::<tauri::Wry>::new().commands(collect_commands![
        // config
        commands::config::get_config,
        commands::config::set_config,
        commands::config::reset_config,
        commands::config::list_engines,
        // translate
        commands::translate::translate_text,
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
        commands::overlay::toggle_selection_listener,
        commands::overlay::trigger_selection_translate,
        commands::overlay::open_accessibility_settings,
        commands::overlay::check_update,
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = specta_builder();

    // 仅 debug 构建导出 TS bindings 到前端 src/lib/bindings.ts。
    #[cfg(debug_assertions)]
    {
        use specta_typescript::Typescript;
        if let Err(e) = specta.export(
            Typescript::default(),
            "../src/lib/bindings.ts",
        ) {
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
                }
            }
        })
        .setup(|app| setup::setup(app))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::specta_builder;
    use specta_typescript::Typescript;

    /// 回归:specta 类型收集 + TS 导出不应爆栈;顺带生成正式 bindings.ts。
    #[test]
    fn export_bindings() {
        specta_builder()
            .export(Typescript::default(), "../src/lib/bindings.ts")
            .expect("specta export should not fail/overflow");
    }
}
