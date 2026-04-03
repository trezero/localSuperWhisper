# Local SuperWhisper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri v2 desktop app for Windows that records microphone audio via a global hotkey, transcribes it using a local Faster-Whisper API, and auto-pastes the result into the active window.

**Architecture:** Single Tauri v2 process with two windows (a non-focusable transparent overlay for recording feedback, and a hidden settings/dashboard window opened from the system tray). Rust backend handles audio capture, API calls, clipboard, and keyboard emulation. React + Tailwind frontend is purely presentational.

**Tech Stack:** Tauri v2, Rust, React 18, TypeScript, Tailwind CSS v3, Vite, SQLite (rusqlite), cpal, hound, reqwest, enigo, arboard, rodio, window-vibrancy

**Spec:** `docs/superpowers/specs/2026-04-03-local-super-whisper-design.md`

---

## File Map

### Rust Backend (`src-tauri/src/`)

| File | Responsibility |
|------|---------------|
| `main.rs` | Windows subsystem entry, calls `lib::run()` |
| `lib.rs` | Tauri builder: plugin registration, state management, command handler, tray setup, window setup |
| `db.rs` | SQLite schema init, migrations, CRUD for history/vocabulary/settings/checklist, stats queries |
| `audio.rs` | cpal device enumeration, mic stream start/stop, PCM buffer, RMS level events, WAV encoding |
| `transcribe.rs` | HTTP multipart POST to Whisper API, response parsing |
| `paste.rs` | Capture foreground window handle, set clipboard, restore focus, simulate Ctrl+V |
| `sounds.rs` | rodio playback of bundled WAV files on background thread |
| `hotkey.rs` | Global shortcut registration, recording state machine (Idle→Recording→Transcribing→Displaying) |
| `state.rs` | Shared `AppState` struct definition, recording state enum |

### React Frontend (`src/`)

| File | Responsibility |
|------|---------------|
| `index.html` | HTML shell |
| `main.tsx` | React DOM render |
| `App.tsx` | HashRouter: `/overlay` → Overlay, `/settings/*` → Settings |
| `overlay/Overlay.tsx` | Overlay root: listens for Tauri events, switches between Recording/Transcribing/Result states |
| `overlay/Waveform.tsx` | Animated bars driven by `audio-level` events |
| `overlay/TranscriptDisplay.tsx` | Shows transcribed text, fades out after 2-3s |
| `settings/Layout.tsx` | Sidebar nav + `<Outlet/>` content area |
| `settings/Home.tsx` | Stats cards, recent history, Get Started checklist |
| `settings/Vocabulary.tsx` | Add/remove vocabulary terms |
| `settings/Configuration.tsx` | Hotkey, API, mic device settings form |
| `settings/History.tsx` | Full scrollable history, click to copy |
| `components/StatCard.tsx` | Reusable stat display card |
| `components/ChecklistItem.tsx` | Checklist row with checkbox |
| `hooks/useTauriEvent.ts` | Wrapper for Tauri event listener with cleanup |
| `styles/tailwind.css` | Tailwind directives |

### Config Files

| File | Responsibility |
|------|---------------|
| `src-tauri/Cargo.toml` | Rust dependencies |
| `src-tauri/tauri.conf.json` | Tauri app config: windows, tray, bundle |
| `src-tauri/capabilities/default.json` | Tauri v2 permissions/capabilities |
| `src-tauri/build.rs` | Tauri build script |
| `package.json` | Frontend dependencies + scripts |
| `vite.config.ts` | Vite config with React plugin + Tauri host |
| `tailwind.config.js` | Tailwind theme (dark colors, purple accent) |
| `postcss.config.js` | PostCSS with Tailwind + autoprefixer |
| `tsconfig.json` | TypeScript config |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `src/index.html`, `src/main.tsx`, `src/App.tsx`, `src/styles/tailwind.css`
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `tailwind.config.js`, `postcss.config.js`

- [ ] **Step 1: Verify prerequisites are installed**

Run:
```bash
rustc --version && cargo --version && node --version && npm --version
```

Expected: Rust 1.77+, Node 18+, npm 9+. If missing, install Rust via `rustup` and Node via `nvm` or installer.

- [ ] **Step 2: Create `package.json`**

```json
{
  "name": "local-super-whisper",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-global-shortcut": "^2",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.26.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.19",
    "postcss": "^8.4.38",
    "tailwindcss": "^3.4.4",
    "typescript": "^5.5.0",
    "vite": "^5.3.0"
  }
}
```

- [ ] **Step 3: Install npm dependencies**

Run:
```bash
npm install
```

Expected: `node_modules/` created, no errors.

- [ ] **Step 4: Create `vite.config.ts`**

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

- [ ] **Step 5: Create `tsconfig.json` and `tsconfig.node.json`**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

`tsconfig.node.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 6: Create `tailwind.config.js` and `postcss.config.js`**

`tailwind.config.js`:
```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#242424",
          dark: "#1a1a1a",
          light: "#2e2e2e",
          hover: "#383838",
        },
        accent: {
          DEFAULT: "#8b5cf6",
          hover: "#7c3aed",
          muted: "rgba(139, 92, 246, 0.15)",
        },
        text: {
          primary: "#e4e4e7",
          secondary: "#a1a1aa",
          muted: "#71717a",
        },
      },
      fontFamily: {
        sans: ['"Segoe UI"', "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};
```

`postcss.config.js`:
```javascript
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 7: Create `src/styles/tailwind.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  margin: 0;
  background: transparent;
  font-family: "Segoe UI", system-ui, sans-serif;
  color: #e4e4e7;
  overflow: hidden;
  -webkit-user-select: none;
  user-select: none;
}
```

- [ ] **Step 8: Create `src/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Local SuperWhisper</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 9: Create `src/main.tsx`**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tailwind.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 10: Create `src/App.tsx` with HashRouter**

```tsx
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";

function OverlayPlaceholder() {
  return <div className="text-white text-center p-4">Overlay</div>;
}

function SettingsPlaceholder() {
  return <div className="text-white text-center p-4">Settings</div>;
}

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<OverlayPlaceholder />} />
        <Route path="/settings" element={<SettingsPlaceholder />} />
        <Route path="*" element={<Navigate to="/settings" replace />} />
      </Routes>
    </HashRouter>
  );
}
```

- [ ] **Step 11: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "local-super-whisper"
version = "0.1.0"
edition = "2021"

[lib]
name = "local_super_whisper_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
cpal = "0.15"
hound = "3.5"
reqwest = { version = "0.12", features = ["multipart", "json"] }
rusqlite = { version = "0.32", features = ["bundled"] }
arboard = "3"
enigo = { version = "0.2", features = ["serde"] }
rodio = "0.19"
window-vibrancy = "0.5"
windows = { version = "0.58", features = [
  "Win32_UI_WindowsAndMessaging",
  "Win32_Foundation",
] }
```

