use crate::capture::{fresh_clipboard, Outcome, Selection, SelectionSource, MAX_TEXT_UNITS};
use std::{
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HGLOBAL, HWND},
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::{
        Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        DataExchange::{
            CloseClipboard, GetClipboardData, GetClipboardOwner, GetClipboardSequenceNumber,
            OpenClipboard, RegisterClipboardFormatW,
        },
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
            UIA_IsTextPatternAvailablePropertyId, UIA_TextPatternId,
        },
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        },
    },
};

#[derive(Clone, Copy)]
pub struct Target {
    window: isize,
    focus: isize,
    process: u32,
    thread: u32,
}

impl Target {
    pub fn current() -> Result<Self, Outcome> {
        unsafe {
            let window = GetForegroundWindow();
            let mut process = 0;
            let thread = GetWindowThreadProcessId(window, Some(&mut process));
            if window.0.is_null() || thread == 0 || process == std::process::id() {
                return Err(Outcome::ReadFailed);
            }
            let mut info = GUITHREADINFO {
                cbSize: size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            GetGUIThreadInfo(thread, &mut info).map_err(|_| Outcome::ReadFailed)?;
            Ok(Self {
                window: window.0 as isize,
                focus: info.hwndFocus.0 as isize,
                process,
                thread,
            })
        }
    }

    pub fn feedback_bounds(&self) -> Option<(i32, i32, u32, u32)> {
        unsafe {
            let hwnd = HWND(self.window as *mut _);
            let mut monitor = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(
                MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                return None;
            }
            let dpi = GetDpiForWindow(hwnd).max(96);
            let width = 300 * dpi / 96;
            let height = 76 * dpi / 96;
            let margin = (24 * dpi / 96) as i32;
            Some((
                monitor.rcWork.right - width as i32 - margin,
                monitor.rcWork.bottom - height as i32 - margin,
                width,
                height,
            ))
        }
    }
}

impl SelectionSource for Target {
    fn is_current(&self) -> bool {
        Self::current().is_ok_and(|now| {
            now.window == self.window
                && now.process == self.process
                && now.thread == self.thread
                && now.focus == self.focus
        })
    }

    fn selected_text(&mut self) -> Result<Selection, Outcome> {
        bounded_selection(*self)
    }

