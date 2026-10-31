// Controlled native test fixture: reserve Someday's shortcut until Enter is pressed.
#[cfg(target_os = "windows")]
fn main() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    };
    unsafe { RegisterHotKey(None, 1, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, b'S' as u32) }
        .expect("close Someday before starting the shortcut fixture");
    println!("Shortcut reserved. Start Someday and check its shortcut-conflict message. Press Enter to release.");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    unsafe {
        let _ = UnregisterHotKey(None, 1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This fixture requires Windows.");
}
