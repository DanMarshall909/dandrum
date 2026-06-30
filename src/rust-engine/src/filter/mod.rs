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
}

#[cfg(test)]
mod tests;
