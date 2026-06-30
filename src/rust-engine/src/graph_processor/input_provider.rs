use std::collections::HashMap;

use crate::compiled_patch::CompiledPatch;
use crate::graph::{ModuleNode, SignalType};

use super::routing::Routing;
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

impl ModuleInputProvider for &Routing {
    fn sum_audio_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32> {
        sum_audio_input(module_idx, port_name, self, all_outputs, frames)
    }

    fn sum_control_input(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) -> Vec<f32> {
        sum_control_input(module_idx, port_name, self, all_outputs, frames)
    }

    fn control_input_or_default(
        &self,
        module_idx: usize,
        port_name: &str,
        all_outputs: &HashMap<usize, ModuleOutputs>,
        frames: usize,
        default: f32,
    ) -> Vec<f32> {
        control_input_or_default(module_idx, port_name, self, all_outputs, frames, default)
    }
}

pub(super) fn sum_audio_input(
    module_idx: usize,
    port_name: &str,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> Vec<f32> {
    let mut result = vec![0.0f32; frames];
    if let Some(sources) = routing.inputs[module_idx].get(port_name) {
        for &(src_idx, ref src_port) in sources {
            if let Some(outputs) = all_outputs.get(&src_idx) {
                if let Some(buffer) = outputs.audio.get(src_port) {
                    for (frame_idx, sample) in buffer.iter().enumerate().take(frames) {
                        result[frame_idx] += sample;
                    }
                }
            }
        }
    }
    result
}

pub(super) fn sum_control_input(
    module_idx: usize,
    port_name: &str,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> Vec<f32> {
    let mut result = vec![0.0f32; frames];
    if let Some(sources) = routing.inputs[module_idx].get(port_name) {
        for &(src_idx, ref src_port) in sources {
            if let Some(outputs) = all_outputs.get(&src_idx) {
                if let Some(buffer) = outputs.control.get(src_port) {
                    for (frame_idx, sample) in buffer.iter().enumerate().take(frames) {
                        result[frame_idx] += sample;
                    }
                }
            }
        }
    }
    result
}

pub(super) fn control_input_or_default(
    module_idx: usize,
    port_name: &str,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
    default: f32,
) -> Vec<f32> {
    if routing.inputs[module_idx].contains_key(port_name) {
        sum_control_input(module_idx, port_name, routing, all_outputs, frames)
    } else {
        vec![default; frames]
    }
}

pub(super) fn gather_event_inputs(
    module_idx: usize,
    module: &ModuleNode,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
) -> Vec<BlockEvent> {
    let mut events = Vec::new();
    for input_port in module.inputs() {
        if input_port.signal_type() != SignalType::Event {
            continue;
        }
        if let Some(sources) = routing.inputs[module_idx].get(input_port.name()) {
            for &(src_idx, _) in sources {
                if let Some(outputs) = all_outputs.get(&src_idx) {
                    events.extend_from_slice(&outputs.events);
                }
            }
        }
    }
    events
}

fn compiled_input_port_index(
    compiled: &CompiledPatch,
    module_idx: usize,
    port_name: &str,
) -> Option<usize> {
    compiled
        .nodes()
        .get(module_idx)?
        .input_port_names
        .iter()
        .position(|name| name == port_name)
}

fn compiled_source_port_name(
    compiled: &CompiledPatch,
    src_idx: usize,
    port_idx: usize,
) -> Option<&str> {
    Some(
        compiled
            .nodes()
            .get(src_idx)?
            .output_port_names
            .get(port_idx)?
            .as_str(),
    )
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

    for &src_ref in &compiled.nodes()[module_idx].input_port_map[port_idx] {
        let Some(src_port_name) =
            compiled_source_port_name(compiled, src_ref.module_index, src_ref.port_index)
        else {
            continue;
        };

        if let Some(outputs) = all_outputs.get(&src_ref.module_index) {
            if let Some(buffer) = outputs.audio.get(src_port_name) {
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

    for &src_ref in &compiled.nodes()[module_idx].input_port_map[port_idx] {
        let Some(src_port_name) =
            compiled_source_port_name(compiled, src_ref.module_index, src_ref.port_index)
        else {
            continue;
        };

        if let Some(outputs) = all_outputs.get(&src_ref.module_index) {
            if let Some(buffer) = outputs.control.get(src_port_name) {
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

    vec![default; frames]
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
        for &src_ref in &node.input_port_map[input_idx] {
            if let Some(outputs) = all_outputs.get(&src_ref.module_index) {
                events.extend_from_slice(&outputs.events);
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
