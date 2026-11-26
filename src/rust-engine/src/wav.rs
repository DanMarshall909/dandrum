use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub fn write_wav_file(
    path: impl AsRef<Path>,
    sample_rate_hz: u32,
    left: &[f32],
    right: &[f32],
) -> io::Result<()> {
    let file = File::create(path)?;
    write_wav_stereo_i16(file, sample_rate_hz, left, right)
}

pub fn write_wav_stereo_i16<W: Write>(
    writer: W,
    sample_rate_hz: u32,
    left: &[f32],
    right: &[f32],
) -> io::Result<()> {
    write_wav_channels_i16(writer, sample_rate_hz, &[left, right])
}

pub fn write_wav_channels_i16<W: Write>(
    mut writer: W,
    sample_rate_hz: u32,
    channels: &[&[f32]],
) -> io::Result<()> {
    let frame_count = write_wav_header(&mut writer, 1, 16, sample_rate_hz, channels)?;
    for frame in 0..frame_count {
        for channel in channels {
            writer.write_all(&float_to_i16(channel[frame]).to_le_bytes())?;
        }
    }
    Ok(())
}

pub fn write_wav_channels_i24<W: Write>(
    mut writer: W,
    sample_rate_hz: u32,
    channels: &[&[i32]],
) -> io::Result<()> {
    const MIN_I24: i32 = -(1 << 23);
    const MAX_I24: i32 = (1 << 23) - 1;
    let frame_count = write_wav_header(&mut writer, 1, 24, sample_rate_hz, channels)?;
    for frame in 0..frame_count {
        for channel in channels {
            let sample = channel[frame];
            if !(MIN_I24..=MAX_I24).contains(&sample) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "24-bit PCM sample out of range",
                ));
            }
            let bytes = sample.to_le_bytes();
            writer.write_all(&bytes[0..3])?;
        }
    }
    Ok(())
}

pub fn write_wav_channels_f32<W: Write>(
    mut writer: W,
    sample_rate_hz: u32,
    channels: &[&[f32]],
) -> io::Result<()> {
    let frame_count = write_wav_header(&mut writer, 3, 32, sample_rate_hz, channels)?;
    for frame in 0..frame_count {
        for channel in channels {
            writer.write_all(&channel[frame].to_le_bytes())?;
        }
    }
    Ok(())
}

fn write_wav_header<W: Write, T>(
    writer: &mut W,
    audio_format: u16,
    bits_per_sample: u16,
    sample_rate_hz: u32,
    channels: &[&[T]],
) -> io::Result<usize> {
    let channel_count = u16::try_from(channels.len())
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid channel count"))?;
    let frame_count = channels
        .iter()
        .map(|channel| channel.len())
        .min()
        .unwrap_or(0);
    let bytes_per_sample = bits_per_sample / 8;
    let block_align = channel_count
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "too many WAV channels"))?;
    let data_size = u32::try_from(frame_count)
        .ok()
        .and_then(|frames| frames.checked_mul(u32::from(block_align)))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "WAV is too large"))?;
    let riff_size = 36u32
        .checked_add(data_size)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "WAV is too large"))?;
    let byte_rate = sample_rate_hz
        .checked_mul(u32::from(block_align))
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "WAV sample rate is too large")
        })?;

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?;
    writer.write_all(&audio_format.to_le_bytes())?;
    writer.write_all(&channel_count.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;
    Ok(frame_count)
}

