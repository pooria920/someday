use serde::Serialize;
use std::path::Path;

pub const MAX_TEXT_UNITS: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Saved,
    Duplicate,
    EmptySelection,
    ProtectedField,
    FocusChanged,
    ReadFailed,
    CopyFailed,
    TooLarge,
    StorageUnavailable,
}

impl Outcome {
    pub fn message(self) -> &'static str {
        match self {
            Self::Saved => "Saved to Someday",
            Self::Duplicate => "Already saved to Someday",
            Self::EmptySelection => "Select some text first",
            Self::ProtectedField => "Password fields are not captured",
            Self::FocusChanged => "Focus changed. Please select and try again",
            Self::ReadFailed => "Could not read this selection",
            Self::CopyFailed => "Could not copy this selection",
            Self::TooLarge => "This selection is too large",
            Self::StorageUnavailable => "Could not save. Please try again",
        }
    }
}

pub enum Selection {
    Text(String),
    EditorProxy,
    Unsupported,
}

pub trait SelectionSource {
    fn is_current(&self) -> bool;
    fn selected_text(&mut self) -> Result<Selection, Outcome>;
    fn copy_selection(&mut self) -> Result<String, Outcome>;
    fn copy_formatted_selection(&mut self) -> Result<(String, Option<String>), Outcome> {
        self.copy_selection().map(|text| (text, None))
    }
}

pub fn read_formatted_content(
    source: &mut impl SelectionSource,
) -> Result<(String, Option<String>), Outcome> {
    if !source.is_current() {
        return Err(Outcome::FocusChanged);
    }
    let selection = source.selected_text()?;
    if !source.is_current() {
        return Err(Outcome::FocusChanged);
    }
    let direct = match selection {
        Selection::Text(text) => {
            if text.trim().is_empty() {
                return Err(Outcome::EmptySelection);
            }
            if text.encode_utf16().count() > MAX_TEXT_UNITS {
                return Err(Outcome::TooLarge);
            }
            Some(text)
        }
        _ => None,
    };
    let copied = source.copy_formatted_selection();
    if !source.is_current() {
        return Err(Outcome::FocusChanged);
    }
    let (text, html) = match copied {
        Ok((text, html)) => {
            // Some UIA providers return only a visible portion; fresh verified copy is authoritative.
            if let Some(direct) = direct {
                let matches = direct.replace("\r\n", "\n").replace('\r', "\n")
                    == text.replace("\r\n", "\n").replace('\r', "\n");
                (if matches { direct } else { text }, html)
            } else {
                (text, html)
            }
        }
        Err(Outcome::CopyFailed) => (direct.ok_or(Outcome::CopyFailed)?, None),
        Err(error) => return Err(error),
    };
    if text.trim().is_empty() {
        return Err(Outcome::EmptySelection);
    }
    if text.encode_utf16().count() > MAX_TEXT_UNITS {
        return Err(Outcome::TooLarge);
    }
    Ok((text, html))
}

pub fn fresh_clipboard(before: u32, after: u32, owner_matches: bool, focus_matches: bool) -> bool {
    before != after && owner_matches && focus_matches
}

pub fn content_type(content: &str) -> &'static str {
    let text = content.trim();
    if !text.chars().any(char::is_whitespace)
        && url::Url::parse(text)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
    {
        "link"
    } else {
        "text"
    }
}

#[cfg(test)]
pub fn save(path: &Path, content: &str) -> Result<(Outcome, u32), Outcome> {
    save_formatted(path, content, None)
}

pub fn save_formatted(
    path: &Path,
    content: &str,
    html: Option<&str>,
) -> Result<(Outcome, u32), Outcome> {
    save_content(path, content, html, false)
}

pub fn save_note(path: &Path, content: &str) -> Result<(Outcome, u32), Outcome> {
    save_content(path, content, None, true)
}

