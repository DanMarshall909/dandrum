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
    mut writer: W,
    sample_rate_hz: u32,
    left: &[f32],
    right: &[f32],
) -> io::Result<()> {
    let frame_count = left.len().min(right.len());
    let data_size = frame_count as u32 * 2 * 2;
    let riff_size = 36 + data_size;
    let byte_rate = sample_rate_hz * 2 * 2;
    let block_align: u16 = 2 * 2;

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&2u16.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&16u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;

    for frame in 0..frame_count {
        writer.write_all(&float_to_i16(left[frame]).to_le_bytes())?;
        writer.write_all(&float_to_i16(right[frame]).to_le_bytes())?;
    }

    Ok(())
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
}
