use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedAudio {
    pub(crate) sample_rate_hz: u32,
    pub(crate) source_channel_count: u16,
    pub(crate) frames: Vec<f32>,
    pub(crate) source_pcm: Option<Vec<i16>>,
    pub(crate) content_revision: [u8; 32],
}

impl LoadedAudio {
    pub fn new(sample_rate_hz: u32, frames: Vec<f32>) -> Self {
        Self::with_source_channels(sample_rate_hz, 1, frames)
    }

    pub fn with_source_channels(
        sample_rate_hz: u32,
        source_channel_count: u16,
        frames: Vec<f32>,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(sample_rate_hz.to_le_bytes());
        hasher.update(source_channel_count.to_le_bytes());
        for frame in &frames {
            hasher.update(frame.to_bits().to_le_bytes());
        }
        Self {
            sample_rate_hz,
            source_channel_count,
            frames,
            source_pcm: None,
            content_revision: hasher.finalize().into(),
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    #[cfg(test)]
    pub fn source_channel_count(&self) -> u16 {
        self.source_channel_count
    }

    pub fn frames(&self) -> &[f32] {
        &self.frames
    }

    #[cfg(test)]
    pub fn source_sample(&self, channel: u16, frame: usize) -> Option<f32> {
        source_sample(
            &self.frames,
            self.source_channel_count,
            self.source_pcm.as_deref(),
            channel,
            frame,
        )
    }

    #[cfg(test)]
    pub fn content_revision(&self) -> &[u8; 32] {
        &self.content_revision
    }
}

pub(crate) fn source_sample(
    frames: &[f32],
    channel_count: u16,
    source_pcm: Option<&[i16]>,
    channel: u16,
    frame: usize,
) -> Option<f32> {
    if channel >= channel_count || frame >= frames.len() {
        return None;
    }
    if channel_count == 1 {
        return frames.get(frame).copied();
    }
    let index = frame
        .checked_mul(usize::from(channel_count))?
        .checked_add(usize::from(channel))?;
    source_pcm?
        .get(index)
        .map(|sample| f32::from(*sample) / 32768.0)
}

pub fn load_pcm_wav(path: &Path, expected_sample_rate_hz: u32) -> Result<LoadedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read file: {error}"))?;
    decode_pcm_wav(&bytes, expected_sample_rate_hz)
}

pub fn load_pcm_wav_any_rate(path: &Path) -> Result<LoadedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read file: {error}"))?;
    decode_pcm_wav_with_expected_rate(&bytes, None)
}

pub fn decode_pcm_wav(bytes: &[u8], expected_sample_rate_hz: u32) -> Result<LoadedAudio, String> {
    decode_pcm_wav_with_expected_rate(bytes, Some(expected_sample_rate_hz))
}

