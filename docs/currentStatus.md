# Local SuperWhisper — Current Status

Last updated: 2026-07-19 (merged upstream main into macOS port)

---

## What This App Does

Local SuperWhisper is a Tauri (Rust + React/TypeScript) desktop app that:
1. Listens for a global hotkey press
2. Records audio from the microphone
3. Sends the audio to a self-hosted Faster-Whisper API for transcription
4. Pastes the transcribed text into whatever window was previously focused

It lives in the system tray and has a settings UI with four tabs: Home, Vocabulary, Configuration, History.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri v2 |
| Frontend | React 18 + TypeScript + Vite |
| Styling | Tailwind CSS (custom theme: `surface`, `accent`, `text-primary`, etc.) |
| Backend | Rust (Tauri commands) |
| Database | SQLite via `rusqlite` |
| Global hotkeys | `tauri-plugin-global-shortcut` v2 |
| Audio recording | `cpal` crate |
| Transcription | HTTP POST to Faster-Whisper OpenAI-compatible API |

---

## Project Structure

```
localSuperWhisper/
├── src/                          # React frontend
│   ├── App.tsx                   # Root router with first-run setup logic
│   ├── main.tsx                  # Vite entry point
│   ├── settings/
│   │   ├── Layout.tsx            # Sidebar nav shell
│   │   ├── Home.tsx              # Stats + checklist + recent history
│   │   ├── Configuration.tsx     # Settings form (hotkey, API, mic)
│   │   ├── Vocabulary.tsx        # Custom vocabulary word list
│   │   ├── History.tsx           # Transcription history table
│   │   └── Setup.tsx             # First-run hotkey setup screen
│   ├── overlay/
│   │   ├── Overlay.tsx           # Transparent recording overlay
│   │   ├── Waveform.tsx          # Animated audio level bars
│   │   └── TranscriptDisplay.tsx # Shows transcribed text after recording
│   ├── components/
│   │   ├── StatCard.tsx          # Reusable stat display card
│   │   └── ChecklistItem.tsx     # Onboarding checklist item
│   └── hooks/
│       └── useTauriEvent.ts      # Hook for listening to Tauri events
├── src-tauri/src/
│   ├── lib.rs                    # App setup, Tauri commands, tray, window management
│   ├── hotkey.rs                 # Hotkey handler (start/stop recording state machine)
│   ├── audio.rs                  # cpal audio recording + device listing
│   ├── transcribe.rs             # HTTP client for Faster-Whisper API
│   ├── db.rs                     # SQLite schema, CRUD, settings
│   ├── paste.rs                  # Windows clipboard paste (Win32 API)
│   ├── sounds.rs                 # Startup/stop/error sound playback
│   ├── state.rs                  # AppState struct (recording state, recorder, db, target window)
│   └── main.rs                   # Entry point
└── src-tauri/tauri.conf.json     # Two windows: "settings" and "overlay"
```

---

## Database Schema (SQLite)

**settings** — key/value store
| key | default | notes |
|-----|---------|-------|
| hotkey | `""` | Empty = not configured; triggers setup screen |
| api_url | `http://172.16.1.222:8028/v1` | Faster-Whisper server |
| api_key | `cant-be-empty` | Auth header |
| model_id | `deepdml/faster-whisper-large-v3-turbo-ct2` | Model name |
| mic_device | `default` | Mic device name or "default" |
| typing_speed_wpm | `40` | Used for time-saved calculation |

**history** — transcription log (capped at 500 entries)
**vocabulary** — custom words sent as hints to Whisper API
**checklist** — onboarding steps (start_recording, customize_shortcuts, add_vocabulary, configure_api)

---

## Tauri Windows

| Label | URL | Visible on start | Notes |
|-------|-----|-----------------|-------|
| settings | `index.html#/settings` | **true** (dev) / false (prod) | Changed to true for WSL2 dev |
| overlay | `index.html#/overlay` | false | Transparent, always-on-top, no decorations |

> **Status (2026-04-03):** Reverted to `visible: false` for Windows native development. The tray icon works on Windows — right-click it to open the Settings window.

---

## First-Run Setup Flow

