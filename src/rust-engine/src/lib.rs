#![deny(dead_code)]

#[macro_use]
mod test_support;

#[cfg(test)]
mod test_allocator;

#[cfg(test)]
mod drum_voice_authoring_tests;

pub mod core;

pub mod compiled_patch;

pub mod preparation;

pub mod graph_processor;

pub(crate) mod builtins;

pub mod graph;
pub mod kernel;
pub mod cli;
pub mod instrument_state;
pub mod module_reference;
pub mod module_package;
pub mod module_library;
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
pub(crate) mod oscillator;

pub(crate) mod audio_loading;

pub(crate) mod convolution;
pub(crate) mod dynamics_processor;
pub(crate) mod saturator;

pub mod ffi;
pub mod ffi_status;

pub mod diagnostics;
