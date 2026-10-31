use crate::capture::Outcome;
use crate::settings::ShortcutSettings;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager};

pub const CAPTURE_EVENT: &str = "capture-result";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureUpdate {
    pub sequence: u64,
    pub outcome: Outcome,
    pub message: &'static str,
    pub pending_count: Option<u32>,
}

#[derive(Default)]
pub struct CaptureState {
    pub settings: Mutex<ShortcutSettings>,
    settings_update: Mutex<()>,
    pub shortcut_available: AtomicBool,
    pub toggle_shortcut_available: AtomicBool,
    pub latest: Mutex<Option<CaptureUpdate>>,
    sequence: AtomicU64,
    gate: HotkeyGate,
    toggle_gate: HotkeyGate,
}

#[derive(Default)]
struct HotkeyGate {
    held: AtomicBool,
    busy: Arc<AtomicBool>,
}
struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl HotkeyGate {
    fn press(&self) -> Option<BusyGuard> {
        if self.held.swap(true, Ordering::AcqRel) || self.busy.swap(true, Ordering::AcqRel) {
            return None;
        }
        Some(BusyGuard(self.busy.clone()))
    }
    fn release(&self) {
        self.held.store(false, Ordering::Release);
    }
}

#[cfg(target_os = "windows")]
pub fn register(app: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
    let path = app.state::<crate::DatabasePath>().0.clone();
    // Load on a worker before registering, so startup never activates stale defaults.
    let Ok(Ok(settings)) = std::thread::spawn(move || {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| rusqlite::Error::InvalidQuery)?;
        }
        let connection = crate::database::open(&path)?;
        crate::settings::load(&connection)
    })
    .join() else {
        return;
    };
    let Ok(shortcut) = settings.capture.parse::<Shortcut>() else {
        return;
    };
    let Ok(toggle) = settings.toggle.parse::<Shortcut>() else {
        return;
    };
    *app.state::<CaptureState>().settings.lock().unwrap() = settings;
    let installed = app
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, received, event| {
                    let state = app.state::<CaptureState>();
                    let settings = match state.settings.lock() {
                        Ok(settings) => settings.clone(),
                        Err(_) => return,
                    };
                    let Ok(shortcut) = settings.capture.parse::<Shortcut>() else {
                        return;
                    };
                    let Ok(toggle) = settings.toggle.parse::<Shortcut>() else {
                        return;
                    };
                    if received == &toggle {
                        if event.state() == ShortcutState::Released {
                            state.toggle_gate.release();
                        } else if let Some(guard) = state.toggle_gate.press() {
                            let handle = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                let _guard = guard;
                                crate::toggle_panel(&handle);
                            });
                        }
                        return;
                    }
                    if received != &shortcut {
                        return;
                    }
                    if event.state() == ShortcutState::Released {
                        state.gate.release();
                        return;
                    }
                    let Some(guard) = state.gate.press() else {
                        return;
                    };
                    let target = crate::windows_selection::Target::current();
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let _guard = guard;
                        let bounds = target
                            .as_ref()
                            .ok()
                            .and_then(|target| target.feedback_bounds());
                        let result = target
                            .and_then(|mut target| {
                                crate::capture::read_formatted_content(&mut target)
                            })
                            .and_then(|(text, html)| {
                                let path = &app.state::<crate::DatabasePath>().0;
                                crate::capture::save_formatted(path, &text, html.as_deref())
                            });
                        let (outcome, count) = match result {
                            Ok((outcome, count)) => (outcome, Some(count)),
                            Err(outcome) => (outcome, None),
                        };
                        publish(&app, outcome, count, bounds);
                    });
                })
                .build(),
        )
        .is_ok();
    let available = installed && app.global_shortcut().register(shortcut).is_ok();
    app.state::<CaptureState>()
        .shortcut_available
        .store(available, Ordering::Release);
    let toggle_available = installed && app.global_shortcut().register(toggle).is_ok();
    app.state::<CaptureState>()
        .toggle_shortcut_available
        .store(toggle_available, Ordering::Release);
}

// Register additions before saving or retiring anything, including when swapping actions.
fn activate(
    settings: &ShortcutSettings,
    mut is_registered: impl FnMut(&str) -> bool,
    mut register: impl FnMut(&str) -> Result<(), &'static str>,
    mut unregister: impl FnMut(&str),
    persist: impl FnOnce() -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    settings.validate()?;
    let mut added = Vec::new();
    let result = (|| {
        for shortcut in [&settings.capture, &settings.toggle] {
            if !is_registered(shortcut) {
                register(shortcut)?;
                added.push(shortcut.as_str());
            }
        }
        persist()
    })();
    if result.is_err() {
        for shortcut in added {
            unregister(shortcut);
        }
    }
    result
}

#[cfg(target_os = "windows")]
pub fn update(
    app: &tauri::AppHandle,
    settings: ShortcutSettings,
) -> Result<ShortcutSettings, &'static str> {
    let state = app.state::<CaptureState>();
    let _update = state
        .settings_update
        .lock()
        .map_err(|_| "settings_unavailable")?;
    let old = state
        .settings
        .lock()
        .map_err(|_| "settings_unavailable")?
        .clone();
    let manager = app
        .try_state::<tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>>()
        .ok_or("shortcut_unavailable")?;
    activate(
        &settings,
        |key| manager.is_registered(key),
        |key| manager.register(key).map_err(|_| "shortcut_unavailable"),
        |key| {
            let _ = manager.unregister(key);
        },
        || {
            let connection = crate::database::open(&app.state::<crate::DatabasePath>().0)
                .map_err(|_| "settings_unavailable")?;
            crate::settings::save(&connection, &settings).map_err(|_| "settings_unavailable")
        },
    )?;
    *state.settings.lock().map_err(|_| "settings_unavailable")? = settings.clone();
    state.gate.release();
    state.toggle_gate.release();
    state.shortcut_available.store(true, Ordering::Release);
    state
        .toggle_shortcut_available
        .store(true, Ordering::Release);
    for key in [&old.capture, &old.toggle] {
        if key != &settings.capture
            && key != &settings.toggle
            && manager.is_registered(key.as_str())
        {
            let _ = manager.unregister(key.as_str());
        }
    }
    Ok(settings)
}

