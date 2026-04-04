# Local SuperWhisper — Current Status

Last updated: 2026-04-04 (Linux port session)

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

## Known Issues / Next Steps

### Unresolved
- **Hotkey key compatibility**: Not all keys work with `tauri-plugin-global-shortcut` on all platforms. F-keys (F9–F12) are the most reliable. The setup screen currently accepts any key and shows an error if registration fails — user must try a different key.
- **WSL2 tray icon**: System tray icon doesn't appear when running via WSLg. Settings window is set to `visible: true` as a workaround for dev. This needs to be reverted to `false` for production builds.

### Ready to work on
- Test the Linux build on an Ubuntu 22 desktop with display (tray icon, overlay transparency, audio recording, paste)
- Test on Windows native to confirm no regressions from the Linux port
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