- [ ] **Step 12: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 13: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicedoc/schemas/master/tauri/v2/tauri.conf.json",
  "productName": "Local SuperWhisper",
  "version": "0.1.0",
  "identifier": "com.localsuperwhisper.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "settings",
        "title": "Local SuperWhisper",
        "url": "index.html#/settings",
        "width": 850,
        "height": 600,
        "minWidth": 700,
        "minHeight": 500,
        "visible": false,
        "center": true,
        "decorations": true
      },
      {
        "label": "overlay",
        "url": "index.html#/overlay",
        "width": 300,
        "height": 120,
        "visible": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "center": true
      }
    ],
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": false
    },
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ],
    "resources": ["sounds/*"]
  }
}
```

- [ ] **Step 14: Create `src-tauri/capabilities/default.json`**

```json
{
  "identifier": "default",
  "description": "Default capabilities for Local SuperWhisper",
  "windows": ["settings", "overlay"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-center",
    "core:window:allow-set-size",
    "global-shortcut:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "shell:default"
  ]
}
```

- [ ] **Step 15: Create `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    local_super_whisper_lib::run()
}
```

- [ ] **Step 16: Create minimal `src-tauri/src/lib.rs`**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 17: Create placeholder tray icon**

Run:
```bash
mkdir -p src-tauri/icons
```

Generate a minimal 32x32 PNG icon (a purple circle on transparent background). You can use ImageMagick if available:

```bash
convert -size 32x32 xc:transparent -fill "#8b5cf6" -draw "circle 16,16 16,2" src-tauri/icons/icon.png
cp src-tauri/icons/icon.png src-tauri/icons/32x32.png
convert -size 128x128 xc:transparent -fill "#8b5cf6" -draw "circle 64,64 64,8" src-tauri/icons/128x128.png
convert -size 256x256 xc:transparent -fill "#8b5cf6" -draw "circle 128,128 128,16" src-tauri/icons/128x128@2x.png
convert src-tauri/icons/128x128.png src-tauri/icons/icon.ico
```

If ImageMagick is not available, create any valid PNG files as placeholders. The icons just need to exist for the build to succeed.

- [ ] **Step 18: Create placeholder sounds directory**

Run:
```bash
mkdir -p src-tauri/sounds
```

Create minimal valid WAV files as placeholders (we'll replace with real sounds later):

```bash
# Generate a short 0.1s sine wave beep using sox if available:
sox -n -r 16000 -c 1 src-tauri/sounds/start.wav synth 0.1 sine 880 vol 0.5
sox -n -r 16000 -c 1 src-tauri/sounds/stop.wav synth 0.1 sine 440 vol 0.5
sox -n -r 16000 -c 1 src-tauri/sounds/error.wav synth 0.2 sine 220 vol 0.5
```

If `sox` is not available, these can be generated programmatically in a later task. The build just needs the directory to exist.

- [ ] **Step 19: Build and verify**

Run:
```bash
cargo tauri build --debug 2>&1 | tail -20
```

Expected: Build completes successfully. The binary is created in `src-tauri/target/debug/`. If there are dependency resolution errors, fix version constraints in `Cargo.toml`.

- [ ] **Step 20: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + React + Tailwind project

Sets up project skeleton with all dependencies, two-window Tauri
config (settings + overlay), tray icon, Vite bundler, and TypeScript."
```

---

## Task 2: SQLite Database Layer

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod db`)

- [ ] **Step 1: Write failing tests for db module**

Create `src-tauri/src/db.rs` with tests first:

```rust
use rusqlite::{Connection, Result, params};
use serde::Serialize;

// -- Types --