**Problem being solved:** `tauri-plugin-global-shortcut` rejects keys like `"AltRight"` on Linux. Previously the app had `"AltRight"` hardcoded as the default hotkey, causing a registration error on startup.

**Solution implemented:**
1. Default hotkey in DB is now `""` (empty string)
2. At startup (`lib.rs`), if stored hotkey fails to register, it is **cleared to `""`** in the DB
3. Frontend (`App.tsx`) fetches settings on load; if `hotkey == ""`, renders `<Setup />` instead of the normal app
4. `Setup.tsx` shows a "Choose Hotkey" button → listens for a keypress (`event.code`) → saves and registers it live
5. On success, calls `onDone()` (flips `needsSetup` to false in `App.tsx`) then navigates to `/settings`

**Key constraint:** Modifier-only keys (`AltRight`, `ControlLeft`, etc.) may not work with `tauri-plugin-global-shortcut`. Recommended keys: F9–F12 and other non-modifier keys.

---

## Tauri Commands (Rust → Frontend)

| Command | Description |
|---------|-------------|
| `get_stats` | Avg WPM, words this week, time saved |
| `get_history(limit)` | Recent transcriptions |
| `get_vocabulary` | Custom word list |
| `add_vocabulary_term(term)` | Add word |
| `remove_vocabulary_term(id)` | Remove word |
| `get_settings` | All settings as `[(key, value)]` |
| `update_setting(key, value)` | Save a single setting |
| `get_checklist` | Onboarding step states |
| `complete_checklist_step(step_id)` | Mark step done |
| `get_audio_devices` | List input devices |
| `register_hotkey(key)` | Unregister all + register new hotkey live |

## Tauri Events (Rust → Frontend via `emit`)

| Event | Payload | Description |
|-------|---------|-------------|
| `recording-started` | — | Hotkey pressed, recording began |
| `recording-transcribing` | — | Audio sent to API |
| `recording-result` | `String` | Transcription text |
| `recording-idle` | — | Back to idle state |
| `recording-error` | `String` | Error message |
| `audio-level` | `f32` | Current mic level (0.0–1.0), polled every 50ms during recording |

---

## Linux Support (added 2026-04-04)

The app now builds and runs on Ubuntu 22.04 (X11) in addition to Windows 10.

### What was done
- **paste.rs**: Added `#[cfg(target_os = "linux")]` implementations of `capture_foreground_window()` (via `xdotool getactivewindow`) and window restore (via `xdotool windowactivate`). Wayland sessions gracefully skip window capture/restore.
- **lib.rs**: `mod paste` is now unconditional (was `#[cfg(windows)]`).
- **hotkey.rs**: `paste::capture_foreground_window()` and `paste::paste_text()` calls are now unconditional (were gated behind `#[cfg(windows)]`).
- **tauri.conf.json**: Added `bundle.linux` section with deb depends, desktop template, and appimage config.
- **manage.sh**: Added platform detection (`uname -s`), Linux autostart via `~/.config/autostart/*.desktop`, and option 9 "Install Build Dependencies" (Linux only).
- **ecosystem.config.cjs**: EXE path is now platform-aware (`process.platform`).
- **desktop-template.hbs**: Handlebars template for `.desktop` file in deb package.

### Linux build dependencies
```bash
sudo apt install -y build-essential libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libasound2-dev libssl-dev \
  pkg-config xdotool libxdo-dev
```

### Linux build artifacts
- Binary: `src-tauri/target/release/local-super-whisper` (19MB)
- Deb: `src-tauri/target/release/bundle/deb/Local SuperWhisper_0.1.0_amd64.deb`
- RPM: `src-tauri/target/release/bundle/rpm/Local SuperWhisper-0.1.0-1.x86_64.rpm`
- AppImage: requires desktop session to build (linuxdeploy needs DISPLAY)

### Linux known limitations
- **Wayland**: Window capture/restore is skipped — clipboard paste still works but can't auto-focus the target window
- **AppImage bundling**: Fails in headless environments; works from desktop terminal
- **Modifier-only hotkeys**: Not supported on Linux (same as current limitation, use F9–F12)

---

## macOS Support (added 2026-04-04)

The app now builds and runs on macOS (Apple Silicon and Intel) in addition to Windows 10 and Linux.

