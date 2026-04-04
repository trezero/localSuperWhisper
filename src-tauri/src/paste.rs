use arboard::Clipboard;
#[cfg(not(target_os = "linux"))]
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

#[cfg(target_os = "linux")]
pub fn capture_foreground_window() -> Option<isize> {
    // Only works on X11; Wayland doesn't allow focus stealing
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if session_type == "wayland" {
        return None;
    }

    let output = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<isize>()
        .ok()
}

/// Apps that use Ctrl+Shift+V for paste instead of Ctrl+V
#[cfg(target_os = "linux")]
const CTRL_SHIFT_V_APPS: &[&str] = &["code", "windsurf", "antigravity"];

#[cfg(target_os = "linux")]
fn detect_paste_key(window_id: &str) -> &'static str {
    let output = std::process::Command::new("xprop")
        .args(["-id", window_id, "WM_CLASS"])
        .output();

    match output {
        Ok(out) => {
            let wm_class = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if CTRL_SHIFT_V_APPS.iter().any(|app| wm_class.contains(app)) {
                "ctrl+shift+v"
            } else {
                "ctrl+v"
            }
        }
        Err(_) => "ctrl+v",
    }
}

pub fn paste_text(text: &str, target_window: Option<isize>) -> Result<(), String> {
    // Set clipboard
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;

    // Restore target window focus and simulate paste
    #[cfg(windows)]
    if let Some(hwnd_val) = target_window {
        let hwnd = HWND(hwnd_val as *mut core::ffi::c_void);
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    #[cfg(target_os = "linux")]
    {
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        if session_type != "wayland" {
            if let Some(window_id) = target_window {
                let wid = window_id.to_string();
                let paste_key = detect_paste_key(&wid);

                let _ = std::process::Command::new("xdotool")
                    .args(["windowfocus", "--sync", &wid])
                    .status();
                let _ = std::process::Command::new("xdotool")
                    .args(["windowactivate", "--sync", &wid])
                    .status();
                std::thread::sleep(std::time::Duration::from_millis(150));

                std::process::Command::new("xdotool")
                    .args(["key", "--window", &wid, "--clearmodifiers", paste_key])
                    .status()
                    .map_err(|e| format!("xdotool key failed: {}", e))?;
            } else {
                let _ = std::process::Command::new("xdotool")
                    .args(["key", "--clearmodifiers", "ctrl+v"])
                    .status();
            }
        }
    }

    // Simulate Ctrl+V on Windows/macOS
    #[cfg(not(target_os = "linux"))]
    {
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
    }

    Ok(())
}
