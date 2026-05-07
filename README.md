<p align="center">
  <img src="superWHisperTrayIcon.png" alt="LocalSuperWhisper" width="200" />
</p>

<h1 align="center">LocalSuperWhisper</h1>

<p align="center"><b>Hit a hotkey. Talk. Your words appear in whatever window you were just using.</b></p>

<p align="center">
  <i>Voice-to-text on your own hardware. Sub-second latency. Zero subscriptions. Zero cloud uploads of your audio.</i>
</p>

<p align="center">
  <a href="#-quick-start-windows">Quick Start</a> ·
  <a href="#why-this-exists">Why</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#-developer-setup">Build from source</a> ·
  <a href="#license">License</a>
</p>

---

## Why this exists

Cloud dictation tools are great until you read the fine print and realize your meeting audio, your half-formed product ideas, and that voice memo you sent yourself at 1am all live on someone else's server now. The good ones cost ~$200/year. The "free" ones cost your data.

**LocalSuperWhisper sends nothing.** Audio never leaves your machine. You press a hotkey, dictate, and the transcribed text gets pasted into whatever window had focus — Slack, VS Code, your terminal, that PR description you've been avoiding for three days.

- 🎙️ **Press → speak → press.** A 3-second clip transcribes in **~0.5s** on a warm GPU over localhost.
- 🛜 **Fully self-hosted.** Pair it with a Faster-Whisper container on your LAN (or `localhost`). Your voice stays in your house.
- 💸 **Free for personal & noncommercial use.** No "Pro" tier, no per-minute fees, no asterisks.
- 🪟🐧🍎 **Windows + Linux + macOS.** One tray icon. Stays out of the way until you summon it.
- ⚡ **Tauri v2 + Rust + React.** ~30 MB installer, low single-digit MB of RAM at idle.

> "It's just a tray icon that turns my voice into text and then disappears."
> *— the entire pitch*

---

## How it works

1. Press your hotkey → a recording overlay appears
2. Speak
3. Press the hotkey again → audio goes to your local Faster-Whisper server
4. Transcribed text is pasted into the window you were just using
5. The overlay vanishes

No chat UI. No "AI assistant" personality. No popup asking you to upgrade. The app lives in the system tray and stays out of your way until you call it.

---

## 🚀 Quick Start (Windows)

**Want to install and use it without building from source?**

1. **Build the installer** (or grab one from a release):
   ```powershell
   .\build-installer.ps1
   ```
2. **Run** `Local SuperWhisper_0.1.0_x64-setup.exe`
3. **Launch from the Start Menu** — the app drops into your system tray
4. **First run:** set your hotkey, point it at your Whisper API, pick a mic. Done.

What you get:

- ✅ One-click install, no toolchain required
- ✅ Auto-starts in the tray on login (optional)
- ✅ Start Menu shortcuts + clean uninstall via Windows Settings
- ✅ Authenticode-signed installer (no SmartScreen scare screen if you import the cert)

📖 Full walkthrough: [Windows Installer Guide](docs/WINDOWS_INSTALLER.md)

> Don't have a Whisper backend yet? Skip down to [Faster-Whisper Server](#faster-whisper-server) — it's one Docker command.

---

## 👨‍💻 Developer Setup

The sections below are for building, modifying, or contributing.

### Prerequisites

**All platforms:**

