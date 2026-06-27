use std::collections::HashMap;

use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::{Graph, ModuleNode, builtin_ports};
use crate::patch::RenderSettings;
use crate::script::ScriptEvent;

fn midi_note_to_hz(note: u8) -> f32 {
    440.0 * (2.0f32).powf((note as f32 - 69.0) / 12.0)
}

fn find_audio_output(graph: &Graph) -> Option<usize> {
    graph
        .modules()
        .iter()
        .position(|m| m.module_type() == "audio_output")
}

fn find_midi_input(graph: &Graph) -> Option<usize> {
    graph
        .modules()
        .iter()
        .position(|m| m.module_type() == "midi_input")
}

/// Topological sort using Kahn's algorithm.
fn topological_sort(graph: &Graph) -> Vec<usize> {
    let n = graph.modules().len();
    let name_to_idx: HashMap<&str, usize> = graph
        .modules()
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id().as_str(), i))
        .collect();

    let mut in_degree = vec![0usize; n];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];

    for cable in graph.cables() {
        let src = name_to_idx[cable.source().module_id().as_str()];
        let dst = name_to_idx[cable.destination().module_id().as_str()];
        adjacency[src].push(dst);
        in_degree[dst] += 1;
    }

    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut order = Vec::new();

    while let Some(idx) = queue.pop() {
        order.push(idx);
        for &next in &adjacency[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }

    order
}

struct Routing {
    /// For each module index, maps input port name → (source_module_idx, source_port_name)
    inputs: Vec<HashMap<String, Vec<(usize, String)>>>,
}

fn build_routing(graph: &Graph) -> Routing {
    let n = graph.modules().len();
    let name_to_idx: HashMap<&str, usize> = graph
        .modules()
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id().as_str(), i))
        .collect();

    let mut inputs: Vec<HashMap<String, Vec<(usize, String)>>> =
        (0..n).map(|_| HashMap::new()).collect();

    for cable in graph.cables() {
        let dst = name_to_idx[cable.destination().module_id().as_str()];
        let src = name_to_idx[cable.source().module_id().as_str()];
        let dst_port = cable.destination().port_name().to_string();
        let src_port = cable.source().port_name().to_string();
        inputs[dst]
            .entry(dst_port)
            .or_default()
            .push((src, src_port));
    }

    Routing { inputs }
}

enum PerModuleState {
    Oscillator {
        phase: f32,
        current_note: Option<u8>,
        sample_rate: f32,
    },
    Adsr {
        level: f32,
        gate_active: bool,
        release_start_frame: u64,
        sample_rate: f32,
    },
    Vca,
    AudioOutput,
    MidiInput,
    AudioDelay,
}

impl PerModuleState {
    fn new(module: &ModuleNode, sample_rate: f32) -> Self {
        match module.module_type() {
            "oscillator" => PerModuleState::Oscillator {
                phase: 0.0,
                current_note: None,
                sample_rate,
            },
            "adsr" => PerModuleState::Adsr {
                level: 0.0,
                gate_active: false,
                release_start_frame: 0,
                sample_rate,
            },
            "gain" => PerModuleState::Vca,
            "audio_output" => PerModuleState::AudioOutput,
            "midi_input" => PerModuleState::MidiInput,
            "audio_delay" => PerModuleState::AudioDelay,
            other => panic!("unknown module type: {other}"),
        }
    }
}

struct ModuleOutputs {
    audio: HashMap<String, Vec<f32>>,
    control: HashMap<String, Vec<f32>>,
    events: Vec<ScriptEvent>,
}

fn sum_audio_input(
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
                if let Some(buf) = outputs.audio.get(src_port) {
                    for (i, s) in buf.iter().enumerate().take(frames) {
                        result[i] += s;
                    }
                }
            }
        }
    }
    result
}

fn sum_control_input(
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
                if let Some(buf) = outputs.control.get(src_port) {
                    for (i, s) in buf.iter().enumerate().take(frames) {
                        result[i] += s;
                    }
                }
            }
        }
    }
    result
}

fn gather_event_inputs(
    module_idx: usize,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
) -> Vec<ScriptEvent> {
    let mut events = Vec::new();
    if let Some(sources) = routing.inputs[module_idx].get(builtin_ports::EVENTS) {
        for &(src_idx, ref src_port) in sources {
            if let Some(outputs) = all_outputs.get(&src_idx) {
                if src_port == builtin_ports::EVENTS {
                    events.extend_from_slice(&outputs.events);
                }
            }
        }
    }
    events
}

