# Local SuperWhisper — Design Spec

## Overview

A lightweight Windows 10/11 desktop application that replicates the core Superwhisper workflow: press a hotkey, dictate into your microphone, and the transcribed text is automatically pasted into the active window. Built with Tauri v2 (Rust backend) and React + Tailwind (frontend). Transcription is handled by a local Faster-Whisper instance via an OpenAI-compatible API.

## Architecture

Single Tauri v2 process with two windows and a Rust backend.

```
┌─────────────────────────────────────────────────┐
│                  Tauri Process                   │
│                                                  │
│  ┌──────────────┐       ┌─────────────────────┐  │
│  │ Overlay Win  │       │   Settings Window   │  │
│  │ (transparent,│       │   (normal window,   │  │
│  │  no-focus,   │       │    opened from      │  │
│  │  center)     │       │    tray icon)       │  │
│  └──────┬───────┘       └──────────┬──────────┘  │
│         │    React + Tailwind      │             │
│         └──────────┬───────────────┘             │
│                    │ Tauri IPC (invoke)           │
│  ┌─────────────────┴────────────────────────┐    │
│  │            Rust Backend                   │    │
│  │                                           │    │
│  │  ┌───────────┐  ┌──────────┐  ┌────────┐ │    │
│  │  │ Audio     │  │ Whisper  │  │Keyboard│ │    │
│  │  │ Capture   │  │ API      │  │Emulate │ │    │
│  │  │ (cpal)    │  │(reqwest) │  │(enigo) │ │    │
│  │  └───────────┘  └──────────┘  └────────┘ │    │
│  │  ┌───────────┐  ┌──────────┐  ┌────────┐ │    │
│  │  │ Sound FX  │  │ SQLite   │  │ Tray   │ │    │
│  │  │ (rodio)   │  │(rusqlite)│  │ Icon   │ │    │
│  │  └───────────┘  └──────────┘  └────────┘ │    │
│  └──────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

- Both windows share one React build, routed by URL (`/overlay` vs `/settings`).
- Overlay window: borderless, transparent, always-on-top, non-focusable (`set_skip_taskbar(true)`, `set_focus(false)`).
- Settings window: standard decorated window, hidden by default, shown from tray context menu.
- All heavy work (audio, API, pasting) stays in Rust — the frontend is purely presentational.
- State flows from Rust to frontend via Tauri events; commands flow from frontend to Rust via `invoke`.

## Core Recording Workflow

### State Machine

```
                    Right Alt
    ┌──────┐      ─────────►  ┌───────────┐
    │ Idle │                  │ Recording │
    └──────┘  ◄─────────────  └─────┬─────┘
        ▲       (auto after          │ Right Alt
        │        dismiss)            ▼
   ┌────┴─────┐              ┌──────────────┐
   │Displaying│  ◄────────── │ Transcribing │
   │ Result   │              └──────────────┘
   └──────────┘
        │ 2-3 sec timeout
        ▼
      Idle
