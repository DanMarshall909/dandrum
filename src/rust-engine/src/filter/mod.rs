mod biquad;
mod comb;
mod moog;
mod one_pole;

pub use biquad::{BiquadFilter, BiquadMode};
pub use comb::{CombFilter, CombType};
pub use moog::MoogLadder;
pub use one_pole::{OnePoleFilter, OnePoleMode};

pub enum Algorithm {
    Biquad,
    Moog,
    Comb,
    OnePole,
}

pub trait FilterAlgorithm {
    fn process(&mut self, input: f32) -> f32;
    fn reset(&mut self);
    fn set_cutoff(&mut self, _hz: f64, _sample_rate: f64) {}
    fn set_cutoff_control(&mut self, control: f32, sample_rate: f64) {
        let base: f64 = 8000.0 / 20.0;
        let hz = 20.0 * base.powf(control as f64);
        self.set_cutoff(hz, sample_rate);
    }
    fn set_resonance_control(&mut self, control: f32) {
        self.set_resonance(control as f64);
    }
    fn set_resonance(&mut self, _q: f64) {}
    fn set_gain_db(&mut self, _db: f64) {}
}

#[cfg(test)]
mod tests;
