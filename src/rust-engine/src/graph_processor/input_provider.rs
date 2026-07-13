use std::collections::HashMap;

use crate::compiled_patch::CompiledPatch;
use crate::graph::SignalType;

use super::{BlockEvent, ModuleOutputs};

pub(super) trait ModuleInputProvider {
    fn sum_audio_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32>;

    fn sum_control_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32>;

    fn control_input_or_default(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
        default: f32,
    ) -> Vec<f32>;
}

fn compiled_input_port_index(
    compiled: &CompiledPatch,
    module_idx: usize,
    port_name: &str,
) -> Option<usize> {
    compiled
        .nodes()
        .get(module_idx)?
        .input_port_indices
        .get(port_name)
        .copied()
}

pub(super) fn compiled_sum_audio_input(
    module_idx: usize,
    port_name: &str,
    compiled: &CompiledPatch,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> Vec<f32> {
    let mut result = vec![0.0f32; frames];
    let Some(port_idx) = compiled_input_port_index(compiled, module_idx, port_name) else {
        return result;
    };

    for source in &compiled.nodes()[module_idx].input_routes[port_idx] {
        if let Some(outputs) = all_outputs.get(&source.module_index) {
            if let Some(buffer) = outputs.audio.get(&source.output_port_name) {
                for (frame_idx, sample) in buffer.iter().enumerate().take(frames) {
                    result[frame_idx] += sample;
                }
            }
        }
    }

    result
}

pub(super) fn compiled_sum_control_input(
    module_idx: usize,
    port_name: &str,
    compiled: &CompiledPatch,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> Vec<f32> {
    let mut result = vec![0.0f32; frames];
    let Some(port_idx) = compiled_input_port_index(compiled, module_idx, port_name) else {
        return result;
    };

    if compiled.nodes()[module_idx].input_routes[port_idx].is_empty() {
        return compiled_control_parameter_input(module_idx, port_name, compiled, frames)
            .unwrap_or(result);
    }

    for source in &compiled.nodes()[module_idx].input_routes[port_idx] {
        if let Some(outputs) = all_outputs.get(&source.module_index) {
            if let Some(buffer) = outputs.control.get(&source.output_port_name) {
                for (frame_idx, sample) in buffer.iter().enumerate().take(frames) {
                    result[frame_idx] += sample;
                }
            }
        }
    }

    result
}

fn compiled_control_input_or_default(
    module_idx: usize,
    port_name: &str,
    compiled: &CompiledPatch,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
    default: f32,
) -> Vec<f32> {
    let port_idx = compiled_input_port_index(compiled, module_idx, port_name);
    if let Some(port_idx) = port_idx {
        if !compiled.nodes()[module_idx].input_port_map[port_idx].is_empty() {
            return compiled_sum_control_input(
                module_idx,
                port_name,
                compiled,
                all_outputs,
                frames,
            );
        }
    }

    if let Some(parameter_input) =
        compiled_control_parameter_input(module_idx, port_name, compiled, frames)
    {
        return parameter_input;
    }

    vec![default; frames]
}

fn compiled_control_parameter_input(
    module_idx: usize,
    port_name: &str,
    compiled: &CompiledPatch,
    frames: usize,
) -> Option<Vec<f32>> {
    if let Some(slot_index) = compiled.nodes()[module_idx]
        .parameter_slot_indices
        .get(port_name)
    {
        return compiled
            .parameter_slot_value(*slot_index)
            .map(|value| vec![value; frames]);
    }

    None
}

pub(super) fn compiled_gather_event_inputs(
    module_idx: usize,
    compiled: &CompiledPatch,
    all_outputs: &HashMap<usize, ModuleOutputs>,
) -> Vec<BlockEvent> {
    let mut events = Vec::new();
    let node = &compiled.nodes()[module_idx];

    for input_idx in 0..node.input_port_names.len() {
        if node.input_port_types[input_idx] != SignalType::Event {
            continue;
        }
        for source in &node.input_routes[input_idx] {
            if let Some(outputs) = all_outputs.get(&source.module_index) {
                if let Some(port_events) = outputs.event_ports.get(&source.output_port_name) {
                    events.extend_from_slice(port_events);
                } else {
                    events.extend_from_slice(&outputs.events);
                }
            }
        }
    }

    events
}

pub(super) struct CompiledInputProvider<'a> {
    pub(super) compiled: &'a CompiledPatch,
}

impl ModuleInputProvider for CompiledInputProvider<'_> {
    fn sum_audio_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32> {
        compiled_sum_audio_input(module_idx, port_name, self.compiled, all_outputs, frames)
    }

    fn sum_control_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32> {
        compiled_sum_control_input(module_idx, port_name, self.compiled, all_outputs, frames)
    }

    fn control_input_or_default(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
        default: f32,
    ) -> Vec<f32> {
        compiled_control_input_or_default(
            module_idx,
            port_name,
            self.compiled,
            all_outputs,
            frames,
            default,
        )
    }
}
