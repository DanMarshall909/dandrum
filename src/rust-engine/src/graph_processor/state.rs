use crate::convolution::Convolution;
use crate::dynamics_processor::DynamicsProcessor;
use crate::echo::Echo;
use crate::filter::MoogLadder;
use crate::graph::ModuleNode;
use crate::reverb::Reverb;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::saturator::Saturator;

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
        filter: MoogLadder,
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
            "filter" => PerModuleState::Filter {
                filter: MoogLadder::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            "echo" => PerModuleState::Echo {
                processor: Echo::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            "reverb" => PerModuleState::Reverb {
                processor: Reverb::new(sample_rate as f64),
                sample_rate: sample_rate as f64,
            },
            other => panic!("unknown module type: {other}"),
        }
    }
}
