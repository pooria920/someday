use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MIGRATIONS: [&str; 11] = [
    include_str!("../migrations/0001_items.sql"),
    include_str!("../migrations/0002_categories.sql"),
    include_str!("../migrations/0003_shortcuts.sql"),
    include_str!("../migrations/0004_images.sql"),
    include_str!("../migrations/0005_auto_images.sql"),
    include_str!("../migrations/0006_formatted_text.sql"),
    include_str!("../migrations/0007_theme.sql"),
    include_str!("../migrations/0008_pins_and_draft.sql"),
    include_str!("../migrations/0009_color_palette.sql"),
    include_str!("../migrations/0010_reminders.sql"),
    include_str!("../migrations/0011_calendar_type.sql"),
];
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub message: String,
    pub rescue_path: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRestore {
    pub token: String,
    pub name: String,
    pub item_count: u64,
}
#[derive(Default)]
pub struct Backups {
    pub prepared: Mutex<Option<PreparedRestore>>,
    pub report: RestoreReport,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    token: String,
    hash: Vec<u8>,
    created_at: i64,
}
fn directory(database: &Path) -> PathBuf {
    database.with_file_name("backups")
}
fn staged(database: &Path, token: &str) -> PathBuf {
    directory(database).join(format!("restore-{token}.db"))
}
fn rescue(database: &Path, token: &str, created_at: i64) -> PathBuf {
    directory(database).join(format!("rescue-{created_at}-{token}.someday-backup"))
}
fn sync(path: &Path) -> Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| "backup_write_failed")
}
fn hash(path: &Path) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path).map_err(|_| "backup_read_failed")?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|_| "backup_read_failed")?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest.finalize().to_vec())
}

#[cfg(target_os = "windows")]
pub fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MoveFileExW(
            windows::core::PCWSTR(source.as_ptr()),
            windows::core::PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(std::io::Error::other)
}
#[cfg(not(target_os = "windows"))]
pub fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

