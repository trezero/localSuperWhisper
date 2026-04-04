use std::sync::OnceLock;

struct SoundData {
    start: SoundSource,
    stop: SoundSource,
    error: SoundSource,
}

enum SoundSource {
    File(std::path::PathBuf),
    Bytes(Vec<u8>),
    Empty,
}

static SOUND_DATA: OnceLock<SoundData> = OnceLock::new();

pub fn init_sounds(resource_dir: std::path::PathBuf) {
    let load = |name: &str| -> SoundSource {
        let path = resource_dir.join("sounds").join(name);
        if !path.exists() {
            eprintln!("Warning: sound file not found: {}", path.display());
            return SoundSource::Empty;
        }

        // On Linux, use aplay for reliable PipeWire/PulseAudio/ALSA playback
        #[cfg(target_os = "linux")]
        {
            return SoundSource::File(path);
        }

        // On other platforms, load bytes for rodio
        #[cfg(not(target_os = "linux"))]
        match std::fs::read(&path) {
            Ok(bytes) => SoundSource::Bytes(bytes),
            Err(e) => {
                eprintln!("Warning: could not load sound {}: {}", path.display(), e);
                SoundSource::Empty
            }
        }
    };

    SOUND_DATA.get_or_init(|| SoundData {
        start: load("start.wav"),
        stop: load("stop.wav"),
        error: load("error.wav"),
    });
}

fn play_sound(source: &SoundSource) {
    match source {
        SoundSource::Empty => return,
        SoundSource::File(path) => {
            let path = path.clone();
            std::thread::spawn(move || {
                let _ = std::process::Command::new("aplay")
                    .arg("-q")
                    .arg(&path)
                    .status();
            });
        }
        SoundSource::Bytes(bytes) => {
            if bytes.is_empty() {
                return;
            }
            let bytes = bytes.clone();
            std::thread::spawn(move || {
                use rodio::{Decoder, OutputStream, Sink};
                use std::io::Cursor;

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
    }
}

pub fn play_start() {
    if let Some(data) = SOUND_DATA.get() {
        play_sound(&data.start);
    }
}

pub fn play_stop() {
    if let Some(data) = SOUND_DATA.get() {
        play_sound(&data.stop);
    }
}

pub fn play_error() {
    if let Some(data) = SOUND_DATA.get() {
        play_sound(&data.error);
    }
}