pub fn render_offline(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
) -> (Vec<f32>, Vec<f32>) {
    let sample_rate = settings.sample_rate_hz as f32;
    let routing = build_routing(graph);
    let topo_order = topological_sort(graph);
    let mut states: Vec<PerModuleState> = graph
        .modules()
        .iter()
        .map(|m| PerModuleState::new(m, sample_rate))
        .collect();

    let midi_idx = find_midi_input(graph);
    let out_idx = find_audio_output(graph);

    let scheduler =
        BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
            .with_input_events(events);

    let mut left_buf: Vec<f32> = Vec::new();
    let mut right_buf: Vec<f32> = Vec::new();

    for block in scheduler {
        let frames = block.frame_count() as usize;

        // Collect external events for this block
        let external_events: Vec<ScriptEvent> = block
            .input_events()
            .iter()
            .map(|e| e.event().clone())
            .collect();

        // Per-block module outputs (cleared each block)
        let mut all_outputs: HashMap<usize, ModuleOutputs> = HashMap::new();

        // Pre-populate midi_input output if present
        if let Some(idx) = midi_idx {
            let outputs = ModuleOutputs {
                audio: HashMap::new(),
                control: HashMap::new(),
                events: external_events,
            };
            all_outputs.insert(idx, outputs);
        }

        for &module_idx in &topo_order {
            let module = &graph.modules()[module_idx];
            let module_type = module.module_type();

            let events_in = match module_type {
                "midi_input" => Vec::new(),
                _ => gather_event_inputs(module_idx, &routing, &all_outputs),
            };

            let outputs = match module_type {
                "midi_input" => ModuleOutputs {
                    audio: HashMap::new(),
                    control: HashMap::new(),
                    events: Vec::new(),
                },
                "oscillator" => {
                    process_oscillator(
                        &mut states[module_idx],
                        &events_in,
                        frames,
                    )
                }
                "adsr" => {
                    process_adsr(
                        &mut states[module_idx],
                        &events_in,
                        block.start_frame(),
                        frames,
                    )
                }
                "gain" => {
                    let audio_in = sum_audio_input(
                        module_idx,
                        builtin_ports::AUDIO_IN,
                        &routing,
                        &all_outputs,
                        frames,
                    );
                    let gain_in = sum_control_input(
                        module_idx,
                        builtin_ports::GAIN,
                        &routing,
                        &all_outputs,
                        frames,
                    );
                    process_vca(audio_in, gain_in)
                }
                "audio_output" => {
                    let left = sum_audio_input(
                        module_idx,
                        builtin_ports::LEFT,
                        &routing,
                        &all_outputs,
                        frames,
                    );
                    let right = sum_audio_input(
                        module_idx,
                        builtin_ports::RIGHT,
                        &routing,
                        &all_outputs,
                        frames,
                    );
                    ModuleOutputs {
                        audio: {
                            let mut m = HashMap::new();
                            m.insert(builtin_ports::LEFT.to_string(), left);
                            m.insert(builtin_ports::RIGHT.to_string(), right);
                            m
                        },
                        control: HashMap::new(),
                        events: Vec::new(),
                    }
                }
                "audio_delay" => {
                    let audio_in = sum_audio_input(
                        module_idx,
                        builtin_ports::AUDIO_IN,
                        &routing,
                        &all_outputs,
                        frames,
                    );
                    process_audio_delay(audio_in)
                }
                other => panic!("unknown module type: {other}"),
            };

            all_outputs.insert(module_idx, outputs);
        }

        // Accumulate audio_output module's contribution
        if let Some(idx) = out_idx {
            if let Some(outputs) = all_outputs.get(&idx) {
                if let Some(left) = outputs.audio.get(builtin_ports::LEFT) {
                    left_buf.extend_from_slice(left);
                } else {
                    left_buf.extend(std::iter::repeat_n(0.0, frames));
                }
                if let Some(right) = outputs.audio.get(builtin_ports::RIGHT) {
                    right_buf.extend_from_slice(right);
                } else {
                    right_buf.extend(std::iter::repeat_n(0.0, frames));
                }
            }
        }
    }

    (left_buf, right_buf)
}