```

### Step-by-Step Flow

1. **Idle** — Tray icon visible, overlay hidden, hotkey listener active.
2. **Right Alt pressed** — Rust captures the currently focused window handle (for later paste target), plays start sound effect, begins `cpal` microphone capture into an in-memory buffer, shows overlay window with animated waveform, emits `recording-started` event to frontend.
3. **Recording** — `cpal` streams PCM samples. Frontend receives audio level data via Tauri events (~50ms intervals) to animate the waveform. Audio accumulates in a `Vec<f32>` in Rust.
4. **Right Alt pressed again** — Stops `cpal` stream, plays stop sound effect, encodes buffer to WAV (16kHz mono, 16-bit PCM) using `hound`, overlay transitions to "transcribing..." spinner state.
5. **Transcribing** — Rust sends WAV as multipart form POST to the configured Whisper API endpoint. Parses JSON response for transcript text.
6. **Result** — Overlay displays transcribed text. Simultaneously, Rust sets clipboard text via `arboard`, restores focus to the saved window handle via `SetForegroundWindow`, simulates Ctrl+V via `enigo`. Stats updated in SQLite (word count, duration, WPM).
7. **Displaying Result** — After 2-3 seconds, overlay auto-hides. Back to Idle.

### Error Handling

- API unreachable: overlay shows error message, plays error sound, auto-dismiss after 3 seconds.
- Empty recording (< 0.5 seconds): discard silently, return to Idle.
- Hotkey pressed during Transcribing: ignored (prevent double-submit).

## Frontend Structure

### Overlay View (`/overlay`)

Minimal — no chrome, no controls. Three states:

- **Recording**: Centered waveform visualization (CSS/Canvas animated bars reacting to audio level events from Rust). Semi-transparent dark background with rounded corners, ~300px wide, height auto-sized to content.
- **Transcribing**: Waveform replaced by a subtle pulsing spinner + "Transcribing..." text.
- **Result**: Spinner replaced by the transcribed text in a readable font, fades out after 2-3 seconds.

The overlay uses `backdrop-filter: blur()` and semi-transparent background for frosted glass look. Non-focusable — never steals focus from the user's active window.

### Settings Window (`/settings`)

Sidebar navigation with dark theme. Tabs:

- **Home** — Stats dashboard (WPM, words this week, time saved) + recent transcription history (most recent entries from history table) + Get Started checklist.
- **Vocabulary** — CRUD list of custom words/terms. Each entry is a string stored in SQLite. Injected into Whisper's `initial_prompt` parameter at transcription time.
- **Configuration** — Hotkey binding (default: Right Alt), API endpoint URL, API key, model ID, microphone device selector (populated from `cpal` device enumeration), audio preview/test button.
- **History** — Full scrollable list of past transcriptions with timestamp, word count, and text. Click to copy.

### Styling

- Tailwind CSS with dark theme: dark gray sidebar (`#1a1a1a`), slightly lighter content area (`#242424`), purple accent for active nav items and buttons.
- Settings window uses Tauri's `window-vibrancy` for native Windows acrylic/mica effect.
- System font stack (Segoe UI on Windows).

## Data Layer

SQLite database stored at `%APPDATA%/local-super-whisper/`.

### Schema

```sql
-- Transcription history (rolling 500 entries)
CREATE TABLE history (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  text        TEXT NOT NULL,
  word_count  INTEGER NOT NULL,
  duration_ms INTEGER NOT NULL,
  wpm         REAL NOT NULL,
  created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Custom vocabulary terms
CREATE TABLE vocabulary (
  id   INTEGER PRIMARY KEY AUTOINCREMENT,
  term TEXT NOT NULL UNIQUE
);

-- User settings (key-value store)
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Get Started checklist progress
CREATE TABLE checklist (
  step_id      TEXT PRIMARY KEY,
  completed    BOOLEAN DEFAULT FALSE,
  completed_at DATETIME
);
```

### History Rolling Cleanup

After each insert, delete excess rows:
```sql
DELETE FROM history WHERE id NOT IN (
  SELECT id FROM history ORDER BY created_at DESC LIMIT 500
);
```

### Default Settings

| Key | Default Value |
|-----|---------------|
| `hotkey` | `RAlt` |
| `api_url` | `http://172.16.1.222:8028/v1` |
| `api_key` | `cant-be-empty` |
| `model_id` | `deepdml/faster-whisper-large-v3-turbo-ct2` |
| `mic_device` | `default` |
| `typing_speed_wpm` | `40` |

### Stats (Computed at Query Time)

- **WPM**: `AVG(wpm)` from history.
- **Words this week**: `SUM(word_count) WHERE created_at >= start_of_week`.
- **Time saved**: `SUM(word_count / 40.0 - duration_ms / 60000.0)` minutes (words at assumed 40 WPM typing speed minus actual dictation time).

## Rust Backend Modules

All in `src-tauri/src/`:

### `audio.rs` — Microphone Capture
- `cpal` to enumerate input devices and open a stream on the selected mic.
- Streams `f32` PCM samples into a shared `Arc<Mutex<Vec<f32>>>` buffer.
- Computes RMS audio level per ~50ms chunk, emits Tauri `audio-level` event for frontend waveform animation.
- `start_recording()` and `stop_recording()` — stop returns the buffer and clears it.
- Encodes buffer to WAV (16kHz mono, 16-bit PCM) using `hound`.

### `transcribe.rs` — Whisper API Client
- Sends WAV bytes as `multipart/form-data` POST to `{api_url}/audio/transcriptions`.
- Headers: `Authorization: Bearer {api_key}`.
- Body fields: `file` (WAV binary), `model` (model ID), `initial_prompt` (vocabulary terms joined by commas).
- Parses OpenAI-compatible JSON response: `{ "text": "..." }`.
- Returns transcript string or error.

