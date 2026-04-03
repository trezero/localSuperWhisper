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
    channels: u16,
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
            channels: 1,
        }
    }

    pub fn start(&mut self, device_name: &str) -> Result<(), String> {
        let device = get_device_by_name(device_name)
            .ok_or_else(|| format!("Audio device not found: {}", device_name))?;

        // Prefer 16 kHz mono. If the device doesn't support mono, use its
        // default config (which may be stereo) and downmix when stopping.
        let (config, sample_rate, channels) = {
            let supports_16k_mono = device
                .supported_input_configs()
                .ok()
                .map(|mut cfgs| {
                    cfgs.any(|c| {
                        c.channels() == 1
                            && c.min_sample_rate().0 <= 16000
                            && c.max_sample_rate().0 >= 16000
                    })
                })
                .unwrap_or(false);

            if supports_16k_mono {
                (
                    StreamConfig {
                        channels: 1,
                        sample_rate: SampleRate(16000),
                        buffer_size: cpal::BufferSize::Default,
                    },
                    16000u32,
                    1u16,
                )
            } else {
                let default = device.default_input_config().map_err(|e| e.to_string())?;
                let sr = default.sample_rate().0;
                let ch = default.channels();
                (
                    StreamConfig {
                        channels: ch,
                        sample_rate: SampleRate(sr),
                        buffer_size: cpal::BufferSize::Default,
                    },
                    sr,
                    ch,
                )
            }
        };

        self.sample_rate = sample_rate;
        self.channels = channels;
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
        let ch = self.channels as usize;
        let needed = 800 * ch;
        if buffer.len() < needed {
            return 0.0;
        }
        let tail = &buffer[buffer.len() - needed..];
        if ch == 1 {
            compute_rms(tail)
        } else {
            let mono: Vec<f32> = tail
                .chunks(ch)
                .map(|frame| frame.iter().sum::<f32>() / ch as f32)
                .collect();
            compute_rms(&mono)
        }
    }

    pub fn stop(&mut self) -> (Vec<u8>, u64) {
        self.stream = None; // Drops the stream, stopping recording
        let samples: Vec<f32> = std::mem::take(&mut *self.buffer.lock().unwrap());

        // Downmix multichannel to mono by averaging channels per frame
        let mono: Vec<f32> = if self.channels > 1 {
            let ch = self.channels as usize;
            samples
                .chunks(ch)
                .map(|frame| frame.iter().sum::<f32>() / ch as f32)
                .collect()
        } else {
            samples
        };

        let duration_ms = if self.sample_rate > 0 {
            (mono.len() as u64 * 1000) / self.sample_rate as u64
        } else {
            0
        };
        let wav = encode_wav(&mono, self.sample_rate);
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
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert!(wav.len() > 44);
        assert_eq!(wav.len(), 16000 * 2 + 44);
    }

    #[test]
    fn test_encode_wav_clamps_values() {
        let samples = vec![2.0, -2.0, 0.5, -0.5];
        let wav = encode_wav(&samples, 16000);
        assert_eq!(&wav[0..4], b"RIFF");
    }

    #[test]
    fn test_encode_wav_empty() {
        let samples: Vec<f32> = vec![];
        let wav = encode_wav(&samples, 16000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(wav.len(), 44); // Header only
    }

    #[test]
    fn test_compute_rms_silence() {
        let samples = vec![0.0f32; 100];
        assert_eq!(compute_rms(&samples), 0.0);
    }

    #[test]
    fn test_compute_rms_known_value() {
        let samples = vec![0.5f32; 100];
        let rms = compute_rms(&samples);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_compute_rms_empty() {
        assert_eq!(compute_rms(&[]), 0.0);
    }
}
