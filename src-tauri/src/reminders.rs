use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const MAX_DATE: i64 = 253402300799999;
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReminder {
    pub id: String,
    pub due_at: i64,
    pub preview: String,
}

#[derive(Default)]
pub struct Reminders {
    target: Mutex<Option<String>>,
}

pub fn set(connection: &mut Connection, id: &str, due_at: Option<i64>, now: i64) -> Result<()> {
    if due_at.is_some_and(|due| due <= 0 || due <= now || due > MAX_DATE) {
        return Err("invalid_reminder_time");
    }
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "storage_unavailable")?;
    let pending: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE id=?1 AND status='pending')",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| "storage_unavailable")?;
    if !pending {
        return Err("not_found");
    }
    if let Some(due) = due_at {
        tx.execute("INSERT INTO item_reminders(item_id,due_at) VALUES(?1,?2) ON CONFLICT(item_id) DO UPDATE SET due_at=excluded.due_at", params![id, due])
    } else {
        tx.execute("DELETE FROM item_reminders WHERE item_id=?1", [id])
    }.map_err(|_| "storage_unavailable")?;
    tx.commit().map_err(|_| "storage_unavailable")
}

pub fn due(connection: &Connection, now: i64) -> rusqlite::Result<Vec<DueReminder>> {
    connection.prepare("SELECT i.id,r.due_at,substr(CASE WHEN i.type='image' THEN i.title ELSE i.content END,1,180) FROM item_reminders r JOIN items i ON i.id=r.item_id WHERE i.status='pending' AND r.due_at<=?1 ORDER BY r.due_at,i.id")?
        .query_map([now], |r| Ok(DueReminder { id: r.get(0)?, due_at: r.get(1)?, preview: r.get(2)? }))?.collect()
}

fn change(connection: &Connection, id: &str, expected: i64, snooze: bool, now: i64) -> Result<()> {
    if expected <= 0 || expected > now {
        return Err("stale_reminder");
    }
    let changed = if snooze {
        let next = now.checked_add(600_000).filter(|n| *n <= MAX_DATE).ok_or("invalid_reminder_time")?;
        connection.execute("UPDATE item_reminders SET due_at=?3 WHERE item_id=?1 AND due_at=?2 AND EXISTS(SELECT 1 FROM items WHERE id=?1 AND status='pending')", params![id, expected, next])
    } else {
        connection.execute("DELETE FROM item_reminders WHERE item_id=?1 AND due_at=?2", params![id, expected])
    }.map_err(|_| "storage_unavailable")?;
    if changed == 1 {
        Ok(())
    } else {
        Err("stale_reminder")
    }
}

fn changed(app: &tauri::AppHandle) {
    let _ = app.emit_to("main", "reminders-changed", ());
}

#[tauri::command]
pub async fn set_item_reminder(
    app: tauri::AppHandle,
    path: tauri::State<'_, crate::DatabasePath>,
    id: String,
    due_at: Option<i64>,
) -> Result<()> {
    let path = path.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut db = crate::database::open(&path).map_err(|_| "storage_unavailable")?;
        set(&mut db, &id, due_at, chrono::Utc::now().timestamp_millis())
    })
    .await
    .map_err(|_| "storage_unavailable")??;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn list_due_reminders(
    path: tauri::State<'_, crate::DatabasePath>,
) -> Result<Vec<DueReminder>> {
    crate::storage(path.0.clone(), |db| {
        due(db, chrono::Utc::now().timestamp_millis())
    })
    .await
}

#[tauri::command]
pub async fn acknowledge_reminder(
    app: tauri::AppHandle,
    path: tauri::State<'_, crate::DatabasePath>,
    id: String,
    expected: i64,
) -> Result<()> {
    act(app, path.0.clone(), id, expected, false).await
}