fn readonly(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "invalid_backup")?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(|_| "invalid_backup")?;
    Ok(connection)
}
fn schema(
    connection: &Connection,
) -> rusqlite::Result<Vec<(String, String, String, Option<String>)>> {
    connection
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name")?
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect()
}
fn validate_schema(connection: &Connection) -> Result<u32> {
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| "invalid_backup")?;
    if version > 11 {
        return Err("backup_newer_version");
    }
    if version == 0 {
        return Err("invalid_backup");
    }
    let expected = Connection::open_in_memory().map_err(|_| "invalid_backup")?;
    for migration in MIGRATIONS.iter().take(version as usize) {
        expected
            .execute_batch(migration)
            .map_err(|_| "invalid_backup")?;
    }
    if schema(connection).map_err(|_| "invalid_backup")?
        != schema(&expected).map_err(|_| "invalid_backup")?
    {
        return Err("invalid_backup");
    }
    let mut check = connection
        .prepare("PRAGMA integrity_check")
        .map_err(|_| "invalid_backup")?;
    let messages = check
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "invalid_backup")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| "invalid_backup")?;
    if messages != ["ok"] {
        return Err("invalid_backup");
    }
    if connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|_| "invalid_backup")?
        .query([])
        .map_err(|_| "invalid_backup")?
        .next()
        .map_err(|_| "invalid_backup")?
        .is_some()
    {
        return Err("invalid_backup");
    }
    Ok(version)
}
fn validate_data(connection: &Connection) -> Result<u64> {
    let invalid_reminders: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM item_reminders r JOIN items i ON i.id=r.item_id WHERE typeof(r.item_id)<>'text' OR typeof(r.due_at)<>'integer' OR r.due_at<=0 OR r.due_at>253402300799999 OR i.status<>'pending')", [], |r| r.get(0)
    ).map_err(|_| "invalid_backup")?;
    if invalid_reminders {
        return Err("invalid_backup");
    }
    let draft: String = connection
        .query_row("SELECT content FROM note_draft WHERE id=1", [], |r| {
            r.get(0)
        })
        .map_err(|_| "invalid_backup")?;
    crate::note_draft::validate(&draft).map_err(|_| "invalid_backup")?;
    let drafts: i64 = connection
        .query_row("SELECT count(*) FROM note_draft", [], |r| r.get(0))
        .map_err(|_| "invalid_backup")?;
    if drafts != 1 {
        return Err("invalid_backup");
    }
    crate::settings::load(connection).map_err(|_| "invalid_backup")?;
    let settings_count: i64 = connection
        .query_row("SELECT count(*) FROM shortcut_settings", [], |row| {
            row.get(0)
        })
        .map_err(|_| "invalid_backup")?;
    if settings_count != 1 {
        return Err("invalid_backup");
    }
    let invalid_types: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM items WHERE typeof(id)<>'text' OR typeof(created_at)<>'text' OR (content IS NOT NULL AND typeof(content)<>'text') OR (title IS NOT NULL AND typeof(title)<>'text') OR (completed_at IS NOT NULL AND typeof(completed_at)<>'text') OR (category_id IS NOT NULL AND typeof(category_id)<>'text') OR (formatted_html IS NOT NULL AND typeof(formatted_html)<>'text')) OR EXISTS(SELECT 1 FROM categories WHERE typeof(id)<>'text' OR typeof(name)<>'text')", [], |r| r.get(0)
    ).map_err(|_| "invalid_backup")?;
    if invalid_types {
        return Err("invalid_backup");
    }
    let oversized: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE length(CAST(content AS BLOB)) > 4194304)",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "invalid_backup")?;
    if oversized {
        return Err("invalid_backup");
    }
    let mut query = connection.prepare("SELECT type,content,formatted_html,image_original,image_thumbnail,image_mime,image_width,image_height,image_hash FROM items ORDER BY rowid").map_err(|_| "invalid_backup")?;
    let mut rows = query.query([]).map_err(|_| "invalid_backup")?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(|_| "invalid_backup")? {
        let kind: String = row.get(0).map_err(|_| "invalid_backup")?;
        if kind == "image" {
            let original: Vec<u8> = row.get(3).map_err(|_| "invalid_backup")?;
            let thumbnail: Vec<u8> = row.get(4).map_err(|_| "invalid_backup")?;
            let image =
                crate::images::prepare(original, "backup".into()).map_err(|_| "invalid_backup")?;
            let mime: String = row.get(5).map_err(|_| "invalid_backup")?;
            let width: u32 = row.get(6).map_err(|_| "invalid_backup")?;
            let height: u32 = row.get(7).map_err(|_| "invalid_backup")?;
            let digest: Vec<u8> = row.get(8).map_err(|_| "invalid_backup")?;
            let thumb = crate::images::decode(&thumbnail).map_err(|_| "invalid_backup")?;
            if image.mime_type != mime
                || image.width != width
                || image.height != height
                || image.hash != digest
                || thumb.width() > 320
                || thumb.height() > 320
                || !thumbnail.starts_with(b"\x89PNG\r\n\x1a\n")
            {
                return Err("invalid_backup");
            }
        } else {
            let content: String = row.get(1).map_err(|_| "invalid_backup")?;
            if content.trim().is_empty()
                || content.encode_utf16().count() > crate::capture::MAX_TEXT_UNITS
            {
                return Err("invalid_backup");
            }
            let html: Option<String> = row.get(2).map_err(|_| "invalid_backup")?;
            if let Some(html) = html {
                if crate::formatted_text::clean(&html).as_deref() != Some(html.as_str()) {
                    return Err("invalid_backup");
                }
            }
        }
        count += 1;
    }
    Ok(count)
}

