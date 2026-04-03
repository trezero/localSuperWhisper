# Local SuperWhisper

A lightweight Windows 10/11 desktop app that replicates the core [Superwhisper](https://superwhisper.com) workflow using a **self-hosted** Faster-Whisper backend. Press a hotkey, dictate, and the transcribed text is automatically pasted into whatever window you were using — no cloud, no subscription.

Built with [Tauri v2](https://tauri.app) (Rust backend) and React + TypeScript + Tailwind CSS (frontend).

---

## How It Works

1. Press your configured hotkey → recording starts, an overlay appears
2. Speak into your microphone
3. Press the hotkey again → audio is sent to your local Faster-Whisper API
4. Transcribed text is pasted into the previously focused window

The app lives in the system tray and stays out of your way until you need it.

---

## Prerequisites

### Windows (required for production use)

- [Rust](https://rustup.rs) (stable toolchain)
- [Node.js](https://nodejs.org) (v18 or later)
- A running [Faster-Whisper](https://github.com/SYSTRAN/faster-whisper) server exposing an OpenAI-compatible `/v1/audio/transcriptions` endpoint

### Faster-Whisper Server

The app sends audio to an HTTP API that is compatible with the OpenAI audio transcription format. A popular option is [faster-whisper-server](https://github.com/fedirz/faster-whisper-server):

```bash
docker run --gpus all -p 8028:8000 fedirz/faster-whisper-server:latest-cuda
```

The default API URL in the app is `http://172.16.1.222:8028/v1` — change this in the Configuration tab to match your server.

---

## Getting Started

### Install dependencies

```bash
npm install
```

### Development (with hot reload)

```bash
npm run tauri -- dev
```

### Production build

```bash
npm run tauri -- build
```

This produces an `.msi` installer at:

```
src-tauri/target/release/bundle/msi/
```

### First run

On first launch, the app detects that no hotkey has been configured and shows a setup screen. Click **Choose Hotkey**, press any key (F9–F12 are most reliable), and the app will save and register it immediately.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Tauri Process                   │
│                                                  │
│  ┌──────────────┐       ┌─────────────────────┐  │
│  │ Overlay Win  │       │   Settings Window   │  │
│  │ (transparent,│       │   (normal window,   │  │
│  │  always-top) │       │    opened from tray)│  │
│  └──────┬───────┘       └──────────┬──────────┘  │
│         │    React + Tailwind      │             │
│         └──────────┬───────────────┘             │
│                    │ Tauri IPC (invoke/emit)      │
│  ┌─────────────────┴────────────────────────┐    │
│  │            Rust Backend                   │    │
│  │                                           │    │
│  │  ┌───────────┐  ┌──────────┐  ┌────────┐ │    │
│  │  │ Audio     │  │ Whisper  │  │Keyboard│ │    │
│  │  │ (cpal)    │  │ API      │  │Emulate │ │    │
│  │  └───────────┘  │(reqwest) │  │(enigo) │ │    │
│  │  ┌───────────┐  └──────────┘  └────────┘ │    │
│  │  │ Sound FX  │  ┌──────────┐  ┌────────┐ │    │
│  │  │ (rodio)   │  │ SQLite   │  │ Tray   │ │    │
│  │  └───────────┘  │(rusqlite)│  │ Icon   │ │    │
│  │                 └──────────┘  └────────┘ │    │
│  └──────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

Both windows share a single React build, routed by URL (`/overlay` vs `/settings`). All heavy work (audio capture, API calls, clipboard, paste) runs in Rust — the frontend is purely presentational.

### Recording State Machine

```
              hotkey press
┌──────┐    ─────────────►  ┌───────────┐
│ Idle │                    │ Recording │
└──────┘  ◄───────────────  └─────┬─────┘
    ▲        (auto-dismiss)        │ hotkey press
    │                              ▼
┌───┴──────┐              ┌──────────────┐
│Displaying│  ◄─────────  │ Transcribing │
│ Result   │              └──────────────┘
└──────────┘
     │ 2–3 sec timeout
     ▼
   Idle
```

---

## Project Structure

```
localSuperWhisper/
├── src/                          # React frontend
│   ├── App.tsx                   # Root router + first-run setup logic
│   ├── main.tsx                  # Vite entry point
│   ├── settings/
│   │   ├── Layout.tsx            # Sidebar nav shell
│   │   ├── Home.tsx              # Stats + checklist + recent history
│   │   ├── Configuration.tsx     # Settings form (hotkey, API, mic)
│   │   ├── Vocabulary.tsx        # Custom vocabulary word list
│   │   ├── History.tsx           # Full transcription history table
│   │   └── Setup.tsx             # First-run hotkey setup screen
│   ├── overlay/
│   │   ├── Overlay.tsx           # Transparent recording overlay
│   │   ├── Waveform.tsx          # Animated audio level bars
│   │   └── TranscriptDisplay.tsx # Shows transcribed text after recording
│   ├── components/
│   │   ├── StatCard.tsx          # Reusable stat display card
│   │   └── ChecklistItem.tsx     # Onboarding checklist item
│   └── hooks/
│       └── useTauriEvent.ts      # Hook for Tauri event listeners
└── src-tauri/src/
    ├── lib.rs                    # App setup, Tauri commands, tray, window management
    ├── hotkey.rs                 # Hotkey handler + recording state machine
    ├── audio.rs                  # cpal audio recording + device enumeration
    ├── transcribe.rs             # HTTP client for Faster-Whisper API
    ├── db.rs                     # SQLite schema, CRUD, settings, stats
    ├── paste.rs                  # Windows clipboard paste (Win32 API)
    ├── sounds.rs                 # Start/stop/error sound playback
    ├── state.rs                  # AppState struct (shared app state)
    └── main.rs                   # Entry point
```

---

## Settings

All settings are persisted in SQLite at `%APPDATA%\local-super-whisper\`.

| Setting | Default | Description |
|---------|---------|-------------|
| `hotkey` | *(empty)* | Global shortcut key. Empty triggers the setup screen on launch. |
| `api_url` | `http://172.16.1.222:8028/v1` | Base URL of your Faster-Whisper server |
| `api_key` | `cant-be-empty` | Bearer token for the API |
| `model_id` | `deepdml/faster-whisper-large-v3-turbo-ct2` | Model name passed to the API |
| `mic_device` | `default` | Microphone device name, or `"default"` |
| `typing_speed_wpm` | `40` | Your typing speed, used to compute "time saved" stats |

---

## UI Tabs

### Home
Stats dashboard showing average WPM, words dictated this week, and estimated time saved. Includes an onboarding checklist and a preview of recent transcriptions.

### Vocabulary
Custom word list. Terms are injected into Whisper's `initial_prompt` parameter at transcription time, improving accuracy for domain-specific or unusual words (names, acronyms, jargon).

### Configuration
- **Hotkey** — rebind the global recording shortcut
- **API** — endpoint URL, key, and model ID for your Whisper server
- **Microphone** — select from detected input devices

### History
Scrollable log of all past transcriptions with timestamp, word count, and text. Capped at 500 entries (oldest are automatically removed). Click an entry to copy it.

---

## Tauri Commands (Rust → Frontend)

| Command | Description |
|---------|-------------|
| `get_stats` | Avg WPM, words this week, time saved |
| `get_history(limit)` | Recent transcriptions |
| `get_vocabulary` | Custom word list |
| `add_vocabulary_term(term)` | Add a word |
| `remove_vocabulary_term(id)` | Remove a word |
| `get_settings` | All settings as `[(key, value)]` |
| `update_setting(key, value)` | Save a single setting |
| `get_checklist` | Onboarding step states |
| `complete_checklist_step(step_id)` | Mark a step done |
| `get_audio_devices` | List available input devices |
| `register_hotkey(key)` | Unregister all → register new hotkey |

## Tauri Events (Rust → Frontend)

| Event | Payload | Description |
|-------|---------|-------------|
| `recording-started` | — | Recording has begun |
| `recording-transcribing` | — | Audio sent to API, awaiting result |
| `recording-result` | `String` | Transcription text |
| `recording-idle` | — | Back to idle |
| `recording-error` | `String` | Error message |
| `audio-level` | `f32` (0.0–1.0) | Mic level, emitted every ~50ms during recording |

---

## Key Dependencies

### Rust

| Crate | Purpose |
|-------|---------|
| `tauri` v2 | Desktop framework |
| `tauri-plugin-global-shortcut` | Global hotkey registration |
| `cpal` | Cross-platform audio capture |
| `hound` | WAV encoding |
| `reqwest` | HTTP client for Whisper API |
| `rusqlite` | SQLite (bundled) |
| `arboard` | Clipboard access |
| `enigo` | Keyboard simulation (Ctrl+V) |
| `rodio` | Sound effects playback |
| `windows` | `GetForegroundWindow` / `SetForegroundWindow` |
| `tokio` | Async runtime |

### Frontend

| Package | Purpose |
|---------|---------|
| `react` + `react-dom` | UI framework |
| `react-router-dom` | Client-side routing |
| `@tauri-apps/api` | Tauri IPC bindings |
| `tailwindcss` | Styling |
| `vite` | Build tool |

---

## Hotkey Compatibility

Not all keys work with `tauri-plugin-global-shortcut` on all platforms. **F-keys (F9–F12) are the most reliable choice.** Modifier-only keys (Alt, Ctrl, Shift alone) are not supported and will fail registration. If a key fails, the setup screen will display an error and let you try again.

---

## WSL2 Development Notes

The app can be developed in WSL2 via WSLg, but with limitations:

- The **system tray icon does not appear** under WSLg — this is a known WSLg limitation
- As a workaround, `src-tauri/tauri.conf.json` sets the settings window to `visible: true` for dev
- **Before building for production**, revert the settings window visibility to `false` — on Windows native, the window is opened via the tray icon

```bash
# Dev in WSL2
npm run tauri -- dev
```

For real-world use, build and run natively on Windows.

---

## Database Schema

```sql
-- Transcription history (rolling cap of 500)
CREATE TABLE history (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  text        TEXT NOT NULL,
  word_count  INTEGER NOT NULL,
  duration_ms INTEGER NOT NULL,
  wpm         REAL NOT NULL,
  created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Custom vocabulary hints for Whisper
CREATE TABLE vocabulary (
  id   INTEGER PRIMARY KEY AUTOINCREMENT,
  term TEXT NOT NULL UNIQUE
);

-- Key-value settings store
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Onboarding checklist progress
CREATE TABLE checklist (
  step_id      TEXT PRIMARY KEY,
  completed    BOOLEAN DEFAULT FALSE,
  completed_at DATETIME
);
```

---

## License

MIT