#[tauri::command]
pub async fn snooze_reminder(
    app: tauri::AppHandle,
    path: tauri::State<'_, crate::DatabasePath>,
    id: String,
    expected: i64,
) -> Result<()> {
    act(app, path.0.clone(), id, expected, true).await
}

async fn act(
    app: tauri::AppHandle,
    path: std::path::PathBuf,
    id: String,
    expected: i64,
    snooze: bool,
) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = crate::database::open(&path).map_err(|_| "storage_unavailable")?;
        change(
            &db,
            &id,
            expected,
            snooze,
            chrono::Utc::now().timestamp_millis(),
        )
    })
    .await
    .map_err(|_| "storage_unavailable")??;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_reminder_item(
    app: tauri::AppHandle,
    path: tauri::State<'_, crate::DatabasePath>,
    id: String,
    expected: i64,
) -> Result<()> {
    let lookup = id.clone();
    let exists = crate::storage(path.0.clone(), move |db| db.query_row(
        "SELECT EXISTS(SELECT 1 FROM item_reminders r JOIN items i ON i.id=r.item_id WHERE r.item_id=?1 AND r.due_at=?2 AND r.due_at<=?3 AND i.status='pending')",
        params![lookup, expected, chrono::Utc::now().timestamp_millis()], |r| r.get::<_, bool>(0))).await?;
    if !exists {
        return Err("stale_reminder");
    }
    *app.state::<Reminders>()
        .target
        .lock()
        .map_err(|_| "panel_unavailable")? = Some(id);
    crate::show_panel(&app);
    app.emit_to("main", "reminder-open-item", ())
        .map_err(|_| "panel_unavailable")
}

#[tauri::command]
pub fn take_reminder_target(state: tauri::State<'_, Reminders>) -> Result<Option<String>> {
    Ok(state.target.lock().map_err(|_| "panel_unavailable")?.take())
}

fn present(app: &tauri::AppHandle, current: &[DueReminder]) -> tauri::Result<()> {
    let window = app
        .get_webview_window("reminders")
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    let show = !current.is_empty();
    if show && !window.is_visible()? {
        crate::position_window(&window);
    }
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_SHOWWINDOW,
        };
        // Tauri's ordinary show can activate an existing window; reminders must not interrupt typing.
        unsafe {
            SetWindowPos(
                window.hwnd()?,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | if show { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW },
            )
        }
        .map_err(|error| tauri::Error::Anyhow(error.into()))?;
    }
    #[cfg(not(target_os = "windows"))]
    if show {
        window.show()?;
    } else {
        window.hide()?;
    }
    app.emit_to("reminders", "reminders-due", current)
}