#[derive(Debug, Serialize, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub word_count: i32,
    pub duration_ms: i64,
    pub wpm: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VocabularyEntry {
    pub id: i64,
    pub term: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Stats {
    pub avg_wpm: f64,
    pub words_this_week: i64,
    pub time_saved_minutes: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChecklistStep {
    pub step_id: String,
    pub completed: bool,
    pub completed_at: Option<String>,
}

// -- Placeholder functions (will implement in step 3) --

pub fn init_db(_conn: &Connection) -> Result<()> {
    todo!()
}

pub fn insert_history(_conn: &Connection, _text: &str, _word_count: i32, _duration_ms: i64, _wpm: f64) -> Result<()> {
    todo!()
}

pub fn get_history(_conn: &Connection, _limit: i32) -> Result<Vec<HistoryEntry>> {
    todo!()
}

pub fn get_stats(_conn: &Connection) -> Result<Stats> {
    todo!()
}

pub fn add_vocabulary(_conn: &Connection, _term: &str) -> Result<()> {
    todo!()
}

pub fn remove_vocabulary(_conn: &Connection, _id: i64) -> Result<()> {
    todo!()
}

pub fn get_vocabulary(_conn: &Connection) -> Result<Vec<VocabularyEntry>> {
    todo!()
}

pub fn get_setting(_conn: &Connection, _key: &str) -> Result<String> {
    todo!()
}

pub fn set_setting(_conn: &Connection, _key: &str, _value: &str) -> Result<()> {
    todo!()
}

pub fn get_all_settings(_conn: &Connection) -> Result<Vec<(String, String)>> {
    todo!()
}

pub fn get_checklist(_conn: &Connection) -> Result<Vec<ChecklistStep>> {
    todo!()
}

pub fn complete_checklist_step(_conn: &Connection, _step_id: &str) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_init_creates_tables() {
        let conn = setup_db();
        // Verify all four tables exist by querying sqlite_master
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"history".to_string()));
        assert!(tables.contains(&"vocabulary".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"checklist".to_string()));
    }

    #[test]
    fn test_default_settings_seeded() {
        let conn = setup_db();
        assert_eq!(get_setting(&conn, "hotkey").unwrap(), "RAlt");
        assert_eq!(get_setting(&conn, "api_url").unwrap(), "http://172.16.1.222:8028/v1");
        assert_eq!(get_setting(&conn, "api_key").unwrap(), "cant-be-empty");
        assert_eq!(get_setting(&conn, "model_id").unwrap(), "deepdml/faster-whisper-large-v3-turbo-ct2");
        assert_eq!(get_setting(&conn, "mic_device").unwrap(), "default");
        assert_eq!(get_setting(&conn, "typing_speed_wpm").unwrap(), "40");
    }

    #[test]
    fn test_insert_and_get_history() {
        let conn = setup_db();
        insert_history(&conn, "hello world", 2, 5000, 24.0).unwrap();
        insert_history(&conn, "second entry", 2, 3000, 40.0).unwrap();
        let entries = get_history(&conn, 10).unwrap();
        assert_eq!(entries.len(), 2);
        // Most recent first
        assert_eq!(entries[0].text, "second entry");
        assert_eq!(entries[1].text, "hello world");
    }

    #[test]
    fn test_history_rolling_cleanup() {
        let conn = setup_db();
        for i in 0..510 {
            insert_history(&conn, &format!("entry {}", i), 1, 1000, 60.0).unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .unwrap();
        assert!(count <= 500, "History should not exceed 500 entries, got {}", count);
    }

    #[test]
    fn test_vocabulary_crud() {
        let conn = setup_db();
        add_vocabulary(&conn, "Kubernetes").unwrap();
        add_vocabulary(&conn, "Tauri").unwrap();
        let terms = get_vocabulary(&conn).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].term, "Kubernetes");
        remove_vocabulary(&conn, terms[0].id).unwrap();
        let terms = get_vocabulary(&conn).unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "Tauri");
    }

    #[test]
    fn test_vocabulary_duplicate_rejected() {
        let conn = setup_db();
        add_vocabulary(&conn, "Kubernetes").unwrap();
        let result = add_vocabulary(&conn, "Kubernetes");
        assert!(result.is_err());
    }

    #[test]
    fn test_settings_update() {
        let conn = setup_db();
        set_setting(&conn, "hotkey", "LCtrl").unwrap();
        assert_eq!(get_setting(&conn, "hotkey").unwrap(), "LCtrl");
    }

    #[test]
    fn test_get_all_settings() {
        let conn = setup_db();
        let settings = get_all_settings(&conn).unwrap();
        assert_eq!(settings.len(), 6);
    }

    #[test]
    fn test_stats_empty_history() {
        let conn = setup_db();
        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.avg_wpm, 0.0);
        assert_eq!(stats.words_this_week, 0);
        assert_eq!(stats.time_saved_minutes, 0.0);
    }

    #[test]
    fn test_stats_with_data() {
        let conn = setup_db();
        insert_history(&conn, "one two three", 3, 3000, 60.0).unwrap();
        insert_history(&conn, "four five six seven", 4, 4000, 60.0).unwrap();
        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.avg_wpm, 60.0);
        assert_eq!(stats.words_this_week, 7);
        // time_saved = (7/40 - 7000/60000) = 0.175 - 0.1167 = 0.0583 minutes
        assert!(stats.time_saved_minutes > 0.0);
    }

    #[test]
    fn test_checklist_default_steps() {
        let conn = setup_db();
        let steps = get_checklist(&conn).unwrap();
        assert_eq!(steps.len(), 4);
        assert!(!steps[0].completed);
    }

    #[test]
    fn test_complete_checklist_step() {
        let conn = setup_db();
        complete_checklist_step(&conn, "start_recording").unwrap();
        let steps = get_checklist(&conn).unwrap();
        let step = steps.iter().find(|s| s.step_id == "start_recording").unwrap();
        assert!(step.completed);
        assert!(step.completed_at.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cd src-tauri && cargo test --lib db::tests 2>&1 | tail -20
```

Expected: All tests FAIL with `not yet implemented` panics from `todo!()`.

- [ ] **Step 3: Implement all db functions**

Replace the placeholder functions in `src-tauri/src/db.rs` (keep the types and tests, replace everything between `// -- Placeholder functions` and `#[cfg(test)]`):

```rust
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            text        TEXT NOT NULL,
            word_count  INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            wpm         REAL NOT NULL,
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS vocabulary (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            term TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS checklist (
            step_id      TEXT PRIMARY KEY,
            completed    BOOLEAN DEFAULT 0,
            completed_at DATETIME
        );

        INSERT OR IGNORE INTO settings (key, value) VALUES
            ('hotkey', 'RAlt'),
            ('api_url', 'http://172.16.1.222:8028/v1'),
            ('api_key', 'cant-be-empty'),
            ('model_id', 'deepdml/faster-whisper-large-v3-turbo-ct2'),
            ('mic_device', 'default'),
            ('typing_speed_wpm', '40');

        INSERT OR IGNORE INTO checklist (step_id) VALUES
            ('start_recording'),
            ('customize_shortcuts'),
            ('add_vocabulary'),
            ('configure_api');
        "
    )
}

pub fn insert_history(conn: &Connection, text: &str, word_count: i32, duration_ms: i64, wpm: f64) -> Result<()> {
    conn.execute(
        "INSERT INTO history (text, word_count, duration_ms, wpm) VALUES (?1, ?2, ?3, ?4)",
        params![text, word_count, duration_ms, wpm],
    )?;
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY created_at DESC LIMIT 500)",
        [],
    )?;
    Ok(())
}

pub fn get_history(conn: &Connection, limit: i32) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, word_count, duration_ms, wpm, created_at FROM history ORDER BY created_at DESC LIMIT ?1"
    )?;
    let entries = stmt.query_map(params![limit], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            text: row.get(1)?,
            word_count: row.get(2)?,
            duration_ms: row.get(3)?,
            wpm: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_stats(conn: &Connection) -> Result<Stats> {
    let avg_wpm: f64 = conn
        .query_row("SELECT COALESCE(AVG(wpm), 0.0) FROM history", [], |row| row.get(0))?;

    let words_this_week: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM history WHERE created_at >= date('now', 'weekday 0', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    let time_saved_minutes: f64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count / 40.0 - duration_ms / 60000.0), 0.0) FROM history WHERE created_at >= date('now', 'weekday 0', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    Ok(Stats {
        avg_wpm,
        words_this_week,
        time_saved_minutes,
    })
}

pub fn add_vocabulary(conn: &Connection, term: &str) -> Result<()> {
    conn.execute("INSERT INTO vocabulary (term) VALUES (?1)", params![term])?;
    Ok(())
}

pub fn remove_vocabulary(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM vocabulary WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_vocabulary(conn: &Connection) -> Result<Vec<VocabularyEntry>> {
    let mut stmt = conn.prepare("SELECT id, term FROM vocabulary ORDER BY id")?;
    let entries = stmt.query_map([], |row| {
        Ok(VocabularyEntry {
            id: row.get(0)?,
            term: row.get(1)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| row.get(0))
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
    let entries = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_checklist(conn: &Connection) -> Result<Vec<ChecklistStep>> {
    let mut stmt = conn.prepare("SELECT step_id, completed, completed_at FROM checklist ORDER BY rowid")?;
    let entries = stmt.query_map([], |row| {
        Ok(ChecklistStep {
            step_id: row.get(0)?,
            completed: row.get(1)?,
            completed_at: row.get(2)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn complete_checklist_step(conn: &Connection, step_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE checklist SET completed = 1, completed_at = CURRENT_TIMESTAMP WHERE step_id = ?1",
        params![step_id],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test --lib db::tests 2>&1
```

Expected: All 11 tests PASS.

- [ ] **Step 5: Register the module in `lib.rs`**

Add `mod db;` to `src-tauri/src/lib.rs`:

```rust
mod db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Verify full build still works**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: add SQLite database layer with full test coverage

Schema: history (rolling 500), vocabulary, settings (key-value),
checklist. Default settings seeded on init. Stats computed at
query time from history table."
```

---

## Task 3: Sound Effects Module

**Files:**
- Create: `src-tauri/src/sounds.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod sounds`)

- [ ] **Step 1: Create `src-tauri/src/sounds.rs`**

```rust
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::OnceLock;

struct SoundData {
    start: Vec<u8>,
    stop: Vec<u8>,
    error: Vec<u8>,
}

static SOUND_DATA: OnceLock<SoundData> = OnceLock::new();

pub fn init_sounds(resource_dir: std::path::PathBuf) {
    let load = |name: &str| -> Vec<u8> {
        let path = resource_dir.join("sounds").join(name);
        std::fs::read(&path).unwrap_or_else(|e| {
            eprintln!("Warning: could not load sound {}: {}", path.display(), e);
            Vec::new()
        })
    };

    SOUND_DATA.get_or_init(|| SoundData {
        start: load("start.wav"),
        stop: load("stop.wav"),
        error: load("error.wav"),
    });
}

fn play_bytes(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let bytes = bytes.to_vec();
    std::thread::spawn(move || {
        let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = Sink::try_new(&stream_handle) else {
            return;
        };
        let cursor = Cursor::new(bytes);
        let Ok(source) = Decoder::new(cursor) else {
            return;
        };
        sink.append(source);
        sink.sleep_until_end();
    });
}

pub fn play_start() {
    if let Some(data) = SOUND_DATA.get() {
        play_bytes(&data.start);
    }
}

pub fn play_stop() {
    if let Some(data) = SOUND_DATA.get() {
        play_bytes(&data.stop);
    }
}

pub fn play_error() {
    if let Some(data) = SOUND_DATA.get() {
        play_bytes(&data.error);
    }
}
```

- [ ] **Step 2: Add `mod sounds` to `lib.rs`**

```rust
mod db;
mod sounds;
```

- [ ] **Step 3: Verify build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sounds.rs src-tauri/src/lib.rs
git commit -m "feat: add sound effects module

Loads start/stop/error WAV files from resource dir. Plays on
background thread via rodio. Gracefully handles missing files."
```

---

## Task 4: Audio Capture Module

**Files:**
- Create: `src-tauri/src/audio.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod audio`)

- [ ] **Step 1: Write test for WAV encoding**

Create `src-tauri/src/audio.rs` with the encoding function and test first:

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleRate, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use serde::Serialize;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<f32>>>,
    stream: Option<Stream>,
    sample_rate: u32,
}

pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).expect("Failed to create WAV writer");
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(int_sample).expect("Failed to write sample");
        }
        writer.finalize().expect("Failed to finalize WAV");
    }
    cursor.into_inner()
}

pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn list_input_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| {
                    let name = d.name().ok()?;
                    Some(AudioDevice {
                        is_default: name == default_name,
                        name,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_device_by_name(name: &str) -> Option<Device> {
    let host = cpal::default_host();
    if name == "default" {
        return host.default_input_device();
    }
    host.input_devices().ok()?.find(|d| d.name().ok().as_deref() == Some(name))
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 16000,
        }
    }

    pub fn start(&mut self, device_name: &str) -> Result<(), String> {
        let device = get_device_by_name(device_name)
            .ok_or_else(|| format!("Audio device not found: {}", device_name))?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        };

        // Try 16kHz mono; fall back to device default if unsupported
        let (config, sample_rate) = match device.supported_input_configs() {
            Ok(mut configs) => {
                if configs.any(|c| {
                    c.channels() == 1
                        && c.min_sample_rate().0 <= 16000
                        && c.max_sample_rate().0 >= 16000
                }) {
                    (config, 16000)
                } else {
                    let default_config = device.default_input_config().map_err(|e| e.to_string())?;
                    let sr = default_config.sample_rate().0;
                    (
                        StreamConfig {
                            channels: 1,
                            sample_rate: SampleRate(sr),
                            buffer_size: cpal::BufferSize::Default,
                        },
                        sr,
                    )
                }
            }
            Err(_) => (config, 16000),
        };

        self.sample_rate = sample_rate;
        self.buffer.lock().unwrap().clear();

        let buffer = Arc::clone(&self.buffer);
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    buffer.lock().unwrap().extend_from_slice(data);
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn get_current_level(&self) -> f32 {
        let buffer = self.buffer.lock().unwrap();
        if buffer.len() < 800 {
            return 0.0;
        }
        compute_rms(&buffer[buffer.len() - 800..])
    }

    pub fn stop(&mut self) -> (Vec<u8>, u64) {
        self.stream = None; // Drops the stream, stopping recording
        let samples: Vec<f32> = std::mem::take(&mut *self.buffer.lock().unwrap());
        let duration_ms = if self.sample_rate > 0 {
            (samples.len() as u64 * 1000) / self.sample_rate as u64
        } else {
            0
        };
        let wav = encode_wav(&samples, self.sample_rate);
        (wav, duration_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_produces_valid_header() {
        let samples = vec![0.0f32; 16000]; // 1 second of silence at 16kHz
        let wav = encode_wav(&samples, 16000);
        // WAV files start with "RIFF"
        assert_eq!(&wav[0..4], b"RIFF");
        // Format chunk contains "WAVE"
        assert_eq!(&wav[8..12], b"WAVE");
        // Data is present (header is 44 bytes for standard WAV)
        assert!(wav.len() > 44);
        // 16000 samples * 2 bytes per sample (16-bit) + 44 byte header
        assert_eq!(wav.len(), 16000 * 2 + 44);
    }

    #[test]
    fn test_encode_wav_clamps_values() {
        let samples = vec![2.0, -2.0, 0.5, -0.5];
        let wav = encode_wav(&samples, 16000);
        assert_eq!(&wav[0..4], b"RIFF");
        // Should not panic — values outside [-1, 1] are clamped
    }

    #[test]
    fn test_encode_wav_empty() {
        let samples: Vec<f32> = vec![];
        let wav = encode_wav(&samples, 16000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(wav.len(), 44); // Header only, no data samples
    }

    #[test]
    fn test_compute_rms_silence() {
        let samples = vec![0.0f32; 100];
        assert_eq!(compute_rms(&samples), 0.0);
    }

    #[test]
    fn test_compute_rms_known_value() {
        // RMS of constant 0.5 = 0.5
        let samples = vec![0.5f32; 100];
        let rms = compute_rms(&samples);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_compute_rms_empty() {
        assert_eq!(compute_rms(&[]), 0.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test --lib audio::tests 2>&1
```

Expected: All 6 tests PASS. (The encoding and RMS functions are self-contained — no hardware needed.)

- [ ] **Step 3: Add `mod audio` to `lib.rs`**

```rust
mod audio;
mod db;
mod sounds;
```

- [ ] **Step 4: Verify full build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/audio.rs src-tauri/src/lib.rs
git commit -m "feat: add audio capture module

cpal-based microphone recording with device enumeration.
WAV encoding via hound (16kHz mono 16-bit). RMS level
computation for waveform visualization. Tests for encoding
and RMS logic."
```

---

## Task 5: Whisper API Client

**Files:**
- Create: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod transcribe`)

