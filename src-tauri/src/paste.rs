use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

#[cfg(windows)]
pub fn capture_foreground_window() -> Option<isize> {
    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

#[cfg(not(windows))]
pub fn capture_foreground_window() -> Option<isize> {
    None
}

pub fn paste_text(text: &str, target_window: Option<isize>) -> Result<(), String> {
    // Set clipboard
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;

    // Restore target window focus
    #[cfg(windows)]
    if let Some(hwnd_val) = target_window {
        let hwnd = HWND(hwnd_val as *mut core::ffi::c_void);
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        // Brief delay to let the window come to front
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    #[cfg(not(windows))]
    let _ = target_window;

    // Simulate Ctrl+V
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo error: {}", e))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key press error: {}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Key click error: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key release error: {}", e))?;

    Ok(())
}
