use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fs;
use std::path::Path;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const KSDATAFORMAT_SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WavEncoding {
    Pcm16,
    Pcm24,
    Float32,
}

impl WavEncoding {
    fn bytes_per_sample(self) -> usize {
        match self {
            Self::Pcm16 => 2,
            Self::Pcm24 => 3,
            Self::Float32 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedAudio {
    pub(crate) sample_rate_hz: u32,
    pub(crate) source_channel_count: u16,
    pub(crate) frames: Vec<f32>,
    pub(crate) right_frames: Option<Vec<f32>>,
    pub(crate) content_revision: [u8; 32],
}

impl LoadedAudio {
    pub fn new(sample_rate_hz: u32, frames: Vec<f32>) -> Self {
        Self::with_source_channels(sample_rate_hz, 1, frames)
    }

    #[cfg(test)]
    pub fn new_stereo(sample_rate_hz: u32, left: Vec<f32>, right: Vec<f32>) -> Self {
        assert_eq!(left.len(), right.len());
        let mut hasher = Sha256::new();
        hasher.update(sample_rate_hz.to_le_bytes());
        hasher.update(2_u16.to_le_bytes());
        for (&l, &r) in left.iter().zip(right.iter()) {
            hasher.update(l.to_bits().to_le_bytes());
            hasher.update(r.to_bits().to_le_bytes());
        }
        Self {
            sample_rate_hz,
            source_channel_count: 2,
            frames: left,
            right_frames: Some(right),
            content_revision: hasher.finalize().into(),
        }
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
            right_frames: None,
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

    #[cfg(test)]
    pub fn frames(&self) -> &[f32] {
        &self.frames
    }

    #[cfg(test)]
    pub fn channel_frames(&self, channel: u16) -> Option<&[f32]> {
        match channel {
            0 => Some(&self.frames),
            1 => self.right_frames.as_deref(),
            _ => None,
        }
    }

    pub fn mono_mix(&self) -> Cow<'_, [f32]> {
        mono_mix_slices(&self.frames, self.right_frames.as_deref())
    }

    #[cfg(test)]
    pub fn source_sample(&self, channel: u16, frame: usize) -> Option<f32> {
        source_sample(
            &self.frames,
            self.source_channel_count,
            self.right_frames.as_deref(),
            channel,
            frame,
        )
    }

    #[cfg(test)]
    pub fn content_revision(&self) -> &[u8; 32] {
        &self.content_revision
    }
}

pub(crate) fn mono_mix_slices<'a>(left: &'a [f32], right: Option<&[f32]>) -> Cow<'a, [f32]> {
    let Some(right) = right else {
        return Cow::Borrowed(left);
    };
    Cow::Owned(
        left.iter()
            .zip(right.iter())
            .map(|(&l, &r)| (l + r) * 0.5)
            .collect(),
    )
}

pub(crate) fn source_sample(
    left: &[f32],
    channel_count: u16,
    right: Option<&[f32]>,
    channel: u16,
    frame: usize,
) -> Option<f32> {
    if channel >= channel_count || frame >= left.len() {
        return None;
    }
    match channel {
        0 => left.get(frame).copied(),
        1 => right?.get(frame).copied(),
        _ => None,
    }
}

pub fn load_pcm_wav(path: &Path, expected_sample_rate_hz: u32) -> Result<LoadedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read file: {error}"))?;
    decode_pcm_wav(&bytes, expected_sample_rate_hz)
}

pub fn load_pcm_wav_any_rate(path: &Path) -> Result<LoadedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read file: {error}"))?;
    decode_pcm_wav_any_rate(&bytes)
}

