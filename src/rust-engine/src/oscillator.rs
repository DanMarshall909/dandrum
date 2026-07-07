use std::f32::consts::TAU;

use crate::builtins::{WAVEFORM_SAW, WAVEFORM_SINE, WAVEFORM_SQUARE, WAVEFORM_TRIANGLE};

/// Reference frequency (Hz) produced by the oscillator when its `pitch` input
/// carries a ratio of `1.0`. A drum body oscillator tunes itself by driving this
/// input with `target_hz / OSCILLATOR_BASE_HZ`.
pub const OSCILLATOR_BASE_HZ: f32 = 220.0;

/// Selectable oscillator waveform shapes.
///
/// `Saw` preserves the historical rising-ramp behaviour and remains the default
/// so existing patches render identically when no `waveform` parameter is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Saw,
    Sine,
    Triangle,
    Square,
}

impl Waveform {
    pub const DEFAULT: Self = Self::Saw;

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            WAVEFORM_SAW => Some(Self::Saw),
            WAVEFORM_SINE => Some(Self::Sine),
            WAVEFORM_TRIANGLE => Some(Self::Triangle),
            WAVEFORM_SQUARE => Some(Self::Square),
            _ => None,
        }
    }

    /// Evaluate the waveform at a normalised phase in `[0.0, 1.0)`, returning a
    /// sample in `[-1.0, 1.0]`.
    pub fn sample(self, phase: f32) -> f32 {
        match self {
            Self::Saw => phase * 2.0 - 1.0,
            Self::Sine => (phase * TAU).sin(),
            Self::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            Self::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_waveform_name_is_rejected() {
        assert!(
            Waveform::from_str("noise").is_none(),
            "only declared oscillator waveforms should parse"
        );
    }

    #[test]
    fn saw_is_the_default_and_preserves_rising_ramp_behaviour() {
        assert_eq!(
            Waveform::DEFAULT,
            Waveform::Saw,
            "saw must remain the default so existing patches are unchanged"
        );
        assert_approx_eq!(
            Waveform::Saw.sample(0.0),
            -1.0,
            1e-6,
            "saw should start at -1.0"
        );
        assert_approx_eq!(
            Waveform::Saw.sample(0.5),
            0.0,
            1e-6,
            "saw should cross zero at half phase"
        );
    }

    #[test]
    fn sine_crosses_zero_at_phase_boundaries_and_peaks_at_quarter() {
        assert_approx_eq!(
            Waveform::Sine.sample(0.0),
            0.0,
            1e-6,
            "sine should be zero at phase 0"
        );
        assert_approx_eq!(
            Waveform::Sine.sample(0.25),
            1.0,
            1e-6,
            "sine should peak at quarter phase"
        );
    }

    #[test]
    fn triangle_and_square_stay_within_unit_range() {
        for step in 0..=100 {
            let phase = step as f32 / 100.0;
            for waveform in [Waveform::Triangle, Waveform::Square] {
                let sample = waveform.sample(phase);
                assert!(
                    (-1.0..=1.0).contains(&sample),
                    "waveform {waveform:?} should stay within [-1, 1] at phase {phase}"
                );
            }
        }
    }
}
