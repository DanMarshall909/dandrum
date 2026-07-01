use std::collections::BTreeMap;

use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::CompiledNode;
use crate::convolution::Convolution;
use crate::crossover::CrossoverPair;
use crate::dynamics_processor::DynamicsProcessor;
use crate::echo::Echo;
use crate::filter::{BiquadFilter, BiquadMode, CombFilter, CombType, FilterAlgorithm, MoogLadder};
use crate::graph::ModuleNode;
use crate::reverb::Reverb;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::saturator::Saturator;
use crate::spectral::SpectralProcessor;

pub(super) enum PerModuleState {
    Oscillator {
        phase: f32,
        sample_rate: f32,
    },
    Adsr {
        level: f32,
        gate_active: bool,
        release_start_frame: u64,
        release_start_level: f32,
        sample_rate: f32,
    },
    Vca,
    AudioOutput,
    MidiInput,
    NoteToRate {
        rate: f32,
    },
    AudioMixer,
    // Intentionally monophonic until the engine has generic per-voice bus support.
    Sampler {
        sample: Option<LoadedSample>,
        position: f32,
        active: bool,
    },
    DynamicsProcessor {
        processor: DynamicsProcessor,
        sample_rate: f32,
    },
    Saturator {
        processor: Saturator,
    },
    Convolution {
        processor: Convolution,
    },
    Filter {
        filter: Box<dyn FilterAlgorithm>,
        sample_rate: f64,
    },
    Echo {
        processor: Echo,
        sample_rate: f64,
    },
    Reverb {
        processor: Reverb,
        sample_rate: f64,
    },
    FrequencySplitter {
        first: CrossoverPair,
        second: CrossoverPair,
        sample_rate: f64,
    },
    SpectralProcessor {
        processor: SpectralProcessor,
    },
}

impl PerModuleState {
    pub(super) fn new(
        module: &ModuleNode,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        let kind = ModuleKind::from_str(module.module_type())
            .unwrap_or_else(|| panic!("unknown module type: {}", module.module_type()));
        Self::from_kind(
            kind,
            module.id().as_str(),
            module.params(),
            sample_rate,
            sampler_assets,
        )
    }

    pub(super) fn new_compiled(
        node: &CompiledNode,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        Self::from_kind(
            node.module_kind,
            node.id.as_str(),
            &node.parameters,
            sample_rate,
            sampler_assets,
        )
    }

    fn from_kind(
        kind: ModuleKind,
        module_id: &str,
        params: &BTreeMap<String, String>,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        match kind {
            ModuleKind::Oscillator => PerModuleState::Oscillator {
                phase: 0.0,
                sample_rate,
            },
            ModuleKind::Adsr => PerModuleState::Adsr {
                level: 0.0,
                gate_active: false,
                release_start_frame: 0,
                release_start_level: 0.0,
                sample_rate,
            },
            ModuleKind::Gain => PerModuleState::Vca,
            ModuleKind::AudioOutput => PerModuleState::AudioOutput,
            ModuleKind::MidiInput => PerModuleState::MidiInput,
            ModuleKind::NoteToRate => PerModuleState::NoteToRate { rate: 1.0 },
            ModuleKind::AudioMixer => PerModuleState::AudioMixer,
            ModuleKind::Sampler => PerModuleState::Sampler {
                sample: sampler_assets.get(module_id).cloned(),
                position: 0.0,
                active: false,
            },
            ModuleKind::DynamicsProcessor => PerModuleState::DynamicsProcessor {
                processor: DynamicsProcessor::new(sample_rate as f64, 5.0, 50.0),
                sample_rate,
            },
            ModuleKind::Saturator => PerModuleState::Saturator {
                processor: Saturator::new(),
            },
            ModuleKind::Convolution => PerModuleState::Convolution {
                processor: Convolution::new(),
            },
            ModuleKind::Filter => {
                let algorithm = params.get("algorithm").map(|s| s.as_str());
                let mode = params.get("mode").map(|s| s.as_str());
                let comb_type = params.get("comb_type").map(|s| s.as_str());
                let sample_rate_f64 = sample_rate as f64;

                let filter: Box<dyn FilterAlgorithm> = match algorithm {
                    Some("moog") => Box::new(MoogLadder::new(sample_rate_f64)),
                    Some("biquad") => {
                        let bq_mode = match mode {
                            Some("highpass") => BiquadMode::Highpass,
                            Some("peaking") => BiquadMode::Peaking,
                            _ => BiquadMode::Lowpass,
                        };
                        let norm = 1000.0 / sample_rate_f64;
                        match bq_mode {
                            BiquadMode::Peaking => {
                                Box::new(BiquadFilter::new_peaking(norm, 0.707, 0.0))
                            }
                            BiquadMode::Highpass => {
                                Box::new(BiquadFilter::new_highpass(norm, 0.707))
                            }
                            BiquadMode::Lowpass => Box::new(BiquadFilter::new_lowpass(norm, 0.707)),
                        }
                    }
                    Some("comb") => {
                        let ct = match comb_type {
                            Some("feedforward") => CombType::Feedforward,
                            _ => CombType::Feedback,
                        };
                        Box::new(CombFilter::new((sample_rate_f64 / 440.0) as usize, 0.5, ct))
                    }
                    _ => Box::new(MoogLadder::new(sample_rate_f64)),
                };
                PerModuleState::Filter {
                    filter,
                    sample_rate: sample_rate_f64,
                }
            }
            ModuleKind::Echo => PerModuleState::Echo {
                processor: Echo::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            ModuleKind::Reverb => PerModuleState::Reverb {
                processor: Reverb::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            ModuleKind::FrequencySplitter => PerModuleState::FrequencySplitter {
                first: CrossoverPair::new(0.02, sample_rate as f64),
                second: CrossoverPair::new(0.08, sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            ModuleKind::SpectralProcessor => PerModuleState::SpectralProcessor {
                processor: SpectralProcessor::new(2048, crate::spectral::SpectralMode::Gate),
            },
            ModuleKind::Lfo
            | ModuleKind::ControlMixer
            | ModuleKind::AudioDelayOneSample
            | ModuleKind::BlockDelay
            | ModuleKind::ControlDelay
            | ModuleKind::Script => {
                panic!(
                    "module kind {kind:?} does not have a per-module state variant"
                )
            }
        }
    }
}
