use crate::builtins::module_kind::ModuleKind;
#[cfg(test)]
use crate::compiled_patch::CompiledNodeData;
use crate::compiled_patch::{
    CompiledConstruction, CompiledFilterAlgorithm, CompiledNode, CompiledResourceHandles,
    CompiledSampleZone, CompiledScriptLanguage, SampleInterpolation, SampleResourceHandle,
    SampleSelectionMode,
};
use crate::convolution::Convolution;
use crate::crossover::LinkwitzRiley4;
use crate::curve_mapper::CurveMapper;
use crate::decay::DecayCurve;
use crate::dynamics_processor::DynamicsProcessor;
use crate::echo::Echo;
use crate::envelope_follower::{DetectionMode, EnvelopeFollower};
use crate::filter::{BiquadFilter, BiquadMode, CombFilter, FilterAlgorithm, MoogLadder};
#[cfg(test)]
use crate::graph::ModuleNode;
use crate::graph::SignalType;
use crate::kernel::document::SampleRegion;
use crate::oscillator::Waveform;
use crate::reverb::Reverb;
use crate::sample::PreparedSamplerAssets;
#[cfg(test)]
use crate::saturator::Saturator;
use crate::script::{RhaiScriptRuntime, ScriptModuleState, ScriptRuntimeLimits};
use crate::spectral::SpectralProcessor;

