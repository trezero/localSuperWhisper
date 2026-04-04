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
    #[cfg(windows)]
    let target = crate::paste::capture_foreground_window();
    #[cfg(not(windows))]
    let target: Option<isize> = None;
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
                // Paste text into target window
                #[cfg(windows)]
                if let Err(e) = crate::paste::paste_text(&text, target_window) {
                    eprintln!("Paste error: {}", e);
                }
                #[cfg(not(windows))]
                let _ = target_window; // suppress unused warning on Linux

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