pub fn start(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let last = std::sync::Arc::new(Mutex::new(None::<Vec<DueReminder>>));
        loop {
            if let Ok(current) =
                crate::storage(app.state::<crate::DatabasePath>().0.clone(), |db| {
                    due(db, chrono::Utc::now().timestamp_millis())
                })
                .await
            {
                let handle = app.clone();
                let last = last.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Ok(mut last) = last.lock() {
                        if last.as_ref() != Some(&current) && present(&handle, &current).is_ok() {
                            *last = Some(current);
                        }
                    }
                });
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_rows_and_constraints_and_reopen_keeps_pending_reminders() {
        let path =
            std::env::temp_dir().join(format!("someday-reminders-{}.db", uuid::Uuid::new_v4()));
        {
            let mut db = crate::database::open(&path).unwrap();
            db.execute_batch("DROP TABLE item_reminders; ALTER TABLE shortcut_settings DROP COLUMN calendar_type; PRAGMA user_version=9; INSERT INTO items(rowid,id,type,content,created_at,pinned) VALUES(42,'a','text','  original  ','then',1); UPDATE note_draft SET content='draft'; UPDATE shortcut_settings SET color_palette='ocean';").unwrap();
            crate::database::migrate(&mut db).unwrap();
            assert_eq!(
                db.query_row("SELECT rowid||content||pinned FROM items", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "42  original  1"
            );
            assert_eq!(
                db.query_row("SELECT content FROM note_draft", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "draft"
            );
            assert_eq!(
                db.query_row("SELECT color_palette FROM shortcut_settings", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                "ocean"
            );
            for value in ["0", "-1", "253402300800000", "1.5", "'invalid'"] {
                assert!(db
                    .execute_batch(&format!("INSERT INTO item_reminders VALUES('a',{value})"))
                    .is_err());
            }
            assert!(db
                .execute("INSERT INTO item_reminders VALUES('missing',200)", [])
                .is_err());
            set(&mut db, "a", Some(200), 100).unwrap();
            assert!(db
                .execute("INSERT INTO item_reminders VALUES('a',300)", [])
                .is_err());
        }
        {
            let db = crate::database::open(&path).unwrap();
            assert_eq!(due(&db, 200).unwrap()[0].due_at, 200);
            assert!(due(&db, 199).unwrap().is_empty());
            assert_eq!(due(&db, 900).unwrap().len(), 1);
        }
        std::fs::remove_file(path).unwrap();
    }
    fn db() -> Connection {
        let mut db = Connection::open_in_memory().unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        crate::database::migrate(&mut db).unwrap();
        db.execute_batch("INSERT INTO items(id,type,content,created_at) VALUES('a','text','hello','now'),('b','link','https://example.com','now');").unwrap();
        db
    }
    #[test]
    fn schedule_replace_validate_and_preserve_until_acknowledged() {
        let mut db = db();
        for time in [0, 100, MAX_DATE + 1] {
            assert_eq!(
                set(&mut db, "a", Some(time), 100),
                Err("invalid_reminder_time")
            );
        }
        assert_eq!(set(&mut db, "missing", Some(200), 100), Err("not_found"));
        set(&mut db, "a", Some(300), 100).unwrap();
        set(&mut db, "a", Some(200), 100).unwrap();
        set(&mut db, "b", Some(150), 100).unwrap();
        assert!(due(&db, 149).unwrap().is_empty());
        assert_eq!(
            due(&db, 200)
                .unwrap()
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert_eq!(due(&db, 200).unwrap(), due(&db, 900).unwrap());
        assert_eq!(change(&db, "a", 300, false, 900), Err("stale_reminder"));
        change(&db, "a", 200, true, 900).unwrap();
        assert_eq!(due(&db, 600899).unwrap().len(), 1);
        assert_eq!(due(&db, 600900).unwrap().len(), 2);
        change(&db, "a", 600900, false, 600900).unwrap();
        assert_eq!(
            change(&db, "a", 600900, false, 600900),
            Err("stale_reminder")
        );
        assert_eq!(crate::database::pending_count(&db).unwrap(), 2);
        set(&mut db, "b", None, 900).unwrap();
        assert!(due(&db, MAX_DATE).unwrap().is_empty());
    }
    #[test]
    fn archive_cancels_atomically_and_delete_cascades() {
        let mut db = db();
        set(&mut db, "a", Some(200), 100).unwrap();
        db.execute_batch("CREATE TRIGGER reject_reminder_delete BEFORE DELETE ON item_reminders BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert!(crate::database::set_item_archived(&db, "a", true).is_err());
        assert_eq!(crate::database::pending_count(&db).unwrap(), 2);
        db.execute_batch("DROP TRIGGER reject_reminder_delete")
            .unwrap();
        crate::database::set_item_archived(&db, "a", true).unwrap();
        assert!(due(&db, 200).unwrap().is_empty());
        assert_eq!(set(&mut db, "a", Some(300), 100), Err("not_found"));
        crate::database::set_item_archived(&db, "a", false).unwrap();
        assert!(due(&db, 200).unwrap().is_empty());
        set(&mut db, "a", Some(200), 100).unwrap();
        crate::database::delete_item(&db, "a").unwrap();
        assert!(due(&db, 200).unwrap().is_empty());
    }
}
