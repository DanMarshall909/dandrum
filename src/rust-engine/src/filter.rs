pub enum Algorithm {
    Biquad,
    Moog,
    Comb,
    OnePole,
}

pub enum BiquadMode {
    Lowpass,
    Highpass,
    Peaking,
}

pub enum CombType {
    Feedback,
    Feedforward,
}

pub trait FilterAlgorithm {
    fn process(&mut self, input: f32) -> f32;
    fn reset(&mut self);
}

pub struct BiquadFilter {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    pub fn new_lowpass(cutoff_norm: f64, q: f64) -> Self {
        let mut f = Self::default();
        f.set_coefficients_lowpass(cutoff_norm, q);
        f
    }

    pub fn new_highpass(cutoff_norm: f64, q: f64) -> Self {
        let mut f = Self::default();
        f.set_coefficients_highpass(cutoff_norm, q);
        f
    }

    pub fn new_peaking(cutoff_norm: f64, q: f64, gain_db: f64) -> Self {
        let mut f = Self::default();
        f.set_coefficients_peaking(cutoff_norm, q, gain_db);
        f
    }

    pub fn set_coefficients_lowpass(&mut self, cutoff_norm: f64, q: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha;

        let omc = (1.0 - cos_w) / 2.0;
        self.b0 = omc / a0;
        self.b1 = (1.0 - cos_w) / a0;
        self.b2 = omc / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_coefficients_highpass(&mut self, cutoff_norm: f64, q: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha;

        let opc = (1.0 + cos_w) / 2.0;
        self.b0 = opc / a0;
        self.b1 = -(1.0 + cos_w) / a0;
        self.b2 = opc / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_coefficients_peaking(&mut self, cutoff_norm: f64, q: f64, gain_db: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha / a;

        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        }
    }
}

impl FilterAlgorithm for BiquadFilter {
    fn process(&mut self, input: f32) -> f32 {
        let x = input as f64;
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y as f32
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

pub struct MoogLadder {
    stage: [f64; 4],
    cutoff: f64,
    resonance: f64,
    sample_rate: f64,
    g_pole: f64,
}

impl MoogLadder {
    pub fn new(sample_rate: f64) -> Self {
        let mut m = Self {
            stage: [0.0; 4],
            cutoff: 1000.0,
            resonance: 0.0,
            sample_rate,
            g_pole: 0.0,
        };
        m.update_coeffs();
        m
    }

    pub fn set_cutoff(&mut self, hz: f64) {
        self.cutoff = hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_coeffs();
    }

    pub fn set_resonance(&mut self, res: f64) {
        self.resonance = res.clamp(0.0, 0.99);
    }

    fn update_coeffs(&mut self) {
        let f = self.cutoff / self.sample_rate;
        self.g_pole = 2.0 * (std::f64::consts::PI * f).sin();
    }
}

impl FilterAlgorithm for MoogLadder {
    fn process(&mut self, input: f32) -> f32 {
        let g = self.g_pole;
        let feedback = self.resonance * 4.0 * (1.0 - g / (1.0 + g));

        let mut u = input as f64 - feedback * self.stage[3];

        for i in 0..4 {
            self.stage[i] += g * (u - self.stage[i]);
            self.stage[i] = self.stage[i].tanh();
            u = self.stage[i];
        }

        self.stage[3] as f32
    }

    fn reset(&mut self) {
        self.stage = [0.0; 4];
    }
}

pub struct CombFilter {
    delay_line: Vec<f64>,
    write_pos: usize,
    delay_samples: usize,
    gain: f64,
    comb_type: CombType,
}

impl CombFilter {
    pub fn new(delay_samples: usize, gain: f64, comb_type: CombType) -> Self {
        let size = delay_samples.max(1);
        Self {
            delay_line: vec![0.0; size],
            write_pos: 0,
            delay_samples: size,
            gain: gain.clamp(0.0, 0.99),
            comb_type,
        }
    }

    pub fn set_delay(&mut self, samples: usize) {
        let new_size = samples.max(1);
        if new_size != self.delay_samples {
            self.delay_line.resize(new_size, 0.0);
            self.delay_samples = new_size;
            self.write_pos %= self.delay_samples;
        }
    }

    pub fn set_gain(&mut self, gain: f64) {
        self.gain = gain.clamp(0.0, 0.99);
    }
}

impl FilterAlgorithm for CombFilter {
    fn process(&mut self, input: f32) -> f32 {
        let read_pos = if self.write_pos >= self.delay_samples - 1 {
            0
        } else {
            self.write_pos + 1
        };
        let delayed = self.delay_line[read_pos];
        let x = input as f64;

        let output = match self.comb_type {
            CombType::Feedforward => x + self.gain * delayed,
            CombType::Feedback => {
                self.delay_line[self.write_pos] = x + self.gain * delayed;
                delayed
            }
        };

        if matches!(self.comb_type, CombType::Feedforward) {
            self.delay_line[self.write_pos] = x;
        }

        self.write_pos = (self.write_pos + 1) % self.delay_samples;
        output as f32
    }

    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.write_pos = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OnePoleMode {
    Lowpass,
    Highpass,
}

pub struct OnePoleFilter {
    mode: OnePoleMode,
    g: f64,
    prev_x: f64,
    prev_y: f64,
    sample_rate: f64,
    cutoff: f64,
}

impl OnePoleFilter {
    pub fn new(sample_rate: f64) -> Self {
        let mut f = Self {
            mode: OnePoleMode::Lowpass,
            g: 0.0,
            prev_x: 0.0,
            prev_y: 0.0,
            sample_rate,
            cutoff: 1000.0,
        };
        f.update_coeffs();
        f
    }

    pub fn set_mode(&mut self, mode: OnePoleMode) {
        self.mode = mode;
    }

    pub fn set_cutoff(&mut self, hz: f64) {
        self.cutoff = hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_coeffs();
    }

    fn update_coeffs(&mut self) {
        let norm = self.cutoff / self.sample_rate;
        self.g = (-2.0 * std::f64::consts::PI * norm).exp();
    }
}

impl FilterAlgorithm for OnePoleFilter {
    fn process(&mut self, input: f32) -> f32 {
        let x = input as f64;
        let y = match self.mode {
            OnePoleMode::Lowpass => (1.0 - self.g) * x + self.g * self.prev_y,
            OnePoleMode::Highpass => self.g * (self.prev_y + x - self.prev_x),
        };
        self.prev_x = x;
        self.prev_y = y;
        y as f32
    }

    fn reset(&mut self) {
        self.prev_x = 0.0;
        self.prev_y = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_filter_response<F>(filter: &mut F, sample_rate: f64, impulse_len: usize) -> Vec<(f64, f64)>
    where
        F: FilterAlgorithm,
    {
        let mut out = vec![0.0f32; impulse_len];
        for i in 0..impulse_len {
            let imp = if i == 0 { 1.0 } else { 0.0 };
            out[i] = filter.process(imp);
        }
        filter.reset();
        crate::fft::compute_magnitude_response(&out, sample_rate).bins
    }

    fn magnitude_at(bins: &[(f64, f64)], target_norm: f64, sample_rate: f64) -> f64 {
        let target_hz = target_norm * sample_rate;
        bins.iter()
            .min_by(|(a, _), (b, _)| {
                (a - target_hz).abs().partial_cmp(&(b - target_hz).abs()).unwrap()
            })
            .map(|&(_f, db)| db)
            .unwrap_or(-100.0)
    }

    #[test]
    fn biquad_lowpass_attenuates_high_frequencies() {
        let mut filter = BiquadFilter::new_lowpass(0.1, 0.707);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let passband_db = magnitude_at(&response, 0.1, 48000.0);
        let stopband_db = magnitude_at(&response, 0.4, 48000.0);
        assert!(stopband_db < passband_db - 12.0,
            "lowpass: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB (expected ≥12 dB difference)");
    }

    #[test]
    fn biquad_highpass_attenuates_low_frequencies() {
        let mut filter = BiquadFilter::new_highpass(0.3, 0.707);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let passband_db = magnitude_at(&response, 0.4, 48000.0);
        let stopband_db = magnitude_at(&response, 0.05, 48000.0);
        assert!(stopband_db < passband_db - 12.0,
            "highpass: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB");
    }

    #[test]
    fn biquad_peaking_boosts_at_center_frequency() {
        let mut filter = BiquadFilter::new_peaking(0.1, 2.0, 12.0);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let peak_db = magnitude_at(&response, 0.1, 48000.0);
        assert!(peak_db > 6.0,
            "peaking boost: expected >6 dB, got {peak_db:.1} dB");
    }

    #[test]
    fn biquad_peaking_cuts_at_center_frequency() {
        let mut filter = BiquadFilter::new_peaking(0.1, 2.0, -12.0);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let cut_db = magnitude_at(&response, 0.1, 48000.0);
        assert!(cut_db < -6.0,
            "peaking cut: expected < -6 dB, got {cut_db:.1} dB");
    }

    #[test]
    fn biquad_resonance_creates_peak() {
        let mut low_q = BiquadFilter::new_lowpass(0.1, 0.707);
        let mut high_q = BiquadFilter::new_lowpass(0.1, 5.0);
        let low_resp = run_filter_response(&mut low_q, 48000.0, 8192);
        let high_resp = run_filter_response(&mut high_q, 48000.0, 8192);
        let low_db = magnitude_at(&low_resp, 0.1, 48000.0);
        let high_db = magnitude_at(&high_resp, 0.1, 48000.0);
        assert!(high_db > low_db + 3.0,
            "resonance peak: high Q {high_db:.1} dB should be > low Q {low_db:.1} dB + 3 dB");
    }

    #[test]
    fn biquad_no_nan_at_extreme_cutoff() {
        let mut filter = BiquadFilter::new_lowpass(0.0, 0.707);
        for _ in 0..100 {
            let out = filter.process(1.0);
            assert!(!out.is_nan(), "output should not be NaN at cutoff=0");
        }
    }

    #[test]
    fn moog_resonance_emphasizes_cutoff() {
        let mut low_res = MoogLadder::new(48000.0);
        low_res.set_cutoff(1000.0);
        low_res.set_resonance(0.0);
        let mut high_res = MoogLadder::new(48000.0);
        high_res.set_cutoff(1000.0);
        high_res.set_resonance(0.8);

        let low_response = run_filter_response(&mut low_res, 48000.0, 8192);
        let high_response = run_filter_response(&mut high_res, 48000.0, 8192);

        let low_db = magnitude_at(&low_response, 1000.0 / 48000.0, 48000.0);
        let high_db = magnitude_at(&high_response, 1000.0 / 48000.0, 48000.0);
        assert!(high_db > low_db + 2.0,
            "moog resonance: high res {high_db:.1} dB vs low res {low_db:.1} dB");
    }

    #[test]
    fn moog_no_nan_at_extreme_resonance() {
        let mut filter = MoogLadder::new(48000.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.99);
        for _ in 0..1000 {
            let out = filter.process(1.0);
            assert!(out.is_finite(), "output should be finite at high resonance");
        }
    }

    #[test]
    fn comb_feedforward_produces_notches() {
        let delay_samples = 96;
        let mut filter = CombFilter::new(delay_samples, 0.7, CombType::Feedforward);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let peak_norm = 1.0 / delay_samples as f64;
        let notch_norm = 0.5 / delay_samples as f64;

        let peak_db = magnitude_at(&response, peak_norm, 48000.0);
        let notch_db = magnitude_at(&response, notch_norm, 48000.0);
        assert!(peak_db > notch_db + 3.0,
            "comb feedforward: expected peak > notch");
    }

    #[test]
    fn comb_feedback_produces_peaks() {
        let delay_samples = 96;
        let mut filter = CombFilter::new(delay_samples, 0.7, CombType::Feedback);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let peak_norm = 1.0 / delay_samples as f64;
        let notch_norm = 0.5 / delay_samples as f64;

        let peak_db = magnitude_at(&response, peak_norm, 48000.0);
        let notch_db = magnitude_at(&response, notch_norm, 48000.0);
        assert!(peak_db > notch_db + 3.0,
            "comb feedback: expected peak > notch");
    }

    #[test]
    fn comb_gain_stability() {
        let mut filter = CombFilter::new(100, 0.99, CombType::Feedback);
        for _ in 0..10000 {
            let out = filter.process(1.0);
            assert!(out.is_finite(), "comb feedback should be bounded at max gain");
        }
    }

    #[test]
    fn comb_no_nan() {
        let mut filter = CombFilter::new(100, 0.0, CombType::Feedback);
        for _ in 0..1000 {
            let out = filter.process(0.0);
            assert!(!out.is_nan(), "comb output should not be NaN");
        }
    }

    #[test]
    fn one_pole_lowpass_dc_gain_is_unity() {
        let mut filter = OnePoleFilter::new(48000.0);
        filter.set_cutoff(1000.0);
        filter.process(1.0);
        // After many samples at DC, output should approach 1.0
        let mut y = 0.0f32;
        for _ in 0..1000 {
            y = filter.process(1.0);
        }
        assert!((y - 1.0).abs() < 0.01, "DC gain should be ~1.0, got {y}");
    }

    #[test]
    fn one_pole_lowpass_attenuates_high_frequencies() {
        let mut filter = OnePoleFilter::new(48000.0);
        filter.set_cutoff(1000.0);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let passband_db = magnitude_at(&response, 0.02, 48000.0);
        let stopband_db = magnitude_at(&response, 0.4, 48000.0);
        assert!(stopband_db < passband_db - 3.0,
            "one-pole LP: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB");
    }

    #[test]
    fn one_pole_highpass_attenuates_low_frequencies() {
        let mut filter = OnePoleFilter::new(48000.0);
        filter.set_mode(OnePoleMode::Highpass);
        filter.set_cutoff(1000.0);
        let response = run_filter_response(&mut filter, 48000.0, 8192);
        let passband_db = magnitude_at(&response, 0.4, 48000.0);
        let stopband_db = magnitude_at(&response, 0.01, 48000.0);
        assert!(stopband_db < passband_db - 6.0,
            "one-pole HP: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB");
    }

    #[test]
    fn one_pole_no_nan_at_extreme_cutoff() {
        let mut filter = OnePoleFilter::new(48000.0);
        filter.set_cutoff(20.0);
        for _ in 0..100 {
            let out = filter.process(1.0);
            assert!(out.is_finite(), "output should be finite at 20 Hz cutoff");
        }
        filter.set_cutoff(20000.0);
        for _ in 0..100 {
            let out = filter.process(1.0);
            assert!(out.is_finite(), "output should be finite at 20 kHz cutoff");
        }
    }

    #[test]
    fn one_pole_reset_clears_state() {
        let mut filter = OnePoleFilter::new(48000.0);
        filter.process(1.0);
        filter.process(1.0);
        filter.reset();
        // After reset, processing a zero input should produce zero
        let out = filter.process(0.0);
        assert!(out.abs() < 1e-6, "after reset with zero input, expected 0, got {out}");
    }
}
