mod audio;
mod db;
mod hotkey;
mod paste;
#[cfg(windows)]
mod win32_hotkey;
mod sounds;
mod state;
mod transcribe;

use audio::AudioDevice;
use db::{ChecklistStep, CorrectionEntry, HistoryEntry, Stats, VocabularyEntry};
use state::{AppState, RecordingState};

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

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
fn get_corrections(state: tauri::State<'_, AppState>) -> Result<Vec<CorrectionEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_corrections(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_correction(state: tauri::State<'_, AppState>, from_text: String, to_text: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::add_correction(&conn, &from_text, &to_text).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_correction(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::remove_correction(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_audio_devices() -> Vec<AudioDevice> {
    audio::list_input_devices()
}

#[tauri::command]
fn register_hotkey(app: tauri::AppHandle, key: String) -> Result<(), String> {
    // Stop any existing Win32 raw hotkey thread first
    #[cfg(windows)]
    {
        let state = app.state::<AppState>();
        let mut tid = state.raw_hotkey_thread_id.lock().unwrap();
        if let Some(thread_id) = tid.take() {
            win32_hotkey::stop(thread_id);
        }
    }

    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;

    if key.is_empty() {
        return Ok(());
    }

    // Bare modifier keys (AltRight, ControlLeft, etc.) can't go through
    // tauri-plugin-global-shortcut — use Win32 RegisterHotKey instead.
    #[cfg(windows)]
    if let Some(vk) = win32_hotkey::win32_vk_for_key(&key) {
        let state = app.state::<AppState>();
        let thread_id = win32_hotkey::start(vk, app.clone())?;
        *state.raw_hotkey_thread_id.lock().unwrap() = Some(thread_id);
        return Ok(());
    }

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(key.as_str(), move |_app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                hotkey::on_hotkey_pressed(&app_handle);
            }
        })
        .map_err(|e| e.to_string())
}

fn show_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        // Tauri's show/unminimize/set_focus can be blocked by Windows focus-steal
        // protection. Go straight to Win32 — SW_RESTORE handles minimized, hidden,
        // and normal-but-behind-other-windows in a single call.
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
            if let Ok(hwnd) = window.hwnd() {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");
            let db_path = app_data_dir.join("local_super_whisper.db");
            let conn = Connection::open(&db_path).expect("Failed to open database");
            db::init_db(&conn).expect("Failed to initialize database");

            // Initialize sounds
            match app.path().resource_dir() {
                Ok(resource_dir) => sounds::init_sounds(resource_dir),
                Err(e) => eprintln!("Failed to get resource_dir for sounds: {}", e),
            }

            // Manage state
            app.manage(AppState {
                recording_state: Mutex::new(RecordingState::Idle),
                recorder: Mutex::new(audio::AudioRecorder::new()),
                db: Mutex::new(conn),
                target_window: Mutex::new(None),
                #[cfg(windows)]
                raw_hotkey_thread_id: Mutex::new(None),
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
                .icon(tauri::include_image!("icons/icon.png"))
                .menu(&menu)
                .tooltip("Local SuperWhisper")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            show_settings_window(app);
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        show_settings_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Settings window close → hide instead of quit
            if let Some(settings_window) = app.get_webview_window("settings") {
                // Hide settings on close instead of quitting
                let settings_window_clone = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_window_clone.hide();
                    }
                });
            }

            // Register global hotkey (skipped if not yet configured)
            {
                let hotkey_str = {
                    let app_state = app.state::<AppState>();
                    let conn = app_state.db.lock().unwrap();
                    db::get_setting(&conn, "hotkey").unwrap_or_default()
                };

                if !hotkey_str.is_empty() {
                    // Bare modifier keys use the Win32 RegisterHotKey path
                    #[cfg(windows)]
                    let registered = if let Some(vk) = win32_hotkey::win32_vk_for_key(&hotkey_str) {
                        match win32_hotkey::start(vk, app.handle().clone()) {
                            Ok(thread_id) => {
                                let app_state = app.state::<AppState>();
                                *app_state.raw_hotkey_thread_id.lock().unwrap() = Some(thread_id);
                                true
                            }
                            Err(e) => {
                                eprintln!("Win32 hotkey '{}' failed ({}). Clearing.", hotkey_str, e);
                                false
                            }
                        }
                    } else {
                        false
                    };

                    #[cfg(not(windows))]
                    let registered = false;

                    if !registered {
                        let app_handle = app.handle().clone();
                        if let Err(e) = app.global_shortcut().on_shortcut(
                            hotkey_str.as_str(),
                            move |_app, _shortcut, event| {
                                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                                    hotkey::on_hotkey_pressed(&app_handle);
                                }
                            },
                        ) {
                            eprintln!("Hotkey '{}' is invalid ({}). Clearing — setup screen will appear.", hotkey_str, e);
                            let app_state = app.state::<AppState>();
                            let conn = app_state.db.lock().unwrap();
                            let _ = db::set_setting(&conn, "hotkey", "");
                        }
                    }
                }
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
            register_hotkey,
            get_corrections,
            add_correction,
            remove_correction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
