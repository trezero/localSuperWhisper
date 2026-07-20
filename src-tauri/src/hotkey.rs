use crate::db;
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
    let target = crate::paste::capture_foreground_window();
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

/// Append one line per recording to `recordings.log` in the app data directory.
///
/// A recording that captured nothing looks identical to one that captured
/// speech once the audio is discarded, so without this a report of "it started
/// hallucinating" cannot be told apart from "the mic was muted" after the fact.
fn log_recording(app: &AppHandle, duration_ms: u64, peak: f32) {
    use std::io::Write;

    let Ok(dir) = app.path().app_data_dir() else { return };
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("recordings.log"))
        .and_then(|mut f| {
            writeln!(
                f,
                "epoch={} duration_ms={} peak={:.5} silent={}",
                secs,
                duration_ms,
                peak,
                peak < crate::audio::SILENCE_PEAK_THRESHOLD
            )
        });
}

fn stop_recording(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Stop recording and get WAV data
    let (wav_bytes, duration_ms, peak) = {
        let mut recorder = state.recorder.lock().unwrap();
        recorder.stop()
    };

    sounds::play_stop();
    log_recording(app, duration_ms, peak);

    // Audio this quiet contains no speech. Sending it to Whisper does not fail —
    // it returns a confident, usually repeated hallucination ("Thank you." or
    // "I'll see you next time."), which is far worse than an error because it
    // looks like a real result. Exact zero means the OS withheld the microphone;
    // a small nonzero floor means the mic is live but muted or turned down.
    if duration_ms >= 500 && peak < crate::audio::SILENCE_PEAK_THRESHOLD {
        eprintln!(
            "Captured {}ms at peak {:.5} (threshold {:.2}) — refusing to transcribe",
            duration_ms, peak, crate::audio::SILENCE_PEAK_THRESHOLD
        );
        sounds::play_error();
        *state.recording_state.lock().unwrap() = RecordingState::Idle;
        let message = if peak == 0.0 {
            if cfg!(target_os = "macos") {
                "No audio at all was captured. Grant microphone access in System \
                 Settings > Privacy & Security > Microphone, then try again."
                    .to_string()
            } else {
                "No audio at all was captured. Check that the selected microphone is \
                 connected and not muted."
                    .to_string()
            }
        } else {
            format!(
                "That recording was almost silent (level {:.3}), so there was nothing \
                 to transcribe. Check that the microphone is not muted — on a Yeti the \
                 mute button makes the light flash — and that its gain is turned up. \
                 You can verify with \"Test microphone\" in Configuration.",
                peak
            )
        };
        let _ = app.emit("recording-error", message);

        let app_for_timer = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            if let Some(overlay) = app_for_timer.get_webview_window("overlay") {
                let _ = overlay.hide();
            }
            let _ = app_for_timer.emit("recording-idle", ());
        });
        return;
    }

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

    // Get API settings, vocabulary, and corrections
    let (api_url, api_key, model_id, vocabulary, corrections) = {
        let conn = state.db.lock().unwrap();
        let api_url = db::get_setting(&conn, "api_url").unwrap_or_default();
        let api_key = db::get_setting(&conn, "api_key").unwrap_or_default();
        let model_id = db::get_setting(&conn, "model_id").unwrap_or_default();
        let vocab_entries = db::get_vocabulary(&conn).unwrap_or_default();
        let vocabulary: Vec<String> = vocab_entries.into_iter().map(|v| v.term).collect();
        let corrections = db::get_corrections(&conn).unwrap_or_default();
        (api_url, api_key, model_id, vocabulary, corrections)
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
            Ok(raw) if !raw.is_empty() => {
                let text = db::apply_corrections(&raw, &corrections);
                // Paste text into target window. The transcription itself
                // succeeded and is on the clipboard, so a paste failure is worth
                // reporting without discarding the result.
                let paste_failed = match crate::paste::paste_text(&text, target_window) {
                    Err(e) => {
                        eprintln!("Paste error: {}", e);
                        let _ = app_handle.emit("paste-error", e);
                        true
                    }
                    Ok(()) => false,
                };

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

                // Auto-hide after 2.5s, or leave it up long enough to read the
                // instructions when the paste failed.
                let visible_ms = if paste_failed { 9000 } else { 2500 };
                let app_for_timer = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(visible_ms)).await;
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