fn decode_pcm_wav_with_expected_rate(
    bytes: &[u8],
    expected_sample_rate_hz: Option<u32>,
) -> Result<LoadedAudio, String> {
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

    if sample_rate == 0 {
        return Err("unsupported format; sample rate must be positive".to_string());
    }

    if let Some(expected_sample_rate_hz) = expected_sample_rate_hz {
        if sample_rate != expected_sample_rate_hz {
            return Err(format!(
                "sample-rate mismatch: asset is {sample_rate} Hz, render is {expected_sample_rate_hz} Hz"
            ));
        }
    }
    if channels == 0 || channels > 2 || bits_per_sample != 16 {
        return Err("unsupported format; expected mono/stereo 16-bit PCM WAV".to_string());
    }

    let frame_bytes = channels as usize * 2;
    if data.len() % frame_bytes != 0 {
        return Err("unsupported format; incomplete PCM frame".to_string());
    }

    let mut frames = Vec::with_capacity(data.len() / frame_bytes);
    let mut source_pcm = (channels == 2).then(|| Vec::with_capacity(data.len() / 2));
    for frame in data.chunks_exact(frame_bytes) {
        let left_pcm = i16::from_le_bytes(frame[0..2].try_into().unwrap());
        let left = f32::from(left_pcm) / 32768.0;
        let sample = if channels == 2 {
            let right_pcm = i16::from_le_bytes(frame[2..4].try_into().unwrap());
            let right = f32::from(right_pcm) / 32768.0;
            source_pcm.as_mut().unwrap().extend([left_pcm, right_pcm]);
            (left + right) * 0.5
        } else {
            left
        };
        frames.push(sample);
    }

    Ok(LoadedAudio {
        sample_rate_hz: sample_rate,
        source_channel_count: channels,
        frames,
        source_pcm,
        content_revision: Sha256::digest(bytes).into(),
    })
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
        assert_eq!(audio.source_channel_count(), 2);
        assert!((audio.frames()[0] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn decodes_an_owned_pcm_wav_snapshot_without_reopening_a_path() {
        let mut bytes = Vec::new();
        write_wav_stereo_i16(&mut bytes, 48_000, &[0.25, -0.5], &[0.25, -0.5])
            .expect("WAV should write");

        let audio = decode_pcm_wav(&bytes, 48_000).expect("owned WAV bytes should decode");

        assert_eq!(audio.sample_rate_hz(), 48_000);
        assert_eq!(audio.frames().len(), 2);
        assert!((audio.frames()[0] - 0.25).abs() < 0.0001);
        assert!((audio.frames()[1] + 0.5).abs() < 0.0001);
    }

    #[test]
    fn stereo_source_samples_remain_distinct_from_the_playback_mix() {
        let mut bytes = Vec::new();
        write_wav_stereo_i16(&mut bytes, 48_000, &[-0.75, 0.5], &[0.5, -0.25])
            .expect("stereo WAV should write");
        let audio = decode_pcm_wav(&bytes, 48_000).expect("stereo WAV should decode");

        for (actual, expected) in audio.frames().iter().zip([-0.125, 0.125]) {
            assert!((actual - expected).abs() < 0.00004);
        }
        for ((channel, frame), expected) in [
            ((0, 0), -0.75),
            ((1, 0), 0.5),
            ((0, 1), 0.5),
            ((1, 1), -0.25),
        ] {
            assert!((audio.source_sample(channel, frame).unwrap() - expected).abs() < 0.00004);
        }
        assert_eq!(audio.source_sample(2, 0), None);
        assert_eq!(audio.source_sample(0, 2), None);
    }

    #[test]
    fn content_revision_changes_when_the_same_source_bytes_change() {
        let mut first = Vec::new();
        let mut second = Vec::new();
        write_wav_stereo_i16(&mut first, 48_000, &[-0.75], &[0.5]).unwrap();
        write_wav_stereo_i16(&mut second, 48_000, &[-0.5], &[0.5]).unwrap();

        let first = decode_pcm_wav(&first, 48_000).unwrap();
        let second = decode_pcm_wav(&second, 48_000).unwrap();
        assert_ne!(first.content_revision(), second.content_revision());
    }

    #[test]
    fn any_rate_sample_load_retains_source_rate_and_rejects_zero_rate() {
        let dir = unique_temp_dir("any_rate_sample_load");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let path = dir.join("hit.wav");
        write_wav_stereo_i16(fs::File::create(&path).unwrap(), 48_000, &[-0.5], &[-0.5])
            .expect("wav should write");
        let loaded = load_pcm_wav_any_rate(&path).expect("source keeps its own sample rate");
        assert_eq!(loaded.sample_rate_hz(), 48_000);
        assert_eq!(loaded.frames(), &[-0.5]);

        let mut invalid = fs::read(path).expect("wav bytes");
        invalid[24..28].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            decode_pcm_wav_with_expected_rate(&invalid, None).unwrap_err(),
            "unsupported format; sample rate must be positive"
        );
    }

    #[test]
    fn decoder_accepts_empty_mono_and_odd_padded_pcm_wavs() {
        let mut empty = Vec::new();
        write_wav_stereo_i16(&mut empty, 48_000, &[], &[]).unwrap();
        assert_eq!(empty.len(), 44);
        assert!(decode_pcm_wav(&empty, 48_000).unwrap().frames().is_empty());

        let mut mono = Vec::new();
        write_wav_stereo_i16(&mut mono, 48_000, &[0.25], &[0.25]).unwrap();
        mono[22..24].copy_from_slice(&1_u16.to_le_bytes());
        mono[40..44].copy_from_slice(&2_u32.to_le_bytes());
        mono.truncate(46);
        let decoded_mono = decode_pcm_wav(&mono, 48_000).unwrap();
        assert_eq!(decoded_mono.source_channel_count(), 1);
        assert_eq!(decoded_mono.frames().len(), 1);
        assert!((decoded_mono.frames()[0] - 0.25).abs() < 0.0001);

        let mut padded = Vec::new();
        write_wav_stereo_i16(&mut padded, 48_000, &[0.25], &[0.25]).unwrap();
        padded.splice(12..12, *b"JUNK\x01\0\0\0x\0");
        let riff_size = (padded.len() - 8) as u32;
        padded[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert_eq!(decode_pcm_wav(&padded, 48_000).unwrap().frames().len(), 1);
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

        let mut bad_riff = vec![0_u8; 44];
        bad_riff[0..4].copy_from_slice(b"NOPE");
        bad_riff[8..12].copy_from_slice(b"WAVE");
        let mut bad_wave = bad_riff.clone();
        bad_wave[0..4].copy_from_slice(b"RIFF");
        bad_wave[8..12].copy_from_slice(b"NOPE");
        for bytes in [bad_riff, bad_wave, b"RIFFxxxx".to_vec()] {
            assert_eq!(
                decode_pcm_wav(&bytes, 48_000).unwrap_err(),
                "unsupported format; expected PCM WAV"
            );
        }
    }

    #[test]
    fn decoder_rejects_malformed_or_unsupported_pcm_chunks() {
        let mut valid = Vec::new();
        write_wav_stereo_i16(&mut valid, 48_000, &[0.25], &[0.25]).unwrap();

        let mut malformed_chunk = b"RIFF\0\0\0\0WAVEdata\x64\0\0\0".to_vec();
        malformed_chunk.resize(24, 0);
        let mut float_format = valid.clone();
        float_format[20..22].copy_from_slice(&3_u16.to_le_bytes());
        let mut zero_channels = valid.clone();
        zero_channels[22..24].copy_from_slice(&0_u16.to_le_bytes());
        let mut incomplete_frame = valid;
        incomplete_frame[40..44].copy_from_slice(&1_u32.to_le_bytes());
        incomplete_frame.truncate(46);

        let mut short_fmt = float_format.clone();
        short_fmt[16..20].copy_from_slice(&8_u32.to_le_bytes());
        assert_eq!(
            decode_pcm_wav(&short_fmt, 48_000).unwrap_err(),
            "unsupported format; malformed fmt chunk"
        );

        for bytes in [malformed_chunk, float_format, incomplete_frame] {
            assert!(
                decode_pcm_wav(&bytes, 48_000)
                    .expect_err("invalid WAV structure should fail")
                    .contains("unsupported format")
            );
        }

        let mut too_many_channels = zero_channels.clone();
        too_many_channels[22..24].copy_from_slice(&3_u16.to_le_bytes());
        let mut wrong_width = zero_channels.clone();
        wrong_width[22..24].copy_from_slice(&2_u16.to_le_bytes());
        wrong_width[34..36].copy_from_slice(&24_u16.to_le_bytes());
        for bytes in [zero_channels, too_many_channels, wrong_width] {
            assert_eq!(
                decode_pcm_wav(&bytes, 48_000).unwrap_err(),
                "unsupported format; expected mono/stereo 16-bit PCM WAV"
            );
        }
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
