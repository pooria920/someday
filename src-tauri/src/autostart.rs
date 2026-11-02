//! Current-user Windows registration is the source of truth, separate from database backups.
use std::{io, path::Path};
use winreg::{enums::*, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "com.someday.desktop";

fn command(executable: &Path) -> Result<String, &'static str> {
    let path = executable.to_str().ok_or("autostart_unavailable")?;
    // Run keys have a documented 260-character command limit. Never publish a truncated path.
    let value = format!("\"{path}\" --autostart");
    if !executable.is_absolute() || path.contains(['"', '\0']) || value.encode_utf16().count() > 260
    {
        return Err("autostart_unavailable");
    }
    Ok(value)
}

fn registered(key: &RegKey, expected: &str) -> io::Result<bool> {
    match key.get_value::<String, _>(VALUE_NAME) {
        Ok(value) => Ok(value == expected),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn enabled() -> Result<bool, &'static str> {
    let expected = command(&std::env::current_exe().map_err(|_| "autostart_unavailable")?)?;
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_KEY) {
        Ok(key) => registered(&key, &expected).map_err(|_| "autostart_unavailable"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("autostart_unavailable"),
    }
}

fn write(key: &RegKey, value: Option<&str>) -> io::Result<()> {
    match value {
        Some(value) => key.set_value(VALUE_NAME, &value),
        None => match key.delete_value(VALUE_NAME) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        },
    }
}

pub fn set(enabled: bool) -> Result<bool, &'static str> {
    let current = std::env::current_exe().map_err(|_| "autostart_unavailable")?;
    let value = if enabled {
        Some(command(&current)?)
    } else {
        None
    };
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
        .map_err(|_| "autostart_unavailable")?;
    write(&key, value.as_deref()).map_err(|_| "autostart_unavailable")?;
    Ok(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_unicode_and_space_paths_and_rejects_invalid_commands() {
        assert_eq!(
            command(Path::new(r"C:\My Apps\یادداشت\Someday.exe")).unwrap(),
            "\"C:\\My Apps\\یادداشت\\Someday.exe\" --autostart"
        );
        assert!(command(Path::new("relative.exe")).is_err());
        assert!(command(Path::new("C:\\bad\"name.exe")).is_err());
        assert!(command(Path::new(&format!("C:\\{}.exe", "x".repeat(260)))).is_err());
    }

    #[test]
    fn registration_is_idempotent_preserves_other_entries_and_failed_writes() {
        // Exercise real registry I/O under a disposable key, never the actual startup key.
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let path = format!(r"Software\SomedayTests\{}", uuid::Uuid::new_v4());
        let (key, _) = root.create_subkey(&path).unwrap();
        let expected = command(Path::new(r"C:\My Apps\Someday.exe")).unwrap();
        key.set_value("other", &"unchanged").unwrap();
        assert!(!registered(&key, &expected).unwrap());
        write(&key, Some(&expected)).unwrap();
        write(&key, Some(&expected)).unwrap();
        drop(key);
        let readonly = root.open_subkey_with_flags(&path, KEY_READ).unwrap();
        assert!(registered(&readonly, &expected).unwrap());
        assert!(!registered(&readonly, "different executable").unwrap());
        assert!(write(&readonly, None).is_err());
        assert!(registered(&readonly, &expected).unwrap());
        drop(readonly);
        let key = root.open_subkey_with_flags(&path, KEY_ALL_ACCESS).unwrap();
        write(&key, None).unwrap();
        write(&key, None).unwrap();
        assert!(!registered(&key, &expected).unwrap());
        assert_eq!(key.get_value::<String, _>("other").unwrap(), "unchanged");
        drop(key);
        root.delete_subkey_all(&path).unwrap();
    }
}
