use crate::filter::{BiquadFilter, FilterAlgorithm};

pub struct CrossoverPair {
    lp: BiquadFilter,
    hp: BiquadFilter,
}

impl CrossoverPair {
    pub fn new(crossover_norm: f64, _sample_rate: f64) -> Self {
        let q = 0.707;
        let lp = BiquadFilter::new_lowpass(crossover_norm, q);
        let hp = BiquadFilter::new_highpass(crossover_norm, q);
        Self { lp, hp }
    }

    pub fn set_crossover(&mut self, crossover_norm: f64) {
        let q = 0.707;
        self.lp.set_coefficients_lowpass(crossover_norm, q);
        self.hp.set_coefficients_highpass(crossover_norm, q);
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        let low = self.lp.process(input);
        let high = self.hp.process(input);
        (low, high)
    }

    pub fn reset(&mut self) {
        self.lp.reset();
        self.hp.reset();
    }
}

pub struct LinkwitzRiley4 {
    lp_a: BiquadFilter,
    lp_b: BiquadFilter,
    hp_a: BiquadFilter,
    hp_b: BiquadFilter,
}

impl LinkwitzRiley4 {
    pub fn new(crossover_norm: f64) -> Self {
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let lp_a = BiquadFilter::new_lowpass(crossover_norm, q);
        let lp_b = BiquadFilter::new_lowpass(crossover_norm, q);
        let hp_a = BiquadFilter::new_highpass(crossover_norm, q);
        let hp_b = BiquadFilter::new_highpass(crossover_norm, q);
        Self {
            lp_a,
            lp_b,
            hp_a,
            hp_b,
        }
    }

    pub fn set_crossover(&mut self, crossover_norm: f64) {
        let q = std::f64::consts::FRAC_1_SQRT_2;
        self.lp_a.set_coefficients_lowpass(crossover_norm, q);
        self.lp_b.set_coefficients_lowpass(crossover_norm, q);
        self.hp_a.set_coefficients_highpass(crossover_norm, q);
        self.hp_b.set_coefficients_highpass(crossover_norm, q);
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        let low = self.lp_b.process(self.lp_a.process(input));
        let high = self.hp_b.process(self.hp_a.process(input));
        (low, high)
    }

    pub fn reset(&mut self) {
        self.lp_a.reset();
        self.lp_b.reset();
        self.hp_a.reset();
        self.hp_b.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fft;

    fn run_crossover_combined_response(
        xover: &mut LinkwitzRiley4,
        sample_rate: f64,
        impulse_len: usize,
    ) -> Vec<(f64, f64)> {
        let mut ir = vec![0.0f32; impulse_len];
        ir[0] = 1.0;
        let mut low_out = vec![0.0f32; impulse_len];
        let mut high_out = vec![0.0f32; impulse_len];
        for i in 0..impulse_len {
            let (l, h) = xover.process(ir[i]);
            low_out[i] = l;
            high_out[i] = h;
        }
        let combined: Vec<f32> = low_out
            .iter()
            .zip(high_out.iter())
            .map(|(a, b)| a + b)
            .collect();
        fft::compute_magnitude_response(&combined, sample_rate).bins
    }

    fn magnitude_at(bins: &[(f64, f64)], target_hz: f64) -> f64 {
        bins.iter()
            .min_by(|(a, _), (b, _)| {
                (a - target_hz)
                    .abs()
                    .partial_cmp(&(b - target_hz).abs())
                    .unwrap()
            })
            .map(|&(_, db)| db)
            .unwrap_or(-100.0)
    }

    #[test]
    fn lr4_sum_is_flat_at_crossover() {
        let sample_rate = 48000.0;
        let crossover_hz = 2000.0;
        let crossover_norm = crossover_hz / sample_rate;
        let mut xover = LinkwitzRiley4::new(crossover_norm);
        let combined = run_crossover_combined_response(&mut xover, sample_rate, 8192);
        let sum_db = magnitude_at(&combined, crossover_hz);
        assert!(
            sum_db > -1.0,
            "LR4 sum at crossover: expected ~0 dB, got {sum_db:.1} dB"
        );
    }

    #[test]
    fn lr4_crossover_extremes() {
        let mut xover = LinkwitzRiley4::new(0.0);
        let (l, h) = xover.process(1.0);
        assert!(!l.is_nan() && !h.is_nan(), "no NaN at crossover=0");

        let mut xover = LinkwitzRiley4::new(0.5);
        for _ in 0..100 {
            let (l, h) = xover.process(1.0);
            assert!(l.is_finite() && h.is_finite(), "finite output");
        }
    }
}
