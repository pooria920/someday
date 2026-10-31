use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShortcutSettings {
    pub capture: String,
    pub toggle: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            capture: "Ctrl+Shift+S".into(),
            toggle: "Ctrl+Shift+Space".into(),
        }
    }
}

impl ShortcutSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_shortcut(&self.capture) || !valid_shortcut(&self.toggle) {
            return Err("invalid_shortcut");
        }
        if self.capture == self.toggle {
            return Err("duplicate_shortcut");
        }
        Ok(())
    }
}

fn valid_shortcut(value: &str) -> bool {
    let Some((modifiers, key)) = value.rsplit_once('+') else {
        return false;
    };
    matches!(
        modifiers,
        "Ctrl" | "Alt" | "Ctrl+Shift" | "Alt+Shift" | "Ctrl+Alt" | "Ctrl+Alt+Shift"
    ) && (key.len() == 1
        && key
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        || key == "Space"
        || (1..=12).any(|n| key == format!("F{n}")))
}

pub fn load(connection: &rusqlite::Connection) -> rusqlite::Result<ShortcutSettings> {
    let settings = connection.query_row(
        "SELECT capture, toggle FROM shortcut_settings WHERE id = 1",
        [],
        |row| {
            Ok(ShortcutSettings {
                capture: row.get(0)?,
                toggle: row.get(1)?,
            })
        },
    )?;
    settings
        .validate()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(settings)
}

pub fn save(
    connection: &rusqlite::Connection,
    settings: &ShortcutSettings,
) -> rusqlite::Result<()> {
    settings
        .validate()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    match connection.execute(
        "UPDATE shortcut_settings SET capture = ?1, toggle = ?2 WHERE id = 1",
        rusqlite::params![settings.capture, settings.toggle],
    )? {
        1 => Ok(()),
        _ => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unmodified_reserved_unsupported_and_duplicate_shortcuts() {
        for value in [
            "S",
            "Shift+S",
            "Super+S",
            "Ctrl+Escape",
            "Ctrl+Tab",
            "Ctrl+F13",
            "Ctrl+Shift+",
            "Ctrl+ctrl+S",
            "Ctrl+ش",
            "Ctrl+S+Alt",
        ] {
            assert!(!valid_shortcut(value), "{value}");
        }
        for value in [
            "Ctrl+S",
            "Alt+Shift+0",
            "Ctrl+Alt+Shift+F12",
            "Ctrl+Shift+Space",
        ] {
            assert!(valid_shortcut(value), "{value}");
        }
        assert_eq!(
            ShortcutSettings {
                capture: "Ctrl+S".into(),
                toggle: "Ctrl+S".into()
            }
            .validate(),
            Err("duplicate_shortcut")
        );
    }

    #[test]
    fn settings_survive_reopening_and_invalid_updates_preserve_them() {
        let path =
            std::env::temp_dir().join(format!("someday-settings-{}.db", uuid::Uuid::new_v4()));
        let connection = crate::database::open(&path).unwrap();
        assert_eq!(load(&connection).unwrap(), ShortcutSettings::default());
        let settings = ShortcutSettings {
            capture: "Alt+Shift+F8".into(),
            toggle: "Ctrl+Alt+Space".into(),
        };
        save(&connection, &settings).unwrap();
        assert!(save(
            &connection,
            &ShortcutSettings {
                capture: "S".into(),
                toggle: "S".into()
            }
        )
        .is_err());
        drop(connection);
        let connection = crate::database::open(&path).unwrap();
        assert_eq!(load(&connection).unwrap(), settings);
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }
}
