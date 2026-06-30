use super::FilterAlgorithm;

pub struct MoogLadder {
    stage: [f64; 4],
    cutoff: f64,
    resonance: f64,
    sample_rate: f64,
    g_pole: f64,
}

impl MoogLadder {
    pub fn new(sample_rate: f64) -> Self {
        let mut filter = Self {
            stage: [0.0; 4],
            cutoff: 1000.0,
            resonance: 0.0,
            sample_rate,
            g_pole: 0.0,
        };
        filter.update_coeffs();
        filter
    }

    pub fn set_cutoff(&mut self, hz: f64) {
        self.cutoff = hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_coeffs();
    }

    pub fn set_resonance(&mut self, resonance: f64) {
        self.resonance = resonance.clamp(0.0, 0.99);
    }

    fn update_coeffs(&mut self) {
        let normalized_frequency = self.cutoff / self.sample_rate;
        self.g_pole = 2.0 * (std::f64::consts::PI * normalized_frequency).sin();
    }
}

impl FilterAlgorithm for MoogLadder {
    fn set_cutoff(&mut self, hz: f64, _sample_rate: f64) {
        self.set_cutoff(hz);
    }

    fn set_resonance_control(&mut self, control: f32) {
        self.set_resonance(control as f64 * 0.99);
    }

    fn set_resonance(&mut self, q: f64) {
        self.set_resonance(q);
    }

    fn process(&mut self, input: f32) -> f32 {
        let g = self.g_pole;
        let feedback = self.resonance * 4.0 * (1.0 - g / (1.0 + g));

        let mut u = input as f64 - feedback * self.stage[3];

        for stage in &mut self.stage {
            *stage += g * (u - *stage);
            *stage = stage.tanh();
            u = *stage;
        }

        self.stage[3] as f32
    }

    fn reset(&mut self) {
        self.stage = [0.0; 4];
    }
}
