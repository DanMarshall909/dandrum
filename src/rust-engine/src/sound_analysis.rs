use std::io::{self, Write};

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct AnalysisSettings {
    pub frame_size: usize,
    pub hop_size: usize,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
    pub silence_rms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisFrame {
    pub start_frame: u64,
    pub time_seconds: f64,
    pub rms: f64,
    pub peak: f64,
    pub spectral_centroid_hz: Option<f64>,
}

pub fn analyze_sound(
    samples: &[f32],
    sample_rate_hz: u32,
    settings: AnalysisSettings,
) -> Result<Vec<AnalysisFrame>, String> {
    validate_settings(sample_rate_hz, settings)?;
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("sound analysis requires finite samples".to_string());
    }
    if samples.len() < settings.frame_size {
        return Ok(Vec::new());
    }

    let hann: Vec<f32> = (0..settings.frame_size)
        .map(|index| {
            (0.5 * (1.0
                - (std::f64::consts::TAU * index as f64 / settings.frame_size as f64).cos()))
                as f32
        })
        .collect();
    let mut frames = Vec::new();

    for start in (0..=samples.len() - settings.frame_size).step_by(settings.hop_size) {
        let samples = &samples[start..start + settings.frame_size];
        let rms = (samples
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / settings.frame_size as f64)
            .sqrt();
        let peak = samples
            .iter()
            .map(|sample| f64::from(sample.abs()))
            .fold(0.0, f64::max);
        let spectral_centroid_hz = if rms <= settings.silence_rms {
            None
        } else {
            let windowed: Vec<f32> = samples
                .iter()
                .zip(&hann)
                .map(|(sample, window)| sample * window)
                .collect();
            let spectrum =
                crate::fft::compute_magnitude_response(&windowed, f64::from(sample_rate_hz));
            let (weighted_sum, magnitude_sum) = spectrum
                .bins
                .into_iter()
                .filter(|(frequency_hz, _)| {
                    *frequency_hz >= settings.min_frequency_hz
                        && *frequency_hz <= settings.max_frequency_hz
                })
                .fold(
                    (0.0, 0.0),
                    |(weighted_sum, magnitude_sum), (frequency_hz, db)| {
                        let magnitude = 10.0_f64.powf(db / 20.0);
                        (
                            weighted_sum + frequency_hz * magnitude,
                            magnitude_sum + magnitude,
                        )
                    },
                );
            (magnitude_sum > 0.0).then_some(weighted_sum / magnitude_sum)
        };
        let center_frame = start + settings.frame_size / 2;
        frames.push(AnalysisFrame {
            start_frame: start as u64,
            time_seconds: center_frame as f64 / f64::from(sample_rate_hz),
            rms,
            peak,
            spectral_centroid_hz,
        });
    }

    Ok(frames)
}

