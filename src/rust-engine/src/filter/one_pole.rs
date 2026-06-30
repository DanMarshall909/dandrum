use super::FilterAlgorithm;

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
        let mut filter = Self {
            mode: OnePoleMode::Lowpass,
            g: 0.0,
            prev_x: 0.0,
            prev_y: 0.0,
            sample_rate,
            cutoff: 1000.0,
        };
        filter.update_coeffs();
        filter
    }

    pub fn set_mode(&mut self, mode: OnePoleMode) {
        self.mode = mode;
    }

    pub fn set_cutoff(&mut self, hz: f64) {
        self.cutoff = hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_coeffs();
    }

    fn update_coeffs(&mut self) {
        let normalized_frequency = self.cutoff / self.sample_rate;
        self.g = (-2.0 * std::f64::consts::PI * normalized_frequency).exp();
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