pub(super) enum PerModuleState {
    Oscillator {
        phase: f32,
        sample_rate: f32,
        waveform: Waveform,
    },
    Lfo {
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
    Slew {
        current: f32,
        sample_rate: f32,
    },
    /// Stateless control→audio promotion (see `unify-graph-kernel` §2.5).
    ControlToAudio,
    /// Structural boundary; child processing starts in task 4.3.
    Poly,
    /// Compiler-injected source whose prepared output buffers/queue are filled
    /// directly by its owning poly runtime region.
    VoiceIntrinsics,
    CompensationDelay {
        samples: Box<[Box<[f32]>]>,
        positions: Box<[usize]>,
    },
    FeedbackDelay {
        samples: Box<[Box<[f32]>]>,
        position: usize,
    },
    #[cfg(test)]
    AudioOutput,
    MidiInput,
    NoteToRate {
        rate: f32,
    },
    AudioMixer,
    // Intentionally monophonic until the engine has generic per-voice bus support.
    Sampler {
        sample: Option<SampleResourceHandle>,
        position: f32,
        active: bool,
    },
    SamplePlayer {
        sample: Option<SampleResourceHandle>,
        region: SampleRegion,
        mode: String,
        interpolation: SampleInterpolation,
        position: f64,
        active: bool,
    },
    SampleMapPlayer {
        zones: Box<[CompiledSampleZone]>,
        selection_mode: SampleSelectionMode,
        reject_new_while_active: bool,
        round_robin_counters: Box<[usize]>,
        initial_seed: u64,
        rng_state: u64,
        selected_zone: Option<usize>,
        selected_pitch_ratio: f32,
        position: f64,
        active: bool,
    },
    DynamicsProcessor {
        processors: Box<[DynamicsProcessor]>,
    },
    Saturator {
        #[cfg(test)]
        processor: Saturator,
    },
    Convolution {
        processors: Box<[Convolution]>,
    },
    Filter {
        filters: Box<[Box<dyn FilterAlgorithm>]>,
        sample_rate: f64,
    },
    Echo {
        processor: Echo,
        #[cfg(test)]
        sample_rate: f64,
    },
    Reverb {
        processor: Reverb,
        #[cfg(test)]
        sample_rate: f64,
    },
    FrequencySplitter {
        filters: Box<[(LinkwitzRiley4, LinkwitzRiley4)]>,
        sample_rate: f64,
    },
    SpectralProcessor {
        processor: SpectralProcessor,
    },
    Noise {
        states: Box<[u32]>,
        initial_seed: u32,
    },
    Impulse,
    Multiply,
    NoteToControl {
        gate_active: bool,
        current_note: Option<u8>,
        current_velocity: f32,
        current_frequency: f32,
        current_pitch_ratio: f32,
        current_slide: bool,
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
        sample_rate: f32,
        curve: DecayCurve,
    },
    Script {
        runtime: RhaiScriptRuntime,
        state: ScriptModuleState,
        #[cfg(test)]
        control_inputs: Vec<String>,
    },
}

impl PerModuleState {
    #[cfg(test)]
    pub(super) fn reset_voice(&mut self) {
        match self {
            Self::Adsr {
                level,
                gate_active,
                release_start_frame,
                release_start_level,
                ..
            } => {
                *level = 0.0;
                *gate_active = false;
                *release_start_frame = 0;
                *release_start_level = 0.0;
            }
            Self::Slew { current, .. } => *current = 0.0,
            Self::Lfo { phase, .. } => *phase = 0.0,
            Self::CompensationDelay { samples, positions } => {
                for channel in samples.iter_mut() {
                    channel.fill(0.0);
                }
                positions.fill(0);
            }
            Self::FeedbackDelay { samples, position } => {
                for channel in samples.iter_mut() {
                    channel.fill(0.0);
                }
                *position = 0;
            }
            Self::NoteToRate { rate } => *rate = 1.0,
            Self::Sampler {
                position, active, ..
            } => {
                *position = 0.0;
                *active = false;
            }
            Self::SamplePlayer {
                position, active, ..
            } => {
                *position = 0.0;
                *active = false;
            }
            Self::SampleMapPlayer {
                round_robin_counters,
                initial_seed,
                rng_state,
                selected_zone,
                selected_pitch_ratio,
                position,
                active,
                ..
            } => {
                round_robin_counters.fill(0);
                *rng_state = *initial_seed;
                *selected_zone = None;
                *selected_pitch_ratio = 1.0;
                *position = 0.0;
                *active = false;
            }
            Self::DynamicsProcessor { processors, .. } => {
                for processor in processors.iter_mut() {
                    processor.reset();
                }
            }
            Self::Convolution { processors } => {
                for processor in processors.iter_mut() {
                    processor.reset();
                }
            }
            Self::Filter { filters, .. } => {
                for filter in filters.iter_mut() {
                    filter.reset();
                }
            }
            Self::Echo { processor, .. } => processor.reset(),
            Self::Reverb { processor, .. } => processor.reset(),
            Self::FrequencySplitter { filters, .. } => {
                for (low, high) in filters.iter_mut() {
                    low.reset();
                    high.reset();
                }
            }
            Self::SpectralProcessor { processor } => processor.reset(),
            Self::Noise {
                states,
                initial_seed,
            } => {
                for (channel, state) in states.iter_mut().enumerate() {
                    *state = initial_seed.wrapping_add(channel as u32);
                }
            }
            Self::NoteToControl {
                gate_active,
                current_note,
                current_velocity,
                current_frequency,
                current_pitch_ratio,
                current_slide,
            } => {
                *gate_active = false;
                *current_note = None;
                *current_velocity = 0.0;
                *current_frequency = 0.0;
                *current_pitch_ratio = 0.0;
                *current_slide = false;
            }
            Self::EnvelopeFollower { detector, .. } => detector.reset(),
            Self::Decay {
                level,
                triggered,
                elapsed_frames,
                ..
            } => {
                *level = 0.0;
                *triggered = false;
                *elapsed_frames = 0;
            }
            Self::Script { state, .. } => *state = ScriptModuleState::default(),
            Self::Oscillator { .. }
            | Self::Vca
            | Self::ControlToAudio
            | Self::Poly
            | Self::VoiceIntrinsics
            | Self::MidiInput
            | Self::AudioMixer
            | Self::Saturator { .. }
            | Self::Impulse
            | Self::Multiply
            | Self::EventFilter { .. }
            | Self::CurveMapper { .. } => {}
            #[cfg(test)]
            Self::AudioOutput => {}
        }
    }

    #[cfg(test)]
    pub(super) fn reset_all(&mut self) {
        self.reset_voice();
        if let Self::Oscillator { phase, .. } = self {
            *phase = 0.0;
        }
    }

