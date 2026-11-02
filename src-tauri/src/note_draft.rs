use std::{path::Path, sync::Mutex};

#[derive(Default)]
pub struct NoteDraft(Mutex<Option<Draft>>);

struct Draft {
    revision: u64,
    content: String,
    dirty: bool,
}

#[derive(serde::Serialize)]
pub struct DraftSnapshot {
    pub content: String,
    pub revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::Outcome, database};

    #[test]
    fn draft_survives_restart_preserves_exact_text_and_rejects_stale_updates() {
        let folder = std::env::temp_dir().join(format!("someday-draft-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("someday.db");
        let draft = NoteDraft::default();
        assert_eq!(draft.load(&path).unwrap(), "");
        let source = "  یادداشت\r\n\n unfinished  ";
        draft.stage(2, source.into()).unwrap();
        draft.stage(1, "stale".into()).unwrap();
        draft.persist(&path).unwrap();
        assert_eq!(NoteDraft::default().load(&path).unwrap(), source);
        let reloaded = draft.snapshot(&path).unwrap();
        assert_eq!(reloaded.revision, 2);
        assert_eq!(reloaded.content, source);
        assert_eq!(draft.stage(3, "x".repeat(1_048_577)), Err("too_large"));
        assert_eq!(draft.load(&path).unwrap(), source);
        draft.stage(4, " \r\n ".into()).unwrap();
        draft.persist(&path).unwrap();
        assert_eq!(NoteDraft::default().load(&path).unwrap(), " \r\n ");
        draft.stage(5, String::new()).unwrap();
        draft.persist(&path).unwrap();
        assert_eq!(NoteDraft::default().load(&path).unwrap(), "");
        drop(draft);
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn save_clears_draft_atomically_and_failures_or_duplicates_keep_it() {
        let folder = std::env::temp_dir().join(format!("someday-draft-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("someday.db");
        let draft = NoteDraft::default();
        draft.load(&path).unwrap();
        draft.stage(1, "unsaved".into()).unwrap();
        draft.persist(&path).unwrap();
        let db = database::open(&path).unwrap();
        db.execute_batch("CREATE TRIGGER fail_draft BEFORE UPDATE ON note_draft BEGIN SELECT RAISE(ABORT, 'blocked'); END;").unwrap();
        assert_eq!(
            draft.save(&path, "unsaved"),
            Err(Outcome::StorageUnavailable)
        );
        assert_eq!(database::pending_count(&db).unwrap(), 0);
        assert_eq!(NoteDraft::default().load(&path).unwrap(), "unsaved");
        db.execute_batch("DROP TRIGGER fail_draft").unwrap();
        assert_eq!(draft.save(&path, "unsaved").unwrap().0, Outcome::Saved);
        draft.persist(&path).unwrap(); // Older workers cannot resurrect the cleared draft.
        assert_eq!(NoteDraft::default().load(&path).unwrap(), "");
        draft.stage(2, "unsaved".into()).unwrap();
        draft.persist(&path).unwrap();
        assert_eq!(draft.save(&path, "unsaved").unwrap().0, Outcome::Duplicate);
        assert_eq!(NoteDraft::default().load(&path).unwrap(), "unsaved");
        draft.stage(3, "new text".into()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_draft BEFORE UPDATE ON note_draft BEGIN SELECT RAISE(ABORT, 'blocked'); END;").unwrap();
        assert!(draft.persist(&path).is_err());
        assert_eq!(draft.load(&path).unwrap(), "new text");
        db.execute_batch("DROP TRIGGER fail_draft").unwrap();
        draft.persist(&path).unwrap();
        assert_eq!(NoteDraft::default().load(&path).unwrap(), "new text");
        drop(db);
        std::fs::remove_dir_all(folder).unwrap();
    }
}

pub fn validate(content: &str) -> Result<(), &'static str> {
    if content.encode_utf16().count() > 1_048_576 {
        Err("too_large")
    } else {
        Ok(())
    }
}

impl NoteDraft {
    pub fn snapshot(&self, path: &Path) -> Result<DraftSnapshot, &'static str> {
        self.load(path)?;
        let latest = self.0.lock().map_err(|_| "storage_unavailable")?;
        let draft = latest.as_ref().ok_or("storage_unavailable")?;
        Ok(DraftSnapshot {
            content: draft.content.clone(),
            revision: draft.revision,
        })
    }
    pub fn stage(&self, revision: u64, content: String) -> Result<(), &'static str> {
        validate(&content)?;
        let mut latest = self.0.lock().map_err(|_| "storage_unavailable")?;
        if latest
            .as_ref()
            .is_none_or(|previous| revision > previous.revision)
        {
            let dirty = latest
                .as_ref()
                .is_none_or(|previous| previous.dirty || previous.content != content);
            *latest = Some(Draft {
                revision,
                content,
                dirty,
            });
        }
        Ok(())
    }

    pub fn load(&self, path: &Path) -> Result<String, &'static str> {
        let mut latest = self.0.lock().map_err(|_| "storage_unavailable")?;
        if let Some(draft) = latest.as_ref() {
            return Ok(draft.content.clone());
        }
        let db = crate::database::open(path).map_err(|_| "storage_unavailable")?;
        let content: String = db
            .query_row("SELECT content FROM note_draft WHERE id=1", [], |r| {
                r.get(0)
            })
            .map_err(|_| "storage_unavailable")?;
        validate(&content)?;
        *latest = Some(Draft {
            revision: 0,
            content: content.clone(),
            dirty: false,
        });
        Ok(content)
    }

    pub fn persist(&self, path: &Path) -> Result<(), &'static str> {
        // Hold the same lock through the write so stale workers cannot overwrite newer text.
        let mut latest = self.0.lock().map_err(|_| "storage_unavailable")?;
        let Some(draft) = latest.as_mut() else {
            return Ok(());
        };
        if !draft.dirty {
            return Ok(());
        }
        let db = crate::database::open(path).map_err(|_| "storage_unavailable")?;
        let count = db
            .execute(
                "UPDATE note_draft SET content=?1 WHERE id=1",
                [&draft.content],
            )
            .map_err(|_| "storage_unavailable")?;
        if count != 1 {
            return Err("storage_unavailable");
        }
        draft.dirty = false;
        Ok(())
    }

    pub fn save(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<(crate::capture::Outcome, u32), crate::capture::Outcome> {
        let mut latest = self
            .0
            .lock()
            .map_err(|_| crate::capture::Outcome::StorageUnavailable)?;
        let result = crate::capture::save_note(path, content)?;
        if result.0 == crate::capture::Outcome::Saved {
            let revision = latest.as_ref().map_or(0, |draft| draft.revision);
            *latest = Some(Draft {
                revision,
                content: String::new(),
                dirty: false,
            });
        }
        Ok(result)
    }
}
