# Linux Port Plan — Local SuperWhisper on Ubuntu 22 Desktop

**Created:** 2026-04-04
**Goal:** Build and run Local SuperWhisper natively on Ubuntu 22.04 desktop, with new `manage.sh` menu options for Linux builds.

---

## Current State

The app is a Tauri v2 (Rust + React/TypeScript) desktop application that currently targets Windows 10. Platform-specific code is isolated behind `#[cfg(windows)]` gates in several Rust files. Most of the codebase (frontend, audio recording, transcription, database, sounds) is already cross-platform thanks to the libraries used (`cpal`, `rodio`, `rusqlite`, `reqwest`, `arboard`, `enigo`).

### Platform-Specific Code Inventory

| File | Windows-specific code | Linux action needed |
|------|----------------------|---------------------|
| `paste.rs` | `capture_foreground_window()` uses Win32 `GetForegroundWindow`/`SetForegroundWindow`. Non-windows stub returns `None`. | Implement X11/Wayland foreground window capture and restore |
| `win32_hotkey.rs` | Entire file — Win32 `RegisterHotKey` for bare modifier keys | Not needed on Linux; `tauri-plugin-global-shortcut` works for non-modifier keys. Drop modifier-only hotkey support on Linux or find alternative |
| `lib.rs` | `show_settings_window()` uses Win32 `ShowWindow`/`SetForegroundWindow`. Non-windows fallback already exists. | Existing fallback (`unminimize`/`show`/`set_focus`) should work on Linux |
| `lib.rs` | `#[cfg(windows)] mod win32_hotkey;` conditional module import | Already gated correctly |
| `lib.rs` | Win32 hotkey registration in `register_hotkey()` and `setup` | Already gated with `#[cfg(windows)]` — Linux path falls through to `tauri-plugin-global-shortcut` |
| `hotkey.rs` | `capture_foreground_window()` call gated with `#[cfg(windows)]` / `#[cfg(not(windows))]` | Needs Linux implementation to actually capture the window |
| `state.rs` | `raw_hotkey_thread_id` field gated with `#[cfg(windows)]` | Already handled |
| `Cargo.toml` | `[target.'cfg(windows)'.dependencies]` for `windows` crate | Already gated — won't compile on Linux |
| `tauri.conf.json` | Bundle targets set to `"all"`, icon list includes `.ico` | Add `.desktop` file, `.deb`/`.AppImage` config |
| `ecosystem.config.cjs` | Hardcoded `.exe` path | Needs Linux binary path |
| `manage.sh` | References `.exe`, uses `pm2-windows-startup` | Needs Linux-aware paths and systemd/autostart alternative |

---

## Phase 1: Rust Backend — Linux Platform Support

### 1.1 Create `paste_linux.rs` — foreground window capture + paste

The paste module needs a Linux implementation for two functions:
- **Capture foreground window** before recording starts (so we can refocus it after transcription)
- **Restore focus and paste** the transcribed text

**Approach — X11 (primary, Ubuntu 22 default is X11 under GNOME):**
- Use the `x11rb` crate (or shell out to `xdotool`) to get/set the focused window
- `xdotool getactivewindow` → capture window ID
- `xdotool windowactivate <id>` → restore focus
- Paste via `arboard` (clipboard) + `enigo` (Ctrl+V) — these already work on Linux

**Approach — Wayland fallback:**
- Wayland does not allow apps to read or set focus on other windows (security model)
- For Wayland: skip window capture/restore; just set clipboard and simulate Ctrl+V — the user's window will still be focused since the overlay is non-focusable
- Detect session type via `$XDG_SESSION_TYPE` environment variable

**Implementation plan for `paste.rs`:**
```rust
#[cfg(target_os = "linux")]
pub fn capture_foreground_window() -> Option<isize> {
    // Check XDG_SESSION_TYPE
    // If "x11": run xdotool getactivewindow, parse window ID
    // If "wayland": return None (can't capture focus)
}

#[cfg(target_os = "linux")]
fn restore_foreground_window(target: Option<isize>) {
    // If X11 and target is Some: xdotool windowactivate <id>
    // Brief sleep to let focus settle
}
```

Update `paste_text()` to call `restore_foreground_window()` on Linux instead of the Win32 path.

**Crate options:**
- `xdotool` via `std::process::Command` — simplest, requires `xdotool` package installed
- `x11rb` crate — pure Rust X11 bindings, no runtime dependency but more code

**Recommendation:** Use `xdotool` via `Command` for the initial port (simpler, Ubuntu has it in apt), document it as a runtime dependency. Can be replaced with `x11rb` later if desired.

