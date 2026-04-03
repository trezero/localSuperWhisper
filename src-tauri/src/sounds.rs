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