fn process_oscillator(
    state: &mut PerModuleState,
    events_in: &[ScriptEvent],
    frames: usize,
) -> ModuleOutputs {
    let (phase, current_note, sample_rate) = match state {
        PerModuleState::Oscillator {
            phase,
            current_note,
            sample_rate,
        } => (phase, current_note, *sample_rate),
        _ => unreachable!(),
    };

    for event in events_in {
        if let ScriptEvent::NoteOn { note, .. } = event {
            *current_note = Some(*note);
        }
    }

    let freq = current_note.map(|n| midi_note_to_hz(n)).unwrap_or(220.0);
    let phase_inc = freq / sample_rate;

    let mut audio = Vec::with_capacity(frames);
    for _ in 0..frames {
        audio.push(*phase * 2.0 - 1.0);
        *phase += phase_inc;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
    }

    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs
        .audio
        .insert(builtin_ports::AUDIO.to_string(), audio);
    outputs
}

fn process_adsr(
    state: &mut PerModuleState,
    events_in: &[ScriptEvent],
    block_start_frame: u64,
    frames: usize,
) -> ModuleOutputs {
    let (level, gate_active, release_start_frame, sample_rate) = match state {
        PerModuleState::Adsr {
            level,
            gate_active,
            release_start_frame,
            sample_rate,
        } => (level, gate_active, release_start_frame, *sample_rate),
        _ => unreachable!(),
    };

    let attack_frames = (sample_rate * 0.005) as u64;  // 5ms
    let decay_frames = (sample_rate * 0.03) as u64;    // 30ms
    let sustain_level = 0.7;
    let release_frames = (sample_rate * 0.2) as u64;    // 200ms

    // Process block-level events (not sample-accurate yet)
    for event in events_in {
        match event {
            ScriptEvent::NoteOn { .. } => {
                *gate_active = true;
                *release_start_frame = block_start_frame;
            }
            ScriptEvent::NoteOff { .. } => {
                *gate_active = false;
                *release_start_frame = block_start_frame;
            }
        }
    }

    let mut adsr_value = Vec::with_capacity(frames);

    for i in 0..frames {
        let absolute_frame = block_start_frame + i as u64;

        if *gate_active {
            let lifetime = absolute_frame - *release_start_frame;
            if lifetime < attack_frames {
                adsr_value.push((lifetime as f32) / (attack_frames as f32));
            } else if lifetime < attack_frames + decay_frames {
                let decay_progress =
                    (lifetime - attack_frames) as f32 / (decay_frames as f32);
                adsr_value.push(1.0 - (1.0 - sustain_level) * decay_progress);
            } else {
                adsr_value.push(sustain_level);
            }
        } else {
            let release_progress =
                (absolute_frame - *release_start_frame) as f32 / (release_frames as f32);
            if release_progress >= 1.0 {
                adsr_value.push(0.0);
            } else {
                adsr_value.push(*level * (1.0 - release_progress));
            }
        }
    }

    *level = *adsr_value.last().unwrap_or(&0.0);

    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs
        .control
        .insert(builtin_ports::VALUE.to_string(), adsr_value);
    outputs
}

fn process_vca(audio_in: Vec<f32>, gain_in: Vec<f32>) -> ModuleOutputs {
    let frames = audio_in.len().min(gain_in.len());
    let mut audio = Vec::with_capacity(frames);
    for i in 0..frames {
        audio.push(audio_in[i] * gain_in[i]);
    }

    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs
        .audio
        .insert(builtin_ports::AUDIO_OUT.to_string(), audio);
    outputs
}

fn process_audio_delay(audio_in: Vec<f32>) -> ModuleOutputs {
    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs
        .audio
        .insert(builtin_ports::AUDIO_OUT.to_string(), audio_in);
    outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TimedInputEvent;
    use crate::graph::*;
    use crate::patch;
    use crate::script::ScriptEvent;

    #[test]
    fn graph_processor_produces_audio_when_events_are_present() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Test
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
modules:
  - id: osc
    type: oscillator
    outputs:
      - name: audio
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: osc.audio
    to: out.left
  - from: osc.audio
    to: out.right
"#,
        )
        .expect("patch should parse");

        patch::validate_patch_schema(&patch).expect("schema should be valid");

        let graph = Graph::from_patch_declarations(&patch);
        graph.validate().expect("graph should validate");

        let (left, right) = render_offline(
            &graph,
            &patch.render,
            vec![TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            )],
        );

        let has_signal =
            left.iter().any(|&s| s != 0.0) || right.iter().any(|&s| s != 0.0);
        assert!(
            has_signal,
            "graph processor should produce non-silent output when events are present"
        );
        assert!(!left.is_empty(), "left buffer should have samples");
        assert!(!right.is_empty(), "right buffer should have samples");
    }
}