pub fn write_metrics_csv<W: Write>(mut writer: W, frames: &[AnalysisFrame]) -> io::Result<()> {
    writeln!(
        writer,
        "start_frame,time_seconds,rms,peak,spectral_centroid_hz"
    )?;
    for frame in frames {
        write!(
            writer,
            "{},{:.9},{:.9},{:.9},",
            frame.start_frame, frame.time_seconds, frame.rms, frame.peak
        )?;
        if let Some(centroid_hz) = frame.spectral_centroid_hz {
            write!(writer, "{centroid_hz:.3}")?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn validate_settings(sample_rate_hz: u32, settings: AnalysisSettings) -> Result<(), String> {
    if sample_rate_hz == 0 {
        return Err("sound analysis sample rate must be positive".to_string());
    }
    if settings.frame_size < 2 {
        return Err("sound analysis frame size must be at least 2".to_string());
    }
    if settings.hop_size == 0 {
        return Err("sound analysis hop size must be positive".to_string());
    }
    if settings.silence_rms < 0.0 || !settings.silence_rms.is_finite() {
        return Err("sound analysis silence RMS must be finite and non-negative".to_string());
    }
    let nyquist_hz = f64::from(sample_rate_hz) / 2.0;
    if !settings.min_frequency_hz.is_finite()
        || !settings.max_frequency_hz.is_finite()
        || settings.min_frequency_hz < 0.0
        || settings.max_frequency_hz <= settings.min_frequency_hz
        || settings.max_frequency_hz > nyquist_hz
    {
        return Err(format!(
            "sound analysis frequency band must satisfy 0 <= min < max <= {nyquist_hz} Hz"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingWriter {
        writes_before_failure: usize,
    }

    impl Write for FailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.writes_before_failure == 0 {
                return Err(io::Error::other("injected write failure"));
            }
            self.writes_before_failure -= 1;
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    const SETTINGS: AnalysisSettings = AnalysisSettings {
        frame_size: 1_024,
        hop_size: 128,
        min_frequency_hz: 30.0,
        max_frequency_hz: 20_000.0,
        silence_rms: 1.0e-6,
    };

    #[test]
    fn audible_and_silent_frames_report_meaningful_metrics() {
        let sample_rate_hz = 48_000;
        let frequency_hz = 1_500.0;
        let tone: Vec<f32> = (0..SETTINGS.frame_size)
            .map(|frame| {
                (std::f64::consts::TAU * frequency_hz * frame as f64 / sample_rate_hz as f64).sin()
                    as f32
                    * 0.5
            })
            .collect();

        let audible = analyze_sound(&tone, sample_rate_hz, SETTINGS)
            .expect("valid audible frame should analyze");
        let silent = analyze_sound(&vec![0.0; SETTINGS.frame_size], sample_rate_hz, SETTINGS)
            .expect("valid silent frame should analyze");

        assert_eq!(audible.len(), 1);
        assert_eq!(audible[0].start_frame, 0);
        assert!((audible[0].time_seconds - 512.0 / 48_000.0).abs() < 1.0e-12);
        assert!((audible[0].rms - 0.5 / 2.0_f64.sqrt()).abs() < 1.0e-6);
        assert!((audible[0].peak - 0.5).abs() < 1.0e-6);
        let centroid = audible[0]
            .spectral_centroid_hz
            .expect("audible tone should have a centroid");
        assert!(
            (centroid - frequency_hz).abs() < 5.0,
            "expected {frequency_hz} Hz centroid, got {centroid} Hz"
        );

        assert_eq!(silent.len(), 1);
        assert_eq!(silent[0].rms, 0.0);
        assert_eq!(silent[0].peak, 0.0);
        assert_eq!(silent[0].spectral_centroid_hz, None);
    }

    #[test]
    fn metrics_csv_records_level_and_spectral_trajectory() {
        let frames = vec![
            AnalysisFrame {
                start_frame: 0,
                time_seconds: 0.0106666667,
                rms: 0.25,
                peak: 0.5,
                spectral_centroid_hz: Some(1_500.0),
            },
            AnalysisFrame {
                start_frame: 128,
                time_seconds: 0.0133333333,
                rms: 0.0,
                peak: 0.0,
                spectral_centroid_hz: None,
            },
        ];
        let mut csv = Vec::new();

        write_metrics_csv(&mut csv, &frames).expect("metrics CSV should write");
        let csv = String::from_utf8(csv).expect("metrics CSV should be UTF-8");

        assert!(csv.starts_with("start_frame,time_seconds,rms,peak,spectral_centroid_hz\n"));
        assert!(csv.contains("0,0.010666667,0.250000000,0.500000000,1500.000"));
        assert!(csv.contains("128,0.013333333,0.000000000,0.000000000,\n"));
    }

    #[test]
    fn analysis_rejects_invalid_settings_and_non_finite_audio() {
        let samples = vec![0.0; SETTINGS.frame_size];
        let invalid_cases = [
            (0, SETTINGS, "sample rate must be positive"),
            (
                48_000,
                AnalysisSettings {
                    frame_size: 1,
                    ..SETTINGS
                },
                "frame size must be at least 2",
            ),
            (
                48_000,
                AnalysisSettings {
                    hop_size: 0,
                    ..SETTINGS
                },
                "hop size must be positive",
            ),
            (
                48_000,
                AnalysisSettings {
                    silence_rms: -1.0,
                    ..SETTINGS
                },
                "silence RMS must be finite and non-negative",
            ),
            (
                48_000,
                AnalysisSettings {
                    silence_rms: f64::NAN,
                    ..SETTINGS
                },
                "silence RMS must be finite and non-negative",
            ),
            (
                48_000,
                AnalysisSettings {
                    min_frequency_hz: -1.0,
                    ..SETTINGS
                },
                "frequency band must satisfy",
            ),
            (
                48_000,
                AnalysisSettings {
                    max_frequency_hz: 30.0,
                    ..SETTINGS
                },
                "frequency band must satisfy",
            ),
            (
                48_000,
                AnalysisSettings {
                    max_frequency_hz: 24_001.0,
                    ..SETTINGS
                },
                "frequency band must satisfy",
            ),
            (
                48_000,
                AnalysisSettings {
                    min_frequency_hz: f64::NAN,
                    ..SETTINGS
                },
                "frequency band must satisfy",
            ),
            (
                48_000,
                AnalysisSettings {
                    max_frequency_hz: f64::NAN,
                    ..SETTINGS
                },
                "frequency band must satisfy",
            ),
        ];

        for (sample_rate_hz, settings, expected) in invalid_cases {
            let error = analyze_sound(&samples, sample_rate_hz, settings)
                .expect_err("invalid analysis settings should fail");
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let mut non_finite = samples;
        non_finite[0] = f32::NAN;
        let error = analyze_sound(&non_finite, 48_000, SETTINGS)
            .expect_err("non-finite samples should fail");
        assert!(error.contains("finite samples"));
    }

    #[test]
    fn analysis_of_audio_shorter_than_one_frame_is_empty() {
        let frames = analyze_sound(&vec![0.0; SETTINGS.frame_size - 1], 48_000, SETTINGS)
            .expect("short finite input remains valid");

        assert!(frames.is_empty());
    }

    #[test]
    fn metrics_csv_propagates_header_and_row_write_failures() {
        let row = AnalysisFrame {
            start_frame: 0,
            time_seconds: 0.0,
            rms: 0.25,
            peak: 0.5,
            spectral_centroid_hz: Some(1_500.0),
        };

        let header_error = write_metrics_csv(
            FailingWriter {
                writes_before_failure: 0,
            },
            std::slice::from_ref(&row),
        )
        .expect_err("header write failure should propagate");
        let row_error = write_metrics_csv(
            FailingWriter {
                writes_before_failure: 1,
            },
            &[row],
        )
        .expect_err("row write failure should propagate");

        assert_eq!(header_error.kind(), io::ErrorKind::Other);
        assert_eq!(row_error.kind(), io::ErrorKind::Other);
        FailingWriter {
            writes_before_failure: 1,
        }
        .flush()
        .expect("test writer flush should succeed");
    }
}
