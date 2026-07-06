use crate::builtins::module_kind::ModuleKind;
use crate::graph::builtin_ports::*;
use ModuleKind::*;
use std::collections::{BTreeMap, HashMap};

use super::ModuleInputProvider;
use super::outputs::{BlockEvent, ModuleOutputs};
use super::processing::{
    EchoControls, ReverbControls, process_adsr, process_convolution, process_curve_mapper,
    process_decay, process_dynamics_processor, process_echo, process_envelope_follower,
    process_event_filter, process_filter, process_frequency_splitter, process_impulse,
    process_multiply, process_noise, process_note_to_control, process_note_to_rate,
    process_oscillator, process_reverb, process_sampler, process_saturator, process_script,
    process_spectral_processor, process_vca,
};
use super::render_plan::default_control_value;
use super::state::PerModuleState;

fn default_control(module_kind: ModuleKind, port_name: &str) -> f32 {
    default_control_value(module_kind, port_name)
        .unwrap_or_else(|| panic!("missing default control value for {module_kind:?}.{port_name}"))
}

pub(super) fn process_module(
    module_idx: usize,
    module_kind: ModuleKind,
    events_in: &[BlockEvent],
    states: &mut [PerModuleState],
    input_provider: &impl ModuleInputProvider,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
    block_start_frame: u64,
) -> ModuleOutputs {
    let audio = |port| input_provider.sum_audio_input(module_idx, port, all_outputs, frames);
    let mod_ctrl = |port| input_provider.sum_control_input(module_idx, port, all_outputs, frames);
    let ctrl = |port| {
        input_provider.control_input_or_default(
            module_idx,
            port,
            all_outputs,
            frames,
            default_control(module_kind, port),
        )
    };

    match module_kind {
        Oscillator => process_oscillator(&mut states[module_idx], &ctrl(PITCH), frames),
        Adsr => process_adsr(
            &mut states[module_idx],
            events_in,
            &ctrl(ATTACK),
            &ctrl(DECAY),
            &ctrl(SUSTAIN),
            &ctrl(RELEASE),
            block_start_frame,
            frames,
        ),
        Gain => process_vca(audio(AUDIO_IN), ctrl(GAIN)),
        Sampler => process_sampler(
            &mut states[module_idx],
            events_in,
            &ctrl(RATE),
            &mod_ctrl(START),
            &mod_ctrl(LOOP_ENABLED),
            &mod_ctrl(LOOP_START),
            &mod_ctrl(LOOP_END),
            frames,
        ),
        NoteToRate => process_note_to_rate(&mut states[module_idx], events_in, frames),
        EventFilter => process_event_filter(&mut states[module_idx], events_in),
        EnvelopeFollower => process_envelope_follower(
            &mut states[module_idx],
            &audio(AUDIO_IN),
            &ctrl(ATTACK),
            &ctrl(RELEASE),
            &ctrl(AMOUNT),
            &ctrl(OFFSET),
            &ctrl(INVERT),
            frames,
        ),
        CurveMapper => process_curve_mapper(
            &mut states[module_idx],
            &mod_ctrl(VALUE),
            &ctrl(AMOUNT),
            &ctrl(BIAS),
            &ctrl(SCALE),
            &ctrl(OFFSET),
            frames,
        ),
        AudioMixer => {
            let mix = audio(INPUTS);
            let mut outputs = ModuleOutputs::empty();
            outputs.audio.insert(MIX.to_string(), mix);
            outputs
        }
        AudioOutput => {
            let left = audio(LEFT);
            let right = audio(RIGHT);
            let mut outputs = ModuleOutputs::empty();
            outputs.audio.insert(LEFT.to_string(), left);
            outputs.audio.insert(RIGHT.to_string(), right);
            outputs
        }
        DynamicsProcessor => {
            let audio_in = audio(AUDIO_IN);
            process_dynamics_processor(
                &mut states[module_idx],
                &audio_in,
                &mod_ctrl(SIDECHAIN_IN),
                &ctrl(THRESHOLD),
                &ctrl(BELOW_RATIO),
                &ctrl(ABOVE_RATIO),
                &ctrl(ATTACK),
                &ctrl(RELEASE),
                &ctrl(KNEE),
                &ctrl(MAKEUP_GAIN),
                &ctrl(ATTACK_GAIN),
                &ctrl(SUSTAIN_GAIN),
                frames,
            )
        }
        Filter => {
            let audio_in = audio(AUDIO_IN);
            process_filter(
                &mut states[module_idx],
                &audio_in,
                &ctrl(CUTOFF),
                &ctrl(RESONANCE),
                &ctrl(GAIN),
                frames,
            )
        }
        Saturator => {
            let audio_in = audio(AUDIO_IN);
            process_saturator(
                &mut states[module_idx],
                &audio_in,
                &ctrl(DRIVE),
                &ctrl(BIAS),
                &ctrl(CURVE_SELECT),
                frames,
            )
        }
        Convolution => {
            let audio_in = audio(AUDIO_IN);
            process_convolution(&mut states[module_idx], &audio_in, &ctrl(MIX), frames)
        }
        Echo => {
            let audio_in_l = audio(AUDIO_IN_L);
            let audio_in_r = audio(AUDIO_IN_R);
            let feedback = ctrl(FEEDBACK);
            let damping = ctrl(DAMPING_CUTOFF);
            let wet = ctrl(WET);
            let dry = ctrl(DRY);
            process_echo(
                &mut states[module_idx],
                &audio_in_l,
                &audio_in_r,
                EchoControls {
                    feedback: &feedback,
                    damping: &damping,
                    wet: &wet,
                    dry: &dry,
                    time_l: &ctrl(TIME_LEFT_MS),
                    time_r: &ctrl(TIME_RIGHT_MS),
                    ping_pong: &ctrl(PING_PONG),
                },
                frames,
            )
        }
        Reverb => {
            let audio_in_l = audio(AUDIO_IN_L);
            let audio_in_r = audio(AUDIO_IN_R);
            let decay_time = ctrl(DECAY_TIME);
            let room_size = ctrl(ROOM_SIZE);
            let damping = ctrl(DAMPING);
            let diffusion = ctrl(DIFFUSION);
            let wet = ctrl(WET);
            let dry = ctrl(DRY);
            let stereo_width = ctrl(STEREO_WIDTH);
            process_reverb(
                &mut states[module_idx],
                &audio_in_l,
                &audio_in_r,
                ReverbControls {
                    decay_time: &decay_time,
                    room_size: &room_size,
                    damping: &damping,
                    diffusion: &diffusion,
                    wet: &wet,
                    dry: &dry,
                    pre_delay: &ctrl(PRE_DELAY),
                    stereo_width: &stereo_width,
                },
                frames,
            )
        }
        FrequencySplitter => {
            let audio_in = audio(AUDIO_IN);
            process_frequency_splitter(
                &mut states[module_idx],
                &audio_in,
                &ctrl(CROSSOVER_HZ),
                frames,
            )
        }
        SpectralProcessor => {
            let audio_in = audio(AUDIO_IN);
            process_spectral_processor(
                &mut states[module_idx],
                &audio_in,
                &ctrl(THRESHOLD),
                &ctrl(MIX),
                frames,
            )
        }
        Noise => process_noise(&mut states[module_idx], frames),
        Impulse => process_impulse(&mut states[module_idx], events_in, frames),
        Decay => process_decay(&mut states[module_idx], events_in, frames),
        Multiply => {
            let a = audio(AUDIO_IN);
            let b = audio(GAIN);
            process_multiply(a, b)
        }
        NoteToControl => process_note_to_control(&mut states[module_idx], events_in, frames),
        Script => {
            let control_input_names = match &states[module_idx] {
                PerModuleState::Script { control_inputs, .. } => control_inputs.clone(),
                _ => unreachable!(),
            };
            let control_inputs: BTreeMap<String, f32> = control_input_names
                .into_iter()
                .map(|port_name| {
                    let values = input_provider.sum_control_input(
                        module_idx,
                        &port_name,
                        all_outputs,
                        frames,
                    );
                    let value = values.first().copied().unwrap_or(0.0);
                    (port_name, value)
                })
                .collect();
            process_script(&mut states[module_idx], events_in, control_inputs, frames)
        }
        _ => panic!(
            "process_module called for unsupported module kind; dispatch is only for render-time module types"
        ),
    }
}