- [ ] **Step 1: Write tests for response parsing**

Create `src-tauri/src/transcribe.rs`:

```rust
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe(
    api_url: &str,
    api_key: &str,
    model_id: &str,
    wav_bytes: Vec<u8>,
    vocabulary: &[String],
) -> Result<String, String> {
    let url = format!("{}/audio/transcriptions", api_url);

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", api_key).parse().map_err(|e| format!("Invalid API key header: {}", e))?,
    );

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model_id.to_string());

    if !vocabulary.is_empty() {
        let prompt = vocabulary.join(", ");
        form = form.text("initial_prompt", prompt);
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let result: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(result.text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transcription_response() {
        let json = r#"{"text": " Hello, world! "}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text.trim(), "Hello, world!");
    }

    #[test]
    fn test_parse_empty_text_response() {
        let json = r#"{"text": ""}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "");
    }

    #[test]
    fn test_parse_response_with_extra_fields() {
        // OpenAI-compatible APIs sometimes include extra fields
        let json = r#"{"text": "hello", "language": "en", "duration": 1.5}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "hello");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test --lib transcribe::tests 2>&1
```

Expected: All 3 tests PASS.

- [ ] **Step 3: Add `mod transcribe` to `lib.rs`**

```rust
mod audio;
mod db;
mod sounds;
mod transcribe;
```

- [ ] **Step 4: Verify build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transcribe.rs src-tauri/src/lib.rs
git commit -m "feat: add Whisper API transcription client

Multipart POST to OpenAI-compatible endpoint. Sends WAV file
with model ID and optional vocabulary prompt. 30s timeout.
Tests for response deserialization."
```

---

## Task 6: Clipboard + Paste Module

**Files:**
- Create: `src-tauri/src/paste.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod paste`)

- [ ] **Step 1: Create `src-tauri/src/paste.rs`**

```rust
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

pub fn capture_foreground_window() -> Option<isize> {
    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

pub fn paste_text(text: &str, target_window: Option<isize>) -> Result<(), String> {
    // Set clipboard
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;

    // Restore target window focus
    if let Some(hwnd_val) = target_window {
        let hwnd = HWND(hwnd_val as *mut _);
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        // Brief delay to let the window come to front
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Simulate Ctrl+V
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

    Ok(())
}
```

- [ ] **Step 2: Add `mod paste` to `lib.rs`**

```rust
mod audio;
mod db;
mod paste;
mod sounds;
mod transcribe;
```

- [ ] **Step 3: Verify build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: Build succeeds. (No automated tests for this module — it requires a live Windows desktop. Will be verified during integration testing in Task 8.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/paste.rs src-tauri/src/lib.rs
git commit -m "feat: add clipboard and keyboard emulation module

Captures foreground window handle via Win32 API. Pastes text
by setting clipboard + simulating Ctrl+V via enigo. Restores
focus to target window before pasting."
```

---

## Task 7: App State + Tauri Commands + Tray + Window Management

**Files:**
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs` (major rewrite — add state, commands, tray, window setup)

- [ ] **Step 1: Create `src-tauri/src/state.rs`**

```rust
use crate::audio::AudioRecorder;
use rusqlite::Connection;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Transcribing,
    Displaying,
}

pub struct AppState {
    pub recording_state: Mutex<RecordingState>,
    pub recorder: Mutex<AudioRecorder>,
    pub db: Mutex<Connection>,
    pub target_window: Mutex<Option<isize>>,
}
```

- [ ] **Step 2: Rewrite `src-tauri/src/lib.rs` with commands, tray, and window management**

```rust
mod audio;
mod db;
mod paste;
mod sounds;
mod state;
mod transcribe;

use audio::AudioDevice;
use db::{ChecklistStep, HistoryEntry, Stats, VocabularyEntry};
use state::{AppState, RecordingState};

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};

// -- Tauri Commands --