### What was done
- **paste.rs**: Added `#[cfg(target_os = "macos")]` implementation of `capture_foreground_window()` using AppleScript (`osascript`) to get the frontmost app's PID. Added macOS-specific window restore via AppleScript and paste simulation using `Cmd+V` (`Key::Meta`) instead of `Ctrl+V`.
- **sounds.rs**: macOS uses `afplay` (built-in) for sound playback instead of `rodio`, matching the Linux approach of using native CLI audio tools for reliability.
- **tauri.conf.json**: Added `macOS` bundle section (`minimumSystemVersion: "10.15"`). Added `icon.icns` to bundle icon list.
- **icons/icon.icns**: Generated from existing `icon.png` via `sips`/`iconutil`.
- **manage.sh**: Added `Darwin` platform detection, macOS autostart via LaunchAgent (`~/Library/LaunchAgents/`), and macOS build dependencies option (Homebrew).

### macOS build requirements
- Xcode Command Line Tools: `xcode-select --install`
- Rust: `rustup` (not Homebrew `rust` — ensure `~/.cargo/bin` is in PATH)
- Node.js

### macOS permission preflight (added 2026-07-19)

macOS withholds permissions *quietly*. A missing microphone grant does not fail
the audio stream — CoreAudio returns zero-filled buffers, so recordings come out
the right length and completely silent, which Whisper turns into a confident
hallucination ("Thank you." every time). That cost a debugging session, so the
app now checks up front and refuses to guess.

- **`permissions.rs`** — `check_permissions` command returns a list of
  `PermissionCheck { id, label, description, state, detail, required, settings_url }`.
  macOS checks: Microphone, Accessibility (`AXIsProcessTrusted`), Automation
  (osascript probe against System Events). `open_permission_settings` deep-links
  to the exact pane and only accepts URLs the app itself generated.
- **The microphone check is empirical, deliberately.** It opens the input for
  400 ms and looks at the samples. An authorization-status API would have
  reported "authorized" for the bug above, because the grant was never requested
  at all. Real hardware always has a nonzero noise floor (measured 0.009 in a
  quiet room), so an exact peak of 0.0 is a reliable signal rather than a
  threshold guess.
- **Preflight gate** — `hotkey.rs` refuses to transcribe a recording whose peak
  is exactly 0.0 and emits an actionable `recording-error` instead of spending a
  round trip to get a hallucination back.
- **UI** — `src/settings/Permissions.tsx`, shown as the first step of the
  first-run `Setup` flow with live status, per-item deep links, and Re-check.
  A "Skip for now" escape hatch avoids trapping anyone.

Windows and Linux get the microphone probe only (it is plain cpal and
platform-neutral); the macOS-specific grants are `cfg`-gated. Windows/Linux
checks were deliberately **not** implemented, since they could not be verified
from this machine.

### macOS accessibility permissions
- **Required**: macOS requires Accessibility permissions for `enigo` to simulate Cmd+V paste. On first run, the OS will prompt to grant access in System Settings > Privacy & Security > Accessibility.
- Without this permission, transcription will succeed but auto-paste into the target window will fail silently.

### macOS build artifacts
- `.app` bundle: `src-tauri/target/release/bundle/macos/Local SuperWhisper.app`
- `.dmg` installer: `src-tauri/target/release/bundle/dmg/Local SuperWhisper_0.1.2_aarch64.dmg` (or `x64`)
- Verified built on 2026-07-19 (aarch64, 8.3 MB dmg, sounds correctly bundled into `Contents/Resources/sounds/`)

> The executable **inside** the `.app` is named `local-super-whisper` (the cargo
> bin name), *not* `Local SuperWhisper` (productName). `manage.sh` depends on
> this path.

### Bundle targets — cross-platform gotcha

`tauri.conf.json` has `bundle.targets: ["nsis"]` so Windows releases produce a
single signed NSIS installer. Tauri v2 has **no per-platform `targets` key** —
that list is global, so leaving it alone would mean macOS and Linux builds
produce no installable artifact at all.

`manage.sh` resolves this with a per-platform `BUNDLES` variable passed to
`tauri build --bundles`:

