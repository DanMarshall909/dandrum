use super::FilterAlgorithm;

pub enum BiquadMode {
    Lowpass,
    Highpass,
    Peaking,
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
    mode: BiquadMode,
    cutoff_norm: f64,
    q: f64,
    gain_db: f64,
}

impl BiquadFilter {
    pub fn new_lowpass(cutoff_norm: f64, q: f64) -> Self {
        let mut filter = Self::default();
        filter.mode = BiquadMode::Lowpass;
        filter.cutoff_norm = cutoff_norm;
        filter.q = q;
        filter.set_coefficients_lowpass(cutoff_norm, q);
        filter
    }

    pub fn new_highpass(cutoff_norm: f64, q: f64) -> Self {
        let mut filter = Self::default();
        filter.mode = BiquadMode::Highpass;
        filter.cutoff_norm = cutoff_norm;
        filter.q = q;
        filter.set_coefficients_highpass(cutoff_norm, q);
        filter
    }

    pub fn new_peaking(cutoff_norm: f64, q: f64, gain_db: f64) -> Self {
        let mut filter = Self::default();
        filter.mode = BiquadMode::Peaking;
        filter.cutoff_norm = cutoff_norm;
        filter.q = q;
        filter.gain_db = gain_db;
        filter.set_coefficients_peaking(cutoff_norm, q, gain_db);
        filter
    }

    pub fn set_coefficients_lowpass(&mut self, cutoff_norm: f64, q: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha;

        let one_minus_cos = (1.0 - cos_w) / 2.0;
        self.b0 = one_minus_cos / a0;
        self.b1 = (1.0 - cos_w) / a0;
        self.b2 = one_minus_cos / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_coefficients_highpass(&mut self, cutoff_norm: f64, q: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha;

        let one_plus_cos = (1.0 + cos_w) / 2.0;
        self.b0 = one_plus_cos / a0;
        self.b1 = -(1.0 + cos_w) / a0;
        self.b2 = one_plus_cos / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn update_coefficients(&mut self) {
        match self.mode {
            BiquadMode::Lowpass => self.set_coefficients_lowpass(self.cutoff_norm, self.q),
            BiquadMode::Highpass => self.set_coefficients_highpass(self.cutoff_norm, self.q),
            BiquadMode::Peaking => {
                self.set_coefficients_peaking(self.cutoff_norm, self.q, self.gain_db)
            }
        }
    }

    pub fn set_coefficients_peaking(&mut self, cutoff_norm: f64, q: f64, gain_db: f64) {
        let omega = std::f64::consts::TAU * cutoff_norm;
        let alpha = omega.sin() / (2.0 * q);
        let gain = 10.0_f64.powf(gain_db / 40.0);
        let cos_w = omega.cos();
        let a0 = 1.0 + alpha / gain;

        self.b0 = (1.0 + alpha * gain) / a0;
        self.b1 = (-2.0 * cos_w) / a0;
        self.b2 = (1.0 - alpha * gain) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha / gain) / a0;
    }
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            mode: BiquadMode::Lowpass,
            cutoff_norm: 0.0,
            q: 0.707,
            gain_db: 0.0,
        }
    }
}

impl FilterAlgorithm for BiquadFilter {
    fn set_cutoff(&mut self, hz: f64, sample_rate: f64) {
        self.cutoff_norm = (hz / sample_rate).clamp(0.0, 0.49);
        self.update_coefficients();
    }

    fn set_resonance_control(&mut self, control: f32) {
        self.set_resonance(0.1 + control as f64 * 9.9);
    }

    fn set_resonance(&mut self, q: f64) {
        self.q = q.clamp(0.1, 20.0);
        self.update_coefficients();
    }

    fn set_gain_db(&mut self, db: f64) {
        self.gain_db = db;
        self.update_coefficients();
    }

    fn process(&mut self, input: f32) -> f32 {
        let x = input as f64;
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
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
