//! 全局快捷键注册 / 解绑(spec 10 §2)。
//!
//! 与 `core/` 的其他模块不同,快捷键在 M1 即需可用:启动阶段按 config 注册,
//! 触发时 emit 对应 `shortcut://*` 事件。配置变更(`shortcuts.*`)后重绑。

use std::str::FromStr;

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::config::schema::ShortcutConfig;
use crate::error::LenzError;

/// 各快捷键触发时 emit 的事件名(spec 00 §4.3)。
const EVENT_SELECTION: &str = "shortcut://selection";
const EVENT_SCREENSHOT: &str = "shortcut://screenshot";
const EVENT_OVERLAY: &str = "shortcut://overlay";
const EVENT_OPEN_MAIN: &str = "shortcut://open-main";

/// 依据配置注册全部全局快捷键(先全部注销,再逐项注册)。
pub fn register_shortcuts<R: Runtime>(
    app: &AppHandle<R>,
    shortcuts: &ShortcutConfig,
) -> Result<(), LenzError> {
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|e| LenzError::internal(format!("unregister_all: {e}")))?;

    register_if_set(app, &shortcuts.selection, EVENT_SELECTION)?;
    register_if_set(app, &shortcuts.screenshot, EVENT_SCREENSHOT)?;
    register_if_set(app, &shortcuts.overlay, EVENT_OVERLAY)?;
    register_if_set(app, &shortcuts.open_main, EVENT_OPEN_MAIN)?;
    Ok(())
}

/// 若快捷键字符串非空,则解析并注册;按下时 emit `event`。
fn register_if_set<R: Runtime>(
    app: &AppHandle<R>,
    accelerator: &Option<String>,
    event: &'static str,
) -> Result<(), LenzError> {
    let Some(acc) = accelerator.as_ref().filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };

    let shortcut = Shortcut::from_str(acc)
        .map_err(|e| LenzError::config(format!("invalid shortcut '{acc}': {e}")))?;

    let manager = app.global_shortcut();
    let app_for_cb = app.clone();
    manager
        .on_shortcut(shortcut, move |_app, _sc, sc_event| {
            // 仅在按下(Pressed)时触发一次,避免 Released 重复。
            if sc_event.state() == ShortcutState::Pressed {
                if let Err(e) = app_for_cb.emit(event, ()) {
                    log::warn!("emit {event} failed: {e}");
                }
            }
        })
        .map_err(|e| LenzError::config(format!("register shortcut '{acc}': {e}")))?;

    log::info!("registered global shortcut '{acc}' -> {event}");
    Ok(())
}