| Platform | `BUNDLES` | Notes |
|----------|-----------|-------|
| macOS | `app,dmg` | |
| Linux | `deb,rpm` | `appimage` omitted — linuxdeploy needs a desktop session |
| Windows | *(empty)* | falls through to `bundle.targets` in the config (signed nsis) |

If you build by hand instead of via `manage.sh`, pass `--bundles` yourself on
macOS/Linux or you will get no bundle.

### macOS known limitations
- **Accessibility permissions**: Must be granted manually for paste simulation to work
- **Modifier-only hotkeys**: Not supported (same as other platforms — use F9–F12)
- **Rust toolchain**: Must use `rustup`-managed Rust (1.88+), not Homebrew `rust` which may be too old.
  Confirmed 2026-07-19: Homebrew installs `rustc` into `/opt/homebrew/bin`, which shadows
  `~/.cargo/bin` on a default PATH. `cargo check` then fails outright ("requires rustc 1.88").
  Put `~/.cargo/bin` **ahead** of `/opt/homebrew/bin` in your shell profile.
- **pm2 not installed**: `manage.sh` options 1–6 (start/stop/restart/logs/redeploy/status) all
  call `check_pm2` and abort. Building works, but `redeploy` cannot run until `npm install -g pm2`.
  The pm2-based process management is Windows-oriented; macOS autostart uses the LaunchAgent instead.

---

## Known Issues / Next Steps

### Unresolved
- **Hotkey key compatibility**: Not all keys work with `tauri-plugin-global-shortcut` on all platforms. F-keys (F9–F12) are the most reliable. The setup screen currently accepts any key and shows an error if registration fails — user must try a different key.
- **WSL2 tray icon**: System tray icon doesn't appear when running via WSLg. Settings window is set to `visible: true` as a workaround for dev. This needs to be reverted to `false` for production builds.

### Recently Fixed
- **Multiple zombie instances (2026-05-18)**: App would accumulate multiple `local-super-whisper.exe` processes over time (one had been running since May 14). Each new instance failed to register the hotkey (previous instance held it), then cleared `hotkey = ""` from the DB, leaving no instance with a working hotkey or tray response. Fixed by adding `tauri-plugin-single-instance` — second launches now focus the running instance and exit immediately. Users who hit this before the fix must re-enter their hotkey in the setup screen on first launch.

### Ready to work on
- Test macOS build: tray icon, overlay, audio recording, paste with accessibility permissions
- Test the Linux build on an Ubuntu 22 desktop with display (tray icon, overlay transparency, audio recording, paste)
- Test on Windows native to confirm no regressions from the Linux/macOS ports
- Revert `settings` window `visible` to `false` before building for production
- Complete the onboarding checklist UX (checklist steps aren't being auto-completed yet)
- The `customize_shortcuts` checklist step should auto-complete after the user sets a hotkey in Setup

---

## Running the App

### Linux (Ubuntu 22.04)
1. Install dependencies: `./manage.sh` → option 9, or run `sudo apt install ...` (see above)
2. Install [Rust](https://rustup.rs) and Node.js
3. `npm install`
4. `npm run tauri -- dev`    ← dev mode with hot reload
5. `npm run tauri -- build`  ← produces binary + `.deb` + `.rpm` in `src-tauri/target/release/bundle/`

### macOS
1. Install Xcode Command Line Tools: `xcode-select --install`
2. Install [Rust](https://rustup.rs) and Node.js (ensure `~/.cargo/bin` is in PATH)
3. `npm install`
4. `npm run tauri -- dev`    ← dev mode with hot reload
5. `npm run tauri -- build`  ← produces `.app` + `.dmg` in `src-tauri/target/release/bundle/`
6. Grant Accessibility permissions when prompted (required for paste simulation)

### Windows native
1. Install [Rust](https://rustup.rs) and Node.js on Windows
2. Clone the repo on Windows
3. `npm install`
4. `npm run tauri -- dev`    ← dev mode with hot reload
5. `npm run tauri -- build`  ← produces `.msi` installer in `src-tauri/target/release/bundle/`

### WSL2 (development only)
```bash
npm run tauri -- dev
```
Window opens automatically (WSLg renders it). Tray icon won't appear — this is a WSLg limitation.
