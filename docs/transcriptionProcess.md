# Transcription Process — localSuperWhisper

This document describes the complete voice-to-text pipeline as implemented in this application. It is written for an AI coding agent that needs to replicate this same backend integration in a different application.

---

## Overview

The pipeline has five stages:

```
Microphone → Audio Capture → WAV Encoding → HTTP API Request → Text Post-Processing → Delivery
```

The backend transcription service is a **self-hosted Faster-Whisper server** that exposes an **OpenAI-compatible `/v1/audio/transcriptions` endpoint**. Any application that can make an HTTP multipart POST request can use it — no special SDK is required.

---

## Stage 1 — Audio Capture

**Source file:** `src-tauri/src/audio.rs`  
**Crate:** [`cpal`](https://crates.io/crates/cpal) v0.15

### Device selection

The app reads the setting `mic_device` from SQLite (default: `"default"`). If the value is `"default"`, it uses `cpal`'s `default_host().default_input_device()`. Otherwise it searches all input devices by exact name match.

### Sample format

The app prefers **16 kHz, mono, f32 samples**. Before opening the stream it checks whether the device's supported configs include `channels == 1` and a sample rate range covering 16000:

```rust
let supports_16k_mono = device.supported_input_configs().ok().map(|mut cfgs| {
    cfgs.any(|c| c.channels() == 1 && c.min_sample_rate().0 <= 16000 && c.max_sample_rate().0 >= 16000)
}).unwrap_or(false);
```

If that check passes, the stream opens with `StreamConfig { channels: 1, sample_rate: SampleRate(16000), buffer_size: Default }`. If not, it falls back to `device.default_input_config()` (whatever the device natively supports — may be stereo or a different sample rate).

### Buffer accumulation

Incoming `f32` samples from the cpal callback are appended to a `Vec<f32>` behind a `Mutex<Vec<f32>>`. There is no maximum buffer size — the entire recording is held in RAM.

### Audio level feedback

A background thread polls `get_current_level()` every 50 ms while recording. That function takes the last 800 samples (50 ms at 16 kHz) from the tail of the buffer, downmixes multichannel to mono by averaging, then computes RMS:

```rust
let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
(sum_sq / samples.len() as f32).sqrt()
```

The result (range 0.0–1.0) is emitted to the frontend as the `audio-level` Tauri event.

---

## Stage 2 — WAV Encoding

**Source file:** `src-tauri/src/audio.rs` — `encode_wav()` and `AudioRecorder::stop()`  
**Crate:** [`hound`](https://crates.io/crates/hound) v3.5

When recording stops, the cpal stream is dropped (which stops the device). The accumulated `Vec<f32>` is taken out of the mutex.

### Multichannel downmix

If the device captured in stereo (or more channels), the interleaved samples are averaged per-frame to produce mono:

```rust
let mono: Vec<f32> = samples.chunks(ch).map(|frame| frame.iter().sum::<f32>() / ch as f32).collect();
```

### WAV container

`encode_wav()` writes a standard RIFF/WAV container in memory using `hound::WavWriter`:

```
WavSpec {
    channels: 1,
    sample_rate: <device sample rate, typically 16000>,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
}
```

Each f32 sample is clamped to `[-1.0, 1.0]` then converted to i16:

```rust
let int_sample = (clamped * i16::MAX as f32) as i16;
```

The result is a `Vec<u8>` — a fully valid PCM WAV file in memory, with a 44-byte RIFF header followed by 2 bytes per sample.

**Minimum duration guard:** If the recording is shorter than 500 ms, it is discarded before any API call is made.

---

## Stage 3 — HTTP API Request

**Source file:** `src-tauri/src/transcribe.rs`  
**Crate:** [`reqwest`](https://crates.io/crates/reqwest) v0.12 with features `multipart`, `json`

### Endpoint

```
POST {api_url}/audio/transcriptions
```

The base URL `api_url` is stored in the SQLite `settings` table. Default in this deployment:

```
http://172.16.1.222:8028/v1
```

Full URL example:

```
http://172.16.1.222:8028/v1/audio/transcriptions
```

This is the **same endpoint shape as the OpenAI Whisper API** (`/v1/audio/transcriptions`). Any server that implements this OpenAI-compatible interface will work identically.

### Authentication

```
Authorization: Bearer <api_key>
```

The `api_key` value is stored in the `settings` table. The default seed value is `cant-be-empty` (the self-hosted Faster-Whisper server does not validate the key, but the header must be present and non-empty). For a real OpenAI API key, substitute the actual key.

### Request body — multipart/form-data

The request is a `multipart/form-data` POST with these fields:

| Field | Value | Notes |
|-------|-------|-------|
| `file` | WAV file bytes | MIME type `audio/wav`, filename `audio.wav` |
| `model` | `deepdml/faster-whisper-large-v3-turbo-ct2` | Stored in `settings.model_id` |
| `initial_prompt` | Comma-separated vocabulary terms | Optional; omitted if vocabulary list is empty |

Exact Rust construction:

```rust
let file_part = reqwest::multipart::Part::bytes(wav_bytes)
    .file_name("audio.wav")
    .mime_str("audio/wav")?;

let mut form = reqwest::multipart::Form::new()
    .part("file", file_part)
    .text("model", model_id.to_string());

if !vocabulary.is_empty() {
    let prompt = vocabulary.join(", ");
    form = form.text("initial_prompt", prompt);
}
```

Equivalent in other languages/tools (e.g., Python `requests`):

```python
import requests

with open("audio.wav", "rb") as f:
    response = requests.post(
        "http://172.16.1.222:8028/v1/audio/transcriptions",
        headers={"Authorization": "Bearer cant-be-empty"},
        files={"file": ("audio.wav", f, "audio/wav")},
        data={
            "model": "deepdml/faster-whisper-large-v3-turbo-ct2",
            "initial_prompt": "Kubernetes, Tauri, React",  # optional
        },
        timeout=30,
    )
text = response.json()["text"].strip()
```

Or with `curl`:

```bash
curl -X POST http://172.16.1.222:8028/v1/audio/transcriptions \
  -H "Authorization: Bearer cant-be-empty" \
  -F "file=@audio.wav;type=audio/wav" \
  -F "model=deepdml/faster-whisper-large-v3-turbo-ct2" \
  -F "initial_prompt=Kubernetes, Tauri"
```

### Timeout

The request has a **30-second timeout**. The Faster-Whisper server typically responds in 1–5 seconds for short clips but can take longer for audio recorded at a non-16 kHz native sample rate (server-side resampling).

### Response

The server returns JSON:

```json
{
  "text": " Hello, world!"
}
```

The `text` field may have leading/trailing whitespace — trim it. The `text` field is the only field used; extra fields (`language`, `duration`, etc.) are ignored.

Error responses return a non-2xx HTTP status code. The body is a plain-text error message from the server.

---

## Stage 4 — Post-Processing

**Source file:** `src-tauri/src/db.rs` — `apply_corrections()`

After transcription, a table of user-defined text corrections is applied. Each correction is a `(from_text, to_text)` pair stored in the SQLite `corrections` table. Matching is **case-insensitive**; replacement preserves the `to_text` casing exactly.

This step is optional — if the corrections table is empty, the transcribed text is returned unchanged.

---

## Stage 5 — Delivery to Application

### In localSuperWhisper (via clipboard paste)

The corrected text is delivered by:

1. Writing the text to the system clipboard using [`arboard`](https://crates.io/crates/arboard) v3.
2. Restoring focus to the window that was focused before recording started.
3. Simulating a Ctrl+V (or Ctrl+Shift+V for terminal apps on Linux) keypress.

Platform details:
- **Windows:** Focus restored via `SetForegroundWindow(hwnd)` (Win32 API); paste via `enigo` Ctrl+V.
- **macOS:** Paste via `enigo` Ctrl+V.
- **Linux X11:** Focus restored and paste key sent via `xdotool`. Apps in `CTRL_SHIFT_V_APPS` list (`code`, `windsurf`, `antigravity`) receive Ctrl+Shift+V instead of Ctrl+V.
- **Linux Wayland:** Window focus/restore is skipped; clipboard is still written so the user can manually paste.

### In a different application (direct text return)

If you don't want clipboard paste behavior, call the `transcribe()` function directly and use the returned `String`. The function signature:

```rust
pub async fn transcribe(
    api_url: &str,
    api_key: &str,
    model_id: &str,
    wav_bytes: Vec<u8>,   // in-memory WAV file
    vocabulary: &[String], // optional hints; pass &[] for none
) -> Result<String, String>
```

It returns `Ok(text)` with the trimmed transcript, or `Err(message)` on network/API failure.

---

## Event Bus (Tauri-specific)

The application emits these Tauri events during the pipeline so the UI overlay can reflect state:

| Event | Payload | When emitted |
|-------|---------|--------------|
| `recording-started` | `()` | After audio stream opens |
| `audio-level` | `f32` (0.0–1.0) | Every 50 ms while recording |
| `recording-transcribing` | `()` | After stream closes, before HTTP request |
| `recording-result` | `String` | Transcription text, after paste |
| `recording-idle` | `()` | 2.5 s after result shown, or empty result |
| `recording-error` | `String` | API or audio failure |

In the frontend, events are subscribed via:

```typescript
import { listen } from "@tauri-apps/api/event";

const unlisten = await listen<string>("recording-result", (e) => {
  console.log("Transcript:", e.payload);
});
// call unlisten() to detach
```

---

## Configuration Reference (SQLite `settings` table)

| Key | Default | Description |
|-----|---------|-------------|
| `api_url` | `http://172.16.1.222:8028/v1` | Faster-Whisper server base URL |
| `api_key` | `cant-be-empty` | Bearer token (required but not validated by the server) |
| `model_id` | `deepdml/faster-whisper-large-v3-turbo-ct2` | Model string passed in the multipart form |
| `mic_device` | `default` | `"default"` or exact device name from `cpal` enumeration |
| `hotkey` | `""` | Global shortcut key string (e.g., `"F9"`, `"AltRight"`) |
| `typing_speed_wpm` | `40` | Used only for time-saved statistics; not involved in transcription |

---

## Cargo Dependencies Required for Transcription Only

To extract just the transcription capability into another Rust project, these are the minimum required dependencies:

```toml
[dependencies]
reqwest = { version = "0.12", features = ["multipart", "json"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }  # or any async runtime

# For audio capture:
cpal = "0.15"
hound = "3.5"  # WAV encoding
```

The entire transcription call fits in `transcribe.rs` (~60 lines) and `audio.rs` (~200 lines) and has no dependency on Tauri.

---

## Minimal Standalone Example (Rust)

```rust
// 1. Record audio (returns Vec<u8> WAV bytes)
let mut recorder = AudioRecorder::new();
recorder.start("default")?;
std::thread::sleep(std::time::Duration::from_secs(5));
let (wav_bytes, _duration_ms) = recorder.stop();

// 2. Transcribe
let text = transcribe::transcribe(
    "http://172.16.1.222:8028/v1",
    "cant-be-empty",
    "deepdml/faster-whisper-large-v3-turbo-ct2",
    wav_bytes,
    &[], // no vocabulary hints
).await?;

println!("Transcript: {}", text);
```

---

## Notes for Other Languages / Frameworks

The transcription service itself has no language requirement. Any HTTP client that can send a `multipart/form-data` POST with a WAV file will work. Key points:

- The WAV file must be a valid PCM WAV (16-bit int, any sample rate; 16 kHz mono is recommended for accuracy and speed).
- The `model` field is required but can be any string the server accepts.
- The `initial_prompt` field is optional; provide it as a comma-separated list of domain-specific terms (proper nouns, acronyms, etc.) to bias the decoder.
- The response is JSON `{"text": "..."}` — identical to the OpenAI Whisper API response shape.
- The server at `172.16.1.222:8028` is a LAN-only self-hosted service. The user must ensure network access to this host and port.
