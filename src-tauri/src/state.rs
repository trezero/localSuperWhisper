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

// AudioRecorder contains a cpal Stream which uses PhantomData<*mut ()> internally
// (NotSendSyncAcrossAllPlatforms). Access is always guarded by Mutex, so this is safe.
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}