    #[cfg(test)]
    pub(super) fn new(
        module: &ModuleNode,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        let kind = ModuleKind::from_str(module.module_type())
            .unwrap_or_else(|| panic!("unknown module type: {}", module.module_type()));
        let data = CompiledNodeData::from_legacy(module).unwrap_or_else(|error| {
            panic!("module {} failed to prepare: {error}", module.id().as_str())
        });
        Self::from_kind(
            kind,
            module.id().as_str(),
            &data.construction,
            &data.resources,
            sample_rate,
            sampler_assets,
            1,
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
            &node.construction,
            &node.resources,
            sample_rate,
            sampler_assets,
            if node.module_kind == ModuleKind::FeedbackDelay {
                node.output_port_spans
                    .iter()
                    .map(|span| span.channel_count)
                    .sum()
            } else {
                node.output_port_spans
                    .iter()
                    .map(|span| span.channel_count)
                    .max()
                    .unwrap_or(1)
            },
        )
    }

    fn from_kind(
        kind: ModuleKind,
        module_id: &str,
        construction: &CompiledConstruction,
        resources: &CompiledResourceHandles,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        channels: usize,
    ) -> Self {
        match kind {
            ModuleKind::Script => {
                let CompiledConstruction::Script { language, source } = construction else {
                    panic!("script module {module_id} has mismatched construction data")
                };
                let runtime = match language {
                    CompiledScriptLanguage::Rhai => {
                        RhaiScriptRuntime::compile(source, ScriptRuntimeLimits::default())
                    }
                }
                .unwrap_or_else(|error| {
                    panic!("script module {module_id} failed to prepare: {error}")
                });
                PerModuleState::Script {
                    runtime,
                    state: ScriptModuleState::default(),
                    #[cfg(test)]
                    control_inputs: Vec::new(),
                }
            }
            ModuleKind::Oscillator => {
                let CompiledConstruction::Oscillator { waveform } = construction else {
                    panic!("oscillator module {module_id} has mismatched construction data")
                };
                PerModuleState::Oscillator {
                    phase: 0.0,
                    sample_rate,
                    waveform: *waveform,
                }
            }
            ModuleKind::Adsr => PerModuleState::Adsr {
                level: 0.0,
                gate_active: false,
                release_start_frame: 0,
                release_start_level: 0.0,
                sample_rate,
            },
            ModuleKind::Lfo => PerModuleState::Lfo {
                phase: 0.0,
                sample_rate,
            },
            ModuleKind::Gain => PerModuleState::Vca,
            ModuleKind::Slew => PerModuleState::Slew {
                current: 0.0,
                sample_rate,
            },
            ModuleKind::ControlToAudio => PerModuleState::ControlToAudio,
            ModuleKind::Poly => PerModuleState::Poly,
            ModuleKind::VoiceIntrinsics => PerModuleState::VoiceIntrinsics,
            ModuleKind::CompensationDelay => {
                let CompiledConstruction::CompensationDelay { samples } = construction else {
                    panic!("compensation delay module {module_id} has mismatched construction data")
                };
                PerModuleState::CompensationDelay {
                    samples: (0..channels)
                        .map(|_| vec![0.0; *samples].into_boxed_slice())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    positions: vec![0; channels].into_boxed_slice(),
                }
            }
            ModuleKind::FeedbackDelay => {
                let CompiledConstruction::FeedbackDelay { samples } = construction else {
                    panic!("feedback delay module {module_id} has mismatched construction data")
                };
                PerModuleState::FeedbackDelay {
                    samples: (0..channels)
                        .map(|_| vec![0.0; *samples].into_boxed_slice())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    position: 0,
                }
            }
            #[cfg(test)]
            ModuleKind::AudioOutput => PerModuleState::AudioOutput,
            ModuleKind::MidiInput => PerModuleState::MidiInput,
            ModuleKind::NoteToRate => PerModuleState::NoteToRate { rate: 1.0 },
            ModuleKind::AudioMixer | ModuleKind::ControlMixer => PerModuleState::AudioMixer,
            ModuleKind::Sampler => PerModuleState::Sampler {
                sample: resources.sample.clone().or_else(|| {
                    sampler_assets
                        .get(module_id)
                        .cloned()
                        .map(SampleResourceHandle::new)
                }),
                position: 0.0,
                active: false,
            },
            ModuleKind::SamplePlayer => {
                let CompiledConstruction::SamplePlayer {
                    region,
                    mode,
                    interpolation,
                } = construction
                else {
                    panic!("sample_player module {module_id} has mismatched construction data")
                };
                PerModuleState::SamplePlayer {
                    sample: resources.sample.clone(),
                    region: region.clone(),
                    mode: mode.clone(),
                    interpolation: *interpolation,
                    position: 0.0,
                    active: false,
                }
            }
            ModuleKind::SampleMapPlayer => {
                let CompiledConstruction::SampleMapPlayer {
                    zones,
                    selection_mode,
                    group_count,
                    selection_seed,
                    reject_new_while_active,
                } = construction
                else {
                    panic!("sample_map_player module {module_id} has mismatched construction data")
                };
                PerModuleState::SampleMapPlayer {
                    zones: zones.clone(),
                    selection_mode: *selection_mode,
                    reject_new_while_active: *reject_new_while_active,
                    round_robin_counters: vec![0; *group_count].into_boxed_slice(),
                    initial_seed: *selection_seed,
                    rng_state: *selection_seed,
                    selected_zone: None,
                    selected_pitch_ratio: 1.0,
                    position: 0.0,
                    active: false,
                }
            }
            ModuleKind::DynamicsProcessor => {
                let CompiledConstruction::Dynamics {
                    mode,
                    detection,
                    topology,
                } = construction
                else {
                    panic!("dynamics module {module_id} has mismatched construction data")
                };
                let processors = (0..channels)
                    .map(|_| {
                        let mut processor = DynamicsProcessor::new(sample_rate as f64, 5.0, 50.0);
                        processor.set_mode(*mode);
                        processor.set_detection(*detection);
                        processor.set_topology(*topology);
                        processor
                    })
                    .collect();
                PerModuleState::DynamicsProcessor { processors }
            }
            ModuleKind::Saturator => PerModuleState::Saturator {
                #[cfg(test)]
                processor: Saturator::new(),
            },
            ModuleKind::Convolution => {
                let ir = resources
                    .impulse_response
                    .as_ref()
                    .map(|handle| handle.sample().frames())
                    .or_else(|| sampler_assets.get(module_id).map(|sample| sample.frames()));
                let processors = (0..channels)
                    .map(|_| {
                        let mut processor = Convolution::new();
                        if let Some(ir) = ir {
                            processor.load_ir(ir.to_vec());
                        }
                        processor
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                PerModuleState::Convolution { processors }
            }
            ModuleKind::Filter => {
                let CompiledConstruction::Filter { algorithm } = construction else {
                    panic!("filter module {module_id} has mismatched construction data")
                };
                let sample_rate_f64 = sample_rate as f64;

                let make_filter = || -> Box<dyn FilterAlgorithm> {
                    match algorithm {
                        CompiledFilterAlgorithm::Moog => Box::new(MoogLadder::new(sample_rate_f64)),
                        CompiledFilterAlgorithm::Biquad(bq_mode) => {
                            let norm = 1000.0 / sample_rate_f64;
                            match bq_mode {
                                BiquadMode::Peaking => {
                                    Box::new(BiquadFilter::new_peaking(norm, 0.707, 0.0))
                                }
                                BiquadMode::Highpass => {
                                    Box::new(BiquadFilter::new_highpass(norm, 0.707))
                                }
                                BiquadMode::Lowpass => {
                                    Box::new(BiquadFilter::new_lowpass(norm, 0.707))
                                }
                            }
                        }
                        CompiledFilterAlgorithm::Comb(comb_type) => Box::new(CombFilter::new(
                            (sample_rate_f64 / 440.0) as usize,
                            0.5,
                            *comb_type,
                        )),
                    }
                };
                PerModuleState::Filter {
                    filters: (0..channels)
                        .map(|_| make_filter())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    sample_rate: sample_rate_f64,
                }
            }
            ModuleKind::Echo => {
                let CompiledConstruction::Echo { interpolation } = construction else {
                    panic!("echo module {module_id} has mismatched construction data")
                };
                let mut processor = Echo::new(sample_rate as f64);
                processor.set_interpolation(*interpolation);
                PerModuleState::Echo {
                    processor,
                    #[cfg(test)]
                    sample_rate: sample_rate as f64,
                }
            }
            ModuleKind::Reverb => {
                let CompiledConstruction::Reverb { interpolation } = construction else {
                    panic!("reverb module {module_id} has mismatched construction data")
                };
                let mut processor = Reverb::new(sample_rate as f64);
                processor.set_interpolation(*interpolation);
                PerModuleState::Reverb {
                    processor,
                    #[cfg(test)]
                    sample_rate: sample_rate as f64,
                }
            }
            ModuleKind::FrequencySplitter => PerModuleState::FrequencySplitter {
                filters: (0..channels)
                    .map(|_| (LinkwitzRiley4::new(0.02), LinkwitzRiley4::new(0.08)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                sample_rate: sample_rate as f64,
            },
            ModuleKind::SpectralProcessor => {
                let CompiledConstruction::SpectralProcessor { fft_size, mode } = construction
                else {
                    panic!("spectral module {module_id} has mismatched construction data")
                };
                PerModuleState::SpectralProcessor {
                    processor: SpectralProcessor::new(*fft_size, *mode),
                }
            }
            ModuleKind::Noise => {
                let CompiledConstruction::Noise { seed } = construction else {
                    panic!("noise module {module_id} has mismatched construction data")
                };
                PerModuleState::Noise {
                    states: (0..channels)
                        .map(|channel| seed.wrapping_add(channel as u32))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    initial_seed: *seed,
                }
            }
            ModuleKind::Impulse => PerModuleState::Impulse,
            ModuleKind::Multiply => PerModuleState::Multiply,
            ModuleKind::NoteToControl => PerModuleState::NoteToControl {
                gate_active: false,
                current_note: None,
                current_velocity: 0.0,
                current_frequency: 0.0,
                current_pitch_ratio: 0.0,
                current_slide: false,
            },
            ModuleKind::EventFilter => {
                let CompiledConstruction::EventFilter { note } = construction else {
                    panic!("event filter module {module_id} has mismatched construction data")
                };
                PerModuleState::EventFilter { note: *note }
            }
            ModuleKind::EnvelopeFollower => {
                let CompiledConstruction::EnvelopeFollower { mode } = construction else {
                    panic!("envelope follower module {module_id} has mismatched construction data")
                };
                PerModuleState::EnvelopeFollower {
                    detector: EnvelopeFollower::new(sample_rate as f64, 5.0, 50.0, *mode),
                    mode: *mode,
                }
            }
            ModuleKind::CurveMapper => {
                let CompiledConstruction::CurveMapper { curve, steps } = construction else {
                    panic!("curve mapper module {module_id} has mismatched construction data")
                };
                PerModuleState::CurveMapper {
                    mapper: CurveMapper::new(*curve, *steps),
                }
            }
            ModuleKind::Decay => {
                let CompiledConstruction::Decay { curve } = construction else {
                    panic!("decay module {module_id} has mismatched construction data")
                };
                PerModuleState::Decay {
                    level: 0.0,
                    triggered: false,
                    elapsed_frames: 0,
                    sample_rate,
                    curve: *curve,
                }
            }
            ModuleKind::AudioDelayOneSample | ModuleKind::BlockDelay | ModuleKind::ControlDelay => {
                panic!("module kind {kind:?} does not have a per-module state variant")
            }
        }
    }

    pub(super) fn new_script_compiled(node: &CompiledNode) -> Self {
        let CompiledConstruction::Script { language, source } = &node.construction else {
            panic!(
                "script module {} has mismatched construction data",
                node.id.as_str()
            )
        };
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
        #[cfg(test)]
        let control_inputs: Vec<String> = node
            .input_port_names
            .iter()
            .zip(node.input_port_types.iter())
            .filter(|(_, signal_type)| **signal_type == SignalType::Control)
            .map(|(name, _)| name.clone())
            .collect();

        let runtime = match language {
            CompiledScriptLanguage::Rhai => RhaiScriptRuntime::compile_with_output_ports(
                source,
                ScriptRuntimeLimits::default(),
                event_outputs,
                control_outputs,
            ),
        }
        .unwrap_or_else(|error| {
            panic!(
                "script module {} failed to prepare: {error}",
                node.id.as_str()
            )
        });

        PerModuleState::Script {
            runtime,
            state: ScriptModuleState::default(),
            #[cfg(test)]
            control_inputs,
        }
    }
}
