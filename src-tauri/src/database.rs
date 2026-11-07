use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;

#[derive(serde::Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CalendarType {
    Persian,
    Gregory,
}

pub fn set_calendar_type(connection: &Connection, calendar: CalendarType) -> rusqlite::Result<()> {
    connection
        .execute(
            "UPDATE shortcut_settings SET calendar_type=?1 WHERE id=1",
            [match calendar {
                CalendarType::Persian => "persian",
                CalendarType::Gregory => "gregory",
            }],
        )
        .map(|_| ())
}

#[derive(serde::Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ColorPalette {
    Lime,
    Violet,
    Ocean,
    Rose,
    Amber,
    Notebook,
    Terminal,
}

impl ColorPalette {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lime => "lime",
            Self::Violet => "violet",
            Self::Ocean => "ocean",
            Self::Rose => "rose",
            Self::Amber => "amber",
            Self::Notebook => "notebook",
            Self::Terminal => "terminal",
        }
    }
}

pub fn set_theme(
    connection: &Connection,
    light: bool,
    palette: Option<ColorPalette>,
) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE shortcut_settings SET theme=?1, color_palette=coalesce(?2, color_palette) WHERE id=1",
        rusqlite::params![if light { "light" } else { "dark" }, palette.map(ColorPalette::as_str)],
    ).map(|_| ())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedItem {
    pub reminder_at: Option<i64>,
    pub id: String,
    pub item_type: String,
    pub content: Option<String>,
    pub formatted_html: Option<String>,
    pub created_at: String,
    pub category_id: Option<String>,
    pub pinned: bool,
    pub image: Option<ImageMetadata>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub name: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_size: u32,
}

#[derive(serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ImageVariant {
    Original,
    Thumbnail,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CopyContent {
    Text(String, Option<String>),
    Image(Vec<u8>),
}

#[derive(serde::Serialize)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ItemView {
    Inbox,
    Archive,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSnapshot {
    pub items: Vec<SavedItem>,
    pub categories: Vec<Category>,
    pub pending_count: u32,
}

pub fn item_link(connection: &Connection, id: &str) -> rusqlite::Result<String> {
    let content: String = connection.query_row(
        "SELECT content FROM items WHERE id = ?1 AND type = 'link'",
        [id],
        |row| row.get(0),
    )?;
    let url = url::Url::parse(content.trim()).map_err(|_| rusqlite::Error::InvalidQuery)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(url.into())
}

