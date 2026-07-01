use std::collections::HashMap;

use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::CompiledPatch;
use crate::graph::builtin_ports;
use crate::script::ScriptEvent;
use crate::voice_allocator::VoiceAllocator;

use super::dispatch::process_module;
use super::input_provider::{CompiledInputProvider, compiled_gather_event_inputs};
use super::outputs::{BlockEvent, ModuleOutputs};
use super::state::PerModuleState;

pub(super) fn collect_audio_output(
    all_outputs: &HashMap<usize, ModuleOutputs>,
    out_idx: Option<usize>,
    frames: usize,
    left_out: &mut Vec<f32>,
    right_out: &mut Vec<f32>,
) {
    if let Some(idx) = out_idx {
        if let Some(outputs) = all_outputs.get(&idx) {
            if let Some(left) = outputs.audio.get(builtin_ports::LEFT) {
                left_out.extend_from_slice(left);
            } else {
                left_out.extend(std::iter::repeat_n(0.0, frames));
            }
            if let Some(right) = outputs.audio.get(builtin_ports::RIGHT) {
                right_out.extend_from_slice(right);
            } else {
                right_out.extend(std::iter::repeat_n(0.0, frames));
            }
        }
    }
}

pub(super) fn process_block_compiled(
    compiled: &CompiledPatch,
    states: &mut [PerModuleState],
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    block_start_frame: u64,
    frames: usize,
    incoming_events: Vec<BlockEvent>,
    left_out: &mut Vec<f32>,
    right_out: &mut Vec<f32>,
) {
    let mut all_outputs: HashMap<usize, ModuleOutputs> = HashMap::new();

    if let Some(idx) = midi_idx {
        let outputs = ModuleOutputs {
            audio: HashMap::new(),
            control: HashMap::new(),
            events: incoming_events,
        };
        all_outputs.insert(idx, outputs);
    }

    let input_provider = CompiledInputProvider { compiled };

    for &module_idx in compiled.voice_node_indices() {
        let node = &compiled.nodes()[module_idx];
        if node.module_kind == ModuleKind::MidiInput {
            continue;
        }
        let events_in = compiled_gather_event_inputs(module_idx, compiled, &all_outputs);
        let outputs = process_module(
            module_idx,
            node.module_kind,
            &events_in,
            states,
            &input_provider,
            &all_outputs,
            frames,
            block_start_frame,
        );
        all_outputs.insert(module_idx, outputs);
    }

    for &module_idx in compiled.global_node_indices() {
        let node = &compiled.nodes()[module_idx];
        if node.module_kind == ModuleKind::MidiInput {
            continue;
        }
        let events_in = compiled_gather_event_inputs(module_idx, compiled, &all_outputs);
        let outputs = process_module(
            module_idx,
            node.module_kind,
            &events_in,
            states,
            &input_provider,
            &all_outputs,
            frames,
            block_start_frame,
        );
        all_outputs.insert(module_idx, outputs);
    }

    collect_audio_output(&all_outputs, out_idx, frames, left_out, right_out);
}

