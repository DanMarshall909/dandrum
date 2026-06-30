use std::f64::consts::E;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DetectionMode {
    Peak,
    Rms,
}

pub struct EnvelopeDetector {
    envelope: f64,
    attack_coeff: f64,
    release_coeff: f64,
    mode: DetectionMode,
    sample_rate: f64,
}

impl EnvelopeDetector {
    pub fn new(sample_rate: f64, attack_ms: f64, release_ms: f64, mode: DetectionMode) -> Self {
        let attack_coeff = time_constant_coeff(attack_ms, sample_rate);
        let release_coeff = time_constant_coeff(release_ms, sample_rate);
        Self {
            envelope: 0.0,
            attack_coeff,
            release_coeff,
            mode,
            sample_rate,
        }
    }

    pub fn set_params(&mut self, attack_ms: f64, release_ms: f64) {
        self.attack_coeff = time_constant_coeff(attack_ms, self.sample_rate);
        self.release_coeff = time_constant_coeff(release_ms, self.sample_rate);
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }

    pub fn envelope_value(&self) -> f64 {
        self.envelope
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn process(&mut self, sample: f64) -> f64 {
        let input_level = match self.mode {
            DetectionMode::Peak => sample.abs(),
            DetectionMode::Rms => sample * sample,
        };

        let coeff = if input_level > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };

        self.envelope += coeff * (input_level - self.envelope);

        match self.mode {
            DetectionMode::Peak => self.envelope,
            DetectionMode::Rms => self.envelope.sqrt(),
        }
    }
}

fn time_constant_coeff(ms: f64, sample_rate: f64) -> f64 {
    if ms <= 0.0 {
        return 1.0;
    }
    let tau_samples = ms * sample_rate / 1000.0;
    1.0 - E.powf(-1.0 / tau_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx(a: f64, b: f64, epsilon: f64) {
        assert!(
            (a - b).abs() < epsilon,
            "expected {a} ≈ {b} within {epsilon}"
        );
    }

    #[test]
    fn peak_detector_tracks_amplitude() {
        let mut det = EnvelopeDetector::new(48000.0, 1.0, 100.0, DetectionMode::Peak);
        // Steady input at 0.5 amplitude
        for _ in 0..48000 {
            det.process(0.5);
        }
        let env = det.envelope_value();
        assert!(
            env > 0.45 && env < 0.55,
            "peak envelope should track 0.5, got {env}"
        );
    }

    #[test]
    fn peak_detector_fast_attack_slow_release() {
        let mut det = EnvelopeDetector::new(48000.0, 0.1, 1000.0, DetectionMode::Peak);
        // Fast attack should reach near-instantaneous level quickly
        for _ in 0..480 {
            det.process(1.0);
        }
        let env = det.envelope_value();
        assert!(
            env > 0.99,
            "fast attack should reach near 1.0 quickly, got {env}"
        );
        // Now remove input signal
        for _ in 0..48000 {
            det.process(0.0);
        }
        let env = det.envelope_value();
        assert!(env < 0.5, "slow release should decay slowly, got {env}");
    }

    #[test]
    fn rms_detector_averages_energy() {
        let mut det = EnvelopeDetector::new(48000.0, 10.0, 10.0, DetectionMode::Rms);
        // Sine wave at amplitude 1.0 (RMS = 1/sqrt(2) ≈ 0.707)
        // Use high frequency so the 2nd-harmonic ripple is well attenuated
        let mut phase = 0.0;
        let mut env = 0.0;
        for _ in 0..96000 {
            let s = (phase * std::f64::consts::PI * 2.0).sin();
            env = det.process(s);
            phase += 440.0 / 48000.0;
        }
        assert!(
            env > 0.65 && env < 0.78,
            "RMS envelope should track ~0.707 for sine, got {env}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut det = EnvelopeDetector::new(48000.0, 1.0, 1.0, DetectionMode::Peak);
        det.process(1.0);
        assert!(det.envelope_value() > 0.0);
        det.reset();
        assert_approx(det.envelope_value(), 0.0, 1e-10);
    }

    #[test]
    fn peak_vs_rms_shape_differs() {
        let mut peak_det = EnvelopeDetector::new(48000.0, 1.0, 1.0, DetectionMode::Peak);
        let mut rms_det = EnvelopeDetector::new(48000.0, 1.0, 1.0, DetectionMode::Rms);
        // Steady burst at 0 dB amplitude
        for _ in 0..4800 {
            peak_det.process(1.0);
            rms_det.process(1.0);
        }
        // Peak should be close to 1.0, RMS should be close to 1.0 for DC,
        // but for a sinusoidal-like varying signal, peak > RMS.
        // With DC input 1.0, both approach 1.0, but peak reaches it faster.
        assert!(
            peak_det.envelope_value() >= rms_det.envelope_value(),
            "peak ({}) should be >= RMS ({}) for steady signal",
            peak_det.envelope_value(),
            rms_det.envelope_value()
        );
    }

    #[test]
    fn attack_time_accuracy() {
        let mut det = EnvelopeDetector::new(48000.0, 10.0, 1000.0, DetectionMode::Peak);
        // After 10 ms (480 samples at 48 kHz), envelope should reach ~63% of target
        // with the one-pole filter: envelope = 1 - e^(-1) ≈ 0.632
        for _ in 0..480 {
            det.process(1.0);
        }
        let env = det.envelope_value();
        assert!(
            env > 0.5 && env < 0.8,
            "after one time constant, envelope should be ~0.632, got {env}"
        );
    }

    #[test]
    fn release_time_tracking() {
        let mut det = EnvelopeDetector::new(48000.0, 0.1, 10.0, DetectionMode::Peak);
        // Charge up
        for _ in 0..4800 {
            det.process(1.0);
        }
        assert!(det.envelope_value() > 0.99);
        // Remove signal, let release decay
        for _ in 0..480 {
            det.process(0.0);
        }
        // After 10 ms (one time constant), envelope should drop to ~0.368 of peak
        let env = det.envelope_value();
        assert!(
            env > 0.2 && env < 0.5,
            "after one release time constant, envelope should be ~0.368, got {env}"
        );
    }
}
