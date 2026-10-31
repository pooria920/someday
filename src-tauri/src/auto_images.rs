use std::sync::{mpsc, Mutex};
use tauri::Manager;
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

#[derive(Default)]
pub struct AutoImages(Mutex<State>);

#[derive(Default)]
struct State {
    enabled: bool,
    generation: u64,
    shutdown: Option<clipboard_win::monitor::Shutdown>,
}

pub fn load(connection: &rusqlite::Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT auto_save_images FROM shortcut_settings WHERE id = 1",
        [],
        |row| row.get(0),
    )
}

pub fn available(app: &tauri::AppHandle) -> bool {
    app.state::<AutoImages>()
        .0
        .lock()
        .is_ok_and(|s| !s.enabled || s.shutdown.is_some())
}

pub fn start(app: &tauri::AppHandle) {
    let handle = app.clone();
    let _ = std::thread::spawn(move || {
        let result = crate::database::open(&handle.state::<crate::DatabasePath>().0)
            .and_then(|connection| load(&connection));
        if result == Ok(true) {
            if set(&handle, true).is_err() {
                if let Ok(mut state) = handle.state::<AutoImages>().0.lock() {
                    state.enabled = true;
                }
                crate::shortcuts::publish_background(
                    &handle,
                    crate::capture::Outcome::ReadFailed,
                    "Automatic image saving could not start. Retry in Settings.",
                    None,
                );
            }
        }
    })
    .join();
}

pub fn set(app: &tauri::AppHandle, enabled: bool) -> Result<bool, &'static str> {
    let shared = app.state::<AutoImages>();
    let mut state = shared.0.lock().map_err(|_| "settings_unavailable")?;
    if state.enabled == enabled && (!enabled || state.shutdown.is_some()) {
        return Ok(enabled);
    }
    let generation = state.generation.wrapping_add(1);
    let shutdown = if enabled {
        let (ready, receiver) = mpsc::sync_channel(1);
        let handle = app.clone();
        std::thread::Builder::new()
            .name("someday-image-clipboard".into())
            .spawn(move || {
                let mut monitor = match clipboard_win::Monitor::new() {
                    Ok(monitor) => monitor,
                    Err(_) => {
                        let _ = ready.send(Err("watcher_unavailable"));
                        return;
                    }
                };
                let mut last = unsafe { GetClipboardSequenceNumber() };
                if ready.send(Ok(monitor.shutdown_channel())).is_err() {
                    return;
                }
                while let Ok(true) = monitor.recv() {
                    let sequence = unsafe { GetClipboardSequenceNumber() };
                    if sequence == last {
                        continue;
                    }
                    last = sequence;
                    if !current(&handle, generation) {
                        break;
                    }
                    let mut result = crate::windows_clipboard::read_image_at(0, Some(sequence));
                    for _ in 0..4 {
                        if !matches!(result, Err("clipboard_unavailable"))
                            || !current(&handle, generation)
                        {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(40));
                        result = crate::windows_clipboard::read_image_at(0, Some(sequence));
                    }
                    let shared = handle.state::<AutoImages>();
                    let Ok(state) = shared.0.lock() else {
                        break;
                    };
                    if !state.enabled || state.generation != generation {
                        break;
                    }
                    match result {
                        Ok(image) => match crate::save_image(
                            &handle.state::<crate::DatabasePath>().0,
                            image,
                        ) {
                            Ok((crate::capture::Outcome::Saved, count)) => {
                                crate::shortcuts::publish_background(
                                    &handle,
                                    crate::capture::Outcome::Saved,
                                    "Image saved automatically",
                                    Some(count),
                                )
                            }
                            Ok(_) => {}
                            Err(_) => crate::shortcuts::publish_background(
                                &handle,
                                crate::capture::Outcome::StorageUnavailable,
                                "Could not save an image automatically. Please try Paste image.",
                                None,
                            ),
                        },
                        Err("no_image" | "clipboard_changed") => {}
                        Err("image_too_large") => crate::shortcuts::publish_background(
                            &handle,
                            crate::capture::Outcome::TooLarge,
                            "An image was too large to save automatically",
                            None,
                        ),
                        Err(_) => crate::shortcuts::publish_background(
                            &handle,
                            crate::capture::Outcome::ReadFailed,
                            "Could not read an image automatically. Please try Paste image.",
                            None,
                        ),
                    }
                }
                if let Ok(mut state) = handle.state::<AutoImages>().0.lock() {
                    if state.generation == generation {
                        state.shutdown = None;
                    }
                }
            })
            .map_err(|_| "watcher_unavailable")?;
        Some(receiver.recv().map_err(|_| "watcher_unavailable")??)
    } else {
        None
    };
    let connection = crate::database::open(&app.state::<crate::DatabasePath>().0)
        .map_err(|_| "settings_unavailable")?;
    let changed = connection
        .execute(
            "UPDATE shortcut_settings SET auto_save_images = ?1 WHERE id = 1",
            [enabled],
        )
        .map_err(|_| "settings_unavailable")?;
    if changed != 1 {
        return Err("settings_unavailable");
    }
    state.enabled = enabled;
    state.generation = generation;
    state.shutdown = shutdown;
    Ok(enabled)
}

fn current(app: &tauri::AppHandle, generation: u64) -> bool {
    app.state::<AutoImages>()
        .0
        .lock()
        .is_ok_and(|s| s.enabled && s.generation == generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setting_defaults_off_and_survives_reopen_without_altering_shortcuts() {
        let path = std::env::temp_dir().join(format!("someday-auto-{}.db", uuid::Uuid::new_v4()));
        let original = rusqlite::Connection::open(&path).unwrap();
        for migration in [
            include_str!("../migrations/0001_items.sql"),
            include_str!("../migrations/0002_categories.sql"),
            include_str!("../migrations/0003_shortcuts.sql"),
            include_str!("../migrations/0004_images.sql"),
        ] {
            original.execute_batch(migration).unwrap();
        }
        original.pragma_update(None, "user_version", 4).unwrap();
        original.execute("INSERT INTO items (id,type,content,created_at) VALUES ('prior','text','Keep me','date')", []).unwrap();
        drop(original);
        let connection = crate::database::open(&path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT content FROM items WHERE id = 'prior'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "Keep me"
        );
        assert!(!load(&connection).unwrap());
        connection
            .execute("UPDATE shortcut_settings SET auto_save_images = 1", [])
            .unwrap();
        assert!(connection
            .execute("UPDATE shortcut_settings SET auto_save_images = 2", [])
            .is_err());
        drop(connection);
        let connection = crate::database::open(&path).unwrap();
        assert!(load(&connection).unwrap());
        assert_eq!(
            crate::settings::load(&connection).unwrap(),
            crate::settings::ShortcutSettings::default()
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }
}
