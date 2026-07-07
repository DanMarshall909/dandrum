#![deny(dead_code)]

#[cfg(test)]
mod test_allocator;

pub mod core;

pub(crate) mod compiled_patch;

pub(crate) mod preparation;

pub mod graph_processor;

pub(crate) mod builtins;

pub mod graph;

pub mod cli;

pub mod instrument_state;

pub mod patch;

pub mod script;

pub(crate) mod sample;

pub(crate) mod synth;

pub mod wav;

pub(crate) mod voice_allocator;

pub(crate) mod fft;

pub(crate) mod delay_line;
pub(crate) mod echo;
pub(crate) mod filter;
pub(crate) mod reverb;

pub(crate) mod realtime;

pub(crate) mod crossover;

pub(crate) mod spectral;

pub(crate) mod curve_mapper;
pub(crate) mod decay;
pub(crate) mod envelope_follower;

pub(crate) mod audio_loading;

pub(crate) mod convolution;
pub(crate) mod dynamics_processor;
pub(crate) mod saturator;

pub mod ffi;

pub mod diagnostics;