#[tauri::command]
fn get_stats(state: tauri::State<'_, AppState>) -> Result<Stats, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history(state: tauri::State<'_, AppState>, limit: i32) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_history(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_vocabulary(state: tauri::State<'_, AppState>) -> Result<Vec<VocabularyEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_vocabulary(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_vocabulary_term(state: tauri::State<'_, AppState>, term: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::add_vocabulary(&conn, &term).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_vocabulary_term(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::remove_vocabulary(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Result<Vec<(String, String)>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_all_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_setting(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_checklist(state: tauri::State<'_, AppState>) -> Result<Vec<ChecklistStep>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_checklist(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn complete_checklist_step(state: tauri::State<'_, AppState>, step_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::complete_checklist_step(&conn, &step_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_audio_devices() -> Vec<AudioDevice> {
    audio::list_input_devices()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");
            let db_path = app_data_dir.join("local_super_whisper.db");
            let conn = Connection::open(&db_path).expect("Failed to open database");
            db::init_db(&conn).expect("Failed to initialize database");

            // Initialize sounds
            if let Ok(resource_dir) = app.path().resource_dir() {
                sounds::init_sounds(resource_dir);
            }

            // Manage state
            app.manage(AppState {
                recording_state: Mutex::new(RecordingState::Idle),
                recorder: Mutex::new(audio::AudioRecorder::new()),
                db: Mutex::new(conn),
                target_window: Mutex::new(None),
            });

            // Build tray menu
            let show_item = MenuItemBuilder::with_id("show", "Open Settings").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Local SuperWhisper")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("settings") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Apply window vibrancy to settings window
            if let Some(settings_window) = app.get_webview_window("settings") {
                #[cfg(target_os = "windows")]
                {
                    use window_vibrancy::apply_mica;
                    let _ = apply_mica(&settings_window, Some(true));
                }
                // Hide settings on close instead of quitting
                let settings_window_clone = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_window_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_stats,
            get_history,
            get_vocabulary,
            add_vocabulary_term,
            remove_vocabulary_term,
            get_settings,
            update_setting,
            get_checklist,
            complete_checklist_step,
            get_audio_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Add `mod state` to verify module is reachable**

`state.rs` is already referenced by `mod state;` in the new `lib.rs`.

- [ ] **Step 4: Verify build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -10
```

Expected: Build succeeds. Fix any API mismatches (Tauri v2 API may vary slightly — check compiler errors and adjust).

- [ ] **Step 5: Verify db tests still pass**

Run:
```bash
cd src-tauri && cargo test --lib db::tests 2>&1
```

Expected: All tests still pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "feat: add app state, Tauri commands, tray, and window management

AppState holds recording state, audio recorder, DB connection,
and target window handle. 11 Tauri commands for frontend IPC.
System tray with Open Settings / Quit. Settings window hides
on close. Mica vibrancy applied on Windows."
```

---

## Task 8: Global Hotkey + Recording State Machine

**Files:**
- Create: `src-tauri/src/hotkey.rs`
- Modify: `src-tauri/src/lib.rs` (register hotkey in setup, add hotkey module)

- [ ] **Step 1: Create `src-tauri/src/hotkey.rs`**

```rust
use crate::db;
use crate::paste;
use crate::sounds;
use crate::state::{AppState, RecordingState};
use crate::transcribe;

use tauri::{AppHandle, Emitter, Manager};

pub fn on_hotkey_pressed(app: &AppHandle) {
    let state = app.state::<AppState>();
    let current = state.recording_state.lock().unwrap().clone();

    match current {
        RecordingState::Idle => start_recording(app),
        RecordingState::Recording => stop_recording(app),
        RecordingState::Transcribing | RecordingState::Displaying => {
            // Ignore hotkey during these states
        }
    }
}

fn start_recording(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Capture the currently focused window before overlay appears
    let target = paste::capture_foreground_window();
    *state.target_window.lock().unwrap() = target;

    // Get mic device from settings
    let mic_device = {
        let conn = state.db.lock().unwrap();
        db::get_setting(&conn, "mic_device").unwrap_or_else(|_| "default".to_string())
    };

    // Start audio recording
    {
        let mut recorder = state.recorder.lock().unwrap();
        if let Err(e) = recorder.start(&mic_device) {
            eprintln!("Failed to start recording: {}", e);
            sounds::play_error();
            let _ = app.emit("recording-error", e);
            return;
        }
    }

    *state.recording_state.lock().unwrap() = RecordingState::Recording;
    sounds::play_start();

    // Show overlay and notify frontend
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.center();
        let _ = overlay.show();
    }
    let _ = app.emit("recording-started", ());

    // Start audio level polling
    let app_handle = app.clone();
    std::thread::spawn(move || {
        loop {
            {
                let state = app_handle.state::<AppState>();
                let current = state.recording_state.lock().unwrap().clone();
                if current != RecordingState::Recording {
                    break;
                }
                let level = state.recorder.lock().unwrap().get_current_level();
                let _ = app_handle.emit("audio-level", level);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

fn stop_recording(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Stop recording and get WAV data
    let (wav_bytes, duration_ms) = {
        let mut recorder = state.recorder.lock().unwrap();
        recorder.stop()
    };

    sounds::play_stop();

    // Discard if too short (< 500ms)
    if duration_ms < 500 {
        *state.recording_state.lock().unwrap() = RecordingState::Idle;
        if let Some(overlay) = app.get_webview_window("overlay") {
            let _ = overlay.hide();
        }
        let _ = app.emit("recording-idle", ());
        return;
    }

    *state.recording_state.lock().unwrap() = RecordingState::Transcribing;
    let _ = app.emit("recording-transcribing", ());

    // Get API settings and vocabulary
    let (api_url, api_key, model_id, vocabulary) = {
        let conn = state.db.lock().unwrap();
        let api_url = db::get_setting(&conn, "api_url").unwrap_or_default();
        let api_key = db::get_setting(&conn, "api_key").unwrap_or_default();
        let model_id = db::get_setting(&conn, "model_id").unwrap_or_default();
        let vocab_entries = db::get_vocabulary(&conn).unwrap_or_default();
        let vocabulary: Vec<String> = vocab_entries.into_iter().map(|v| v.term).collect();
        (api_url, api_key, model_id, vocabulary)
    };

    let target_window = *state.target_window.lock().unwrap();

    // Transcribe async
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = transcribe::transcribe(
            &api_url,
            &api_key,
            &model_id,
            wav_bytes,
            &vocabulary,
        )
        .await;

        let state = app_handle.state::<AppState>();

        match result {
            Ok(text) if !text.is_empty() => {
                // Paste text into target window
                if let Err(e) = paste::paste_text(&text, target_window) {
                    eprintln!("Paste error: {}", e);
                }

                // Save to history
                let word_count = text.split_whitespace().count() as i32;
                let wpm = if duration_ms > 0 {
                    (word_count as f64 / duration_ms as f64) * 60000.0
                } else {
                    0.0
                };
                {
                    let conn = state.db.lock().unwrap();
                    let _ = db::insert_history(&conn, &text, word_count, duration_ms as i64, wpm);
                }

                // Show result in overlay
                *state.recording_state.lock().unwrap() = RecordingState::Displaying;
                let _ = app_handle.emit("recording-result", text);

                // Auto-hide after 2.5 seconds
                let app_for_timer = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                    let state = app_for_timer.state::<AppState>();
                    let mut rs = state.recording_state.lock().unwrap();
                    if *rs == RecordingState::Displaying {
                        *rs = RecordingState::Idle;
                        if let Some(overlay) = app_for_timer.get_webview_window("overlay") {
                            let _ = overlay.hide();
                        }
                        let _ = app_for_timer.emit("recording-idle", ());
                    }
                });
            }
            Ok(_) => {
                // Empty transcription
                *state.recording_state.lock().unwrap() = RecordingState::Idle;
                if let Some(overlay) = app_handle.get_webview_window("overlay") {
                    let _ = overlay.hide();
                }
                let _ = app_handle.emit("recording-idle", ());
            }
            Err(e) => {
                eprintln!("Transcription error: {}", e);
                sounds::play_error();
                *state.recording_state.lock().unwrap() = RecordingState::Idle;
                let _ = app_handle.emit("recording-error", e);
                // Auto-hide overlay after 3 seconds on error
                let app_for_timer = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    if let Some(overlay) = app_for_timer.get_webview_window("overlay") {
                        let _ = overlay.hide();
                    }
                    let _ = app_for_timer.emit("recording-idle", ());
                });
            }
        }
    });
}
```

- [ ] **Step 2: Add hotkey registration to `lib.rs` setup**

In `lib.rs`, add `mod hotkey;` at the top, then inside the `.setup(|app| { ... })` closure, after the tray setup, add:

```rust
mod hotkey;
```

And add this block at the end of the `.setup()` closure, before `Ok(())`:

```rust
            // Register global hotkey
            {
                let conn = app.state::<AppState>().db.lock().unwrap();
                let hotkey_str = db::get_setting(&conn, "hotkey").unwrap_or_else(|_| "RAlt".to_string());
                drop(conn);

                let app_handle = app.handle().clone();
                app.global_shortcut().on_shortcut(
                    hotkey_str.parse().unwrap_or_else(|_| "RAlt".parse().unwrap()),
                    move |_app, _shortcut, event| {
                        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            hotkey::on_hotkey_pressed(&app_handle);
                        }
                    },
                )?;
            }
```

- [ ] **Step 3: Verify build**

Run:
```bash
cd src-tauri && cargo build 2>&1 | tail -10
```

Expected: Build succeeds. If there are API issues with `tauri_plugin_global_shortcut`, check the plugin docs for the correct registration API and adjust.

- [ ] **Step 4: Verify all existing tests still pass**

Run:
```bash
cd src-tauri && cargo test --lib 2>&1
```

Expected: All db, audio, and transcribe tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/hotkey.rs src-tauri/src/lib.rs
git commit -m "feat: add global hotkey and recording state machine

State machine: Idle → Recording → Transcribing → Displaying → Idle.
Hotkey triggers state transitions. Coordinates audio capture,
transcription API call, clipboard paste, overlay show/hide,
sound effects, and history logging. Auto-dismiss after 2.5s."
```

---

## Task 9: Overlay Frontend

**Files:**
- Create: `src/overlay/Overlay.tsx`, `src/overlay/Waveform.tsx`, `src/overlay/TranscriptDisplay.tsx`
- Create: `src/hooks/useTauriEvent.ts`
- Modify: `src/App.tsx` (replace placeholder)

- [ ] **Step 1: Create `src/hooks/useTauriEvent.ts`**

```typescript
import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<T>(event, (e) => {
      handler(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [event, handler]);
}
```

- [ ] **Step 2: Create `src/overlay/Waveform.tsx`**

```tsx
import { useCallback, useRef, useEffect, useState } from "react";
import { useTauriEvent } from "../hooks/useTauriEvent";

export default function Waveform() {
  const [levels, setLevels] = useState<number[]>(new Array(24).fill(0));
  const indexRef = useRef(0);

  const handleLevel = useCallback((level: number) => {
    setLevels((prev) => {
      const next = [...prev];
      next[indexRef.current % 24] = Math.min(level * 8, 1); // Normalize and cap
      indexRef.current += 1;
      return next;
    });
  }, []);

  useTauriEvent<number>("audio-level", handleLevel);

  return (
    <div className="flex items-center justify-center gap-[3px] h-12">
      {levels.map((level, i) => (
        <div
          key={i}
          className="w-[4px] rounded-full bg-accent transition-all duration-75"
          style={{
            height: `${Math.max(4, level * 48)}px`,
            opacity: 0.5 + level * 0.5,
          }}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Create `src/overlay/TranscriptDisplay.tsx`**

```tsx
import { useEffect, useState } from "react";

interface Props {
  text: string;
}

export default function TranscriptDisplay({ text }: Props) {
  const [opacity, setOpacity] = useState(1);

  useEffect(() => {
    // Start fade out after 1.5 seconds
    const timer = setTimeout(() => setOpacity(0), 1500);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div
      className="text-text-primary text-sm leading-relaxed px-2 max-h-20 overflow-hidden transition-opacity duration-1000"
      style={{ opacity }}
    >
      {text}
    </div>
  );
}
```

- [ ] **Step 4: Create `src/overlay/Overlay.tsx`**

```tsx
import { useCallback, useState } from "react";
import { useTauriEvent } from "../hooks/useTauriEvent";
import Waveform from "./Waveform";
import TranscriptDisplay from "./TranscriptDisplay";

type OverlayState = "idle" | "recording" | "transcribing" | "result" | "error";

export default function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");
  const [resultText, setResultText] = useState("");
  const [errorText, setErrorText] = useState("");

  useTauriEvent("recording-started", useCallback(() => setState("recording"), []));
  useTauriEvent("recording-transcribing", useCallback(() => setState("transcribing"), []));
  useTauriEvent<string>("recording-result", useCallback((text: string) => {
    setResultText(text);
    setState("result");
  }, []));
  useTauriEvent<string>("recording-error", useCallback((err: string) => {
    setErrorText(err);
    setState("error");
  }, []));
  useTauriEvent("recording-idle", useCallback(() => setState("idle"), []));

  if (state === "idle") {
    return null;
  }

  return (
    <div className="flex items-center justify-center w-full h-full">
      <div className="bg-surface-dark/80 backdrop-blur-xl rounded-2xl px-6 py-4 min-w-[280px] max-w-[400px] shadow-2xl border border-white/5">
        {state === "recording" && (
          <div className="text-center">
            <Waveform />
            <p className="text-text-secondary text-xs mt-2">Listening...</p>
          </div>
        )}

        {state === "transcribing" && (
          <div className="text-center py-2">
            <div className="inline-block w-5 h-5 border-2 border-accent border-t-transparent rounded-full animate-spin" />
            <p className="text-text-secondary text-xs mt-2">Transcribing...</p>
          </div>
        )}

        {state === "result" && <TranscriptDisplay text={resultText} />}

        {state === "error" && (
          <div className="text-center py-2">
            <p className="text-red-400 text-xs">{errorText || "Transcription failed"}</p>
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Update `src/App.tsx`**

```tsx
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import Overlay from "./overlay/Overlay";

function SettingsPlaceholder() {
  return <div className="text-white text-center p-4">Settings</div>;
}

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<Overlay />} />
        <Route path="/settings/*" element={<SettingsPlaceholder />} />
        <Route path="*" element={<Navigate to="/settings" replace />} />
      </Routes>
    </HashRouter>
  );
}
```

- [ ] **Step 6: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds with no TypeScript errors.

- [ ] **Step 7: Commit**

```bash
git add src/overlay/ src/hooks/ src/App.tsx
git commit -m "feat: add overlay frontend with waveform and transcript display

Three overlay states: recording (animated waveform bars),
transcribing (spinner), result (text with fade-out). Driven
entirely by Tauri events from Rust backend. Non-focusable
frosted-glass overlay."
```

---

## Task 10: Settings Layout + Navigation

**Files:**
- Create: `src/settings/Layout.tsx`
- Modify: `src/App.tsx` (wire up settings routes)

- [ ] **Step 1: Create `src/settings/Layout.tsx`**

```tsx
import { NavLink, Outlet } from "react-router-dom";

const navItems = [
  { to: "/settings", label: "Home", icon: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0a1 1 0 01-1-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 01-1 1" },
  { to: "/settings/vocabulary", label: "Vocabulary", icon: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" },
  { to: "/settings/configuration", label: "Configuration", icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" },
  { to: "/settings/history", label: "History", icon: "M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" },
];

function NavIcon({ d }: { d: string }) {
  return (
    <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d={d} />
    </svg>
  );
}

export default function Layout() {
  return (
    <div className="flex h-screen bg-surface">
      {/* Sidebar */}
      <nav className="w-52 bg-surface-dark flex flex-col py-4 px-2 border-r border-white/5">
        <div className="px-3 mb-6">
          <h1 className="text-text-primary font-semibold text-sm">Local SuperWhisper</h1>
        </div>
        <div className="flex flex-col gap-1">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/settings"}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive
                    ? "bg-accent-muted text-accent"
                    : "text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                }`
              }
            >
              <NavIcon d={item.icon} />
              {item.label}
            </NavLink>
          ))}
        </div>
      </nav>

      {/* Content */}
      <main className="flex-1 overflow-y-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}
```

- [ ] **Step 2: Create placeholder page components**

Create `src/settings/Home.tsx`:
```tsx
export default function Home() {
  return <div className="text-text-primary">Home — coming next</div>;
}
```

Create `src/settings/Vocabulary.tsx`:
```tsx
export default function Vocabulary() {
  return <div className="text-text-primary">Vocabulary — coming soon</div>;
}
```

Create `src/settings/Configuration.tsx`:
```tsx
export default function Configuration() {
  return <div className="text-text-primary">Configuration — coming soon</div>;
}
```

Create `src/settings/History.tsx`:
```tsx
export default function History() {
  return <div className="text-text-primary">History — coming soon</div>;
}
```

- [ ] **Step 3: Update `src/App.tsx` with full routing**

```tsx
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import Overlay from "./overlay/Overlay";
import Layout from "./settings/Layout";
import Home from "./settings/Home";
import Vocabulary from "./settings/Vocabulary";
import Configuration from "./settings/Configuration";
import History from "./settings/History";

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<Overlay />} />
        <Route path="/settings" element={<Layout />}>
          <Route index element={<Home />} />
          <Route path="vocabulary" element={<Vocabulary />} />
          <Route path="configuration" element={<Configuration />} />
          <Route path="history" element={<History />} />
        </Route>
        <Route path="*" element={<Navigate to="/settings" replace />} />
      </Routes>
    </HashRouter>
  );
}
```

- [ ] **Step 4: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/settings/ src/App.tsx
git commit -m "feat: add settings layout with sidebar navigation

Dark-themed sidebar with Home, Vocabulary, Configuration, History
nav items. SVG icons, active state with purple accent. Outlet
renders child routes. Placeholder page components."
```

---

## Task 11: Settings — Home Tab (Stats + Checklist + Recent History)

**Files:**
- Create: `src/components/StatCard.tsx`, `src/components/ChecklistItem.tsx`
- Modify: `src/settings/Home.tsx`

- [ ] **Step 1: Create `src/components/StatCard.tsx`**

```tsx
interface Props {
  label: string;
  value: string | number;
  sublabel?: string;
}

export default function StatCard({ label, value, sublabel }: Props) {
  return (
    <div className="bg-surface-dark rounded-xl p-4 border border-white/5">
      <p className="text-2xl font-bold text-text-primary">{value}</p>
      <p className="text-xs text-text-secondary mt-1">{label}</p>
      {sublabel && <p className="text-xs text-text-muted">{sublabel}</p>}
    </div>
  );
}
```

- [ ] **Step 2: Create `src/components/ChecklistItem.tsx`**

```tsx
interface Props {
  label: string;
  description: string;
  completed: boolean;
  onComplete: () => void;
}

export default function ChecklistItem({ label, description, completed, onComplete }: Props) {
  return (
    <button
      onClick={completed ? undefined : onComplete}
      className={`flex items-start gap-3 w-full text-left p-3 rounded-lg transition-colors ${
        completed ? "opacity-50" : "hover:bg-surface-hover"
      }`}
      disabled={completed}
    >
      <div
        className={`mt-0.5 w-5 h-5 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
          completed ? "border-accent bg-accent" : "border-text-muted"
        }`}
      >
        {completed && (
          <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
          </svg>
        )}
      </div>
      <div>
        <p className="text-sm text-text-primary font-medium">{label}</p>
        <p className="text-xs text-text-secondary">{description}</p>
      </div>
    </button>
  );
}
```

- [ ] **Step 3: Implement `src/settings/Home.tsx`**

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import StatCard from "../components/StatCard";
import ChecklistItem from "../components/ChecklistItem";

interface Stats {
  avg_wpm: number;
  words_this_week: number;
  time_saved_minutes: number;
}

interface HistoryEntry {
  id: number;
  text: string;
  word_count: number;
  duration_ms: number;
  wpm: number;
  created_at: string;
}

interface ChecklistStep {
  step_id: string;
  completed: boolean;
  completed_at: string | null;
}

const CHECKLIST_META: Record<string, { label: string; description: string }> = {
  start_recording: { label: "Start recording", description: "Tap your voice to text with your hotkey." },
  customize_shortcuts: { label: "Customize your shortcuts", description: "Change the keyboard shortcut for SuperWhisper." },
  add_vocabulary: { label: "Add vocabulary", description: "Teach SuperWhisper custom words, names, or industry terms." },
  configure_api: { label: "Configure API", description: "Set up your Faster-Whisper endpoint." },
};

export default function Home() {
  const [stats, setStats] = useState<Stats>({ avg_wpm: 0, words_this_week: 0, time_saved_minutes: 0 });
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [checklist, setChecklist] = useState<ChecklistStep[]>([]);

  const loadData = async () => {
    const [s, h, c] = await Promise.all([
      invoke<Stats>("get_stats"),
      invoke<HistoryEntry[]>("get_history", { limit: 5 }),
      invoke<ChecklistStep[]>("get_checklist"),
    ]);
    setStats(s);
    setHistory(h);
    setChecklist(c);
  };

  useEffect(() => {
    loadData();
  }, []);

  const completeStep = async (stepId: string) => {
    await invoke("complete_checklist_step", { stepId });
    loadData();
  };

  return (
    <div className="space-y-8">
      {/* Stats */}
      <div>
        <div className="grid grid-cols-3 gap-4">
          <StatCard label="Average speed" value={`${Math.round(stats.avg_wpm)} WPM`} />
          <StatCard label="Words this week" value={stats.words_this_week.toLocaleString()} />
          <StatCard
            label="Time saved"
            value={`${Math.max(0, Math.round(stats.time_saved_minutes))} min`}
            sublabel="this week"
          />
        </div>
      </div>

      {/* Get Started */}
      <div>
        <h2 className="text-text-primary font-semibold text-sm mb-3">Get started</h2>
        <div className="space-y-1">
          {checklist.map((step) => {
            const meta = CHECKLIST_META[step.step_id];
            if (!meta) return null;
            return (
              <ChecklistItem
                key={step.step_id}
                label={meta.label}
                description={meta.description}
                completed={step.completed}
                onComplete={() => completeStep(step.step_id)}
              />
            );
          })}
        </div>
      </div>

      {/* Recent History */}
      <div>
        <h2 className="text-text-primary font-semibold text-sm mb-3">Recent transcriptions</h2>
        {history.length === 0 ? (
          <p className="text-text-muted text-sm">No transcriptions yet. Press your hotkey to get started.</p>
        ) : (
          <div className="space-y-2">
            {history.map((entry) => (
              <div key={entry.id} className="bg-surface-dark rounded-lg p-3 border border-white/5">
                <p className="text-sm text-text-primary line-clamp-2">{entry.text}</p>
                <div className="flex gap-4 mt-1">
                  <span className="text-xs text-text-muted">{entry.word_count} words</span>
                  <span className="text-xs text-text-muted">{Math.round(entry.wpm)} WPM</span>
                  <span className="text-xs text-text-muted">{new Date(entry.created_at).toLocaleString()}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/components/ src/settings/Home.tsx
git commit -m "feat: add Home tab with stats, checklist, and recent history

Three stat cards (WPM, words this week, time saved). Get Started
checklist with completion state. Recent 5 transcriptions with
word count, WPM, and timestamp."
```

---

## Task 12: Settings — Configuration Tab

**Files:**
- Modify: `src/settings/Configuration.tsx`

- [ ] **Step 1: Implement `src/settings/Configuration.tsx`**

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export default function Configuration() {
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    loadSettings();
    loadDevices();
  }, []);

  const loadSettings = async () => {
    const pairs = await invoke<[string, string][]>("get_settings");
    const map: Record<string, string> = {};
    pairs.forEach(([k, v]) => (map[k] = v));
    setSettings(map);
  };

  const loadDevices = async () => {
    const d = await invoke<AudioDevice[]>("get_audio_devices");
    setDevices(d);
  };

  const update = async (key: string, value: string) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
    await invoke("update_setting", { key, value });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="space-y-8 max-w-lg">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary font-semibold">Configuration</h2>
        {saved && <span className="text-xs text-green-400">Saved</span>}
      </div>

      {/* Hotkey */}
      <Field label="Hotkey" description="Global keyboard shortcut to start/stop recording.">
        <select
          value={settings.hotkey || "RAlt"}
          onChange={(e) => update("hotkey", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="RAlt">Right Alt</option>
          <option value="LAlt">Left Alt</option>
          <option value="RControl">Right Ctrl</option>
          <option value="LControl">Left Ctrl</option>
          <option value="F9">F9</option>
          <option value="F10">F10</option>
          <option value="F11">F11</option>
          <option value="F12">F12</option>
        </select>
      </Field>

      {/* API URL */}
      <Field label="API URL" description="Faster-Whisper server endpoint.">
        <input
          type="text"
          value={settings.api_url || ""}
          onChange={(e) => update("api_url", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="http://172.16.1.222:8028/v1"
        />
      </Field>

      {/* API Key */}
      <Field label="API Key" description="Authorization key for the Whisper API.">
        <input
          type="password"
          value={settings.api_key || ""}
          onChange={(e) => update("api_key", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="cant-be-empty"
        />
      </Field>

      {/* Model ID */}
      <Field label="Model ID" description="Whisper model identifier on the server.">
        <input
          type="text"
          value={settings.model_id || ""}
          onChange={(e) => update("model_id", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="deepdml/faster-whisper-large-v3-turbo-ct2"
        />
      </Field>

      {/* Microphone */}
      <Field label="Microphone" description="Audio input device for recording.">
        <select
          value={settings.mic_device || "default"}
          onChange={(e) => update("mic_device", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="default">System Default</option>
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name} {d.is_default ? "(default)" : ""}
            </option>
          ))}
        </select>
      </Field>
    </div>
  );
}

function Field({ label, description, children }: { label: string; description: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-sm font-medium text-text-primary mb-1">{label}</label>
      <p className="text-xs text-text-muted mb-2">{description}</p>
      {children}
    </div>
  );
}
```

- [ ] **Step 2: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/settings/Configuration.tsx
git commit -m "feat: add Configuration tab

Hotkey selector (Right Alt default), API URL, API key (masked),
model ID, and microphone device dropdown populated from cpal.
Settings saved immediately on change."
```

---

## Task 13: Settings — Vocabulary Tab

**Files:**
- Modify: `src/settings/Vocabulary.tsx`

- [ ] **Step 1: Implement `src/settings/Vocabulary.tsx`**

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VocabularyEntry {
  id: number;
  term: string;
}

export default function Vocabulary() {
  const [terms, setTerms] = useState<VocabularyEntry[]>([]);
  const [newTerm, setNewTerm] = useState("");
  const [error, setError] = useState("");

  const loadTerms = async () => {
    const t = await invoke<VocabularyEntry[]>("get_vocabulary");
    setTerms(t);
  };

  useEffect(() => {
    loadTerms();
  }, []);

  const addTerm = async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    try {
      await invoke("add_vocabulary_term", { term: trimmed });
      setNewTerm("");
      setError("");
      loadTerms();
    } catch {
      setError("Term already exists.");
    }
  };

  const removeTerm = async (id: number) => {
    await invoke("remove_vocabulary_term", { id });
    loadTerms();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      addTerm();
    }
  };

  return (
    <div className="space-y-6 max-w-lg">
      <div>
        <h2 className="text-text-primary font-semibold">Vocabulary</h2>
        <p className="text-xs text-text-muted mt-1">
          Custom words, names, and terms to improve transcription accuracy.
          These are injected into Whisper's initial prompt.
        </p>
      </div>

      {/* Add term */}
      <div className="flex gap-2">
        <input
          type="text"
          value={newTerm}
          onChange={(e) => setNewTerm(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="Add a word or phrase..."
        />
        <button
          onClick={addTerm}
          disabled={!newTerm.trim()}
          className="px-4 py-2 bg-accent hover:bg-accent-hover disabled:opacity-40 rounded-lg text-sm text-white font-medium transition-colors"
        >
          Add
        </button>
      </div>
      {error && <p className="text-xs text-red-400">{error}</p>}

      {/* Term list */}
      {terms.length === 0 ? (
        <p className="text-text-muted text-sm">No vocabulary terms yet.</p>
      ) : (
        <div className="space-y-1">
          {terms.map((entry) => (
            <div
              key={entry.id}
              className="flex items-center justify-between bg-surface-dark rounded-lg px-3 py-2 border border-white/5 group"
            >
              <span className="text-sm text-text-primary">{entry.term}</span>
              <button
                onClick={() => removeTerm(entry.id)}
                className="text-text-muted hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity text-xs"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/settings/Vocabulary.tsx
git commit -m "feat: add Vocabulary tab

Add/remove custom vocabulary terms. Enter key submits.
Duplicate detection with error message. Terms shown with
hover-reveal remove button."
```

---

## Task 14: Settings — History Tab

**Files:**
- Modify: `src/settings/History.tsx`

- [ ] **Step 1: Implement `src/settings/History.tsx`**

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HistoryEntry {
  id: number;
  text: string;
  word_count: number;
  duration_ms: number;
  wpm: number;
  created_at: string;
}

export default function History() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedId, setCopiedId] = useState<number | null>(null);

  useEffect(() => {
    invoke<HistoryEntry[]>("get_history", { limit: 500 }).then(setEntries);
  }, []);

  const copyText = async (entry: HistoryEntry) => {
    await navigator.clipboard.writeText(entry.text);
    setCopiedId(entry.id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const formatDuration = (ms: number) => {
    const seconds = Math.round(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary font-semibold">History</h2>
        <span className="text-xs text-text-muted">{entries.length} entries (max 500)</span>
      </div>

      {entries.length === 0 ? (
        <p className="text-text-muted text-sm">No transcriptions yet.</p>
      ) : (
        <div className="space-y-2">
          {entries.map((entry) => (
            <button
              key={entry.id}
              onClick={() => copyText(entry)}
              className="w-full text-left bg-surface-dark rounded-lg p-3 border border-white/5 hover:border-accent/30 transition-colors group"
            >
              <p className="text-sm text-text-primary">{entry.text}</p>
              <div className="flex items-center gap-4 mt-2">
                <span className="text-xs text-text-muted">{entry.word_count} words</span>
                <span className="text-xs text-text-muted">{Math.round(entry.wpm)} WPM</span>
                <span className="text-xs text-text-muted">{formatDuration(entry.duration_ms)}</span>
                <span className="text-xs text-text-muted">{new Date(entry.created_at).toLocaleString()}</span>
                <span className="text-xs text-accent ml-auto opacity-0 group-hover:opacity-100 transition-opacity">
                  {copiedId === entry.id ? "Copied!" : "Click to copy"}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify frontend builds**

Run:
```bash
npm run build 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/settings/History.tsx
git commit -m "feat: add History tab

Scrollable list of all transcriptions (up to 500). Shows text,
word count, WPM, duration, timestamp. Click to copy to clipboard
with confirmation feedback."
```

---

## Task 15: Integration Build + Polish

**Files:**
- Possibly modify: `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, various frontend files

- [ ] **Step 1: Run full Tauri dev build**

Run:
```bash
cargo tauri build --debug 2>&1 | tail -30
```

Expected: Build succeeds. If there are compile errors, fix them now. Common issues:
- Tauri v2 API differences (method names, trait bounds)
- `windows` crate feature flags
- `enigo` API changes between versions
- Missing `Serialize`/`Deserialize` derives

Fix any issues before proceeding.

- [ ] **Step 2: Run all Rust tests**

Run:
```bash
cd src-tauri && cargo test --lib 2>&1
```

Expected: All tests in `db::tests`, `audio::tests`, and `transcribe::tests` pass.

- [ ] **Step 3: Run TypeScript type check**

Run:
```bash
npx tsc --noEmit 2>&1
```

Expected: No type errors.

- [ ] **Step 4: Manual smoke test — tray icon**

Run the app in dev mode:
```bash
cargo tauri dev
```

Verify:
- App starts with no visible windows
- Tray icon appears in the Windows system tray
- Right-clicking the tray shows "Open Settings" and "Quit" menu items
- Clicking "Open Settings" shows the settings window
- Closing the settings window hides it (doesn't quit the app)
- Double-clicking the tray icon opens settings
- Clicking "Quit" exits the app

- [ ] **Step 5: Manual smoke test — settings window**

With the app running (`cargo tauri dev`):

Verify:
- Settings window shows the dark-themed sidebar with 4 nav items
- Home tab displays stats (all zeros initially), empty recent history, and the Get Started checklist
- Configuration tab shows all fields populated with defaults (Right Alt, API URL, etc.)
- Vocabulary tab lets you add/remove terms
- History tab shows "No transcriptions yet"
- Navigation between tabs works

- [ ] **Step 6: Manual smoke test — recording workflow**

With the app running and Faster-Whisper server accessible:

Verify:
- Press Right Alt → overlay appears center-screen with animated waveform + start sound
- Speak a few words
- Press Right Alt again → stop sound, overlay shows "Transcribing..."
- Text appears in overlay, then is pasted into the previously focused window
- Overlay auto-dismisses after ~2.5 seconds
- History tab now shows the transcription
- Home tab stats are updated

If the Faster-Whisper server is not available, verify that an error is shown in the overlay and auto-dismissed after 3 seconds.

- [ ] **Step 7: Fix any issues found during smoke testing**

Address any bugs or UX issues found in steps 4-6. Common fixes:
- Overlay window not centering → check `center: true` in tauri.conf.json
- Overlay stealing focus → may need to add Windows API call to set `WS_EX_NOACTIVATE` extended window style
- Paste not working → check `enigo` permissions, timing delay
- Sound not playing → check resource bundling in tauri.conf.json

- [ ] **Step 8: Commit any fixes**

```bash
git add -A
git commit -m "fix: address integration issues from smoke testing"
```

(Only create this commit if there were actual fixes. Skip if everything worked.)

- [ ] **Step 9: Final commit — verify clean state**

Run:
```bash
git status && cargo tauri build --debug 2>&1 | tail -5
```

Expected: Working tree clean (or only expected untracked files), build succeeds.