pub fn list_items(connection: &mut Connection, view: ItemView) -> rusqlite::Result<ItemSnapshot> {
    let transaction = connection.transaction()?;
    let status = match view {
        ItemView::Inbox => "pending",
        ItemView::Archive => "completed",
    };
    let items = {
        let mut statement = transaction.prepare("SELECT id, type, content, created_at, category_id, title, image_mime, image_width, image_height, length(image_original), formatted_html, pinned, (SELECT due_at FROM item_reminders WHERE item_id=items.id) FROM items WHERE status = ?1 ORDER BY pinned DESC, created_at DESC, id DESC")?;
        let rows = statement.query_map([status], |row| {
            let item_type: String = row.get(1)?;
            let image = if item_type == "image" {
                Some(ImageMetadata {
                    name: row.get(5)?,
                    mime_type: row.get(6)?,
                    width: row.get(7)?,
                    height: row.get(8)?,
                    byte_size: row.get(9)?,
                })
            } else {
                None
            };
            Ok(SavedItem {
                reminder_at: row.get(12)?,
                id: row.get(0)?,
                item_type,
                content: row.get(2)?,
                formatted_html: row.get(10)?,
                created_at: row.get(3)?,
                category_id: row.get(4)?,
                pinned: row.get(11)?,
                image,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let categories = {
        let mut statement =
            transaction.prepare("SELECT id, name FROM categories ORDER BY name, id")?;
        let rows = statement.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let pending_count = pending_count(&transaction)?;
    transaction.commit()?;
    Ok(ItemSnapshot {
        items,
        categories,
        pending_count,
    })
}

fn require_row(count: usize) -> rusqlite::Result<()> {
    if count == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}
pub fn item_for_copy(connection: &Connection, id: &str) -> rusqlite::Result<CopyContent> {
    connection.query_row(
        "SELECT type, content, image_original, formatted_html FROM items WHERE id = ?1",
        [id],
        |row| match row.get::<_, String>(0)?.as_str() {
            "image" => Ok(CopyContent::Image(row.get(2)?)),
            _ => Ok(CopyContent::Text(row.get(1)?, row.get(3)?)),
        },
    )
}

pub fn load_item_image(
    connection: &Connection,
    id: &str,
    variant: ImageVariant,
) -> rusqlite::Result<Vec<u8>> {
    let query = match variant {
        ImageVariant::Original => {
            "SELECT image_original FROM items WHERE id = ?1 AND type = 'image'"
        }
        ImageVariant::Thumbnail => {
            "SELECT image_thumbnail FROM items WHERE id = ?1 AND type = 'image'"
        }
    };
    connection.query_row(query, [id], |row| row.get(0))
}

pub fn save_image(
    connection: &mut Connection,
    image: &crate::images::PreparedImage,
) -> rusqlite::Result<(crate::capture::Outcome, u32)> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let duplicate: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM (SELECT type, image_hash, image_original FROM items ORDER BY rowid DESC LIMIT 1) WHERE type = 'image' AND image_hash = ?1 AND image_original = ?2)",
        rusqlite::params![image.hash, image.original],
        |row| row.get(0),
    )?;
    if !duplicate {
        transaction.execute(
            "INSERT INTO items (id, type, title, created_at, image_original, image_thumbnail, image_mime, image_width, image_height, image_hash) VALUES (?1, 'image', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(), image.name, chrono::Utc::now().to_rfc3339(),
                image.original, image.thumbnail, image.mime_type, image.width, image.height, image.hash,
            ],
        )?;
    }
    let count = pending_count(&transaction)?;
    transaction.commit()?;
    Ok((
        if duplicate {
            crate::capture::Outcome::Duplicate
        } else {
            crate::capture::Outcome::Saved
        },
        count,
    ))
}

pub fn delete_item(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    require_row(connection.execute("DELETE FROM items WHERE id = ?1", [id])?)
}
pub fn set_item_pinned(connection: &Connection, id: &str, pinned: bool) -> rusqlite::Result<()> {
    require_row(connection.execute(
        "UPDATE items SET pinned = ?1 WHERE id = ?2",
        rusqlite::params![pinned, id],
    )?)
}
pub fn set_item_archived(
    connection: &Connection,
    id: &str,
    archived: bool,
) -> rusqlite::Result<()> {
    let completed_at = archived.then(|| chrono::Utc::now().to_rfc3339());
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    require_row(transaction.execute(
        "UPDATE items SET status = ?2, completed_at = ?3 WHERE id = ?1",
        rusqlite::params![
            id,
            if archived { "completed" } else { "pending" },
            completed_at
        ],
    )?)?;
    if archived {
        transaction.execute("DELETE FROM item_reminders WHERE item_id=?1", [id])?;
    }
    transaction.commit()
}
pub fn set_item_category(
    connection: &Connection,
    id: &str,
    category_id: Option<&str>,
) -> rusqlite::Result<()> {
    require_row(connection.execute(
        "UPDATE items SET category_id = ?2 WHERE id = ?1",
        rusqlite::params![id, category_id],
    )?)
}
fn category_name(name: &str) -> rusqlite::Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        Err(rusqlite::Error::InvalidQuery)
    } else {
        Ok(name)
    }
}
pub fn create_category(connection: &Connection, name: &str) -> rusqlite::Result<Category> {
    let category = Category {
        id: uuid::Uuid::new_v4().to_string(),
        name: category_name(name)?.to_owned(),
    };
    connection.execute(
        "INSERT INTO categories (id, name) VALUES (?1, ?2)",
        rusqlite::params![category.id, category.name],
    )?;
    Ok(category)
}
pub fn rename_category(connection: &Connection, id: &str, name: &str) -> rusqlite::Result<()> {
    require_row(connection.execute(
        "UPDATE categories SET name = ?2 WHERE id = ?1",
        rusqlite::params![id, category_name(name)?],
    )?)
}
pub fn delete_category(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    require_row(connection.execute("DELETE FROM categories WHERE id = ?1", [id])?)
}
pub fn pending_count(connection: &Connection) -> rusqlite::Result<u32> {
    connection.query_row(
        "SELECT count(*) FROM items WHERE status = 'pending'",
        [],
        |row| row.get(0),
    )
}
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut connection = Connection::open(path)?;
    connection.busy_timeout(std::time::Duration::from_secs(2))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    migrate(&mut connection)?;
    Ok(connection)
}
pub fn migrate(connection: &mut Connection) -> rusqlite::Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > 12 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version == 0 {
        transaction.execute_batch(include_str!("../migrations/0001_items.sql"))?;
    }
    if version < 2 {
        transaction.execute_batch(include_str!("../migrations/0002_categories.sql"))?;
        transaction.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        transaction.execute_batch(include_str!("../migrations/0003_shortcuts.sql"))?;
        transaction.pragma_update(None, "user_version", 3)?;
    }
    if version < 4 {
        transaction.execute_batch(include_str!("../migrations/0004_images.sql"))?;
        transaction.pragma_update(None, "user_version", 4)?;
    }
    if version < 5 {
        transaction.execute_batch(include_str!("../migrations/0005_auto_images.sql"))?;
        transaction.pragma_update(None, "user_version", 5)?;
    }
    if version < 6 {
        transaction.execute_batch(include_str!("../migrations/0006_formatted_text.sql"))?;
        transaction.pragma_update(None, "user_version", 6)?;
    }
    if version < 7 {
        transaction.execute_batch(include_str!("../migrations/0007_theme.sql"))?;
        transaction.pragma_update(None, "user_version", 7)?;
    }
    if version < 8 {
        transaction.execute_batch(include_str!("../migrations/0008_pins_and_draft.sql"))?;
        transaction.pragma_update(None, "user_version", 8)?;
    }
    if version < 9 {
        transaction.execute_batch(include_str!("../migrations/0009_color_palette.sql"))?;
        transaction.pragma_update(None, "user_version", 9)?;
    }
    if version < 10 {
        transaction.execute_batch(include_str!("../migrations/0010_reminders.sql"))?;
        transaction.pragma_update(None, "user_version", 10)?;
    }
    if version < 11 {
        transaction.execute_batch(include_str!("../migrations/0011_calendar_type.sql"))?;
        transaction.pragma_update(None, "user_version", 11)?;
    }
    if version < 12 {
        transaction.execute_batch(include_str!("../migrations/0012_visual_themes.sql"))?;
        transaction.pragma_update(None, "user_version", 12)?;
    }
    transaction.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_migration_preserves_v10_data_and_validates_choices() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        db.execute_batch("ALTER TABLE shortcut_settings DROP COLUMN calendar_type; PRAGMA user_version=10; INSERT INTO items(rowid,id,type,content,created_at) VALUES(42,'calendar','text','keep','then'); INSERT INTO item_reminders VALUES('calendar',2000); UPDATE note_draft SET content='draft'; UPDATE shortcut_settings SET color_palette='ocean';").unwrap();
        migrate(&mut db).unwrap();
        let read = || {
            db.query_row("SELECT calendar_type FROM shortcut_settings", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap()
        };
        assert_eq!(read(), "persian");
        assert_eq!(
            db.query_row("SELECT rowid FROM items WHERE id='calendar'", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(
            db.query_row("SELECT due_at FROM item_reminders", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2000
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
        set_calendar_type(&db, CalendarType::Gregory).unwrap();
        assert_eq!(read(), "gregory");
        assert!(db
            .execute("UPDATE shortcut_settings SET calendar_type='unknown'", [])
            .is_err());
        assert!(db
            .execute("UPDATE shortcut_settings SET calendar_type=NULL", [])
            .is_err());
        assert!(serde_json::from_str::<CalendarType>("\"unknown\"").is_err());
        db.execute_batch("CREATE TRIGGER fail_calendar BEFORE UPDATE ON shortcut_settings BEGIN SELECT RAISE(ABORT,'fail'); END;").unwrap();
        assert!(set_calendar_type(&db, CalendarType::Persian).is_err());
        assert_eq!(read(), "gregory");
    }

    #[test]
    fn palette_migration_preserves_v8_data_and_rolls_back_on_failure() {
        for broken in [false, true] {
            let mut db = Connection::open_in_memory().unwrap();
            for migration in [
                include_str!("../migrations/0001_items.sql"),
                include_str!("../migrations/0002_categories.sql"),
                include_str!("../migrations/0003_shortcuts.sql"),
                include_str!("../migrations/0004_images.sql"),
                include_str!("../migrations/0005_auto_images.sql"),
                include_str!("../migrations/0006_formatted_text.sql"),
                include_str!("../migrations/0007_theme.sql"),
                include_str!("../migrations/0008_pins_and_draft.sql"),
            ] {
                db.execute_batch(migration).unwrap();
            }
            db.execute_batch("PRAGMA user_version=8; INSERT INTO items(rowid,id,type,content,created_at,pinned) VALUES(42,'keep','text','  keep  ','then',1); UPDATE shortcut_settings SET theme='light',auto_save_images=1; UPDATE note_draft SET content='draft';").unwrap();
            if broken {
                db.execute_batch("ALTER TABLE shortcut_settings ADD COLUMN color_palette TEXT;")
                    .unwrap();
            }
            assert_eq!(migrate(&mut db).is_err(), broken);
            assert_eq!(
                db.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                    .unwrap(),
                if broken { 8 } else { 12 }
            );
            assert_eq!(
                db.query_row("SELECT rowid||content||pinned FROM items", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "42  keep  1"
            );
            assert_eq!(
                db.query_row("SELECT content FROM note_draft", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "draft"
            );
            assert_eq!(
                db.query_row(
                    "SELECT theme||auto_save_images FROM shortcut_settings",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "light1"
            );
            let palette: Option<String> = db
                .query_row("SELECT color_palette FROM shortcut_settings", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(palette.as_deref(), if broken { None } else { Some("lime") });
        }
    }

    #[test]
    fn palettes_are_constrained_and_mode_toggle_preserves_the_palette() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        for palette in [
            ColorPalette::Lime,
            ColorPalette::Violet,
            ColorPalette::Ocean,
            ColorPalette::Rose,
            ColorPalette::Notebook,
            ColorPalette::Terminal,
            ColorPalette::Amber,
        ] {
            set_theme(&db, false, Some(palette)).unwrap();
            set_theme(&db, true, None).unwrap();
            let value: String = db
                .query_row(
                    "SELECT theme||color_palette FROM shortcut_settings",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(value, format!("light{}", palette.as_str()));
        }
        assert!(db
            .execute("UPDATE shortcut_settings SET color_palette='unknown'", [])
            .is_err());
        assert!(db
            .execute("UPDATE shortcut_settings SET color_palette=NULL", [])
            .is_err());
        assert!(serde_json::from_str::<ColorPalette>("\"unknown\"").is_err());
        db.execute_batch("CREATE TRIGGER reject_theme BEFORE UPDATE ON shortcut_settings BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert!(set_theme(&db, false, Some(ColorPalette::Ocean)).is_err());
        assert_eq!(
            db.query_row(
                "SELECT theme||color_palette FROM shortcut_settings",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "lightamber"
        );
    }

    #[test]
    fn v7_upgrade_preserves_rows_and_pins_sort_first_without_reordering_insertion_history() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        db.execute_batch("INSERT INTO items(id,type,content,created_at) VALUES ('old','text','older','2020'),('new','link','https://example.com','2026');").unwrap();
        let last: i64 = db
            .query_row("SELECT max(rowid) FROM items", [], |r| r.get(0))
            .unwrap();
        assert!(!list_items(&mut db, ItemView::Inbox).unwrap().items[0].pinned);
        set_item_pinned(&db, "old", true).unwrap();
        let items = list_items(&mut db, ItemView::Inbox).unwrap().items;
        assert_eq!(items[0].id, "old");
        assert!(items[0].pinned);
        assert_eq!(
            db.query_row("SELECT max(rowid) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            last
        );
        set_item_archived(&db, "old", true).unwrap();
        assert!(list_items(&mut db, ItemView::Archive).unwrap().items[0].pinned);
        set_item_archived(&db, "old", false).unwrap();
        assert!(list_items(&mut db, ItemView::Inbox).unwrap().items[0].pinned);
        set_item_pinned(&db, "old", false).unwrap();
        assert_eq!(
            list_items(&mut db, ItemView::Inbox).unwrap().items[0].id,
            "new"
        );
        assert!(set_item_pinned(&db, "missing", true).is_err());
        assert!(db
            .execute("UPDATE items SET pinned=2 WHERE id='old'", [])
            .is_err());
        assert!(db
            .execute("INSERT INTO note_draft(id,content) VALUES(2,'bad')", [])
            .is_err());
        assert!(db
            .execute("UPDATE note_draft SET content=zeroblob(1)", [])
            .is_err());
        assert!(db
            .execute("UPDATE note_draft SET content=?1", ["x".repeat(4_194_305)])
            .is_err());
    }

    #[test]
    fn v8_migration_preserves_v7_data_and_rolls_back_on_failure() {
        for broken in [false, true] {
            let mut db = Connection::open_in_memory().unwrap();
            for migration in [
                include_str!("../migrations/0001_items.sql"),
                include_str!("../migrations/0002_categories.sql"),
                include_str!("../migrations/0003_shortcuts.sql"),
                include_str!("../migrations/0004_images.sql"),
                include_str!("../migrations/0005_auto_images.sql"),
                include_str!("../migrations/0006_formatted_text.sql"),
                include_str!("../migrations/0007_theme.sql"),
            ] {
                db.execute_batch(migration).unwrap();
            }
            db.execute_batch("PRAGMA user_version=7; INSERT INTO items(id,type,content,created_at) VALUES('keep','text','  keep\nme  ','then'); UPDATE shortcut_settings SET theme='dark',auto_save_images=1;").unwrap();
            if broken {
                db.execute_batch("CREATE TABLE note_draft(unexpected TEXT)")
                    .unwrap();
            }
            assert_eq!(migrate(&mut db).is_err(), broken);
            assert_eq!(
                db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                if broken { 7 } else { 12 }
            );
            assert_eq!(
                db.query_row("SELECT content FROM items", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "  keep\nme  "
            );
            if broken {
                assert!(db.prepare("SELECT pinned FROM items").is_err());
            } else {
                assert!(!list_items(&mut db, ItemView::Inbox).unwrap().items[0].pinned);
                assert_eq!(
                    db.query_row(
                        "SELECT theme||auto_save_images FROM shortcut_settings",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                    "dark1"
                );
                assert_eq!(
                    db.query_row("SELECT content FROM note_draft", [], |r| r
                        .get::<_, String>(0))
                        .unwrap(),
                    ""
                );
            }
        }
    }

    #[test]
    fn v2_upgrade_keeps_categories_and_items_and_rolls_back_on_failure() {
        for broken in [false, true] {
            let mut connection = Connection::open_in_memory().unwrap();
            connection
                .execute_batch(include_str!("../migrations/0001_items.sql"))
                .unwrap();
            connection
                .execute_batch(include_str!("../migrations/0002_categories.sql"))
                .unwrap();
            connection.pragma_update(None, "user_version", 2).unwrap();
            connection
                .execute(
                    "INSERT INTO categories (id,name) VALUES ('work','Work')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO items (id,type,content,created_at,category_id) VALUES ('a','text','Keep me','2026-10-31T00:00:00Z','work')", []).unwrap();
            if broken {
                connection
                    .execute("CREATE TABLE shortcut_settings (unexpected TEXT)", [])
                    .unwrap();
            }
            assert_eq!(migrate(&mut connection).is_err(), broken);
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                if broken { 2 } else { 12 }
            );
            let saved: (String, String) = connection
                .query_row(
                    "SELECT content, category_id FROM items WHERE id = 'a'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(saved, ("Keep me".into(), "work".into()));
            assert_eq!(
                connection
                    .query_row("SELECT name FROM categories WHERE id = 'work'", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .unwrap(),
                "Work"
            );
            if !broken {
                assert_eq!(
                    crate::settings::load(&connection).unwrap(),
                    crate::settings::ShortcutSettings::default()
                );
            }
        }
    }

    #[test]
    fn pending_list_preserves_content_orders_newest_first_and_does_not_modify_rows() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        assert!(list_items(&mut connection, ItemView::Inbox)
            .unwrap()
            .items
            .is_empty());
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('old','text',?1,'2026-10-30T00:00:00Z')", ["  سلام\nجهان  "]).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('new','link','https://example.com','2026-10-31T00:00:00Z')", []).unwrap();
        connection.execute("INSERT INTO items (id,type,content,status,created_at,completed_at) VALUES ('done','text','completed','completed','2026-10-31T00:00:00Z','2026-10-31T01:00:00Z')", []).unwrap();
        let snapshot = list_items(&mut connection, ItemView::Inbox).unwrap();
        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].id, "new");
        assert_eq!(snapshot.items[0].item_type, "link");
        assert_eq!(snapshot.items[1].content.as_deref(), Some("  سلام\nجهان  "));
        assert!(snapshot.items[1].image.is_none());
        assert_eq!(snapshot.pending_count, 2);
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM items", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            3
        );
    }

    #[test]
    fn copy_lookup_preserves_full_content_in_both_views_and_rejects_missing_ids() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let content = "  سلام\r\nFull text 📝  ";
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('copy','text',?1,'2026-10-31T00:00:00Z')", [content]).unwrap();
        assert_eq!(
            item_for_copy(&connection, "copy").unwrap(),
            CopyContent::Text(content.into(), None)
        );
        set_item_archived(&connection, "copy", true).unwrap();
        assert_eq!(
            item_for_copy(&connection, "copy").unwrap(),
            CopyContent::Text(content.into(), None)
        );
        assert!(matches!(
            item_for_copy(&connection, "missing"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
        assert!(item_for_copy(&connection, "copy' OR 1=1 --").is_err());
        delete_item(&connection, "copy").unwrap();
        assert!(item_for_copy(&connection, "copy").is_err());
    }

    #[test]
    fn saved_items_survive_reopening_the_database() {
        let path =
            std::env::temp_dir().join(format!("someday-storage-test-{}.db", uuid::Uuid::new_v4()));
        let connection = open(&path).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('persisted','text','Keep me','2026-10-31T00:00:00Z')", []).unwrap();
        drop(connection);
        let connection = open(&path).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT content FROM items WHERE id = 'persisted'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "Keep me"
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    fn image_fixture() -> crate::images::PreparedImage {
        crate::images::PreparedImage {
            original: vec![1, 2, 3],
            thumbnail: vec![4, 5],
            name: "عکس.png".into(),
            mime_type: "image/png".into(),
            width: 2,
            height: 1,
            hash: vec![7; 32],
        }
    }

    #[test]
    fn images_keep_original_bytes_and_metadata_through_actions_and_reopening() {
        let path = std::env::temp_dir().join(format!("someday-image-{}.db", uuid::Uuid::new_v4()));
        let mut connection = open(&path).unwrap();
        let image = image_fixture();
        assert_eq!(
            save_image(&mut connection, &image).unwrap(),
            (crate::capture::Outcome::Saved, 1)
        );
        let snapshot = list_items(&mut connection, ItemView::Inbox).unwrap();
        assert_eq!(snapshot.items.len(), 1);
        let saved = &snapshot.items[0];
        assert_eq!(saved.item_type, "image");
        assert!(saved.content.is_none());
        let metadata = saved.image.as_ref().unwrap();
        assert_eq!(metadata.name, image.name);
        assert_eq!(metadata.mime_type, image.mime_type);
        assert_eq!(
            (metadata.width, metadata.height, metadata.byte_size),
            (2, 1, 3)
        );
        let json = serde_json::to_value(&snapshot).unwrap();
        assert!(json["items"][0].get("imageOriginal").is_none());
        assert!(json["items"][0]["image"].get("original").is_none());
        assert!(json["items"][0]["image"].get("thumbnail").is_none());
        let id = saved.id.clone();
        assert_eq!(
            item_for_copy(&connection, &id).unwrap(),
            CopyContent::Image(image.original.clone())
        );
        assert_eq!(
            load_item_image(&connection, &id, ImageVariant::Thumbnail).unwrap(),
            image.thumbnail
        );
        let category = create_category(&connection, "Photos").unwrap();
        set_item_category(&connection, &id, Some(&category.id)).unwrap();
        set_item_archived(&connection, &id, true).unwrap();
        assert_eq!(
            save_image(&mut connection, &image).unwrap(),
            (crate::capture::Outcome::Duplicate, 0)
        );
        drop(connection);
        let mut connection = open(&path).unwrap();
        let archived = list_items(&mut connection, ItemView::Archive).unwrap();
        assert_eq!(archived.items[0].id, id);
        assert_eq!(
            archived.items[0].category_id.as_deref(),
            Some(category.id.as_str())
        );
        assert_eq!(
            load_item_image(&connection, &id, ImageVariant::Original).unwrap(),
            image.original
        );
        delete_category(&connection, &category.id).unwrap();
        set_item_archived(&connection, &id, false).unwrap();
        let inbox = list_items(&mut connection, ItemView::Inbox).unwrap();
        assert!(inbox.items[0].category_id.is_none());
        assert_eq!(inbox.pending_count, 1);
        delete_item(&connection, &id).unwrap();
        for variant in [ImageVariant::Original, ImageVariant::Thumbnail] {
            assert!(matches!(
                load_item_image(&connection, &id, variant),
                Err(rusqlite::Error::QueryReturnedNoRows)
            ));
            assert!(load_item_image(&connection, "' OR 1=1 --", variant).is_err());
        }
        assert_eq!(
            save_image(&mut connection, &image).unwrap(),
            (crate::capture::Outcome::Saved, 1)
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn image_duplicates_require_matching_bytes_even_when_hashes_match() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let mut image = image_fixture();
        save_image(&mut connection, &image).unwrap();
        image.original.push(9);
        assert_eq!(
            save_image(&mut connection, &image).unwrap(),
            (crate::capture::Outcome::Saved, 2)
        );
        image.name = "Another filename.png".into();
        assert_eq!(
            save_image(&mut connection, &image).unwrap(),
            (crate::capture::Outcome::Duplicate, 2)
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM items", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
    }

    #[test]
    fn duplicate_checks_use_the_last_inserted_item_across_types_and_archive() {
        let path =
            std::env::temp_dir().join(format!("someday-last-item-{}.db", uuid::Uuid::new_v4()));
        let mut connection = open(&path).unwrap();
        let image = image_fixture();
        assert_eq!(
            crate::capture::save(&path, "first").unwrap().0,
            crate::capture::Outcome::Saved
        );
        save_image(&mut connection, &image).unwrap();
        assert_eq!(
            crate::capture::save(&path, "first").unwrap().0,
            crate::capture::Outcome::Saved
        );
        assert_eq!(
            save_image(&mut connection, &image).unwrap().0,
            crate::capture::Outcome::Saved
        );
        assert_eq!(
            save_image(&mut connection, &image).unwrap().0,
            crate::capture::Outcome::Duplicate
        );
        // Save order remains authoritative even when the clock moves backwards.
        connection.execute("UPDATE items SET created_at='1900-01-01' WHERE rowid=(SELECT max(rowid) FROM items)",[]).unwrap();
        assert_eq!(
            save_image(&mut connection, &image).unwrap().0,
            crate::capture::Outcome::Duplicate
        );
        let id: String = connection
            .query_row(
                "SELECT id FROM items ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        set_item_archived(&connection, &id, true).unwrap();
        assert_eq!(
            save_image(&mut connection, &image).unwrap().0,
            crate::capture::Outcome::Duplicate
        );
        connection.execute("INSERT INTO items(id,type,content,created_at) VALUES('latest','text','other','1900-01-01')",[]).unwrap();
        assert_eq!(
            save_image(&mut connection, &image).unwrap().0,
            crate::capture::Outcome::Saved
        );
        assert_eq!(update_note(&mut connection, "latest", "first"), Ok(()));
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn concurrent_image_saves_insert_only_once() {
        let path = std::env::temp_dir().join(format!(
            "someday-image-concurrent-{}.db",
            uuid::Uuid::new_v4()
        ));
        drop(open(&path).unwrap());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut connection = open(&path).unwrap();
                    barrier.wait();
                    save_image(&mut connection, &image_fixture()).unwrap()
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(
            results
                .iter()
                .filter(|result| result.0 == crate::capture::Outcome::Saved)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| result.0 == crate::capture::Outcome::Duplicate)
                .count(),
            1
        );
        assert!(results.iter().all(|result| result.1 == 1));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn image_constraints_reject_incomplete_or_oversized_payloads() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        save_image(&mut connection, &image_fixture()).unwrap();
        for update in [
            "image_original = NULL",
            "image_original = x''",
            "image_original = 'not bytes'",
            "image_original = zeroblob(20971521)",
            "image_thumbnail = NULL",
            "image_thumbnail = zeroblob(524289)",
            "title = NULL",
            "title = '  '",
            "content = 'image text'",
            "image_mime = NULL",
            "image_mime = 'image/svg+xml'",
            "image_width = 0",
            "image_width = 16385",
            "image_width = 1.5",
            "image_width = 10000, image_height = 10000",
            "image_hash = NULL",
            "image_hash = zeroblob(31)",
        ] {
            assert!(
                connection
                    .execute(
                        &format!("UPDATE items SET {update} WHERE type = 'image'"),
                        []
                    )
                    .is_err(),
                "accepted {update}"
            );
        }
        assert!(connection
            .execute(
                "INSERT INTO items (id,type,created_at) VALUES ('incomplete','image','date')",
                []
            )
            .is_err());
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('text','text','Keep me','date')", []).unwrap();
        assert!(connection
            .execute(
                "UPDATE items SET image_original = x'00' WHERE id = 'text'",
                []
            )
            .is_err());
        assert!(load_item_image(&connection, "text", ImageVariant::Original).is_err());
        assert_eq!(
            item_for_copy(&connection, "text").unwrap(),
            CopyContent::Text("Keep me".into(), None)
        );
    }

    #[test]
    fn v3_image_upgrade_preserves_all_data_and_rolls_back_after_table_rebuild_failure() {
        for broken in [false, true] {
            let mut connection = Connection::open_in_memory().unwrap();
            connection
                .pragma_update(None, "foreign_keys", true)
                .unwrap();
            for migration in [
                include_str!("../migrations/0001_items.sql"),
                include_str!("../migrations/0002_categories.sql"),
                include_str!("../migrations/0003_shortcuts.sql"),
            ] {
                connection.execute_batch(migration).unwrap();
            }
            connection.pragma_update(None, "user_version", 3).unwrap();
            connection
                .execute(
                    "INSERT INTO categories (id,name) VALUES ('photos','Photos')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO items (id,type,content,title,status,created_at,completed_at,category_id) VALUES ('old','text',?1,'Title','completed','created','completed','photos')", ["  Keep\r\nme  "]).unwrap();
            connection
                .execute(
                    "UPDATE shortcut_settings SET capture = 'Alt+F8', toggle = 'Alt+F9'",
                    [],
                )
                .unwrap();
            if broken {
                connection
                    .execute("CREATE INDEX items_image_hash ON categories(name)", [])
                    .unwrap();
            }
            assert_eq!(migrate(&mut connection).is_err(), broken);
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                    .unwrap(),
                if broken { 3 } else { 12 }
            );
            let row = connection.query_row("SELECT id,type,content,title,status,created_at,completed_at,category_id FROM items", [], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?))
            }).unwrap();
            assert_eq!(
                row,
                (
                    "old".into(),
                    "text".into(),
                    "  Keep\r\nme  ".into(),
                    "Title".into(),
                    "completed".into(),
                    "created".into(),
                    "completed".into(),
                    "photos".into()
                )
            );
            assert_eq!(
                crate::settings::load(&connection).unwrap(),
                crate::settings::ShortcutSettings {
                    capture: "Alt+F8".into(),
                    toggle: "Alt+F9".into()
                }
            );
            assert_eq!(connection.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('items_status_created_at', 'items_category_id')", [], |row| row.get::<_, u32>(0)).unwrap(), 2);
            assert_eq!(
                connection
                    .prepare("SELECT image_original FROM items")
                    .is_err(),
                broken
            );
            assert!(connection.prepare("SELECT * FROM items_v4").is_err());
            assert!(connection
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .query([])
                .unwrap()
                .next()
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn migration_preserves_rows_and_enforces_item_constraints() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('a','text','Keep me','2026-10-31T00:00:00Z')", []).unwrap();
        migrate(&mut connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT content FROM items", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "Keep me"
        );
        assert!(connection
            .execute("UPDATE items SET status = 'completed'", [])
            .is_err());
        assert!(connection
            .execute("UPDATE items SET type = 'file'", [])
            .is_err());
        assert!(connection
            .execute("UPDATE items SET content = '  '", [])
            .is_err());
    }

    #[test]
    fn v1_upgrade_preserves_rows_and_category_actions_preserve_items() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_items.sql"))
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('a','text',?1,'2026-10-31T00:00:00Z')", ["  سلام\nKeep me  "]).unwrap();
        migrate(&mut connection).unwrap();
        let initial = list_items(&mut connection, ItemView::Inbox).unwrap();
        assert_eq!(
            initial.items[0].content.as_deref(),
            Some("  سلام\nKeep me  ")
        );
        assert!(initial.items[0].category_id.is_none());
        assert_eq!(initial.pending_count, 1);
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            12
        );
        assert!(create_category(&connection, " \n\t").is_err());
        let category = create_category(&connection, "  کار  ").unwrap();
        assert_eq!(category.name, "کار");
        assert!(create_category(&connection, "کار").is_err());
        let other = create_category(&connection, "Other").unwrap();
        assert!(rename_category(&connection, &other.id, " کار ").is_err());
        assert!(rename_category(&connection, &category.id, " ").is_err());
        rename_category(&connection, &category.id, "  Work  ").unwrap();
        set_item_category(&connection, "a", Some(&category.id)).unwrap();
        assert!(set_item_category(&connection, "a", Some("missing")).is_err());
        assert!(set_item_category(&connection, "missing", None).is_err());
        assert!(rename_category(&connection, "missing", "Name").is_err());
        assert!(delete_category(&connection, "missing").is_err());
        delete_category(&connection, &category.id).unwrap();
        let snapshot = list_items(&mut connection, ItemView::Inbox).unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items[0].category_id.is_none());
        assert_eq!(snapshot.categories.len(), 1);
        assert_eq!(snapshot.pending_count, 1);
    }

    #[test]
    fn archive_restore_delete_and_categories_survive_reopening() {
        let path =
            std::env::temp_dir().join(format!("someday-actions-{}.db", uuid::Uuid::new_v4()));
        let mut connection = open(&path).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('a','text','Keep me','2026-10-31T00:00:00Z')", []).unwrap();
        let category = create_category(&connection, "Work").unwrap();
        set_item_category(&connection, "a", Some(&category.id)).unwrap();
        set_item_archived(&connection, "a", true).unwrap();
        assert_eq!(
            list_items(&mut connection, ItemView::Inbox)
                .unwrap()
                .pending_count,
            0
        );
        let completed_at: String = connection
            .query_row("SELECT completed_at FROM items WHERE id = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(&completed_at).is_ok());
        drop(connection);
        let mut connection = open(&path).unwrap();
        let snapshot = list_items(&mut connection, ItemView::Archive).unwrap();
        assert_eq!(
            snapshot.items[0].category_id.as_deref(),
            Some(category.id.as_str())
        );
        assert_eq!(snapshot.categories[0].name, "Work");
        set_item_archived(&connection, "a", false).unwrap();
        assert_eq!(
            list_items(&mut connection, ItemView::Inbox)
                .unwrap()
                .items
                .len(),
            1
        );
        let completed_at: Option<String> = connection
            .query_row("SELECT completed_at FROM items WHERE id = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(completed_at.is_none());
        assert!(set_item_archived(&connection, "missing", true).is_err());
        assert!(delete_item(&connection, "missing").is_err());
        delete_item(&connection, "a").unwrap();
        drop(connection);
        let mut connection = open(&path).unwrap();
        assert!(list_items(&mut connection, ItemView::Inbox)
            .unwrap()
            .items
            .is_empty());
        assert!(list_items(&mut connection, ItemView::Archive)
            .unwrap()
            .items
            .is_empty());
        assert_eq!(
            list_items(&mut connection, ItemView::Inbox)
                .unwrap()
                .categories
                .len(),
            1
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_upgrade_rolls_back_without_resetting_rows_or_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_items.sql"))
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('a','text','Keep me','2026-10-31T00:00:00Z')", []).unwrap();
        connection
            .execute("CREATE TABLE categories (unexpected TEXT)", [])
            .unwrap();
        assert!(migrate(&mut connection).is_err());
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT content FROM items", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "Keep me"
        );
        assert!(connection.prepare("SELECT category_id FROM items").is_err());
    }

    #[test]
    fn newer_database_is_rejected_without_modification() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 13).unwrap();
        assert!(migrate(&mut connection).is_err());
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            13
        );
    }

    #[test]
    fn v5_upgrade_preserves_rows_settings_and_enforces_html_limits() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in [
            include_str!("../migrations/0001_items.sql"),
            include_str!("../migrations/0002_categories.sql"),
            include_str!("../migrations/0003_shortcuts.sql"),
            include_str!("../migrations/0004_images.sql"),
            include_str!("../migrations/0005_auto_images.sql"),
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 5).unwrap();
        connection.execute("INSERT INTO items(id,type,content,created_at) VALUES ('old','text','  سلام  ','date')", []).unwrap();
        connection
            .execute("UPDATE shortcut_settings SET auto_save_images = 1", [])
            .unwrap();
        migrate(&mut connection).unwrap();
        migrate(&mut connection).unwrap();
        let item = &list_items(&mut connection, ItemView::Inbox).unwrap().items[0];
        assert_eq!(item.content.as_deref(), Some("  سلام  "));
        assert!(item.formatted_html.is_none());
        assert_eq!(
            connection
                .query_row(
                    "SELECT auto_save_images FROM shortcut_settings",
                    [],
                    |row| row.get::<_, i32>(0)
                )
                .unwrap(),
            1
        );
        assert!(connection
            .execute(
                "UPDATE items SET formatted_html = ?1",
                ["ی".repeat(crate::formatted_text::MAX_HTML_BYTES / 2 + 1)]
            )
            .is_err());
        connection
            .execute("UPDATE items SET formatted_html = '<b>سلام</b>'", [])
            .unwrap();
        assert_eq!(
            item_for_copy(&connection, "old").unwrap(),
            CopyContent::Text("  سلام  ".into(), Some("<b>سلام</b>".into()))
        );
    }
}

#[cfg(test)]
mod link_tests {
    use super::*;
    #[test]
    fn opens_only_saved_http_links() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection.execute("INSERT INTO items (id,type,content,status,created_at) VALUES ('link','link','https://example.com/سلام','pending','now'), ('bad','link','file:///C:/secret','pending','now'), ('text','text','https://example.com','pending','now')", []).unwrap();
        assert!(item_link(&connection, "link")
            .unwrap()
            .starts_with("https://example.com/"));
        for id in ["bad", "text", "missing"] {
            assert!(item_link(&connection, id).is_err());
        }
    }
}

pub fn update_note(
    connection: &mut Connection,
    id: &str,
    content: &str,
) -> Result<(), &'static str> {
    if content.trim().is_empty() {
        return Err("empty_selection");
    }
    if content.encode_utf16().count() > crate::capture::MAX_TEXT_UNITS {
        return Err("too_large");
    }
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "storage_unavailable")?;
    let old: String = tx
        .query_row(
            "SELECT content FROM items WHERE id=?1 AND type IN ('text','link')",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| "item_unavailable")?;
    if old == content {
        return Ok(());
    }
    let duplicate: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM (SELECT id,type,content FROM items ORDER BY rowid DESC LIMIT 1) WHERE id<>?1 AND type IN ('text','link') AND content=?2)", rusqlite::params![id,content], |r| r.get(0)).map_err(|_| "storage_unavailable")?;
    if duplicate {
        return Err("duplicate_content");
    }
    tx.execute(
        "UPDATE items SET content=?2,type=?3,formatted_html=NULL WHERE id=?1",
        rusqlite::params![id, content, crate::capture::content_type(content)],
    )
    .map_err(|_| "storage_unavailable")?;
    tx.commit().map_err(|_| "storage_unavailable")
}

#[cfg(test)]
mod daily_tests {
    use super::*;
    #[test]
    fn v6_theme_migration_preserves_data_and_constraints() {
        let mut db = Connection::open_in_memory().unwrap();
        for sql in [
            include_str!("../migrations/0001_items.sql"),
            include_str!("../migrations/0002_categories.sql"),
            include_str!("../migrations/0003_shortcuts.sql"),
            include_str!("../migrations/0004_images.sql"),
            include_str!("../migrations/0005_auto_images.sql"),
            include_str!("../migrations/0006_formatted_text.sql"),
        ] {
            db.execute_batch(sql).unwrap();
        }
        db.pragma_update(None, "user_version", 6).unwrap();
        db.execute(
            "INSERT INTO items(id,type,content,created_at) VALUES('a','text','keep','now')",
            [],
        )
        .unwrap();
        migrate(&mut db).unwrap();
        assert_eq!(
            db.query_row("SELECT theme FROM shortcut_settings", [], |r| r
                .get::<_, Option<String>>(0))
                .unwrap(),
            None
        );
        assert_eq!(
            db.query_row("SELECT content FROM items WHERE id='a'", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
        assert!(db
            .execute("UPDATE shortcut_settings SET theme='invalid'", [])
            .is_err());
        db.execute("UPDATE shortcut_settings SET theme='light'", [])
            .unwrap();
        migrate(&mut db).unwrap();
        assert_eq!(
            db.query_row("SELECT theme FROM shortcut_settings", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "light"
        );
    }
    #[test]
    fn edits_preserve_identity_status_and_formatting_until_content_changes() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        db.execute("INSERT INTO items(id,type,content,created_at,status,formatted_html,completed_at) VALUES('a','text','old','now','completed','<b>old</b>','now'),('b','text','duplicate','now','pending',NULL,NULL)",[]).unwrap();
        update_note(&mut db, "a", "old").unwrap();
        assert_eq!(
            db.query_row("SELECT formatted_html FROM items WHERE id='a'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "<b>old</b>"
        );
        assert_eq!(
            update_note(&mut db, "a", "duplicate"),
            Err("duplicate_content")
        );
        assert_eq!(update_note(&mut db, "a", "  "), Err("empty_selection"));
        assert_eq!(
            update_note(
                &mut db,
                "a",
                &"x".repeat(crate::capture::MAX_TEXT_UNITS + 1)
            ),
            Err("too_large")
        );
        assert_eq!(
            update_note(&mut db, "missing", "new"),
            Err("item_unavailable")
        );
        update_note(&mut db, "a", "  https://example.com/  ").unwrap();
        let row = db
            .query_row(
                "SELECT type,content,status,created_at,formatted_html FROM items WHERE id='a'",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            row,
            (
                "link".into(),
                "  https://example.com/  ".into(),
                "completed".into(),
                "now".into(),
                None
            )
        );
    }
}
