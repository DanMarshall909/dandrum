use std::collections::BTreeMap;

use crate::builtins::module_kind::ModuleKind;
use crate::builtins::{
    CURVE_LINEAR, CURVE_PARAMETER, DETECTION_MODE_PARAMETER, DETECTION_MODE_RMS,
    EVENT_FILTER_NOTE_PARAMETER, EVENT_FILTER_NOTE_SELECTOR, EVENT_FILTER_SELECTOR_PARAMETER,
    SCRIPT_SOURCE_PARAMETER, STEPS_PARAMETER,
};
use crate::compiled_patch::CompiledNode;
use crate::convolution::Convolution;
use crate::crossover::CrossoverPair;
use crate::curve_mapper::{CurveKind, CurveMapper};
use crate::decay::DecayCurve;
use crate::dynamics_processor::DynamicsProcessor;
use crate::echo::Echo;
use crate::envelope_follower::{DetectionMode, EnvelopeFollower};
use crate::filter::{BiquadFilter, BiquadMode, CombFilter, CombType, FilterAlgorithm, MoogLadder};
#[cfg(test)]
use crate::graph::ModuleNode;
use crate::graph::SignalType;
use crate::reverb::Reverb;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::saturator::Saturator;
use crate::script::{RhaiScriptRuntime, ScriptModuleState, ScriptRuntimeLimits};
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
    Noise {
        state: u32,
    },
    Impulse,
    Multiply,
    NoteToControl {
        gate_active: bool,
        current_note: Option<u8>,
        current_velocity: f32,
        current_frequency: f32,
        current_pitch_ratio: f32,
    },
    EventFilter {
        note: Option<u8>,
    },
    EnvelopeFollower {
        detector: EnvelopeFollower,
        mode: DetectionMode,
    },
    CurveMapper {
        mapper: CurveMapper,
    },
    Decay {
        level: f32,
        triggered: bool,
        elapsed_frames: u64,
        decay_frames: f32,
        curve: DecayCurve,
    },
    Script {
        runtime: RhaiScriptRuntime,
        state: ScriptModuleState,
        control_inputs: Vec<String>,
    },
}

impl PerModuleState {
    #[cfg(test)]
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
        if node.module_kind == ModuleKind::Script {
            return Self::new_script_compiled(node);
        }

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
            ModuleKind::Script => {
                let source = params
                    .get(SCRIPT_SOURCE_PARAMETER)
                    .unwrap_or_else(|| panic!("script module {module_id} source is required"));
                let runtime = RhaiScriptRuntime::compile(source, ScriptRuntimeLimits::default())
                    .unwrap_or_else(|error| {
                        panic!("script module {module_id} failed to prepare: {error}")
                    });
                PerModuleState::Script {
                    runtime,
                    state: ScriptModuleState::default(),
                    control_inputs: Vec::new(),
                }
            }
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
            ModuleKind::Noise => {
                let seed = params
                    .get("seed")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                PerModuleState::Noise { state: seed }
            }
            ModuleKind::Impulse => PerModuleState::Impulse,
            ModuleKind::Multiply => PerModuleState::Multiply,
            ModuleKind::NoteToControl => PerModuleState::NoteToControl {
                gate_active: false,
                current_note: None,
                current_velocity: 0.0,
                current_frequency: 0.0,
                current_pitch_ratio: 0.0,
            },
            ModuleKind::EventFilter => {
                let selector = params
                    .get(EVENT_FILTER_SELECTOR_PARAMETER)
                    .map(String::as_str)
                    .unwrap_or(EVENT_FILTER_NOTE_SELECTOR);
                let note = if selector == EVENT_FILTER_NOTE_SELECTOR {
                    params
                        .get(EVENT_FILTER_NOTE_PARAMETER)
                        .and_then(|value| value.parse::<u8>().ok())
                } else {
                    None
                };
                PerModuleState::EventFilter { note }
            }
            ModuleKind::EnvelopeFollower => {
                let mode = match params.get(DETECTION_MODE_PARAMETER).map(String::as_str) {
                    Some(DETECTION_MODE_RMS) => DetectionMode::Rms,
                    _ => DetectionMode::Peak,
                };
                PerModuleState::EnvelopeFollower {
                    detector: EnvelopeFollower::new(sample_rate as f64, 5.0, 50.0, mode),
                    mode,
                }
            }
            ModuleKind::CurveMapper => {
                let curve = params
                    .get(CURVE_PARAMETER)
                    .map(String::as_str)
                    .and_then(CurveKind::from_str)
                    .unwrap_or_else(|| CurveKind::from_str(CURVE_LINEAR).unwrap());
                let steps = params
                    .get(STEPS_PARAMETER)
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(CurveMapper::DEFAULT_STEPS);
                PerModuleState::CurveMapper {
                    mapper: CurveMapper::new(curve, steps),
                }
            }
            ModuleKind::Decay => {
                let curve_str = params.get("curve").map(String::as_str);
                let curve = curve_str
                    .and_then(DecayCurve::from_str)
                    .unwrap_or(DecayCurve::Exponential);
                let time_ms = params
                    .get("time_ms")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(100.0);
                let decay_frames = (sample_rate * time_ms / 1000.0).max(1.0);
                PerModuleState::Decay {
                    level: 0.0,
                    triggered: false,
                    elapsed_frames: 0,
                    decay_frames,
                    curve,
                }
            }
            ModuleKind::Lfo
            | ModuleKind::ControlMixer
            | ModuleKind::AudioDelayOneSample
            | ModuleKind::BlockDelay
            | ModuleKind::ControlDelay => {
                panic!("module kind {kind:?} does not have a per-module state variant")
            }
        }
    }

    pub(super) fn new_script_compiled(node: &CompiledNode) -> Self {
        let source = node
            .parameters
            .get(SCRIPT_SOURCE_PARAMETER)
            .unwrap_or_else(|| panic!("script module {} source is required", node.id.as_str()));
        let event_outputs: Vec<String> = node
            .output_port_names
            .iter()
            .zip(node.output_port_types.iter())
            .filter(|(_, signal_type)| **signal_type == SignalType::Event)
            .map(|(name, _)| name.clone())
            .collect();
        let control_outputs: Vec<String> = node
            .output_port_names
            .iter()
            .zip(node.output_port_types.iter())
            .filter(|(_, signal_type)| **signal_type == SignalType::Control)
            .map(|(name, _)| name.clone())
            .collect();
        let control_inputs: Vec<String> = node
            .input_port_names
            .iter()
            .zip(node.input_port_types.iter())
            .filter(|(_, signal_type)| **signal_type == SignalType::Control)
            .map(|(name, _)| name.clone())
            .collect();

        let runtime = RhaiScriptRuntime::compile_with_output_ports(
            source,
            ScriptRuntimeLimits::default(),
            event_outputs,
            control_outputs,
        )
        .unwrap_or_else(|error| {
            panic!(
                "script module {} failed to prepare: {error}",
                node.id.as_str()
            )
        });

        PerModuleState::Script {
            runtime,
            state: ScriptModuleState::default(),
            control_inputs,
        }
    }
}
