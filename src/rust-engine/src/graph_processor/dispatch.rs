use std::collections::HashMap;

use crate::builtins::module_kind::ModuleKind;
use crate::graph::builtin_ports;

use super::outputs::{BlockEvent, ModuleOutputs};
use super::processing::{
    process_adsr, process_convolution, process_dynamics_processor, process_echo, process_filter,
    process_frequency_splitter, process_note_to_rate, process_oscillator, process_reverb,
    process_sampler, process_saturator, process_spectral_processor, process_vca,
};
use super::state::PerModuleState;
use super::ModuleInputProvider;

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
    match module_kind {
        ModuleKind::Oscillator => {
            let pitch_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::PITCH,
                all_outputs,
                frames,
                1.0,
            );
            process_oscillator(&mut states[module_idx], &pitch_in, frames)
        }
        ModuleKind::Adsr => {
            let attack_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::ATTACK,
                all_outputs,
                frames,
            );
            let decay_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::DECAY,
                all_outputs,
                frames,
            );
            let sustain_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::SUSTAIN,
                all_outputs,
                frames,
            );
            let release_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::RELEASE,
                all_outputs,
                frames,
            );
            process_adsr(
                &mut states[module_idx],
                events_in,
                &attack_in,
                &decay_in,
                &sustain_in,
                &release_in,
                block_start_frame,
                frames,
            )
        }
        ModuleKind::Gain => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let gain_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::GAIN,
                all_outputs,
                frames,
            );
            process_vca(audio_in, gain_in)
        }
        ModuleKind::Sampler => {
            let rate_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::RATE,
                all_outputs,
                frames,
                1.0,
            );
            let start_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::START,
                all_outputs,
                frames,
            );
            let loop_enabled_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::LOOP_ENABLED,
                all_outputs,
                frames,
            );
            let loop_start_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::LOOP_START,
                all_outputs,
                frames,
            );
            let loop_end_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::LOOP_END,
                all_outputs,
                frames,
            );
            process_sampler(
                &mut states[module_idx],
                events_in,
                &rate_in,
                &start_in,
                &loop_enabled_in,
                &loop_start_in,
                &loop_end_in,
                frames,
            )
        }
        ModuleKind::NoteToRate => process_note_to_rate(&mut states[module_idx], events_in, frames),
        ModuleKind::AudioMixer => {
            let mix = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::INPUTS,
                all_outputs,
                frames,
            );
            let mut outputs = ModuleOutputs::empty();
            outputs.audio.insert(builtin_ports::MIX.to_string(), mix);
            outputs
        }
        ModuleKind::AudioOutput => {
            let left = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::LEFT,
                all_outputs,
                frames,
            );
            let right = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::RIGHT,
                all_outputs,
                frames,
            );
            let mut outputs = ModuleOutputs::empty();
            outputs.audio.insert(builtin_ports::LEFT.to_string(), left);
            outputs
                .audio
                .insert(builtin_ports::RIGHT.to_string(), right);
            outputs
        }
        ModuleKind::DynamicsProcessor => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let sidechain_in = input_provider.sum_control_input(
                module_idx,
                builtin_ports::SIDECHAIN_IN,
                all_outputs,
                frames,
            );
            let threshold_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::THRESHOLD,
                all_outputs,
                frames,
                0.3,
            );
            let below_ratio_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::BELOW_RATIO,
                all_outputs,
                frames,
                0.05,
            );
            let above_ratio_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::ABOVE_RATIO,
                all_outputs,
                frames,
                0.077,
            );
            let attack_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::ATTACK,
                all_outputs,
                frames,
                0.05,
            );
            let release_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::RELEASE,
                all_outputs,
                frames,
                0.1,
            );
            let knee_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::KNEE,
                all_outputs,
                frames,
                0.0,
            );
            let makeup_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::MAKEUP_GAIN,
                all_outputs,
                frames,
                0.0,
            );
            let attack_gain_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::ATTACK_GAIN,
                all_outputs,
                frames,
                0.5,
            );
            let sustain_gain_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::SUSTAIN_GAIN,
                all_outputs,
                frames,
                0.5,
            );
            process_dynamics_processor(
                &mut states[module_idx],
                &audio_in,
                &sidechain_in,
                &threshold_in,
                &below_ratio_in,
                &above_ratio_in,
                &attack_in,
                &release_in,
                &knee_in,
                &makeup_in,
                &attack_gain_in,
                &sustain_gain_in,
                frames,
            )
        }
        ModuleKind::Filter => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let cutoff_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::CUTOFF,
                all_outputs,
                frames,
                0.5,
            );
            let resonance_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::RESONANCE,
                all_outputs,
                frames,
                0.0,
            );
            let gain_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::GAIN,
                all_outputs,
                frames,
                0.5,
            );
            process_filter(
                &mut states[module_idx],
                &audio_in,
                &cutoff_in,
                &resonance_in,
                &gain_in,
                frames,
            )
        }
        ModuleKind::Saturator => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let drive_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::DRIVE,
                all_outputs,
                frames,
                0.0,
            );
            let bias_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::BIAS,
                all_outputs,
                frames,
                0.0,
            );
            let curve_select_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::CURVE_SELECT,
                all_outputs,
                frames,
                0.0,
            );
            process_saturator(
                &mut states[module_idx],
                &audio_in,
                &drive_in,
                &bias_in,
                &curve_select_in,
                frames,
            )
        }
        ModuleKind::Convolution => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let mix_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::MIX,
                all_outputs,
                frames,
                1.0,
            );
            process_convolution(&mut states[module_idx], &audio_in, &mix_in, frames)
        }
        ModuleKind::Echo => {
            let audio_in_l = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN_L,
                all_outputs,
                frames,
            );
            let audio_in_r = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN_R,
                all_outputs,
                frames,
            );
            process_echo(
                &mut states[module_idx],
                &audio_in_l,
                &audio_in_r,
                module_idx,
                input_provider,
                all_outputs,
                frames,
            )
        }
        ModuleKind::Reverb => {
            let audio_in_l = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN_L,
                all_outputs,
                frames,
            );
            let audio_in_r = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN_R,
                all_outputs,
                frames,
            );
            process_reverb(
                &mut states[module_idx],
                &audio_in_l,
                &audio_in_r,
                module_idx,
                input_provider,
                all_outputs,
                frames,
            )
        }
        ModuleKind::FrequencySplitter => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let crossover_hz_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::CROSSOVER_HZ,
                all_outputs,
                frames,
                0.2,
            );
            process_frequency_splitter(&mut states[module_idx], &audio_in, &crossover_hz_in, frames)
        }
        ModuleKind::SpectralProcessor => {
            let audio_in = input_provider.sum_audio_input(
                module_idx,
                builtin_ports::AUDIO_IN,
                all_outputs,
                frames,
            );
            let threshold_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::THRESHOLD,
                all_outputs,
                frames,
                0.5,
            );
            let mix_in = input_provider.control_input_or_default(
                module_idx,
                builtin_ports::MIX,
                all_outputs,
                frames,
                0.5,
            );
            process_spectral_processor(
                &mut states[module_idx],
                &audio_in,
                &threshold_in,
                &mix_in,
                frames,
            )
        }
        _ => panic!("process_module called for unsupported module kind; dispatch is only for render-time module types"),
    }
}