### `hotkey.rs` — Global Shortcut Management
- Uses `tauri-plugin-global-shortcut`.
- Registers configured hotkey at startup.
- On trigger, transitions the recording state machine (Idle → Recording → Transcribing → Displaying).
- When hotkey changes in settings, unregisters old and registers new.

### `paste.rs` — Clipboard + Keyboard Emulation
- Before recording: captures foreground window handle via `windows` crate (`GetForegroundWindow`).
- After transcription: sets clipboard text via `arboard`, brings saved window to front via `SetForegroundWindow`, simulates Ctrl+V via `enigo`.
- Small delay (~50ms) between focus restore and paste.

### `db.rs` — SQLite Data Access
- Initializes DB with migrations on first run.
- CRUD functions for history, vocabulary, settings, checklist.
- History insert triggers rolling cleanup (cap at 500).
- Stats query functions.

### `sounds.rs` — Sound Effects
- `rodio` to play bundled WAV files from the app's resource directory.
- Three sounds: `start.wav`, `stop.wav`, `error.wav`.
- Non-blocking playback on a dedicated thread.

### Tauri Commands (exposed to frontend)

- `get_stats()`, `get_history()`, `get_vocabulary()`, `add_vocabulary_term()`, `remove_vocabulary_term()`, `get_settings()`, `update_setting()`, `get_checklist()`, `complete_checklist_step()`, `get_audio_devices()`, `test_microphone()`

## File Structure

```
localSuperWhisper/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   ├── sounds/
│   │   ├── start.wav
│   │   ├── stop.wav
│   │   └── error.wav
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── audio.rs
│       ├── transcribe.rs
│       ├── hotkey.rs
│       ├── paste.rs
│       ├── db.rs
│       └── sounds.rs
├── src/
│   ├── index.html
│   ├── main.tsx
│   ├── App.tsx
│   ├── overlay/
│   │   ├── Overlay.tsx
│   │   ├── Waveform.tsx
│   │   └── TranscriptDisplay.tsx
│   ├── settings/
│   │   ├── Layout.tsx
│   │   ├── Home.tsx
│   │   ├── Vocabulary.tsx
│   │   ├── Configuration.tsx
│   │   └── History.tsx
│   ├── components/
│   │   ├── StatCard.tsx
│   │   └── ChecklistItem.tsx
│   ├── hooks/
│   │   └── useTauriEvent.ts
│   └── styles/
│       └── tailwind.css
├── package.json
├── tsconfig.json
├── tailwind.config.js
├── vite.config.ts
└── README.md
```

## Key Dependencies

### Rust (Cargo.toml)
- `tauri` v2
- `tauri-plugin-global-shortcut`
- `tauri-plugin-shell`
- `cpal` — cross-platform audio I/O
- `hound` — WAV encoding
- `reqwest` — HTTP client (with `multipart` feature)
- `serde` / `serde_json` — serialization
- `rusqlite` — SQLite (with `bundled` feature)
- `arboard` — clipboard access
- `enigo` — keyboard/mouse emulation
- `rodio` — audio playback for sound effects
- `windows` — Windows API bindings (`GetForegroundWindow`, `SetForegroundWindow`)
- `window-vibrancy` — acrylic/mica window effects
- `tokio` — async runtime

### Frontend (package.json)
- `react`, `react-dom`
- `react-router-dom`
- `@tauri-apps/api` — Tauri IPC bindings
- `tailwindcss`, `postcss`, `autoprefixer`
- `typescript`
- `vite`, `@vitejs/plugin-react`

## Phase 1 Scope

Included:
- Full recording workflow (hotkey → record → transcribe → paste)
- Overlay with waveform, transcribing state, result display
- Settings window with Home, Vocabulary, Configuration, History tabs
- Stats dashboard, Get Started checklist, recent history
- System tray with context menu
- SQLite persistence with rolling history (500 entries)
- Configurable hotkey, API endpoint, mic device
- Vocabulary injection into Whisper prompt

Not included (future phases):
- Modes (transcription presets/profiles)
- System audio / loopback capture
- Multi-language support
- Auto-update mechanism