fn snapshot(connection: &Connection, destination: &Path) -> Result<()> {
    let temporary =
        destination.with_file_name(format!(".someday-backup-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        connection
            .backup(rusqlite::MAIN_DB, &temporary, None)
            .map_err(|_| "backup_write_failed")?;
        sync(&temporary)?;
        replace_file(&temporary, destination).map_err(|_| "backup_write_failed")
    })();
    let _ = fs::remove_file(temporary);
    result
}
fn external_destination(database: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .ok_or("backup_write_failed")?;
    let home = database
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .ok_or("backup_write_failed")?;
    // Keep backups outside app data so no internal database, sidecar or restore file can be overwritten.
    if parent.starts_with(&home) {
        return Err("backup_internal_path");
    }
    if destination.exists()
        && destination
            .canonicalize()
            .map_err(|_| "backup_write_failed")?
            .starts_with(&home)
    {
        return Err("backup_internal_path");
    }
    Ok(())
}
pub fn create(database: &Path, destination: &Path) -> Result<()> {
    external_destination(database, destination)?;
    let connection = crate::database::open(database).map_err(|_| "storage_unavailable")?;
    snapshot(&connection, destination)
}
pub fn prepare(
    database: &Path,
    source: &Path,
    previous: &mut Option<PreparedRestore>,
) -> Result<PreparedRestore> {
    if directory(database).join("pending.json").exists() {
        return Err("restore_pending");
    }
    if let Some(old) = previous.take() {
        let _ = fs::remove_file(staged(database, &old.token));
    }
    fs::create_dir_all(directory(database)).map_err(|_| "backup_write_failed")?;
    let token = uuid::Uuid::new_v4().to_string();
    let destination = staged(database, &token);
    let result = (|| {
        fs::copy(source, &destination).map_err(|_| "backup_read_failed")?;
        let connection = readonly(&destination)?;
        let version = validate_schema(&connection)?;
        drop(connection);
        let mut connection = Connection::open(&destination).map_err(|_| "invalid_backup")?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|_| "invalid_backup")?;
        // Migration v4 rebuilds items; retain original insertion rowids, including gaps.
        if version < 4 {
            connection
                .execute_batch(
                    "CREATE TEMP TABLE restore_order AS SELECT rowid AS saved_rowid,id FROM items;",
                )
                .map_err(|_| "invalid_backup")?;
        }
        crate::database::migrate(&mut connection).map_err(|_| "invalid_backup")?;
        if version < 4 {
            let transaction = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| "invalid_backup")?;
            transaction.execute_batch("CREATE TEMP TABLE restore_items AS SELECT * FROM items; DELETE FROM items;
                INSERT INTO items(rowid,id,type,content,title,status,created_at,completed_at,category_id,image_original,image_thumbnail,image_mime,image_width,image_height,image_hash,formatted_html,pinned)
                SELECT o.saved_rowid,i.* FROM temp.restore_items i JOIN temp.restore_order o ON i.id=o.id ORDER BY o.saved_rowid;
                DROP TABLE temp.restore_items; DROP TABLE temp.restore_order;").map_err(|_| "invalid_backup")?;
            transaction.commit().map_err(|_| "invalid_backup")?;
        }
        validate_schema(&connection)?;
        let item_count = validate_data(&connection)?;
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|_| "invalid_backup")?;
        drop(connection);
        sync(&destination)?;
        Ok(PreparedRestore {
            token,
            name: source
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            item_count,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    if let Ok(prepared) = &result {
        *previous = Some(prepared.clone());
    }
    result
}
pub fn schedule(
    database: &Path,
    token: &str,
    prepared: &mut Option<PreparedRestore>,
) -> Result<()> {
    if !prepared.as_ref().is_some_and(|p| p.token == token) {
        return Err("restore_expired");
    }
    let pending = Pending {
        token: token.into(),
        hash: hash(&staged(database, token))?,
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    let folder = directory(database);
    let temporary = folder.join("pending.tmp");
    let bytes = serde_json::to_vec(&pending).map_err(|_| "backup_write_failed")?;
    let mut file = fs::File::create(&temporary).map_err(|_| "backup_write_failed")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "backup_write_failed")?;
    drop(file);
    replace_file(&temporary, &folder.join("pending.json")).map_err(|_| "backup_write_failed")?;
    *prepared = None;
    Ok(())
}

pub fn apply_pending(database: &Path) -> RestoreReport {
    let marker = directory(database).join("pending.json");
    if !marker.exists() {
        cleanup_staged(database);
        return RestoreReport::default();
    }
    let mut rescue_path = None;
    let result = (|| {
        let pending: Pending =
            serde_json::from_slice(&fs::read(&marker).map_err(|_| "backup_read_failed")?)
                .map_err(|_| "invalid_backup")?;
        if uuid::Uuid::parse_str(&pending.token).is_err() || pending.hash.len() != 32 {
            return Err("invalid_backup");
        }
        let destination = staged(database, &pending.token);
        let safety = rescue(database, &pending.token, pending.created_at);
        rescue_path = Some(safety.to_string_lossy().into_owned());
        // The atomic rename may have succeeded immediately before a process interruption.
        if !destination.exists() && hash(database)? == pending.hash {
            return Ok(());
        }
        if hash(&destination)? != pending.hash {
            return Err("invalid_backup");
        }
        let restored = readonly(&destination)?;
        validate_schema(&restored)?;
        validate_data(&restored)?;
        drop(restored);
        let current = crate::database::open(database).map_err(|_| "storage_unavailable")?;
        snapshot(&current, &safety)?;
        // Finish any WAL before replacing the database, with no other workers running yet.
        current
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|_| "storage_unavailable")?;
        drop(current);
        replace_file(&destination, database).map_err(|_| "backup_write_failed")?;
        Ok(())
    })();
    // A failed marker cleanup must not allow a later restart to apply old data again.
    let cleared = fs::remove_file(&marker).is_ok();
    cleanup_staged(database);
    RestoreReport {
        message: match result {
            Ok(()) => if cleared { "Backup restored. Someday restarted with your restored data and settings." } else { "Backup restored, but its recovery marker could not be removed. Check the backup folder before restarting." },
            Err(_) => "Could not restore the backup. Your current database was kept. Check available disk space and try again.",
        }.into(),
        rescue_path: rescue_path.filter(|path| Path::new(path).is_file()),
    }
}
fn cleanup_staged(database: &Path) {
    if let Ok(entries) = fs::read_dir(directory(database)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("restore-") && name.ends_with(".db") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        folder: PathBuf,
        database: PathBuf,
        backup: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let folder =
                std::env::temp_dir().join(format!("someday-backup-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(folder.join("app")).unwrap();
            Self {
                database: folder.join("app/someday.db"),
                backup: folder.join("saved.someday-backup"),
                folder,
            }
        }
        fn save(&self) {
            create(&self.database, &self.backup).unwrap();
        }
        fn prepare(&self) -> PreparedRestore {
            prepare(&self.database, &self.backup, &mut None).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn full_roundtrip_preserves_bytes_settings_rowids_and_last_item_policy() {
        let f = Fixture::new();
        let mut db = crate::database::open(&f.database).unwrap();
        crate::capture::save_formatted(
            &f.database,
            "  سلام\r\nworld  ",
            Some("<p><strong>سلام</strong></p>"),
        )
        .unwrap();
        let first: String = db
            .query_row("SELECT id FROM items", [], |r| r.get(0))
            .unwrap();
        let category = crate::database::create_category(&db, "کار").unwrap();
        crate::database::set_item_category(&db, &first, Some(&category.id)).unwrap();
        crate::database::set_item_archived(&db, &first, true).unwrap();
        crate::database::set_item_pinned(&db, &first, true).unwrap();
        db.execute(
            "UPDATE note_draft SET content=?1 WHERE id=1",
            ["  پیش‌نویس\r\n  "],
        )
        .unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 3)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let image = crate::images::prepare(bytes.into_inner(), "test.png".into()).unwrap();
        crate::database::save_image(&mut db, &image).unwrap();
        let image_id: String = db
            .query_row("SELECT id FROM items WHERE type='image'", [], |r| r.get(0))
            .unwrap();
        crate::reminders::set(&mut db, &image_id, Some(2000), 1000).unwrap();
        db.execute_batch("UPDATE shortcut_settings SET capture='Alt+F8',toggle='Alt+F9',theme='dark',color_palette='ocean',calendar_type='gregory',auto_save_images=1; UPDATE items SET rowid=rowid+50;").unwrap();
        let expected =
            crate::database::list_items(&mut db, crate::database::ItemView::Archive).unwrap();
        f.save();
        let p = f.prepare();
        assert_eq!(p.item_count, 2);
        assert_eq!(
            schedule(&f.database, "expired", &mut Some(p.clone())),
            Err("restore_expired")
        );
        schedule(&f.database, &p.token, &mut Some(p.clone())).unwrap();
        crate::capture::save(&f.database, "after confirmation").unwrap();
        drop(db);
        let report = apply_pending(&f.database);
        assert!(report.message.starts_with("Backup restored."));
        let rescue = readonly(Path::new(report.rescue_path.as_ref().unwrap())).unwrap();
        assert_eq!(
            rescue
                .query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        let mut restored = crate::database::open(&f.database).unwrap();
        assert_eq!(
            crate::reminders::due(&restored, 2000).unwrap()[0].id,
            image_id
        );
        assert_eq!(
            restored
                .query_row("SELECT content FROM note_draft", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "  پیش‌نویس\r\n  "
        );
        let actual =
            crate::database::list_items(&mut restored, crate::database::ItemView::Archive).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(
            crate::database::load_item_image(
                &restored,
                &image_id,
                crate::database::ImageVariant::Original
            )
            .unwrap(),
            image.original
        );
        assert_eq!(
            restored
                .query_row("SELECT max(rowid) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            52
        );
        assert_eq!(
            restored
                .query_row(
                    "SELECT capture||toggle||theme||color_palette||calendar_type||auto_save_images FROM shortcut_settings",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "Alt+F8Alt+F9darkoceangregory1"
        );
        assert_eq!(
            crate::database::save_image(&mut restored, &image)
                .unwrap()
                .0,
            crate::capture::Outcome::Duplicate
        );
        assert_eq!(
            crate::capture::save(&f.database, "  سلام\r\nworld  ")
                .unwrap()
                .0,
            crate::capture::Outcome::Saved
        );
        drop(restored);
        assert!(apply_pending(&f.database).message.is_empty());
    }
    #[test]
    fn rejects_invalid_or_archived_reminders_in_backups() {
        let f = Fixture::new();
        let mut db = crate::database::open(&f.database).unwrap();
        db.execute(
            "INSERT INTO items(id,type,content,created_at) VALUES('a','text','note','then')",
            [],
        )
        .unwrap();
        crate::reminders::set(&mut db, "a", Some(2000), 1000).unwrap();
        f.save();
        let source = Connection::open(&f.backup).unwrap();
        source
            .execute_batch(
                "PRAGMA ignore_check_constraints=ON; UPDATE item_reminders SET due_at=0;",
            )
            .unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
        source.execute_batch("UPDATE item_reminders SET due_at=2000; UPDATE items SET status='completed',completed_at='then';").unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
    }

    #[test]
    fn rejects_untrusted_schema_future_version_constraints_and_bad_images() {
        let f = Fixture::new();
        crate::database::open(&f.database).unwrap();
        f.save();
        let source = Connection::open(&f.backup).unwrap();
        source
            .execute_batch(
                "CREATE TRIGGER sqliteXuntrusted AFTER INSERT ON items BEGIN DELETE FROM items; END;",
            )
            .unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
        source
            .execute_batch("DROP TRIGGER sqliteXuntrusted; PRAGMA user_version=12;")
            .unwrap();
        assert_eq!(f.prepare_error(), "backup_newer_version");
        source.execute_batch("PRAGMA user_version=11; PRAGMA ignore_check_constraints=ON; UPDATE shortcut_settings SET theme='unknown';").unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
        source.execute_batch("UPDATE shortcut_settings SET theme=NULL; INSERT INTO items(id,type,title,created_at,image_original,image_thumbnail,image_mime,image_width,image_height,image_hash) VALUES ('bad','image','bad','now',X'00',X'00','image/png',1,1,zeroblob(32));").unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
        drop(source);
        fs::write(&f.backup, b"not sqlite").unwrap();
        assert_eq!(f.prepare_error(), "invalid_backup");
        assert!(directory(&f.database).read_dir().unwrap().next().is_none());
    }
    impl Fixture {
        fn prepare_error(&self) -> &'static str {
            match prepare(&self.database, &self.backup, &mut None) {
                Err(e) => e,
                Ok(_) => panic!("accepted invalid backup"),
            }
        }
    }
    #[test]
    fn migrates_old_backups_without_changing_source() {
        let f = Fixture::new();
        crate::database::open(&f.database).unwrap();
        let old = Connection::open(&f.backup).unwrap();
        old.execute_batch(MIGRATIONS[0]).unwrap();
        old.execute_batch("PRAGMA user_version=1; INSERT INTO items(id,type,content,created_at) VALUES ('old','text','old note','then'); UPDATE items SET rowid=42;").unwrap();
        drop(old);
        let before = hash(&f.backup).unwrap();
        let p = f.prepare();
        assert_eq!(p.item_count, 1);
        assert_eq!(
            readonly(&staged(&f.database, &p.token))
                .unwrap()
                .query_row("SELECT rowid FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(hash(&f.backup).unwrap(), before);
        assert!(crate::reminders::due(
            &readonly(&staged(&f.database, &p.token)).unwrap(),
            i64::MAX
        )
        .unwrap()
        .is_empty());
        schedule(&f.database, &p.token, &mut Some(p.clone())).unwrap();
        assert!(apply_pending(&f.database)
            .message
            .starts_with("Backup restored."));
        assert_eq!(
            readonly(&f.database)
                .unwrap()
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            11
        );
    }
    #[test]
    fn failed_publication_keeps_previous_files_and_current_database() {
        let f = Fixture::new();
        crate::database::open(&f.database).unwrap();
        f.save();
        let before = hash(&f.backup).unwrap();
        assert_eq!(
            create(&f.database, &f.database),
            Err("backup_internal_path")
        );
        assert!(create(&f.database, &f.folder).is_err());
        assert_eq!(hash(&f.backup).unwrap(), before);
        let p = f.prepare();
        schedule(&f.database, &p.token, &mut Some(p.clone())).unwrap();
        let before = hash(&f.database).unwrap();
        fs::write(staged(&f.database, &p.token), b"damaged after confirmation").unwrap();
        assert!(apply_pending(&f.database)
            .message
            .starts_with("Could not restore"));
        assert_eq!(hash(&f.database).unwrap(), before);
        assert!(apply_pending(&f.database).message.is_empty());
    }
    #[test]
    fn rescue_write_failure_aborts_before_replacing_current_database() {
        let f = Fixture::new();
        crate::database::open(&f.database).unwrap();
        crate::capture::save(&f.database, "snapshot").unwrap();
        f.save();
        let p = f.prepare();
        schedule(&f.database, &p.token, &mut Some(p.clone())).unwrap();
        let pending: Pending =
            serde_json::from_slice(&fs::read(directory(&f.database).join("pending.json")).unwrap())
                .unwrap();
        fs::create_dir(rescue(&f.database, &p.token, pending.created_at)).unwrap();
        crate::capture::save(&f.database, "keep me").unwrap();
        let before = hash(&f.database).unwrap();
        let report = apply_pending(&f.database);
        assert!(report.message.starts_with("Could not restore"));
        assert!(report.rescue_path.is_none());
        assert_eq!(hash(&f.database).unwrap(), before);
    }

    #[test]
    fn interrupted_atomic_replacement_is_recognized_and_never_reapplied() {
        let f = Fixture::new();
        crate::database::open(&f.database).unwrap();
        crate::capture::save(&f.database, "backup").unwrap();
        f.save();
        let p = f.prepare();
        schedule(&f.database, &p.token, &mut Some(p.clone())).unwrap();
        let marker = directory(&f.database).join("pending.json");
        let pending: Pending = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
        let db = crate::database::open(&f.database).unwrap();
        snapshot(&db, &rescue(&f.database, &p.token, pending.created_at)).unwrap();
        drop(db);
        replace_file(&staged(&f.database, &p.token), &f.database).unwrap();
        assert!(apply_pending(&f.database)
            .message
            .starts_with("Backup restored."));
        crate::capture::save(&f.database, "later").unwrap();
        assert!(apply_pending(&f.database).message.is_empty());
        assert_eq!(
            readonly(&f.database)
                .unwrap()
                .query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn sqlite_snapshot_includes_committed_wal_without_copying_sidecars() {
        let f = Fixture::new();
        let db = crate::database::open(&f.database).unwrap();
        db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        crate::capture::save(&f.database, "in WAL").unwrap();
        f.save();
        assert_eq!(
            readonly(&f.backup)
                .unwrap()
                .query_row("SELECT content FROM items", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "in WAL"
        );
    }
}