    fn copy_selection(&mut self) -> Result<String, Outcome> {
        self.copy_with_format().map(|(text, _)| text)
    }
    fn copy_formatted_selection(&mut self) -> Result<(String, Option<String>), Outcome> {
        self.copy_with_format()
    }
}

impl Target {
    fn copy_with_format(&mut self) -> Result<(String, Option<String>), Outcome> {
        let deadline = Instant::now() + Duration::from_millis(750);
        while !keys_released() {
            if !self.is_current() {
                return Err(Outcome::FocusChanged);
            }
            if Instant::now() >= deadline {
                return Err(Outcome::CopyFailed);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Recheck password/selection after the user's shortcut modifiers have been released.
        let editor_proxy = match bounded_selection(*self)? {
            Selection::Text(text) if text.trim().is_empty() => return Err(Outcome::EmptySelection),
            Selection::Text(_) => false,
            Selection::EditorProxy => true,
            Selection::Unsupported => false,
        };
        let _clipboard_guard = crate::windows_clipboard::lock().map_err(|_| Outcome::CopyFailed)?;
        if !self.is_current() {
            return Err(Outcome::FocusChanged);
        }
        if !keys_released() {
            return Err(Outcome::CopyFailed);
        }
        unsafe {
            let sequence = GetClipboardSequenceNumber();
            let events = [
                key(VK_CONTROL, false),
                key(VIRTUAL_KEY(b'C' as u16), false),
                key(VIRTUAL_KEY(b'C' as u16), true),
                key(VK_CONTROL, true),
            ];
            let sent = SendInput(&events, size_of::<INPUT>() as i32);
            if sent != events.len() as u32 {
                if sent > 0 {
                    SendInput(
                        &[key(VIRTUAL_KEY(b'C' as u16), true), key(VK_CONTROL, true)],
                        size_of::<INPUT>() as i32,
                    );
                }
                return Err(Outcome::CopyFailed);
            }
            let deadline = Instant::now() + Duration::from_millis(750);
            loop {
                if !self.is_current() {
                    return Err(Outcome::FocusChanged);
                }
                if GetClipboardSequenceNumber() != sequence {
                    if let Some(text) = clipboard_text(self, sequence, editor_proxy)? {
                        return Ok(text);
                    }
                }
                if Instant::now() >= deadline {
                    return Err(Outcome::CopyFailed);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn keys_released() -> bool {
    // Wait for every supported shortcut key before injecting the fallback copy.
    [
        VK_CONTROL.0,
        VK_SHIFT.0,
        VK_MENU.0,
        VK_LWIN.0,
        VK_RWIN.0,
        0x20,
    ]
    .into_iter()
    .chain(b'0' as u16..=b'9' as u16)
    .chain(b'A' as u16..=b'Z' as u16)
    .chain(0x70..=0x7b)
    .all(|key| unsafe { GetAsyncKeyState(key as i32) >= 0 })
}

fn key(code: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    }
}

unsafe fn clipboard_text(
    target: &Target,
    before: u32,
    editor_proxy: bool,
) -> Result<Option<(String, Option<String>)>, Outcome> {
    if OpenClipboard(None).is_err() {
        return Ok(None);
    }
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }
    let _clipboard = Clipboard;
    let sequence = GetClipboardSequenceNumber();
    let mut owner_pid = 0;
    let owner = GetClipboardOwner().map_err(|_| Outcome::CopyFailed)?;
    GetWindowThreadProcessId(owner, Some(&mut owner_pid));
    if !fresh_clipboard(
        before,
        sequence,
        owner_pid == target.process,
        target.is_current(),
    ) {
        return Err(Outcome::CopyFailed);
    }
    if editor_proxy {
        let format =
            RegisterClipboardFormatW(windows::core::w!("Chromium Web Custom MIME Data Format"));
        let handle = GetClipboardData(format).map_err(|_| Outcome::CopyFailed)?;
        let memory = HGLOBAL(handle.0);
        let size = GlobalSize(memory);
        if size == 0 || size > MAX_TEXT_UNITS * 4 {
            return Err(Outcome::CopyFailed);
        }
        let pointer = GlobalLock(memory) as *const u8;
        if pointer.is_null() {
            return Err(Outcome::CopyFailed);
        }
        let result = editor_copy_selection(std::slice::from_raw_parts(pointer, size));
        let _ = GlobalUnlock(memory);
        result?;
    }
    let handle = GetClipboardData(13).map_err(|_| Outcome::CopyFailed)?; // CF_UNICODETEXT
    let memory = HGLOBAL(handle.0);
    let units = GlobalSize(memory) / 2;
    if units == 0 {
        return Err(Outcome::EmptySelection);
    }
    let pointer = GlobalLock(memory) as *const u16;
    if pointer.is_null() {
        return Err(Outcome::CopyFailed);
    }
    struct Locked(HGLOBAL);
    impl Drop for Locked {
        fn drop(&mut self) {
            unsafe {
                let _ = GlobalUnlock(self.0);
            }
        }
    }
    let _locked = Locked(memory);
    let data = std::slice::from_raw_parts(pointer, units.min(MAX_TEXT_UNITS + 1));
    let end = data.iter().position(|c| *c == 0).ok_or(Outcome::TooLarge)?;
    let text = String::from_utf16(&data[..end]).map_err(|_| Outcome::ReadFailed)?;
    let html = clipboard_html();
    if sequence != GetClipboardSequenceNumber() || !target.is_current() {
        return Err(Outcome::FocusChanged);
    }
    Ok(Some((text, html)))
}

// Called only while the fresh text snapshot holds the clipboard lock open.
unsafe fn clipboard_html() -> Option<String> {
    let format = RegisterClipboardFormatW(windows::core::w!("HTML Format"));
    if format == 0 {
        return None;
    }
    let handle = GetClipboardData(format).ok()?;
    let memory = HGLOBAL(handle.0);
    let size = GlobalSize(memory);
    if size == 0 || size > crate::formatted_text::MAX_HTML_BYTES {
        return None;
    }
    let pointer = GlobalLock(memory) as *const u8;
    if pointer.is_null() {
        return None;
    }
    let html = crate::formatted_text::clipboard_fragment(std::slice::from_raw_parts(pointer, size));
    let _ = GlobalUnlock(memory);
    html
}

static UIA_BUSY: AtomicBool = AtomicBool::new(false);

fn bounded_selection(target: Target) -> Result<Selection, Outcome> {
    if UIA_BUSY.swap(true, Ordering::AcqRel) {
        return Err(Outcome::ReadFailed);
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("someday-selection".into())
        .spawn(move || {
            struct Slot;
            impl Drop for Slot {
                fn drop(&mut self) {
                    UIA_BUSY.store(false, Ordering::Release);
                }
            }
            let slot = Slot;
            let result = unsafe { selected_text(target) };
            drop(slot);
            let _ = sender.send(result);
        });
    if spawned.is_err() {
        UIA_BUSY.store(false, Ordering::Release);
        return Err(Outcome::ReadFailed);
    }
    // A stalled provider retains the slot; timed-out calls never accumulate UIA workers or send input.
    receiver
        .recv_timeout(Duration::from_millis(800))
        .map_err(|_| Outcome::ReadFailed)?
}

unsafe fn selected_text(target: Target) -> Result<Selection, Outcome> {
    CoInitializeEx(None, COINIT_MULTITHREADED)
        .ok()
        .map_err(|_| Outcome::ReadFailed)?;
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let _com = Com;
    let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        .map_err(|_| Outcome::ReadFailed)?;
    let focused = automation
        .GetFocusedElement()
        .map_err(|_| Outcome::ReadFailed)?;
    if focused
        .CurrentIsPassword()
        .map_err(|_| Outcome::ReadFailed)?
        .as_bool()
    {
        return Err(Outcome::ProtectedField);
    }
    let walker = automation
        .RawViewWalker()
        .map_err(|_| Outcome::ReadFailed)?;
    let mut element = focused.clone();
    let mut editor_proxy = false;
    let mut text_element = None;
    let mut text_pattern_failed = false;
    for _ in 0..8 {
        if !target.is_current() {
            return Err(Outcome::FocusChanged);
        }
        if element
            .CurrentIsPassword()
            .map_err(|_| Outcome::ReadFailed)?
            .as_bool()
        {
            return Err(Outcome::ProtectedField);
        }
        editor_proxy |= element
            .CurrentClassName()
            .is_ok_and(|class| editor_class(&class.to_string()));
        if text_element.is_none() {
            let available = element
                .GetCurrentPropertyValue(UIA_IsTextPatternAvailablePropertyId)
                .and_then(|value| bool::try_from(&value));
            match available {
                Ok(true) => text_element = Some(element.clone()),
                Ok(false) => {}
                Err(_) => text_pattern_failed = true,
            }
        }
        let Ok(parent) = walker.GetParentElement(&element) else {
            break;
        };
        if parent.CurrentProcessId().map_err(|_| Outcome::ReadFailed)? == 0 {
            break;
        }
        element = parent;
    }
    // Monaco accessibility text can be paged or omit secondary selections.
    let selection = if editor_proxy {
        Ok(Selection::EditorProxy)
    } else {
        let selection = match text_element {
            Some(element) => text_selection(&element).map(Selection::Text),
            None if text_pattern_failed => Err(Outcome::ReadFailed),
            None => Ok(Selection::Unsupported),
        };
        editor_selection(selection, editor_process(target.process))
    };
    let current = automation
        .GetFocusedElement()
        .map_err(|_| Outcome::ReadFailed)?;
    if !automation
        .CompareElements(&focused, &current)
        .map_err(|_| Outcome::ReadFailed)?
        .as_bool()
    {
        return Err(Outcome::FocusChanged);
    }
    selection
}

fn editor_class(class: &str) -> bool {
    let mut classes = class.split_whitespace();
    classes.clone().any(|class| class == "native-edit-context")
        || (classes.clone().any(|class| class == "inputarea")
            && classes.any(|class| class == "monaco-mouse-cursor-text"))
}

unsafe fn editor_process(process: u32) -> bool {
    let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process) else {
        return false;
    };
    let mut path = [0u16; 32768];
    let mut size = path.len() as u32;
    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(path.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(handle);
    result.is_ok() && editor_executable(&String::from_utf16_lossy(&path[..size as usize]))
}

fn editor_executable(path: &str) -> bool {
    path.rsplit(['\\', '/']).next().is_some_and(|name| {
        name.eq_ignore_ascii_case("Code.exe") || name.eq_ignore_ascii_case("Code - Insiders.exe")
    })
}

fn editor_selection(
    selection: Result<Selection, Outcome>,
    editor_process: bool,
) -> Result<Selection, Outcome> {
    match selection {
        Ok(Selection::Text(text)) if editor_process && text.trim().is_empty() => {
            Ok(Selection::EditorProxy)
        }
        Ok(Selection::Unsupported) | Err(Outcome::ReadFailed) if editor_process => {
            Ok(Selection::EditorProxy)
        }
        result => result,
    }
}

unsafe fn text_selection(element: &IUIAutomationElement) -> Result<String, Outcome> {
    let pattern = element
        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        .map_err(|_| Outcome::ReadFailed)?;
    let ranges = pattern.GetSelection().map_err(|_| Outcome::ReadFailed)?;
    let count = ranges.Length().map_err(|_| Outcome::ReadFailed)?;
    if count > 32 {
        return Err(Outcome::TooLarge);
    }
    let mut parts = Vec::new();
    let mut units = 0;
    for index in 0..count {
        let text = ranges
            .GetElement(index)
            .and_then(|range| range.GetText((MAX_TEXT_UNITS + 1) as i32))
            .map_err(|_| Outcome::ReadFailed)?;
        units += text.len();
        if units > MAX_TEXT_UNITS {
            return Err(Outcome::TooLarge);
        }
        parts.push(text.to_string());
    }
    Ok(parts.join("\n"))
}

// Chromium stores custom MIME pairs as length-prefixed UTF-16 strings in a Pickle.
fn editor_copy_selection(data: &[u8]) -> Result<(), Outcome> {
    fn number(data: &mut &[u8]) -> Option<usize> {
        let (value, rest) = data.split_at_checked(4)?;
        *data = rest;
        Some(u32::from_le_bytes(value.try_into().ok()?) as usize)
    }
    fn string(data: &mut &[u8]) -> Option<String> {
        let size = number(data)?.checked_mul(2)?;
        let (value, rest) = data.split_at_checked(size.checked_add(3)? & !3)?;
        *data = rest;
        let units: Vec<_> = value[..size]
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect();
        String::from_utf16(&units).ok()
    }
    let parse = || -> Option<bool> {
        let mut header = data;
        let size = number(&mut header)?;
        let mut payload = header.get(..size)?;
        let count = number(&mut payload)?;
        for _ in 0..count {
            let mime = string(&mut payload)?;
            let value = string(&mut payload)?;
            if mime == "vscode-editor-data" {
                let metadata: serde_json::Value = serde_json::from_str(&value).ok()?;
                if metadata.get("version")?.as_u64()? != 1 {
                    return None;
                }
                return metadata.get("isFromEmptySelection")?.as_bool();
            }
        }
        None
    };
    match parse() {
        Some(false) => Ok(()),
        Some(true) => Err(Outcome::EmptySelection),
        None => Err(Outcome::CopyFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_editor_proxies_and_executables_are_recognized() {
        for class in [
            "native-edit-context",
            "inputarea monaco-mouse-cursor-text",
            "monaco-mouse-cursor-text inputarea",
        ] {
            assert!(editor_class(class));
        }
        for class in [
            "",
            "inputarea",
            "monaco-mouse-cursor-text",
            "monaco-editor",
            "input",
            "find-input",
            "not-inputarea",
        ] {
            assert!(!editor_class(class));
        }
        for path in [r"C:\Apps\Code.exe", r"C:\Apps\CODE - INSIDERS.EXE"] {
            assert!(editor_executable(path));
        }
        for path in [r"C:\Code.exe\other.exe", "not-Code.exe", "chrome.exe", ""] {
            assert!(!editor_executable(path));
        }
    }

    #[test]
    fn ambiguous_vscode_selection_requires_verified_editor_copy() {
        for selection in [
            Ok(Selection::Text(String::new())),
            Ok(Selection::Text("\r\n".into())),
            Ok(Selection::Unsupported),
            Err(Outcome::ReadFailed),
        ] {
            assert!(matches!(
                editor_selection(selection, true),
                Ok(Selection::EditorProxy)
            ));
        }
        for editor in [true, false] {
            let result = editor_selection(Ok(Selection::Text("  find text\r\n".into())), editor);
            assert!(matches!(result, Ok(Selection::Text(text)) if text == "  find text\r\n"));
            for error in [
                Outcome::ProtectedField,
                Outcome::FocusChanged,
                Outcome::TooLarge,
            ] {
                assert!(
                    matches!(editor_selection(Err(error), editor), Err(actual) if actual == error)
                );
            }
        }
        assert!(
            matches!(editor_selection(Ok(Selection::Text(String::new())), false), Ok(Selection::Text(text)) if text.is_empty())
        );
        assert!(matches!(
            editor_selection(Err(Outcome::ReadFailed), false),
            Err(Outcome::ReadFailed)
        ));
        assert!(matches!(
            editor_selection(Ok(Selection::Unsupported), false),
            Ok(Selection::Unsupported)
        ));
    }

    fn editor_data(metadata: &str) -> Vec<u8> {
        let mut payload = 1u32.to_le_bytes().to_vec();
        for text in ["vscode-editor-data", metadata] {
            let units: Vec<_> = text.encode_utf16().collect();
            payload.extend_from_slice(&(units.len() as u32).to_le_bytes());
            for unit in units {
                payload.extend_from_slice(&unit.to_le_bytes());
            }
            while payload.len() % 4 != 0 {
                payload.push(0);
            }
        }
        let mut data = (payload.len() as u32).to_le_bytes().to_vec();
        data.extend(payload);
        data
    }

    #[test]
    fn editor_copy_requires_explicit_nonempty_selection_metadata() {
        let data = editor_data(r#"{"version":1,"isFromEmptySelection":false}"#);
        assert_eq!(editor_copy_selection(&data), Ok(()));
        assert_eq!(
            editor_copy_selection(&editor_data(r#"{"version":1,"isFromEmptySelection":true}"#)),
            Err(Outcome::EmptySelection)
        );
        for metadata in [
            "{}",
            "invalid",
            r#"{"version":2,"isFromEmptySelection":false}"#,
        ] {
            assert_eq!(
                editor_copy_selection(&editor_data(metadata)),
                Err(Outcome::CopyFailed)
            );
        }
        for end in 0..data.len() {
            assert_eq!(
                editor_copy_selection(&data[..end]),
                Err(Outcome::CopyFailed)
            );
        }
        let mut corrupt = data;
        corrupt[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(editor_copy_selection(&corrupt), Err(Outcome::CopyFailed));
    }
}
