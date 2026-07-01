use super::module_types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    MidiInput,
    AudioOutput,
    Oscillator,
    Gain,
    AudioMixer,
    ControlMixer,
    Adsr,
    Lfo,
    Filter,
    AudioDelayOneSample,
    BlockDelay,
    ControlDelay,
    Script,
    Sampler,
    NoteToRate,
    DynamicsProcessor,
    Saturator,
    Convolution,
    Echo,
    Reverb,
    FrequencySplitter,
    SpectralProcessor,
}

impl ModuleKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            module_types::MIDI_INPUT => Some(Self::MidiInput),
            module_types::AUDIO_OUTPUT => Some(Self::AudioOutput),
            module_types::OSCILLATOR => Some(Self::Oscillator),
            module_types::GAIN => Some(Self::Gain),
            module_types::AUDIO_MIXER => Some(Self::AudioMixer),
            module_types::CONTROL_MIXER => Some(Self::ControlMixer),
            module_types::ADSR => Some(Self::Adsr),
            module_types::LFO => Some(Self::Lfo),
            module_types::FILTER => Some(Self::Filter),
            module_types::AUDIO_DELAY_ONE_SAMPLE => Some(Self::AudioDelayOneSample),
            module_types::BLOCK_DELAY => Some(Self::BlockDelay),
            module_types::CONTROL_DELAY => Some(Self::ControlDelay),
            module_types::SCRIPT => Some(Self::Script),
            module_types::SAMPLER => Some(Self::Sampler),
            module_types::NOTE_TO_RATE => Some(Self::NoteToRate),
            module_types::DYNAMICS_PROCESSOR => Some(Self::DynamicsProcessor),
            module_types::SATURATOR => Some(Self::Saturator),
            module_types::CONVOLUTION => Some(Self::Convolution),
            module_types::ECHO => Some(Self::Echo),
            module_types::REVERB => Some(Self::Reverb),
            module_types::FREQUENCY_SPLITTER => Some(Self::FrequencySplitter),
            module_types::SPECTRAL_PROCESSOR => Some(Self::SpectralProcessor),
            _ => None,
        }
    }

    pub fn is_render_supported(self) -> bool {
        matches!(
            self,
            Self::MidiInput
                | Self::AudioOutput
                | Self::Oscillator
                | Self::Gain
                | Self::AudioMixer
                | Self::Adsr
                | Self::Filter
                | Self::Sampler
                | Self::NoteToRate
                | Self::DynamicsProcessor
                | Self::Saturator
                | Self::Convolution
                | Self::Echo
                | Self::Reverb
                | Self::FrequencySplitter
                | Self::SpectralProcessor
        )
    }
}
