//! Transactional shortcut registration: preserve unchanged bindings and roll back
//! newly registered/removed keys on failure before accepting the config.
//!
//! Shortcuts that could not be registered (another app owns the key) are kept as
//! *inactive* instead of failing everything: at startup and on reset each key is
//! registered on its own, so one conflict never disables the other shortcuts, and an
//! inactive key is retried whenever the shortcut config is applied again.
use crate::config::schema::ShortcutConfig;
use crate::error::LenzError;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

type Bindings = BTreeMap<u32, (Shortcut, &'static str)>;

#[derive(Default)]
struct Registered {
    /// Desired bindings from the applied config (drives event dispatch).
    bindings: Bindings,
    /// Ids in `bindings` the OS refused; they are not registered right now.
    inactive: BTreeSet<u32>,
}

#[derive(Default)]
pub struct ShortcutRegistry(Mutex<Registered>);

/// How newly added keys are treated when the OS refuses them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The user is choosing these keys now: refuse the whole change and roll back.
    Strict,
    /// Startup / reset: keep every key that works and report the rest as inactive.
    BestEffort,
}

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
///
/// Returns the ids of `next` that ended up unregistered. Keys new to this config
/// follow `mode`; keys that were already desired but inactive are retried without
/// blocking the change, so a lingering conflict on one key never prevents editing
/// another.
fn transition(
    old: &Bindings,
    inactive: &BTreeSet<u32>,
    next: &Bindings,
    mode: Mode,
    mut register: impl FnMut(Shortcut) -> Result<(), LenzError>,
    mut unregister: impl FnMut(Shortcut) -> Result<(), LenzError>,
) -> Result<BTreeSet<u32>, LenzError> {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut next_inactive = BTreeSet::new();
    let result = (|| {
        for (id, (key, _)) in next {
            if !old.contains_key(id) {
                match register(*key) {
                    Ok(()) => added.push(*key),
                    Err(e) if mode == Mode::BestEffort => {
                        log::warn!("{e}");
                        next_inactive.insert(*id);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        for (id, (key, _)) in old {
            if !next.contains_key(id) && !inactive.contains(id) {
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
    // Unchanged keys the OS refused earlier: the other app may have released them.
    for id in inactive {
        if let (Some((key, _)), true) = (next.get(id), old.contains_key(id)) {
            if let Err(e) = register(*key) {
                log::warn!("{e}");
                next_inactive.insert(*id);
            }
        }
    }
    Ok(next_inactive)
}

/// Applies `config`. Returns the shortcuts that could not be registered (only
/// possible in [`Mode::BestEffort`], or for keys that were already inactive).
pub fn register_shortcuts<R: Runtime>(
    app: &AppHandle<R>,
    config: &ShortcutConfig,
    mode: Mode,
) -> Result<Vec<String>, LenzError> {
    let next = bindings(config)?;
    let state = app.state::<ShortcutRegistry>();
    let mut current = state
        .0
        .lock()
        .map_err(|_| LenzError::internal("shortcut lock poisoned"))?;
    let inactive = transition(
        &current.bindings,
        &current.inactive,
        &next,
        mode,
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
                        .and_then(|map| map.bindings.get(&key.id()).map(|(_, event)| *event));
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
    let failed = inactive
        .iter()
        .filter_map(|id| next.get(id).map(|(key, _)| key.to_string()))
        .collect();
    current.bindings = next;
    current.inactive = inactive;
    Ok(failed)
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
    fn none() -> BTreeSet<u32> {
        BTreeSet::new()
    }

    #[test]
    fn conflict_preserves_every_previous_binding() {
        let old = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let next = bindings(&config("Alt+Q", "Alt+W")).unwrap();
        let active = RefCell::new(old.keys().copied().collect::<BTreeSet<_>>());
        let conflict = *next.keys().last().unwrap();
        assert!(transition(
            &old,
            &none(),
            &next,
            Mode::Strict,
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

    /// 回归:启动时一个快捷键被别的程序占用,不能连带其它快捷键全部失效。
    #[test]
    fn startup_conflict_keeps_the_other_shortcuts() {
        let next = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let occupied = Shortcut::from_str("Alt+S").unwrap().id();
        let active = RefCell::new(BTreeSet::new());
        let inactive = transition(
            &Bindings::new(),
            &none(),
            &next,
            Mode::BestEffort,
            |key| {
                if key.id() == occupied {
                    return Err(LenzError::config("occupied"));
                }
                active.borrow_mut().insert(key.id());
                Ok(())
            },
            |_| panic!("nothing to unregister"),
        )
        .unwrap();
        assert_eq!(inactive, BTreeSet::from([occupied]));
        assert_eq!(
            *active.borrow(),
            BTreeSet::from([Shortcut::from_str("Alt+D").unwrap().id()])
        );
    }

    /// 某个快捷键仍被占用时,编辑另一个快捷键不能被它拖累;仍占用的键保持 inactive,
    /// 已释放的键会被重新注册。
    #[test]
    fn inactive_key_never_blocks_editing_another() {
        let old = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let next = bindings(&config("Alt+Q", "Alt+S")).unwrap();
        let alt_s = Shortcut::from_str("Alt+S").unwrap().id();
        let alt_d = Shortcut::from_str("Alt+D").unwrap().id();
        let alt_q = Shortcut::from_str("Alt+Q").unwrap().id();
        let registered = RefCell::new(BTreeSet::from([alt_d]));
        let still_occupied = transition(
            &old,
            &BTreeSet::from([alt_s]),
            &next,
            Mode::Strict,
            |key| {
                if key.id() == alt_s {
                    return Err(LenzError::config("occupied"));
                }
                registered.borrow_mut().insert(key.id());
                Ok(())
            },
            |key| {
                registered.borrow_mut().remove(&key.id());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(still_occupied, BTreeSet::from([alt_s]));
        assert_eq!(*registered.borrow(), BTreeSet::from([alt_q]));

        // The other app released Alt+S: the next apply registers it.
        let released = transition(
            &next,
            &still_occupied,
            &next,
            Mode::Strict,
            |_| Ok(()),
            |_| panic!("nothing to unregister"),
        )
        .unwrap();
        assert!(released.is_empty());
    }

    /// 从未注册成功的键被移出配置时不能去注销它(会误报失败)。
    #[test]
    fn removing_an_inactive_key_does_not_unregister_it() {
        let old = bindings(&config("Alt+D", "Alt+S")).unwrap();
        let alt_s = Shortcut::from_str("Alt+S").unwrap().id();
        let next = bindings(&ShortcutConfig {
            screenshot: None,
            ..config("Alt+D", "Alt+S")
        })
        .unwrap();
        transition(
            &old,
            &BTreeSet::from([alt_s]),
            &next,
            Mode::Strict,
            |_| panic!("unnecessary register"),
            |_| panic!("inactive key must not be unregistered"),
        )
        .unwrap();
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
            &none(),
            &next,
            Mode::Strict,
            |_| panic!("unnecessary register"),
            |_| panic!("unnecessary unregister"),
        )
        .unwrap();
    }
}
