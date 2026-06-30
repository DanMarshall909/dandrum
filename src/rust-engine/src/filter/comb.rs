use super::FilterAlgorithm;

pub enum CombType {
    Feedback,
    Feedforward,
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
    fn set_cutoff_control(&mut self, control: f32, sample_rate: f64) {
        let delay_ms = 1.0 + control.clamp(0.0, 1.0) as f64 * 99.0;
        let delay_samples = (sample_rate * delay_ms / 1000.0).round() as usize;
        self.set_delay(delay_samples.max(1));
    }

    fn set_resonance_control(&mut self, control: f32) {
        self.set_gain(control.clamp(0.0, 1.0) as f64 * 0.99);
    }

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