### 1.2 Update `paste.rs` module structure

Refactor `paste.rs` to have a clean platform-dispatch pattern:

```rust
// paste.rs

#[cfg(windows)]
mod windows_impl;
#[cfg(target_os = "linux")]
mod linux_impl;

#[cfg(windows)]
pub use windows_impl::*;
#[cfg(target_os = "linux")]
pub use linux_impl::*;
```

Or keep it in one file with `#[cfg]` blocks (current pattern) — simpler for the small amount of code involved.

### 1.3 Update `lib.rs` — remove `#[cfg(windows)]` gate on `mod paste`

Currently `mod paste` is only compiled on Windows:
```rust
#[cfg(windows)]
mod paste;
```

Change to unconditional import since paste will now have Linux support:
```rust
mod paste;
```

Update `hotkey.rs` to always call `paste::capture_foreground_window()` and `paste::paste_text()` instead of gating with `#[cfg(windows)]`.

### 1.4 Audio — verify `cpal` works on Linux

`cpal` uses ALSA on Linux by default. Requirements:
- Install `libasound2-dev` (build-time) and `libasound2` (runtime)
- PulseAudio/PipeWire should work through ALSA compatibility layer
- No code changes expected — `cpal` abstracts this

### 1.5 Sound playback — verify `rodio` works on Linux

`rodio` uses `cpal` under the hood, so same ALSA dependency. Should work without code changes.

### 1.6 Clipboard + keyboard simulation

- `arboard` — uses X11/Wayland clipboard natively, should work
- `enigo` — uses X11 (`XTest` extension) for key simulation; on Ubuntu 22 with X11 this works. Wayland support in `enigo 0.2` is limited but may work under XWayland

### 1.7 Overlay window — transparent, always-on-top

Tauri's transparent + always-on-top window config should work on X11. Verify:
- Transparency requires a compositor (Ubuntu 22 GNOME has one by default)
- `alwaysOnTop` maps to X11 `_NET_WM_STATE_ABOVE`
- `skipTaskbar` maps to `_NET_WM_STATE_SKIP_TASKBAR`
- `decorations: false` should work

---

## Phase 2: Build Configuration

### 2.1 Tauri bundle config for Linux

Update `tauri.conf.json` bundle section:

```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico",
      "icons/icon.png"
    ],
    "resources": ["sounds/*"],
    "linux": {
      "deb": {
        "depends": ["libasound2", "libwebkit2gtk-4.1-0", "libgtk-3-0", "xdotool"],
        "desktopEntry": {
          "categories": "Utility;Audio;",
          "comment": "Voice-to-text transcription using local Whisper API",
          "startupNotify": true
        }
      },
      "appimage": {
        "bundleMediaFramework": true
      }
    }
  }
}
```

Tauri v2 on Linux produces `.deb` and `.AppImage` bundles by default when `targets` is `"all"`.

### 2.2 Linux system dependencies for building

Create a script or document the prerequisites:

```bash
# Ubuntu 22.04 build dependencies
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libasound2-dev \
  libssl-dev \
  pkg-config \
  xdotool
```

### 2.3 Icon files

Current icons are sufficient for Linux (PNG files). The `.ico` is Windows-only but Tauri ignores it on Linux. No changes needed.

---

## Phase 3: `manage.sh` Updates

### 3.1 Detect platform and set paths accordingly

```bash
# At the top of manage.sh
OS="$(uname -s)"
case "$OS" in
  Linux*)
    PLATFORM="linux"
    EXE="$SCRIPT_DIR/src-tauri/target/release/local-super-whisper"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    PLATFORM="windows"
    EXE="$SCRIPT_DIR/src-tauri/target/release/local-super-whisper.exe"
    ;;
  *)
    error "Unsupported platform: $OS"
    exit 1
    ;;
esac
```

### 3.2 Add Linux-specific menu options

Replace the Windows-only startup options with platform-aware versions:

| Option | Windows | Linux |
|--------|---------|-------|
| 7) Enable Startup | `pm2-windows-startup` | Create `~/.config/autostart/local-super-whisper.desktop` |
| 8) Disable Startup | `pm2-startup uninstall` | Remove the `.desktop` file |
| 9) Install Linux deps | N/A (new) | Run `apt install` for build dependencies |

### 3.3 Linux autostart via `.desktop` file