fn save_content(
    path: &Path,
    content: &str,
    html: Option<&str>,
    clear_draft: bool,
) -> Result<(Outcome, u32), Outcome> {
    let html = html.and_then(crate::formatted_text::clean);
    if content.trim().is_empty() {
        return Err(Outcome::EmptySelection);
    }
    if content.encode_utf16().count() > MAX_TEXT_UNITS {
        return Err(Outcome::TooLarge);
    }
    let parent = path.parent().ok_or(Outcome::StorageUnavailable)?;
    std::fs::create_dir_all(parent).map_err(|_| Outcome::StorageUnavailable)?;
    let mut connection = crate::database::open(path).map_err(|_| Outcome::StorageUnavailable)?;
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| Outcome::StorageUnavailable)?;
    if transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM (SELECT type, content FROM items ORDER BY rowid DESC LIMIT 1) WHERE type IN ('text', 'link') AND content = ?1)",
            [content],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|_| Outcome::StorageUnavailable)?
    {
        let count = crate::database::pending_count(&transaction)
            .map_err(|_| Outcome::StorageUnavailable)?;
        transaction
            .commit()
            .map_err(|_| Outcome::StorageUnavailable)?;
        return Ok((Outcome::Duplicate, count));
    }
    transaction.execute(
        "INSERT INTO items (id, type, content, status, created_at, formatted_html) VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), content_type(content), content, chrono::Utc::now().to_rfc3339(), html],
    ).map_err(|_| Outcome::StorageUnavailable)?;
    if clear_draft {
        transaction
            .execute("UPDATE note_draft SET content = '' WHERE id = 1", [])
            .map_err(|_| Outcome::StorageUnavailable)?;
    }
    let count =
        crate::database::pending_count(&transaction).map_err(|_| Outcome::StorageUnavailable)?;
    transaction
        .commit()
        .map_err(|_| Outcome::StorageUnavailable)?;
    Ok((Outcome::Saved, count))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn read_content(source: &mut impl SelectionSource) -> Result<String, Outcome> {
        read_formatted_content(source).map(|(text, _)| text)
    }

    struct Fake {
        current: std::cell::Cell<bool>,
        result: Option<Result<Selection, Outcome>>,
        copied: bool,
        change_focus: bool,
        copy_error: Option<Outcome>,
        copy_text: Option<String>,
    }
    impl SelectionSource for Fake {
        fn is_current(&self) -> bool {
            self.current.get()
        }
        fn selected_text(&mut self) -> Result<Selection, Outcome> {
            if self.change_focus {
                self.current.set(false);
            }
            self.result.take().unwrap()
        }
        fn copy_selection(&mut self) -> Result<String, Outcome> {
            self.copied = true;
            self.copy_error.map_or_else(
                || Ok(self.copy_text.clone().unwrap_or_else(|| "fallback".into())),
                Err,
            )
        }
        fn copy_formatted_selection(&mut self) -> Result<(String, Option<String>), Outcome> {
            self.copy_selection()
                .map(|text| (text, Some("<b>fallback</b>".into())))
        }
    }
    fn fake(selection: Result<Selection, Outcome>) -> Fake {
        Fake {
            current: true.into(),
            result: Some(selection),
            copied: false,
            change_focus: false,
            copy_error: None,
            copy_text: None,
        }
    }
    #[test]
    fn direct_selection_preserves_persian_and_newlines() {
        let text = "  سلام\nجهان  ";
        let mut source = fake(Ok(Selection::Text(text.into())));
        source.copy_text = Some(text.replace('\n', "\r\n"));
        assert_eq!(read_content(&mut source).unwrap(), text);
        assert!(source.copied);
    }
    #[test]
    fn formatting_copy_failure_keeps_only_valid_direct_text() {
        let mut source = fake(Ok(Selection::Text("  سلام\r\n📝  ".into())));
        source.copy_error = Some(Outcome::CopyFailed);
        assert_eq!(
            read_formatted_content(&mut source),
            Ok(("  سلام\r\n📝  ".into(), None))
        );
        for error in [Outcome::FocusChanged, Outcome::ProtectedField] {
            let mut source = fake(Ok(Selection::Text("safe".into())));
            source.copy_error = Some(error);
            assert_eq!(read_formatted_content(&mut source), Err(error));
        }
    }

    #[test]
    fn fresh_verified_copy_takes_precedence_over_partial_uia_text() {
        let mut source = fake(Ok(Selection::Text("fallback".into())));
        assert_eq!(
            read_formatted_content(&mut source),
            Ok(("fallback".into(), Some("<b>fallback</b>".into())))
        );
        let mut source = fake(Ok(Selection::Text("different selection".into())));
        assert_eq!(
            read_formatted_content(&mut source),
            Ok(("fallback".into(), Some("<b>fallback</b>".into())))
        );
        let mut source = fake(Ok(Selection::EditorProxy));
        assert_eq!(
            read_formatted_content(&mut source),
            Ok(("fallback".into(), Some("<b>fallback</b>".into())))
        );
    }

    #[test]
    fn partial_uia_selection_does_not_truncate_a_long_fresh_copy() {
        struct LongSelection;
        impl SelectionSource for LongSelection {
            fn is_current(&self) -> bool {
                true
            }
            fn selected_text(&mut self) -> Result<Selection, Outcome> {
                Ok(Selection::Text("start\n".into()))
            }
            fn copy_selection(&mut self) -> Result<String, Outcome> {
                Ok(format!("start\n{}\nend", "سلام 📝\n".repeat(4000)))
            }
        }
        let (text, html) = read_formatted_content(&mut LongSelection).unwrap();
        assert!(text.ends_with("\nend"));
        assert_eq!(text.matches("سلام 📝").count(), 4000);
        assert!(html.is_none());
    }

    #[test]
    fn formatted_saves_preserve_plain_text_and_never_replace_duplicate_formatting() {
        let path = std::env::temp_dir().join(format!("someday-format-{}.db", uuid::Uuid::new_v4()));
        let text = "  سلام\r\n📝  ";
        assert_eq!(
            save_formatted(&path, text, Some("<strong>سلام</strong>")),
            Ok((Outcome::Saved, 1))
        );
        let mut connection = crate::database::open(&path).unwrap();
        let snapshot =
            crate::database::list_items(&mut connection, crate::database::ItemView::Inbox).unwrap();
        let item = &snapshot.items[0];
        assert_eq!(item.content.as_deref(), Some(text));
        assert_eq!(item.formatted_html.as_deref(), Some("<strong>سلام</strong>"));
        crate::database::set_item_archived(&connection, &item.id, true).unwrap();
        assert_eq!(
            save_formatted(&path, text, Some("<em>different</em>")),
            Ok((Outcome::Duplicate, 0))
        );
        assert_eq!(
            crate::database::item_for_copy(&connection, &item.id).unwrap(),
            crate::database::CopyContent::Text(text.into(), item.formatted_html.clone())
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn unsupported_controls_and_editor_proxies_use_copy() {
        let mut source = fake(Ok(Selection::Unsupported));
        assert_eq!(read_content(&mut source).unwrap(), "fallback");
        assert!(source.copied);
        let mut source = fake(Ok(Selection::EditorProxy));
        assert_eq!(read_content(&mut source).unwrap(), "fallback");
        assert!(source.copied);
        for result in [
            Ok(Selection::Text(" \r\n".into())),
            Err(Outcome::ProtectedField),
            Err(Outcome::ReadFailed),
        ] {
            let mut source = fake(result);
            assert!(read_content(&mut source).is_err());
            assert!(!source.copied);
        }
    }
    #[test]
    fn focus_change_stops_before_copy() {
        let mut source = fake(Ok(Selection::Unsupported));
        source.change_focus = true;
        assert_eq!(read_content(&mut source), Err(Outcome::FocusChanged));
        assert!(!source.copied);
    }
    #[test]
    fn old_or_unrelated_clipboard_is_never_accepted() {
        assert!(!fresh_clipboard(10, 10, true, true));
        assert!(!fresh_clipboard(10, 11, false, true));
        assert!(!fresh_clipboard(10, 11, true, false));
        assert!(fresh_clipboard(u32::MAX, 0, true, true));
    }
    #[test]
    fn copy_failure_or_timeout_never_returns_old_content() {
        let mut source = fake(Ok(Selection::Unsupported));
        source.copy_error = Some(Outcome::CopyFailed);
        assert_eq!(read_content(&mut source), Err(Outcome::CopyFailed));
    }
    #[test]
    fn oversized_selection_is_rejected_without_copying() {
        let mut source = fake(Ok(Selection::Text("x".repeat(MAX_TEXT_UNITS + 1))));
        assert_eq!(read_content(&mut source), Err(Outcome::TooLarge));
        assert!(!source.copied);
    }
    #[test]
    fn detects_only_a_single_http_url() {
        assert_eq!(content_type(" https://example.com/a?q=1 "), "link");
        for text in [
            "سلام",
            "ftp://example.com",
            "https://",
            "Read https://example.com",
            "https://example.com\nhello",
        ] {
            assert_eq!(content_type(text), "text");
        }
    }
    #[test]
    fn rejects_only_consecutive_duplicates_across_calls_and_reopens() {
        let path =
            std::env::temp_dir().join(format!("someday-capture-{}.db", uuid::Uuid::new_v4()));
        let text = "  سلام\nجهان  ";
        assert_eq!(save(&path, text).unwrap(), (Outcome::Saved, 1));
        for _ in 0..3 {
            assert_eq!(save(&path, text).unwrap(), (Outcome::Duplicate, 1));
        }
        assert_eq!(
            save(&path, "another selection").unwrap(),
            (Outcome::Saved, 2)
        );
        assert_eq!(save(&path, text).unwrap(), (Outcome::Saved, 3));
        // Exact comparison keeps distinct whitespace, case and links intact.
        for (index, content) in [
            "سلام\nجهان",
            "Hello",
            "hello",
            "https://example.com/A",
            "https://example.com/a",
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(
                save(&path, content).unwrap(),
                (Outcome::Saved, index as u32 + 4)
            );
        }
        assert_eq!(save(&path, " \n"), Err(Outcome::EmptySelection));
        assert_eq!(
            save(&path, &"x".repeat(MAX_TEXT_UNITS + 1)),
            Err(Outcome::TooLarge)
        );
        let connection = crate::database::open(&path).unwrap();
        let (id, stored, created, status): (String, String, String, String) = connection
            .query_row(
                "SELECT id, content, created_at, status FROM items WHERE content = ?1",
                [text],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(stored, text);
        assert!(uuid::Uuid::parse_str(&id).is_ok());
        assert!(chrono::DateTime::parse_from_rfc3339(&created).is_ok());
        assert_eq!(status, "pending");
        assert_eq!(crate::database::pending_count(&connection).unwrap(), 8);
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn existing_archived_content_is_duplicate_until_deleted() {
        let path =
            std::env::temp_dir().join(format!("someday-recapture-{}.db", uuid::Uuid::new_v4()));
        let mut connection = crate::database::open(&path).unwrap();
        connection.execute("INSERT INTO items (id,type,content,created_at) VALUES ('old','text','hello','2026-10-30T00:00:00Z')", []).unwrap();
        let category = crate::database::create_category(&connection, "Work").unwrap();
        crate::database::set_item_category(&connection, "old", Some(&category.id)).unwrap();
        assert_eq!(save(&path, "hello").unwrap(), (Outcome::Duplicate, 1));
        crate::database::set_item_archived(&connection, "old", true).unwrap();
        assert_eq!(save(&path, "hello").unwrap(), (Outcome::Duplicate, 0));
        let snapshot =
            crate::database::list_items(&mut connection, crate::database::ItemView::Archive)
                .unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].id, "old");
        assert_eq!(snapshot.items[0].created_at, "2026-10-30T00:00:00Z");
        assert_eq!(
            snapshot.items[0].category_id.as_deref(),
            Some(category.id.as_str())
        );
        crate::database::delete_item(&connection, "old").unwrap();
        assert_eq!(save(&path, "hello").unwrap(), (Outcome::Saved, 1));
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn concurrent_saves_create_only_one_item() {
        let path =
            std::env::temp_dir().join(format!("someday-concurrent-{}.db", uuid::Uuid::new_v4()));
        drop(crate::database::open(&path).unwrap());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    save(&path, "same selection").unwrap()
                })
            })
            .collect();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(
            outcomes
                .iter()
                .filter(|result| **result == (Outcome::Saved, 1))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|result| **result == (Outcome::Duplicate, 1))
                .count(),
            1
        );
        let connection = crate::database::open(&path).unwrap();
        assert_eq!(crate::database::pending_count(&connection).unwrap(), 1);
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_storage_does_not_report_success() {
        assert_eq!(
            save(&std::env::temp_dir(), "hello"),
            Err(Outcome::StorageUnavailable)
        );
    }
}