#[cfg(target_os = "windows")]
pub fn publish_background(
    app: &tauri::AppHandle,
    outcome: Outcome,
    message: &'static str,
    pending_count: Option<u32>,
) {
    let state = app.state::<CaptureState>();
    let update = CaptureUpdate {
        sequence: state.sequence.fetch_add(1, Ordering::AcqRel) + 1,
        outcome,
        message,
        pending_count,
    };
    if let Ok(mut latest) = state.latest.lock() {
        *latest = Some(update.clone());
    }
    let _ = app.emit(CAPTURE_EVENT, update);
}

fn publish(
    app: &tauri::AppHandle,
    outcome: Outcome,
    pending_count: Option<u32>,
    bounds: Option<(i32, i32, u32, u32)>,
) {
    let state = app.state::<CaptureState>();
    let sequence = state.sequence.fetch_add(1, Ordering::AcqRel) + 1;
    let update = CaptureUpdate {
        sequence,
        outcome,
        message: outcome.message(),
        pending_count,
    };
    if let Ok(mut latest) = state.latest.lock() {
        *latest = Some(update.clone());
    }
    let _ = app.emit(CAPTURE_EVENT, update);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("feedback") {
            if let Some((x, y, width, height)) = bounds {
                let _ = window.set_size(tauri::PhysicalSize::new(width, height));
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
            let _ = window.show();
        }
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let state = handle.state::<CaptureState>();
            if state
                .latest
                .lock()
                .ok()
                .and_then(|latest| latest.as_ref().map(|latest| latest.sequence))
                == Some(sequence)
            {
                if let Some(window) = handle.get_webview_window("feedback") {
                    let _ = window.hide();
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeSet};

    #[test]
    fn conflicts_and_storage_failure_keep_previous_registrations() {
        for fail_registration in [true, false] {
            let old = ShortcutSettings::default();
            let registered = RefCell::new(BTreeSet::from([old.capture, old.toggle]));
            let original = registered.borrow().clone();
            let settings = ShortcutSettings {
                capture: "Alt+F8".into(),
                toggle: "Alt+F9".into(),
            };
            let result = activate(
                &settings,
                |key| registered.borrow().contains(key),
                |key| {
                    if fail_registration && key == "Alt+F9" {
                        return Err("shortcut_unavailable");
                    }
                    registered.borrow_mut().insert(key.to_owned());
                    Ok(())
                },
                |key| {
                    registered.borrow_mut().remove(key);
                },
                || Err("settings_unavailable"),
            );
            assert!(result.is_err());
            assert_eq!(*registered.borrow(), original);
        }
    }

    #[test]
    fn swapped_actions_reuse_existing_registrations_and_invalid_input_never_registers() {
        let old = ShortcutSettings::default();
        let settings = ShortcutSettings {
            capture: old.toggle.clone(),
            toggle: old.capture.clone(),
        };
        let result = activate(
            &settings,
            |key| key == old.capture || key == old.toggle,
            |_| panic!("must reuse existing shortcuts"),
            |_| panic!("must retain existing shortcuts"),
            || Ok(()),
        );
        assert_eq!(result, Ok(()));
        let invalid = ShortcutSettings {
            capture: "S".into(),
            toggle: "T".into(),
        };
        assert_eq!(
            activate(
                &invalid,
                |_| panic!("validate first"),
                |_| unreachable!(),
                |_| unreachable!(),
                || unreachable!()
            ),
            Err("invalid_shortcut")
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn every_supported_shortcut_parses_for_the_native_plugin() {
        use tauri_plugin_global_shortcut::Shortcut;
        for modifiers in [
            "Ctrl",
            "Alt",
            "Ctrl+Shift",
            "Alt+Shift",
            "Ctrl+Alt",
            "Ctrl+Alt+Shift",
        ] {
            let keys = ('A'..='Z')
                .chain('0'..='9')
                .map(|key| key.to_string())
                .chain(["Space".to_owned()])
                .chain((1..=12).map(|n| format!("F{n}")));
            for key in keys {
                assert!(format!("{modifiers}+{key}").parse::<Shortcut>().is_ok());
            }
        }
    }
    #[test]
    fn toggle_and_capture_have_independent_repeat_guards() {
        let state = CaptureState::default();
        let capture = state.gate.press().unwrap();
        drop(state.toggle_gate.press().unwrap());
        assert!(state.toggle_gate.press().is_none());
        state.toggle_gate.release();
        assert!(state.toggle_gate.press().is_some());
        assert!(state.gate.press().is_none());
        drop(capture);
    }

    #[test]
    fn repeat_and_overlapping_presses_are_ignored() {
        let gate = HotkeyGate::default();
        let operation = gate.press().unwrap();
        assert!(gate.press().is_none());
        drop(operation);
        assert!(gate.press().is_none());
        gate.release();
        let operation = gate.press().unwrap();
        gate.release();
        assert!(gate.press().is_none());
        drop(operation);
        gate.release();
        assert!(gate.press().is_some());
    }
}
