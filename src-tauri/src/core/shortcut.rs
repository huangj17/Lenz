//! Transactional shortcut registration: preserve unchanged bindings and roll back
//! newly registered/removed keys on failure before accepting the config.
use crate::config::schema::ShortcutConfig;
use crate::error::LenzError;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

type Bindings = BTreeMap<u32, (Shortcut, &'static str)>;
#[derive(Default)]
pub struct ShortcutRegistry(Mutex<Bindings>);

fn bindings(config: &ShortcutConfig) -> Result<Bindings, LenzError> {
    let mut result = BTreeMap::new();
    for (value, event) in [
        (&config.selection, "shortcut://selection"),
        (&config.screenshot, "shortcut://screenshot"),
        (&config.overlay, "shortcut://overlay"),
        (&config.open_main, "shortcut://open-main"),
    ] {
        let Some(value) = value.as_ref().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let shortcut = Shortcut::from_str(value)
            .map_err(|e| LenzError::config(format!("invalid shortcut '{value}': {e}")))?;
        if result.insert(shortcut.id(), (shortcut, event)).is_some() {
            return Err(LenzError::config(format!("duplicate shortcut '{value}'")));
        }
    }
    Ok(result)
}

/// OS operations are injected so failure/rollback paths can be tested without
/// stealing real system shortcuts from the test machine.
fn transition(
    old: &Bindings,
    next: &Bindings,
    mut register: impl FnMut(Shortcut) -> Result<(), LenzError>,
    mut unregister: impl FnMut(Shortcut) -> Result<(), LenzError>,
) -> Result<(), LenzError> {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let result = (|| {
        for (id, (key, _)) in next {
            if !old.contains_key(id) {
                register(*key)?;
                added.push(*key);
            }
        }
        for (id, (key, _)) in old {
            if !next.contains_key(id) {
                unregister(*key)?;
                removed.push(*key);
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut failures = Vec::new();
        for key in removed {
            if let Err(e) = register(key) {
                failures.push(e.to_string());
            }
        }
        for key in added {
            if let Err(e) = unregister(key) {
                failures.push(e.to_string());
            }
        }
        if !failures.is_empty() {
            return Err(LenzError::config(format!(
                "{error}; shortcut rollback failed: {}",
                failures.join("; ")
            )));
        }
        return Err(error);
    }
    Ok(())
}

pub fn register_shortcuts<R: Runtime>(
    app: &AppHandle<R>,
    config: &ShortcutConfig,
) -> Result<(), LenzError> {
    let next = bindings(config)?;
    let state = app.state::<ShortcutRegistry>();
    let mut current = state
        .0
        .lock()
        .map_err(|_| LenzError::internal("shortcut lock poisoned"))?;
    transition(
        &current,
        &next,
        |key| {
            app.global_shortcut()
                .on_shortcut(key, |app, key, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let event = app
                        .state::<ShortcutRegistry>()
                        .0
                        .lock()
                        .ok()
                        .and_then(|map| map.get(&key.id()).map(|(_, event)| *event));
                    if let Some(event) = event {
                        let _ = app.emit(event, ());
                    }
                })
                .map_err(|e| LenzError::config(format!("register shortcut {key}: {e}")))
        },
        |key| {
            app.global_shortcut()
                .unregister(key)
                .map_err(|e| LenzError::config(format!("unregister shortcut {key}: {e}")))
        },
    )?;
    *current = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeSet;
    fn config(selection: &str, screenshot: &str) -> ShortcutConfig {
        ShortcutConfig {
            selection: Some(selection.into()),
            screenshot: Some(screenshot.into()),
            overlay: None,
            open_main: None,
        }
    }
    #[test]
    fn conflict_preserves_every_previous_binding() {
        let old = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let next = bindings(&config("Alt+Q", "Alt+W")).unwrap();
        let active = RefCell::new(old.keys().copied().collect::<BTreeSet<_>>());
        let conflict = *next.keys().last().unwrap();
        assert!(transition(
            &old,
            &next,
            |key| {
                if key.id() == conflict {
                    return Err(LenzError::config("occupied"));
                }
                active.borrow_mut().insert(key.id());
                Ok(())
            },
            |key| {
                active.borrow_mut().remove(&key.id());
                Ok(())
            }
        )
        .is_err());
        assert_eq!(*active.borrow(), old.keys().copied().collect());
    }
    #[test]
    fn duplicates_fail_before_os_registration() {
        assert!(bindings(&config("Alt+D", "Alt+D")).is_err());
    }
    #[test]
    fn swapping_actions_keeps_registered_keys() {
        let old = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let next = bindings(&config("Alt+S", "Alt+D")).unwrap();
        transition(
            &old,
            &next,
            |_| panic!("unnecessary register"),
            |_| panic!("unnecessary unregister"),
        )
        .unwrap();
    }
}