pub(crate) fn decode_pcm_wav_any_rate(bytes: &[u8]) -> Result<LoadedAudio, String> {
    decode_pcm_wav_with_expected_rate(bytes, None)
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
    let mut fmt_chunk: Option<&[u8]> = None;
    let mut data: Option<&[u8]> = None;

    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let end = offset
            .checked_add(chunk_size)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| "unsupported format; malformed WAV chunk".to_string())?;

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err("unsupported format; malformed fmt chunk".to_string());
            }
            fmt_chunk = Some(&bytes[offset..end]);
        } else if chunk_id == b"data" {
            data = Some(&bytes[offset..end]);
        }

        offset = end
            .checked_add(chunk_size % 2)
            .ok_or_else(|| "unsupported format; malformed WAV chunk".to_string())?;
    }

    let fmt = fmt_chunk.ok_or_else(|| "unsupported format; missing fmt chunk".to_string())?;
    let data = data.ok_or_else(|| "unsupported format; missing data chunk".to_string())?;
    let (channels, sample_rate, encoding) = parse_wav_format(fmt, expected_sample_rate_hz)?;

    let frame_bytes = usize::from(channels) * encoding.bytes_per_sample();
    if data.len() % frame_bytes != 0 {
        return Err("unsupported format; incomplete PCM frame".to_string());
    }

    let (frames, right_frames) = decode_wav_samples(data, channels, encoding)?;

    Ok(LoadedAudio {
        sample_rate_hz: sample_rate,
        source_channel_count: channels,
        frames,
        right_frames,
        content_revision: Sha256::digest(bytes).into(),
    })
}