```bash
cmd_setup_startup_linux() {
  header "Configure Linux Autostart"
  
  AUTOSTART_DIR="$HOME/.config/autostart"
  mkdir -p "$AUTOSTART_DIR"
  
  cat > "$AUTOSTART_DIR/local-super-whisper.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Local SuperWhisper
Exec=$EXE
Icon=$SCRIPT_DIR/src-tauri/icons/icon.png
Comment=Voice-to-text transcription
Categories=Utility;Audio;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

  success "Autostart enabled. App will launch on next login."
}

cmd_remove_startup_linux() {
  header "Remove Linux Autostart"
  rm -f "$HOME/.config/autostart/local-super-whisper.desktop"
  success "Autostart disabled."
}
```

### 3.4 Linux dependency installer menu option

```bash
cmd_install_linux_deps() {
  header "Install Linux Build Dependencies"
  info "This requires sudo access."
  
  sudo apt update
  sudo apt install -y \
    build-essential curl wget \
    libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev \
    libasound2-dev libssl-dev pkg-config xdotool
  
  success "Dependencies installed."
}
```

### 3.5 Update `ecosystem.config.cjs` for Linux

Make the PM2 config platform-aware:

```javascript
const isWindows = process.platform === "win32";
const EXE = path.join(
  PROJECT_ROOT,
  "src-tauri",
  "target",
  "release",
  isWindows ? "local-super-whisper.exe" : "local-super-whisper"
);
```

---

## Phase 4: Testing & Validation

### 4.1 Test matrix

| Test | How |
|------|-----|
| App launches, tray icon appears | Run on Ubuntu 22 desktop |
| Settings window opens from tray | Click tray icon or right-click → Open Settings |
| Hotkey registration | Set F10 via setup screen, verify it triggers recording |
| Audio recording | Speak into mic, verify waveform shows in overlay |
| Transcription | Verify text comes back from Faster-Whisper API |
| Paste into target window | Verify transcribed text is pasted into the previously-focused app |
| Overlay transparency | Verify overlay has transparent background |
| Overlay auto-hide | Verify overlay hides after showing result |
| Sound playback | Verify start/stop/error sounds play |
| `manage.sh` build | Run option 5, verify `.deb` and/or binary are produced |
| `manage.sh` startup | Run option 7, verify `.desktop` file is created and works on login |
| PM2 process management | Start/stop/restart/status via manage.sh |

### 4.2 Known risks

| Risk | Mitigation |
|------|-----------|
| `enigo` Ctrl+V may not work on Wayland | Test on X11 first (Ubuntu 22 default); Wayland support is stretch goal |
| Transparent overlay may not render on some compositors | Test with GNOME (Mutter) — should work; document as requirement |
| `xdotool` not available | List as runtime dependency in `.deb` package, check at app startup |
| PulseAudio vs PipeWire audio differences | `cpal` ALSA backend should work with both via compatibility layer |
| `tauri-plugin-global-shortcut` may not work on all Linux DEs | F-keys should work; modifier-only keys won't work (same as current limitation) |

---

## Phase 5: Updated Menu Layout for `manage.sh`

```
╔══════════════════════════════════════╗
║   Local SuperWhisper — Manager       ║
╚══════════════════════════════════════╝

  1) Start
  2) Stop
  3) Restart
  4) View Logs
  5) Redeploy  (build + restart)
  6) Status
  7) Enable Startup on Login
  8) Disable Startup on Login
  9) Install Build Dependencies  [Linux only]
  0) Exit
```

Options 7 and 8 will automatically use the correct method for the detected platform (pm2-windows-startup on Windows, `.desktop` file on Linux). Option 9 only appears/works on Linux.

---

## Implementation Order

1. **Phase 2.2** — Install Linux build dependencies on the Ubuntu machine
2. **Phase 1.3** — Remove `#[cfg(windows)]` gate on `mod paste`, make paste module cross-platform
3. **Phase 1.1** — Implement `capture_foreground_window()` and `restore_foreground_window()` for Linux
4. **Phase 1.4–1.6** — Verify audio, sound, clipboard, keyboard simulation compile and work
5. **Phase 2.1** — Update `tauri.conf.json` for Linux bundles
6. **Phase 3** — Update `manage.sh` and `ecosystem.config.cjs` for Linux
7. **Phase 4** — Test on Ubuntu 22 desktop
8. Iterate on any issues found

---

## Out of Scope (for now)

- Wayland-native paste (requires different approach — `wl-copy`/`wtype` instead of `xdotool`/`enigo`)
- Flatpak/Snap packaging
- macOS support
- CI/CD pipeline for cross-platform builds
- Bare modifier key hotkeys on Linux (Win32 `RegisterHotKey` equivalent doesn't exist cleanly on X11)
