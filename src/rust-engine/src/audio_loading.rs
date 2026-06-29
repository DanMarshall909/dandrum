use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedAudio {
    sample_rate_hz: u32,
    frames: Vec<f32>,
}

impl LoadedAudio {
    pub fn new(sample_rate_hz: u32, frames: Vec<f32>) -> Self {
        Self {
            sample_rate_hz,
            frames,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn frames(&self) -> &[f32] {
        &self.frames
    }
}

pub fn load_pcm_wav(path: &Path, expected_sample_rate_hz: u32) -> Result<LoadedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read file: {error}"))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("unsupported format; expected PCM WAV".to_string());
    }

    let mut offset = 12usize;
    let mut channels = None;
    let mut sample_rate = None;
    let mut bits_per_sample = None;
    let mut data = None;

    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        if offset + chunk_size > bytes.len() {
            return Err("unsupported format; malformed WAV chunk".to_string());
        }

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err("unsupported format; malformed fmt chunk".to_string());
            }
            let audio_format = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
            if audio_format != 1 {
                return Err("unsupported format; expected PCM WAV".to_string());
            }
            channels = Some(u16::from_le_bytes(
                bytes[offset + 2..offset + 4].try_into().unwrap(),
            ));
            sample_rate = Some(u32::from_le_bytes(
                bytes[offset + 4..offset + 8].try_into().unwrap(),
            ));
            bits_per_sample = Some(u16::from_le_bytes(
                bytes[offset + 14..offset + 16].try_into().unwrap(),
            ));
        } else if chunk_id == b"data" {
            data = Some(bytes[offset..offset + chunk_size].to_vec());
        }

        offset += chunk_size + (chunk_size % 2);
    }

    let channels = channels.ok_or_else(|| "unsupported format; missing fmt chunk".to_string())?;
    let sample_rate =
        sample_rate.ok_or_else(|| "unsupported format; missing fmt chunk".to_string())?;
    let bits_per_sample =
        bits_per_sample.ok_or_else(|| "unsupported format; missing fmt chunk".to_string())?;
    let data = data.ok_or_else(|| "unsupported format; missing data chunk".to_string())?;

    if sample_rate != expected_sample_rate_hz {
        return Err(format!(
            "sample-rate mismatch: asset is {sample_rate} Hz, render is {expected_sample_rate_hz} Hz"
        ));
    }
    if channels == 0 || channels > 2 || bits_per_sample != 16 {
        return Err("unsupported format; expected mono/stereo 16-bit PCM WAV".to_string());
    }

    let frame_bytes = channels as usize * 2;
    if data.len() % frame_bytes != 0 {
        return Err("unsupported format; incomplete PCM frame".to_string());
    }

    let mut frames = Vec::with_capacity(data.len() / frame_bytes);
    for frame in data.chunks_exact(frame_bytes) {
        let left = i16::from_le_bytes(frame[0..2].try_into().unwrap()) as f32 / 32768.0;
        let sample = if channels == 2 {
            let right = i16::from_le_bytes(frame[2..4].try_into().unwrap()) as f32 / 32768.0;
            (left + right) * 0.5
        } else {
            left
        };
        frames.push(sample);
    }

    Ok(LoadedAudio::new(sample_rate, frames))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wav::write_wav_stereo_i16;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn loads_readable_pcm_wav() {
        let dir = unique_temp_dir("loads_readable_pcm_wav");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let wav_path = dir.join("hit.wav");
        write_wav_stereo_i16(fs::File::create(&wav_path).unwrap(), 48_000, &[0.5], &[0.5])
            .expect("wav should write");

        let audio = load_pcm_wav(&wav_path, 48_000).expect("wav should load");

        assert_eq!(audio.sample_rate_hz(), 48_000);
        assert!((audio.frames()[0] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn reports_missing_file() {
        let dir = unique_temp_dir("reports_missing_file");
        let wav_path = dir.join("missing.wav");
        let error = load_pcm_wav(&wav_path, 48_000).expect_err("missing file should fail");

        assert!(error.contains("failed to read"));
    }

    #[test]
    fn reports_unsupported_format() {
        let dir = unique_temp_dir("reports_unsupported_format");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        fs::write(dir.join("test.txt"), b"not wave").expect("fixture should write");

        let error = load_pcm_wav(&dir.join("test.txt"), 48_000)
            .expect_err("unsupported format should fail");

        assert!(error.contains("unsupported format"));
    }

    #[test]
    fn reports_sample_rate_mismatch() {
        let dir = unique_temp_dir("reports_sample_rate_mismatch");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let wav_path = dir.join("hit.wav");
        write_wav_stereo_i16(fs::File::create(&wav_path).unwrap(), 44_100, &[0.5], &[0.5])
            .expect("wav should write");

        let error = load_pcm_wav(&wav_path, 48_000).expect_err("rate mismatch should fail");

        assert!(error.contains("sample-rate mismatch"));
        assert!(error.contains("44100"));
        assert!(error.contains("48000"));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dandrum-{name}-{}", std::process::id()))
    }
}
