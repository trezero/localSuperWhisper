//! OS-level permission preflight.
//!
//! macOS grants (and silently withholds) several things this app needs. The
//! failure modes are quiet rather than loud — a missing microphone grant does
//! not fail the audio stream, it just yields zero-filled buffers, which the
//! transcription API happily turns into a plausible-looking hallucination. So
//! these checks are deliberately empirical where they can be: we look at what
//! the OS actually gives us rather than at what it claims to allow.

use serde::Serialize;

/// How long to listen when probing the microphone. Long enough to cover a few
/// CoreAudio buffers, short enough not to feel like a hang.
const MIC_PROBE_MS: u64 = 400;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    Denied,
    /// Checked, but the result is not conclusive on this platform.
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
pub struct PermissionCheck {
    pub id: String,
    pub label: String,
    /// What the app needs it for, in the user's terms.
    pub description: String,
    pub state: PermissionState,
    /// What actually happened, shown when a check fails.
    pub detail: Option<String>,
    /// Whether transcription is broken without it.
    pub required: bool,
    /// Deep link that opens the exact settings pane, when the OS has one.
    pub settings_url: Option<String>,
}

impl PermissionCheck {
    fn new(id: &str, label: &str, description: &str, required: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: description.into(),
            state: PermissionState::Unknown,
            detail: None,
            required,
            settings_url: None,
        }
    }

    fn with(mut self, state: PermissionState, detail: Option<String>) -> Self {
        self.state = state;
        self.detail = detail;
        self
    }

    fn url(mut self, url: &str) -> Self {
        self.settings_url = Some(url.into());
        self
    }
}

/// Probe the microphone by actually recording from it.
///
/// This is the check that matters. An authorization-status API would have
/// reported "authorized" in the case that shipped silent audio for every
/// transcription, because the grant was never requested at all.
fn check_microphone(device: &str) -> PermissionCheck {
    let check = PermissionCheck::new(
        "microphone",
        "Microphone",
        "Records your voice so it can be transcribed.",
        true,
    );

    #[cfg(target_os = "macos")]
    let check = check.url("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone");

    match crate::audio::probe_input_peak(device, MIC_PROBE_MS) {
        Ok(peak) if peak > 0.0 => check.with(
            PermissionState::Granted,
            Some(format!("Input detected (peak {:.4}).", peak)),
        ),
        Ok(_) => check.with(
            PermissionState::Denied,
            Some(
                "The microphone returned pure silence. The OS is most likely \
                 withholding microphone access, or the input device is muted."
                    .into(),
            ),
        ),
        Err(e) => check.with(
            PermissionState::Denied,
            Some(format!("Could not open the microphone: {}", e)),
        ),
    }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Whether macOS currently trusts this process to post synthetic input.
///
/// This flips to false on its own whenever the app is rebuilt without a stable
/// signing identity: TCC pins an ad-hoc signed app by cdhash, and every build
/// produces a new one. The grant still *looks* enabled in System Settings, but
/// synthetic key events are silently dropped.
#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(target_os = "macos")]
fn check_accessibility() -> PermissionCheck {
    let trusted = accessibility_trusted();
    PermissionCheck::new(
        "accessibility",
        "Accessibility",
        "Lets the app press Cmd+V to paste your transcription into the app you were using.",
        true,
    )
    .url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
    .with(
        if trusted { PermissionState::Granted } else { PermissionState::Denied },
        if trusted {
            None
        } else {
            Some(
                "Without this, transcription succeeds but nothing gets pasted — the \
                 text is copied to the clipboard and you have to press Cmd+V yourself. \
                 If this app is already ticked in the list, the grant has gone stale: \
                 macOS pins unsigned apps to an exact build, and installing a new \
                 version invalidates it. Remove the app from the list with the minus \
                 button, then add it again."
                    .into(),
            )
        },
    )
}

/// Sending Apple events is how the app finds and re-focuses the window you were
/// typing in. Probing it can surface the consent dialog, which is what we want
/// during setup.
#[cfg(target_os = "macos")]
fn check_automation() -> PermissionCheck {
    let ok = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get name of first process"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    PermissionCheck::new(
        "automation",
        "Automation (System Events)",
        "Returns focus to the app you were typing in before pasting.",
        false,
    )
    .url("x-apple.systempreferences:com.apple.preference.security?Privacy_Automation")
    .with(
        if ok { PermissionState::Granted } else { PermissionState::Denied },
        if ok {
            None
        } else {
            Some(
                "Without this, text is still pasted, but it may land in the wrong \
                 window if focus moved."
                    .into(),
            )
        },
    )
}

/// Run every check for the current platform.
///
/// Only macOS has OS-level grants implemented today. Windows and Linux get the
/// microphone probe — which is plain cpal and platform-neutral — and nothing
/// else, rather than checks that have not been verified on those systems.
pub fn run_checks(device: &str) -> Vec<PermissionCheck> {
    let mut checks = vec![check_microphone(device)];

    #[cfg(target_os = "macos")]
    {
        checks.push(check_accessibility());
        checks.push(check_automation());
    }

    checks
}

#[tauri::command]
pub async fn check_permissions(device: Option<String>) -> Result<Vec<PermissionCheck>, String> {
    let device = device.unwrap_or_else(|| "default".to_string());
    // The mic probe sleeps; keep it off the main thread so the UI stays live.
    tauri::async_runtime::spawn_blocking(move || run_checks(&device))
        .await
        .map_err(|e| format!("Permission check failed: {}", e))
}

/// Open the OS settings page for a given check id.
#[tauri::command]
pub fn open_permission_settings(url: String) -> Result<(), String> {
    // Only ever hand the OS a URL we produced ourselves.
    let known = [
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation",
    ];
    if !known.contains(&url.as_str()) {
        return Err("Unrecognised settings URL".into());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .status()
            .map_err(|e| format!("Could not open settings: {}", e))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err("No settings deep link on this platform".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microphone_check_is_required_on_every_platform() {
        let checks = run_checks("definitely-not-a-real-device");
        let mic = checks.iter().find(|c| c.id == "microphone").unwrap();
        assert!(mic.required);
    }

    #[test]
    fn unopenable_device_reports_denied_not_granted() {
        // A device that cannot be opened must never read as Granted, or the
        // preflight would wave through a broken setup.
        let checks = run_checks("definitely-not-a-real-device");
        let mic = checks.iter().find(|c| c.id == "microphone").unwrap();
        assert_eq!(mic.state, PermissionState::Denied);
        assert!(mic.detail.is_some());
    }

    #[test]
    fn rejects_urls_we_did_not_generate() {
        assert!(open_permission_settings("file:///etc/passwd".into()).is_err());
        assert!(open_permission_settings("".into()).is_err());
    }
}
