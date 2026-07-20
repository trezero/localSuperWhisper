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

#[cfg(target_os = "macos")]
pub fn capture_foreground_window() -> Option<isize> {
    // Get the PID of the frontmost application via AppleScript
    let output = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get unix id of first process whose frontmost is true"])
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

    #[cfg(target_os = "macos")]
    {
        // Bail out before posting events that macOS would silently discard.
        // Without Accessibility, CGEventPost succeeds and returns no error, the
        // keystroke simply never arrives — which looks exactly like "paste is
        // broken" even though the clipboard was set correctly.
        if !crate::permissions::accessibility_trusted() {
            return Err(
                "macOS is not allowing this app to press Cmd+V. The text is on your \
                 clipboard — press Cmd+V to paste it. To fix this permanently, open \
                 System Settings > Privacy & Security > Accessibility. If Local \
                 SuperWhisper is already ticked there, remove it with the minus \
                 button and add it back: macOS ties the permission to one exact \
                 build, so installing a new version silently invalidates it."
                    .into(),
            );
        }

        // Restore focus to the target app by PID
        if let Some(pid) = target_window {
            let script = format!(
                "tell application \"System Events\" to set frontmost of first process whose unix id is {} to true",
                pid
            );
            let _ = std::process::Command::new("osascript")
                .args(["-e", &script])
                .status();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Simulate Cmd+V (macOS paste).
        //
        // Use the raw virtual keycode rather than Key::Unicode('v'). enigo maps
        // Key::Unicode through get_layoutdependent_keycode(), which brute-forces
        // keycodes 0..128 through the Carbon/HIToolbox Text Input Source APIs
        // (TISCopyCurrentKeyboardInputSource / TISGetInputSourceProperty). Those
        // assert they are on the main dispatch queue, and paste_text() runs on a
        // tokio worker — so that path SIGTRAPs the moment a transcription lands.
        // Key::Other is a straight cast to CGKeyCode and touches no TSM API.
        //
        // kVK_ANSI_V is also the *correct* choice for a Cmd shortcut: macOS
        // resolves Cmd-key shortcuts by physical key position (cf. Apple's
        // "Dvorak - QWERTY ⌘" layout), which is what this constant encodes.
        const KVK_ANSI_V: u32 = 0x09;

        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo error: {}", e))?;
        enigo
            .key(Key::Meta, Direction::Press)
            .map_err(|e| format!("Key press error: {}", e))?;
        let click = enigo.key(Key::Other(KVK_ANSI_V), Direction::Click);
        // Release Cmd even if the click failed, or the modifier stays stuck down
        // system-wide.
        let release = enigo.key(Key::Meta, Direction::Release);
        click.map_err(|e| format!("Key click error: {}", e))?;
        release.map_err(|e| format!("Key release error: {}", e))?;
    }

    // Simulate Ctrl+V on Windows
    #[cfg(windows)]
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