fn parse_wav_format(
    fmt: &[u8],
    expected_sample_rate_hz: Option<u32>,
) -> Result<(u16, u32, WavEncoding), String> {
    let audio_format = u16::from_le_bytes(fmt[0..2].try_into().unwrap());
    let channels = u16::from_le_bytes(fmt[2..4].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
    let block_align = u16::from_le_bytes(fmt[12..14].try_into().unwrap());
    let bits_per_sample = u16::from_le_bytes(fmt[14..16].try_into().unwrap());

    if sample_rate == 0 {
        return Err("unsupported format; sample rate must be positive".to_string());
    }
    if let Some(expected_sample_rate_hz) = expected_sample_rate_hz
        && sample_rate != expected_sample_rate_hz
    {
        return Err(format!(
            "sample-rate mismatch: asset is {sample_rate} Hz, render is {expected_sample_rate_hz} Hz"
        ));
    }
    if channels == 0 || channels > 2 {
        return Err(
            "unsupported format; expected mono/stereo PCM16/PCM24/float32 WAV".to_string(),
        );
    }

    let encoding = match (audio_format, bits_per_sample) {
        (WAVE_FORMAT_PCM, 16) => WavEncoding::Pcm16,
        (WAVE_FORMAT_PCM, 24) => WavEncoding::Pcm24,
        (WAVE_FORMAT_IEEE_FLOAT, 32) => WavEncoding::Float32,
        (WAVE_FORMAT_EXTENSIBLE, 16 | 24 | 32) => {
            if fmt.len() < 40 {
                return Err("unsupported format; malformed fmt chunk".to_string());
            }
            let cb_size = u16::from_le_bytes(fmt[16..18].try_into().unwrap());
            let valid_bits = u16::from_le_bytes(fmt[18..20].try_into().unwrap());
            let subformat: [u8; 16] = fmt[24..40].try_into().unwrap();
            if cb_size < 22
                || 18 + usize::from(cb_size) > fmt.len()
                || (valid_bits != 0 && valid_bits != bits_per_sample)
            {
                return Err("unsupported format; malformed fmt chunk".to_string());
            }
            match (subformat, bits_per_sample) {
                (KSDATAFORMAT_SUBTYPE_PCM, 16) => WavEncoding::Pcm16,
                (KSDATAFORMAT_SUBTYPE_PCM, 24) => WavEncoding::Pcm24,
                (KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, 32) => WavEncoding::Float32,
                _ => {
                    return Err(
                        "unsupported format; expected mono/stereo PCM16/PCM24/float32 WAV"
                            .to_string(),
                    );
                }
            }
        }
        _ => {
            return Err(
                "unsupported format; expected mono/stereo PCM16/PCM24/float32 WAV".to_string(),
            );
        }
    };

    let expected_block_align = channels * (encoding.bytes_per_sample() as u16);
    if block_align != expected_block_align {
        return Err("unsupported format; malformed fmt chunk".to_string());
    }

    Ok((channels, sample_rate, encoding))
}

fn decode_wav_samples(
    data: &[u8],
    channels: u16,
    encoding: WavEncoding,
) -> Result<(Vec<f32>, Option<Vec<f32>>), String> {
    let bps = encoding.bytes_per_sample();
    let frame_bytes = usize::from(channels) * bps;
    let frame_count = data.len() / frame_bytes;
    let mut left = Vec::with_capacity(frame_count);
    let mut right = (channels == 2).then(|| Vec::with_capacity(frame_count));

    for frame in data.chunks_exact(frame_bytes) {
        left.push(decode_wav_sample(&frame[0..bps], encoding)?);
        if let Some(right_channel) = right.as_mut() {
            right_channel.push(decode_wav_sample(&frame[bps..frame_bytes], encoding)?);
        }
    }

    Ok((left, right))
}

fn decode_wav_sample(bytes: &[u8], encoding: WavEncoding) -> Result<f32, String> {
    match encoding {
        WavEncoding::Pcm16 => Ok(f32::from(i16::from_le_bytes(bytes.try_into().unwrap())) / 32_768.0),
        WavEncoding::Pcm24 => {
            let raw = i32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]) >> 8;
            Ok(raw as f32 / 8_388_608.0)
        }
        WavEncoding::Float32 => {
            let sample = f32::from_le_bytes(bytes.try_into().unwrap());
            if !sample.is_finite() {
                return Err("unsupported format; non-finite float32 sample".to_string());
            }
            Ok(sample)
        }
    }
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
    fn stereo_source_samples_preserve_independent_planar_channels() {
        let mut bytes = Vec::new();
        write_wav_stereo_i16(&mut bytes, 48_000, &[-0.75, 0.5], &[0.5, -0.25])
            .expect("stereo WAV should write");
        let audio = decode_pcm_wav(&bytes, 48_000).expect("stereo WAV should decode");

        for (actual, expected) in audio.frames().iter().zip([-0.75, 0.5]) {
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
        let pos_half = 16_383.0 / 32_768.0;
        assert_eq!(audio.channel_frames(0), Some(&[-0.75, pos_half][..]));
        assert_eq!(audio.channel_frames(1), Some(&[pos_half, -0.25][..]));
        assert_eq!(audio.channel_frames(2), None);
        assert_eq!(
            audio.mono_mix().as_ref(),
            &[(-0.75 + pos_half) * 0.5, (pos_half - 0.25) * 0.5]
        );

        let constructed = LoadedAudio::new_stereo(48_000, vec![-0.5, 0.25], vec![0.5, -0.25]);
        assert_eq!(constructed.source_channel_count(), 2);
        assert_eq!(constructed.channel_frames(0), Some(&[-0.5, 0.25][..]));
        assert_eq!(constructed.channel_frames(1), Some(&[0.5, -0.25][..]));
        assert_eq!(constructed.mono_mix().as_ref(), &[0.0, 0.0]);
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
        crate::wav::write_wav_channels_i16(&mut mono, 48_000, &[&[0.25]]).unwrap();
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

        // Keep a complete valid header so rejection reaches the chunk-length
        // check, rather than the earlier short-file check.
        let mut malformed_chunk = valid.clone();
        malformed_chunk[40..44].copy_from_slice(&100_u32.to_le_bytes());
        assert_eq!(
            decode_pcm_wav(&malformed_chunk, 48_000).unwrap_err(),
            "unsupported format; malformed WAV chunk"
        );
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

        for bytes in [float_format, incomplete_frame] {
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
        wrong_width[34..36].copy_from_slice(&8_u16.to_le_bytes());
        for bytes in [zero_channels, too_many_channels, wrong_width] {
            assert_eq!(
                decode_pcm_wav(&bytes, 48_000).unwrap_err(),
                "unsupported format; expected mono/stereo PCM16/PCM24/float32 WAV"
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

    fn extensible_wav(
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
        valid_bits: u16,
        cb_size: u16,
        subformat: [u8; 16],
        payload: &[u8],
    ) -> Vec<u8> {
        let fmt_size = 18_usize + usize::from(cb_size);
        let riff_size = 4 + 8 + fmt_size + 8 + payload.len();
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let mut bytes = Vec::with_capacity(8 + riff_size);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(riff_size as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&(fmt_size as u32).to_le_bytes());
        bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(&cb_size.to_le_bytes());
        if cb_size >= 22 {
            bytes.extend_from_slice(&valid_bits.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            bytes.extend_from_slice(&subformat);
            bytes.resize(bytes.len() + usize::from(cb_size - 22), 0);
        } else {
            bytes.resize(bytes.len() + usize::from(cb_size), 0);
        }
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn decodes_pcm24_and_float32_wavs_with_extrema_interleaving_and_headroom() {
        use crate::wav::{write_wav_channels_f32, write_wav_channels_i24};

        let mut i24_mono = Vec::new();
        write_wav_channels_i24(&mut i24_mono, 48_000, &[&[-8_388_608, 0, 8_388_607, 1]]).unwrap();
        let decoded_i24_mono = decode_pcm_wav(&i24_mono, 48_000).unwrap();
        assert_eq!(decoded_i24_mono.source_channel_count(), 1);
        assert_eq!(decoded_i24_mono.frames()[0], -1.0);
        assert_eq!(decoded_i24_mono.frames()[1], 0.0);
        assert_eq!(decoded_i24_mono.frames()[2], 8_388_607.0 / 8_388_608.0);
        assert_eq!(decoded_i24_mono.frames()[3], 1.0 / 8_388_608.0);

        let mut i24_stereo = Vec::new();
        write_wav_channels_i24(
            &mut i24_stereo,
            44_100,
            &[&[-8_388_608, 4_194_304], &[8_388_607, -2_097_152]],
        )
        .unwrap();
        let decoded_i24_stereo = decode_pcm_wav_any_rate(&i24_stereo).unwrap();
        assert_eq!(decoded_i24_stereo.sample_rate_hz(), 44_100);
        assert_eq!(decoded_i24_stereo.source_channel_count(), 2);
        assert_eq!(decoded_i24_stereo.source_sample(0, 0), Some(-1.0));
        assert_eq!(
            decoded_i24_stereo.source_sample(1, 0),
            Some(8_388_607.0 / 8_388_608.0)
        );
        assert_eq!(decoded_i24_stereo.source_sample(0, 1), Some(0.5));
        assert_eq!(decoded_i24_stereo.source_sample(1, 1), Some(-0.25));

        let mut f32_stereo = Vec::new();
        write_wav_channels_f32(&mut f32_stereo, 48_000, &[&[-1.5, 0.125], &[1.25, -0.75]])
            .unwrap();
        let decoded_f32 = decode_pcm_wav(&f32_stereo, 48_000).unwrap();
        assert_eq!(decoded_f32.source_channel_count(), 2);
        assert_eq!(decoded_f32.source_sample(0, 0), Some(-1.5));
        assert_eq!(decoded_f32.source_sample(1, 0), Some(1.25));
        assert_eq!(decoded_f32.source_sample(0, 1), Some(0.125));
        assert_eq!(decoded_f32.source_sample(1, 1), Some(-0.75));

        const SUBFORMAT_PCM: [u8; 16] = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38,
            0x9b, 0x71,
        ];
        const SUBFORMAT_FLOAT: [u8; 16] = [
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38,
            0x9b, 0x71,
        ];
        let ext_pcm16 = extensible_wav(
            2,
            48_000,
            16,
            16,
            22,
            SUBFORMAT_PCM,
            &[0x00, 0x80, 0xff, 0x7f],
        );
        let decoded_ext_pcm16 = decode_pcm_wav(&ext_pcm16, 48_000).unwrap();
        assert_eq!(decoded_ext_pcm16.source_sample(0, 0), Some(-1.0));
        assert_eq!(
            decoded_ext_pcm16.source_sample(1, 0),
            Some(32_767.0 / 32_768.0)
        );

        let ext_pcm24 = extensible_wav(
            2,
            48_000,
            24,
            24,
            22,
            SUBFORMAT_PCM,
            &[0x00, 0x00, 0x80, 0xff, 0xff, 0x7f],
        );
        let decoded_ext_pcm24 = decode_pcm_wav(&ext_pcm24, 48_000).unwrap();
        assert_eq!(decoded_ext_pcm24.source_sample(0, 0), Some(-1.0));
        assert_eq!(
            decoded_ext_pcm24.source_sample(1, 0),
            Some(8_388_607.0 / 8_388_608.0)
        );

        let ext_f32 = extensible_wav(1, 48_000, 32, 0, 22, SUBFORMAT_FLOAT, &(-0.5_f32).to_le_bytes());
        let decoded_ext_f32 = decode_pcm_wav(&ext_f32, 48_000).unwrap();
        assert_eq!(decoded_ext_f32.source_sample(0, 0), Some(-0.5));
    }

    #[test]
    fn rejects_nonfinite_float32_and_malformed_or_unsupported_wav_formats() {
        use crate::wav::write_wav_channels_f32;

        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut mono = Vec::new();
            write_wav_channels_f32(&mut mono, 48_000, &[&[0.25, bad]]).unwrap();
            assert!(
                decode_pcm_wav(&mono, 48_000)
                    .unwrap_err()
                    .contains("non-finite")
            );
            let mut stereo_right = Vec::new();
            write_wav_channels_f32(&mut stereo_right, 48_000, &[&[0.25], &[bad]]).unwrap();
            assert!(
                decode_pcm_wav(&stereo_right, 48_000)
                    .unwrap_err()
                    .contains("non-finite")
            );
        }

        const SUBFORMAT_PCM: [u8; 16] = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38,
            0x9b, 0x71,
        ];
        const SUBFORMAT_UNKNOWN: [u8; 16] = [0x99; 16];
        for bad_ext in [
            extensible_wav(1, 48_000, 16, 16, 20, SUBFORMAT_PCM, &[0, 0]),
            extensible_wav(1, 48_000, 24, 16, 22, SUBFORMAT_PCM, &[0, 0, 0]),
            extensible_wav(1, 48_000, 16, 16, 22, SUBFORMAT_UNKNOWN, &[0, 0]),
        ] {
            assert!(
                decode_pcm_wav(&bad_ext, 48_000)
                    .unwrap_err()
                    .contains("unsupported format")
            );
        }

        let mut bad_block_align = Vec::new();
        write_wav_channels_f32(&mut bad_block_align, 48_000, &[&[0.5]]).unwrap();
        bad_block_align[32..34].copy_from_slice(&2_u16.to_le_bytes());
        assert!(
            decode_pcm_wav(&bad_block_align, 48_000)
                .unwrap_err()
                .contains("unsupported format")
        );

        let mut surround = Vec::new();
        crate::wav::write_wav_channels_i16(&mut surround, 48_000, &[&[0.1], &[0.2], &[0.3]]).unwrap();
        assert!(
            decode_pcm_wav(&surround, 48_000)
                .unwrap_err()
                .contains("unsupported format")
        );
    }
}