pub(super) fn process_block_compiled_polyphonic(
    compiled: &CompiledPatch,
    states: &mut [Vec<PerModuleState>],
    allocator: &mut VoiceAllocator,
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    block_start_frame: u64,
    frames: usize,
    incoming_events: Vec<BlockEvent>,
    left_out: &mut Vec<f32>,
    right_out: &mut Vec<f32>,
) {
    let mut voice_events: Vec<Vec<BlockEvent>> = vec![Vec::new(); allocator.max_voices()];

    for event in &incoming_events {
        if let ScriptEvent::NoteOn { note, velocity } = &event.event {
            if let Some(slot) = allocator.note_on(*note, *velocity) {
                voice_events[slot].push(event.clone());
            }
        }
    }

    let slot_notes: Vec<Option<u8>> = (0..allocator.max_voices())
        .map(|i| allocator.slot(i).filter(|s| s.active).map(|s| s.note))
        .collect();

    for event in &incoming_events {
        if let ScriptEvent::NoteOff { note } = &event.event {
            for (slot_idx, sn) in slot_notes.iter().enumerate() {
                if *sn == Some(*note) {
                    voice_events[slot_idx].push(event.clone());
                }
            }
        }
    }

    let active_voices: Vec<usize> = (0..allocator.max_voices())
        .filter(|&i| allocator.slot(i).is_some_and(|s| s.active))
        .collect();

    if active_voices.is_empty() {
        left_out.extend(std::iter::repeat_n(0.0, frames));
        right_out.extend(std::iter::repeat_n(0.0, frames));
        return;
    }

    let mut accum: HashMap<usize, ModuleOutputs> = HashMap::new();
    let input_provider = CompiledInputProvider { compiled };

    for &voice_idx in &active_voices {
        let mut all_outputs: HashMap<usize, ModuleOutputs> = HashMap::new();
        if let Some(idx) = midi_idx {
            all_outputs.insert(
                idx,
                ModuleOutputs {
                    audio: HashMap::new(),
                    control: HashMap::new(),
                    events: voice_events[voice_idx].clone(),
                },
            );
        }

        let voice_states = &mut states[voice_idx];

        for &module_idx in compiled.voice_node_indices() {
            let node = &compiled.nodes()[module_idx];
            if node.module_kind == ModuleKind::MidiInput {
                continue;
            }

            let events_in = compiled_gather_event_inputs(module_idx, compiled, &all_outputs);
            let outputs = process_module(
                module_idx,
                node.module_kind,
                &events_in,
                voice_states,
                &input_provider,
                &all_outputs,
                frames,
                block_start_frame,
            );

            all_outputs.insert(module_idx, outputs);
        }

        for &idx in compiled.voice_node_indices() {
            if let Some(outputs) = all_outputs.remove(&idx) {
                let entry = accum.entry(idx).or_insert_with(ModuleOutputs::empty);
                for (port, buf) in outputs.audio {
                    let acc = entry.audio.entry(port).or_insert_with(|| vec![0.0; frames]);
                    for (i, s) in buf.iter().enumerate().take(frames) {
                        acc[i] += s;
                    }
                }
                for (port, buf) in outputs.control {
                    let acc = entry
                        .control
                        .entry(port)
                        .or_insert_with(|| vec![0.0; frames]);
                    for (i, s) in buf.iter().enumerate().take(frames) {
                        acc[i] += s;
                    }
                }
                entry.events.extend(outputs.events);
            }
        }
    }

    let mut all_outputs = accum;

    for &module_idx in compiled.global_node_indices() {
        let node = &compiled.nodes()[module_idx];
        if node.module_kind == ModuleKind::MidiInput {
            continue;
        }

        let events_in = compiled_gather_event_inputs(module_idx, compiled, &all_outputs);
        let outputs = process_module(
            module_idx,
            node.module_kind,
            &events_in,
            &mut states[0],
            &input_provider,
            &all_outputs,
            frames,
            block_start_frame,
        );

        all_outputs.insert(module_idx, outputs);
    }

    collect_audio_output(&all_outputs, out_idx, frames, left_out, right_out);

    for i in 0..allocator.max_voices() {
        if allocator.slot(i).is_none_or(|s| !s.active) {
            continue;
        }
        let has_adsr = states[i]
            .iter()
            .any(|s| matches!(s, PerModuleState::Adsr { .. }));
        let has_sampler = states[i]
            .iter()
            .any(|s| matches!(s, PerModuleState::Sampler { .. }));
        if !has_adsr && !has_sampler {
            continue;
        }
        let adsr_done = !has_adsr
            || states[i].iter().any(|s| match s {
                PerModuleState::Adsr {
                    level, gate_active, ..
                } => !gate_active && *level < 0.001,
                _ => false,
            });
        let sampler_done = !has_sampler
            || states[i].iter().any(|s| match s {
                PerModuleState::Sampler { active, .. } => !active,
                _ => false,
            });
        if adsr_done && sampler_done {
            allocator.set_slot_inactive(i);
        }
    }
}