fn float_to_i16(sample: f32) -> i16 {
    let sample = sample.clamp(-1.0, 1.0);
    if sample < 0.0 {
        (sample * 32768.0) as i16
    } else {
        (sample * 32767.0) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_stereo_pcm_wav_header_and_samples() {
        let mut bytes = Vec::new();

        write_wav_stereo_i16(&mut bytes, 48_000, &[0.0, 1.0], &[-1.0, 0.5])
            .expect("wav bytes should write");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1);
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            48_000
        );
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            8
        );
        assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), 0);
        assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), -32768);
        assert_eq!(i16::from_le_bytes([bytes[48], bytes[49]]), 32767);
        assert_eq!(i16::from_le_bytes([bytes[50], bytes[51]]), 16383);
    }

    #[test]
    fn wav_writer_uses_the_shorter_stereo_buffer_length() {
        let mut bytes = Vec::new();

        write_wav_stereo_i16(&mut bytes, 44_100, &[0.0, 0.0], &[0.0])
            .expect("wav bytes should write");

        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            4
        );
        assert_eq!(bytes.len(), 48);
    }

    #[test]
    fn planar_wav_writer_preserves_mono_and_six_channel_samples() {
        let mut mono = Vec::new();
        write_wav_channels_i16(&mut mono, 48_000, &[&[-0.5, 0.5]]).expect("mono WAV should write");
        assert_eq!(u16::from_le_bytes([mono[22], mono[23]]), 1);
        assert_eq!(i16::from_le_bytes([mono[44], mono[45]]), -16384);
        assert_eq!(i16::from_le_bytes([mono[46], mono[47]]), 16383);

        let channels = vec![vec![0.25]; 6];
        let planes: Vec<_> = channels.iter().map(Vec::as_slice).collect();
        let mut surround = Vec::new();
        write_wav_channels_i16(&mut surround, 48_000, &planes)
            .expect("six channel WAV should write");
        assert_eq!(u16::from_le_bytes([surround[22], surround[23]]), 6);
        assert_eq!(
            u32::from_le_bytes([surround[40], surround[41], surround[42], surround[43]]),
            12
        );
        assert_eq!(surround.len(), 56);
    }

    #[test]
    fn writes_pcm24_and_float32_wav_headers_and_samples() {
        let mut i24_bytes = Vec::new();
        write_wav_channels_i24(&mut i24_bytes, 44_100, &[&[-8_388_608, 0], &[8_388_607, -1]])
            .expect("24-bit WAV should write");
        assert_eq!(u16::from_le_bytes([i24_bytes[20], i24_bytes[21]]), 1);
        assert_eq!(u16::from_le_bytes([i24_bytes[22], i24_bytes[23]]), 2);
        assert_eq!(u16::from_le_bytes([i24_bytes[32], i24_bytes[33]]), 6);
        assert_eq!(u16::from_le_bytes([i24_bytes[34], i24_bytes[35]]), 24);
        assert_eq!(
            u32::from_le_bytes([i24_bytes[40], i24_bytes[41], i24_bytes[42], i24_bytes[43]]),
            12
        );
        assert_eq!(&i24_bytes[44..47], &[0x00, 0x00, 0x80]);
        assert_eq!(&i24_bytes[47..50], &[0xff, 0xff, 0x7f]);
        assert_eq!(&i24_bytes[50..53], &[0x00, 0x00, 0x00]);
        assert_eq!(&i24_bytes[53..56], &[0xff, 0xff, 0xff]);

        let mut overflow = Vec::new();
        assert!(write_wav_channels_i24(&mut overflow, 48_000, &[&[8_388_608]]).is_err());

        let mut f32_bytes = Vec::new();
        write_wav_channels_f32(&mut f32_bytes, 48_000, &[&[-1.5, f32::NAN], &[0.25, 1.0]])
            .expect("float32 WAV should write");
        assert_eq!(u16::from_le_bytes([f32_bytes[20], f32_bytes[21]]), 3);
        assert_eq!(u16::from_le_bytes([f32_bytes[22], f32_bytes[23]]), 2);
        assert_eq!(u16::from_le_bytes([f32_bytes[32], f32_bytes[33]]), 8);
        assert_eq!(u16::from_le_bytes([f32_bytes[34], f32_bytes[35]]), 32);
        assert_eq!(
            f32::from_le_bytes(f32_bytes[44..48].try_into().unwrap()),
            -1.5
        );
        assert_eq!(
            f32::from_le_bytes(f32_bytes[48..52].try_into().unwrap()),
            0.25
        );
        assert!(f32::from_le_bytes(f32_bytes[52..56].try_into().unwrap()).is_nan());
    }
}
