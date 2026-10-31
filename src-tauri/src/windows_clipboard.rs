use std::sync::{Mutex, MutexGuard};
use windows::Win32::{
    Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
            OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
    },
};

static CLIPBOARD: Mutex<()> = Mutex::new(());
static OWN_IMAGE_SEQUENCE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn lock() -> Result<MutexGuard<'static, ()>, &'static str> {
    CLIPBOARD.lock().map_err(|_| "clipboard_unavailable")
}

struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

struct Allocation(HGLOBAL);
impl Allocation {
    unsafe fn new(bytes: &[u8]) -> Result<Self, &'static str> {
        let memory =
            Self(GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|_| "clipboard_unavailable")?);
        let pointer = GlobalLock(memory.0) as *mut u8;
        if pointer.is_null() {
            return Err("clipboard_unavailable");
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
        let _ = GlobalUnlock(memory.0);
        Ok(memory)
    }
    unsafe fn transfer(self, format: u32) -> Result<(), &'static str> {
        SetClipboardData(format, Some(HANDLE(self.0 .0))).map_err(|_| "clipboard_unavailable")?;
        std::mem::forget(self);
        Ok(())
    }
}
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}

pub fn write_formatted(
    owner: isize,
    content: &str,
    html: Option<&str>,
) -> Result<(), &'static str> {
    if content.contains('\0') {
        return Err("clipboard_unavailable");
    }
    let _guard = lock()?;
    let text: Vec<u8> = content
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let html = html
        .and_then(crate::formatted_text::clean)
        .map(|html| crate::formatted_text::clipboard_document(&html));
    unsafe {
        let text = Allocation::new(&text)?;
        let formatted = if let Some(html) = html {
            let format = RegisterClipboardFormatW(windows::core::w!("HTML Format"));
            if format == 0 {
                return Err("clipboard_unavailable");
            }
            let mut bytes = html.into_bytes();
            bytes.push(0);
            Some((format, Allocation::new(&bytes)?))
        } else {
            None
        };
        OpenClipboard(Some(HWND(owner as *mut _))).map_err(|_| "clipboard_unavailable")?;
        let _clipboard = Clipboard;
        EmptyClipboard().map_err(|_| "clipboard_unavailable")?;
        text.transfer(13)?;
        if let Some((format, memory)) = formatted {
            memory.transfer(format)?;
        }
    }
    Ok(())
}

pub fn read_image(owner: isize) -> Result<crate::images::PreparedImage, &'static str> {
    read_image_at(owner, None)
}

pub fn read_image_at(
    owner: isize,
    expected: Option<u32>,
) -> Result<crate::images::PreparedImage, &'static str> {
    let (bytes, png) = {
        let _guard = lock()?;
        unsafe {
            OpenClipboard(Some(HWND(owner as *mut _))).map_err(|_| "clipboard_unavailable")?;
            let _clipboard = Clipboard;
            if let Some(sequence) = expected {
                use windows::Win32::{
                    System::DataExchange::{GetClipboardOwner, GetClipboardSequenceNumber},
                    UI::WindowsAndMessaging::GetWindowThreadProcessId,
                };
                if GetClipboardSequenceNumber() != sequence
                    || OWN_IMAGE_SEQUENCE.load(std::sync::atomic::Ordering::Acquire) == sequence
                {
                    return Err("clipboard_changed");
                }
                let mut process = 0;
                if let Ok(owner) = GetClipboardOwner() {
                    GetWindowThreadProcessId(owner, Some(&mut process));
                }
                if process == std::process::id() {
                    return Err("clipboard_changed");
                }
            }
            let png_format = RegisterClipboardFormatW(windows::core::w!("PNG"));
            let (format, max_bytes, png) =
                if png_format != 0 && IsClipboardFormatAvailable(png_format).is_ok() {
                    (png_format, crate::images::MAX_IMAGE_BYTES, true)
                } else if IsClipboardFormatAvailable(17).is_ok() {
                    (17, crate::images::MAX_DIB_BYTES, false) // CF_DIBV5
                } else if IsClipboardFormatAvailable(8).is_ok() {
                    (8, crate::images::MAX_DIB_BYTES, false) // CF_DIB
                } else {
                    return Err("no_image");
                };
            let memory = HGLOBAL(
                GetClipboardData(format)
                    .map_err(|_| "clipboard_unavailable")?
                    .0,
            );
            let size = GlobalSize(memory);
            if size == 0 {
                return Err("invalid_image");
            }
            if size > max_bytes {
                return Err("image_too_large");
            }
            let pointer = GlobalLock(memory) as *const u8;
            if pointer.is_null() {
                return Err("clipboard_unavailable");
            }
            let bytes = std::slice::from_raw_parts(pointer, size).to_vec();
            let _ = GlobalUnlock(memory);
            (bytes, png)
        }
    };
    if png {
        crate::images::clipboard_png(bytes)
    } else {
        crate::images::clipboard_dib(bytes)
    }
}

pub fn write_image(_owner: isize, original: &[u8]) -> Result<(), &'static str> {
    let image = crate::images::decode(original)?.into_rgba8();
    let _guard = lock()?;
    arboard::Clipboard::new()
        .map_err(|_| "clipboard_unavailable")?
        .set_image(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: std::borrow::Cow::Borrowed(image.as_raw()),
        })
        .map_err(|_| "clipboard_unavailable")?;
    // arboard may leave a null clipboard owner, so retain its completed write's sequence.
    OWN_IMAGE_SEQUENCE.store(
        unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() },
        std::sync::atomic::Ordering::Release,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_embedded_null_before_touching_clipboard() {
        assert_eq!(
            super::write_formatted(0, "before\0after", None),
            Err("clipboard_unavailable")
        );
    }
}