- [Rust](https://rustup.rs) (stable toolchain)
- [Node.js](https://nodejs.org) (v18 or later)
- A running [Faster-Whisper](https://github.com/SYSTRAN/faster-whisper) server exposing an OpenAI-compatible `/v1/audio/transcriptions` endpoint

**Linux (Ubuntu 22.04+):**

```bash
sudo apt install -y build-essential libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libasound2-dev libssl-dev \
  pkg-config xdotool libxdo-dev
```

Or use the manage.sh menu: option **9) Install Build Dependencies**.

### Faster-Whisper Server

The app talks to any HTTP API compatible with OpenAI's audio transcription format. The easy path is [faster-whisper-server](https://github.com/fedirz/faster-whisper-server):

```bash
docker run --gpus all -p 8028:8000 fedirz/faster-whisper-server:latest-cuda
```

Set the API URL in the **Configuration** tab to match (e.g. `http://localhost:8028/v1`, or your LAN IP for a remote box).

#### Make it actually fast

By default the server unloads the Whisper model after 5 minutes of idle (`WHISPER__TTL=300`) and lazy-loads on the next request. The reload alone is ~30s on a CUDA cold path — which makes the first dictation after a coffee break feel completely broken. Two env vars fix it:

```yaml
environment:
  - WHISPER__TTL=-1                                                # never unload
  - PRELOAD_MODELS=["deepdml/faster-whisper-large-v3-turbo-ct2"]   # warm at container start
```

With these set, a 3-second clip transcribes end-to-end in **~0.5–0.7s over localhost** and **~0.4s over LAN**. Only the very first request after a container restart pays a one-time ~6s cuDNN/ctranslate2 init cost.

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

> **⚠️ Heads up:** if you ever rebuild the binary directly with `cargo build --release` (e.g. for PM2 or CI), you **must** pass `--features custom-protocol`. Tauri-codegen decides dev-vs-prod at compile time via that feature; without it the binary embeds empty assets and the WebView falls back to `http://localhost:1420` at runtime, producing `ERR_CONNECTION_REFUSED` in every window. `tauri build` adds the feature automatically — `cargo build` does not. This will eat hours of your life if you don't know about it.

Build artifacts by platform:

| Platform | Output |
|----------|--------|
| Windows | `.msi` installer in `src-tauri/target/release/bundle/msi/` |
| Linux | `.deb` and `.rpm` in `src-tauri/target/release/bundle/deb/` and `rpm/` |

### Code signing (Windows)

Windows installers must be Authenticode-signed or end users will hit a silent
SmartScreen / UAC failure when they double-click — the installer process never
launches and there is no error dialog. Welcome to Windows.

We sign with a **self-signed cert** for internal/team distribution. (For public
distribution use a commercial OV/EV cert or Azure Trusted Signing — see
[`signing/README.md`](signing/README.md).)

`tauri build` reads the signing config from `src-tauri/tauri.conf.json` and
calls `signtool` automatically — but it needs the cert in the local cert store
first.

**Build machine setup (one-time per machine):**

1. Get `signing/cert.pfx` from a teammate (it's gitignored — share it via
   1Password or a secure channel, never via the repo).
2. Import it into your personal cert store:

   ```powershell
   .\signing\install-signing-cert.ps1
   ```

3. From now on, `npm run tauri -- build` produces signed `.exe` and `.msi`
   installers automatically.

**End-user machine setup (one-time per user):**

A signed installer with a self-signed cert is still rejected by SmartScreen
unless the cert is added to that user's Trusted Root + Trusted Publisher
stores. Distribute `signing/cert.cer` (public, safe to share) and have each
user run, in an elevated PowerShell:

```powershell
.\signing\trust-cert.ps1
```

After that, your signed installers run cleanly on their machine — no
SmartScreen warning, the UAC prompt shows your verified publisher name
instead of "Unknown publisher."

> Without signing, an unsigned `.msi` or `.exe` downloaded from GitHub
> Releases carries the Mark-of-the-Web tag (`Zone.Identifier:$DATA`) and
> Windows silently refuses to launch it — a known cause of "I double-clicked
> the installer and nothing happened." Signing fixes this for any machine
> that trusts the cert; for public distribution you need a cert with broad
> trust (commercial OV/EV) instead.

Full details, cert rotation, and rationale are in
[`signing/README.md`](signing/README.md).

### First run

On first launch the app detects that no hotkey has been configured and shows a setup screen. Click **Choose Hotkey**, press any key (F9–F12 are the most reliable), and the app saves and registers it immediately.

---

## Running as a Background Service (PM2)

> **Note for end users:** if you installed via the Windows installer, you don't need PM2. The app runs from the Start Menu or tray. This section is for developers and advanced users who want process management during development.

The app can be managed as a persistent background process using [PM2](https://pm2.keymetrics.io). It keeps things running, restarts on crash, and can launch at login.

### 1. Install PM2

```bash
npm install -g pm2
```

### 2. First deploy

Build the app and start it under PM2 in one step:

```bash
./manage.sh redeploy
```

Runs the full Rust + frontend build and registers the process with PM2.

### 3. Enable startup on login

```bash
./manage.sh startup
```

- **Windows:** saves the PM2 process list and installs a Task Scheduler entry via `pm2-windows-startup`.
- **Linux:** creates a `.desktop` autostart entry in `~/.config/autostart/`.

> Run `./manage.sh startup` again any time you add or remove processes from PM2 to refresh the saved list.

---

## manage.sh — Process Manager CLI

Interactive shell tool for managing the app's lifecycle. Run it with no arguments for a menu, or pass a command directly.

```
Usage: ./manage.sh [command]

Commands:
  start      Start the app under PM2
  stop       Stop the running app
  restart    Restart the app
  logs       Tail live logs (Ctrl+C to exit)
  redeploy   Full rebuild (frontend + Rust) then restart
  status     Show PM2 process table
  startup    Enable auto-start on Windows login
```

**Interactive menu:**

```bash
./manage.sh
```

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

### Restart behavior

| Scenario | PM2 behavior |
|---|---|
| App crashes (non-zero exit) | Auto-restarts, up to 10 times |
| User closes via tray icon | **Not** restarted — intentional exit is respected |
| Login (Windows/Linux) | PM2 resurrects the saved process list |

Controlled by `stop_exit_codes: [0]` in `ecosystem.config.cjs`: PM2 only auto-restarts on non-zero exit codes (crashes), not on clean shutdowns.

### Log files

```
logs/app-err.log
logs/app-out.log
```

The `logs/` directory is git-ignored. View live logs with `./manage.sh logs` or `pm2 logs localSuperWhisper`.

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
    ├── paste.rs                  # Clipboard paste + window focus (Win32 / xdotool)
    ├── sounds.rs                 # Start/stop/error sound playback
    ├── state.rs                  # AppState struct (shared app state)
    └── main.rs                   # Entry point
```

---

## Settings

Settings are persisted in SQLite at:
- **Windows:** `%APPDATA%\com.localsuperwhisper.app\`
- **Linux:** `~/.local/share/com.localsuperwhisper.app/`

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
Scrollable log of past transcriptions with timestamp, word count, and text. Capped at 500 entries (oldest are automatically removed). Click an entry to copy it.

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
| `enigo` | Keyboard simulation — Ctrl+V (Windows) |
| `rodio` | Sound effects playback (Windows) |
| `windows` | `GetForegroundWindow` / `SetForegroundWindow` (Windows only) |
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

Not all keys work with `tauri-plugin-global-shortcut` on every platform. **F-keys (F9–F12) are the most reliable choice.** Modifier-only keys (Alt, Ctrl, Shift alone) are not supported and will fail registration. If a key fails, the setup screen surfaces the error and lets you try again.

---

## Platform Notes

### Linux

- Uses **X11** via `xdotool` for window focus capture/restore and keyboard simulation
- Sound playback uses `aplay` (ALSA), which works with PipeWire and PulseAudio
- **Wayland:** window capture/restore is skipped; clipboard paste still works but can't auto-focus the target window
- **Code editors** (VS Code, Windsurf, Antigravity): the app detects the window class and uses `Ctrl+Shift+V` instead of `Ctrl+V`
- Runtime dependencies: `xdotool`, `xprop` (part of `x11-utils`)

### WSL2

The app can be developed in WSL2 via WSLg, with caveats:

- The **system tray icon does not appear** under WSLg
- As a workaround, `src-tauri/tauri.conf.json` sets the settings window to `visible: true` for dev
- For real-world use, build and run natively on Windows or Linux desktop

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

LocalSuperWhisper is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).

**You can use it for free** for personal projects, research, education, hobby use, and noncommercial organizations (charities, public schools, public research, government, etc.).

**You cannot sell it** — that includes selling copies, selling a hosted/SaaS version, or any other commercial use. Commercial use of any kind requires a separate license; please open an issue if you want to discuss one.

This is a source-available license, not OSI-approved open source. The full terms (and the official URL) are in the [LICENSE](LICENSE) file.

---

<p align="center">
  <i>Built because dictation should belong to you, not to a billing dashboard.</i>
</p>
