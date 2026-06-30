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
        match module.module_type() {
            "oscillator" => PerModuleState::Oscillator {
                phase: 0.0,
                sample_rate,
            },
            "adsr" => PerModuleState::Adsr {
                level: 0.0,
                gate_active: false,
                release_start_frame: 0,
                release_start_level: 0.0,
                sample_rate,
            },
            "gain" => PerModuleState::Vca,
            "audio_output" => PerModuleState::AudioOutput,
            "midi_input" => PerModuleState::MidiInput,
            "note_to_rate" => PerModuleState::NoteToRate { rate: 1.0 },
            "audio_mixer" => PerModuleState::AudioMixer,
            "sampler" => PerModuleState::Sampler {
                sample: sampler_assets.get(module.id().as_str()).cloned(),
                position: 0.0,
                active: false,
            },
            "dynamics-processor" => PerModuleState::DynamicsProcessor {
                processor: DynamicsProcessor::new(sample_rate as f64, 5.0, 50.0),
                sample_rate,
            },
            "saturator" => PerModuleState::Saturator {
                processor: Saturator::new(),
            },
            "convolution" => PerModuleState::Convolution {
                processor: Convolution::new(),
            },
            "filter" => {
                let algorithm = module.params().get("algorithm").map(|s| s.as_str());
                let mode = module.params().get("mode").map(|s| s.as_str());
                let comb_type = module.params().get("comb_type").map(|s| s.as_str());
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
            "echo" => PerModuleState::Echo {
                processor: Echo::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            "reverb" => PerModuleState::Reverb {
                processor: Reverb::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            "frequency_splitter" => PerModuleState::FrequencySplitter {
                first: CrossoverPair::new(0.02, sample_rate as f64),
                second: CrossoverPair::new(0.08, sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            "spectral_processor" => PerModuleState::SpectralProcessor {
                processor: SpectralProcessor::new(2048, crate::spectral::SpectralMode::Gate),
            },
            other => panic!("unknown module type: {other}"),
        }
    }
}
