use crate::hotkey;
use std::sync::mpsc;
use tauri::AppHandle;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PostThreadMessageW, WM_HOTKEY, WM_QUIT,
};

const HOTKEY_REG_ID: i32 = 9001;
const MOD_NOREPEAT: u32 = 0x4000;

/// Returns the Win32 virtual key code for bare modifier keys that
/// can't be registered via tauri-plugin-global-shortcut.
pub fn win32_vk_for_key(code: &str) -> Option<u32> {
    match code {
        "AltRight"     => Some(0xA5), // VK_RMENU
        "AltLeft"      => Some(0xA4), // VK_LMENU
        "ControlRight" => Some(0xA3), // VK_RCONTROL
        "ControlLeft"  => Some(0xA2), // VK_LCONTROL
        "ShiftRight"   => Some(0xA1), // VK_RSHIFT
        "ShiftLeft"    => Some(0xA0), // VK_LSHIFT
        _ => None,
    }
}

/// Spawn a dedicated Win32 RegisterHotKey thread.
/// Returns the Windows thread ID, which is needed to stop the thread via `stop()`.
pub fn start(vk: u32, app_handle: AppHandle) -> Result<u32, String> {
    let (tx, rx) = mpsc::channel::<Result<u32, String>>();

    std::thread::spawn(move || unsafe {
        let thread_id = GetCurrentThreadId();

        if let Err(e) = RegisterHotKey(None, HOTKEY_REG_ID, HOT_KEY_MODIFIERS(MOD_NOREPEAT), vk) {
            let _ = tx.send(Err(format!("RegisterHotKey failed: {}", e)));
            return;
        }

        // Send thread ID back so the caller can stop this thread later
        let _ = tx.send(Ok(thread_id));

        let mut msg = MSG::default();
        loop {
            // GetMessageW returns BOOL: .0 is i32, 0 = WM_QUIT, -1 = error, >0 = message
            if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                break;
            }
            if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == HOTKEY_REG_ID {
                hotkey::on_hotkey_pressed(&app_handle);
            }
        }

        let _ = UnregisterHotKey(None, HOTKEY_REG_ID);
    });

    rx.recv().map_err(|e| e.to_string())?
}

/// Stop the Win32 hotkey thread by posting WM_QUIT to its message queue.
pub fn stop(thread_id: u32) {
    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}
