use std::collections::HashMap;

use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::{Graph, ModuleNode, SignalType, builtin_ports};
use crate::patch::RenderSettings;
use crate::script::ScriptEvent;

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
    module: &ModuleNode,
    routing: &Routing,
    all_outputs: &HashMap<usize, ModuleOutputs>,
) -> Vec<ScriptEvent> {
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

    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);

    let mut left_buf: Vec<f32> = Vec::new();
    let mut right_buf: Vec<f32> = Vec::new();

    for block in scheduler {
        let frames = block.frame_count() as usize;

        let external_events: Vec<ScriptEvent> = block
            .input_events()
            .iter()
            .map(|e| e.event().clone())
            .collect();

        process_block(
            graph,
            &routing,
            &topo_order,
            &mut states,
            midi_idx,
            out_idx,
            block.start_frame(),
            frames,
            external_events,
            &mut left_buf,
            &mut right_buf,
        );
    }

    (left_buf, right_buf)
}

fn process_block(
    graph: &Graph,
    routing: &Routing,
    topo_order: &[usize],
    states: &mut [PerModuleState],
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    block_start_frame: u64,
    frames: usize,
    incoming_events: Vec<ScriptEvent>,
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

    for &module_idx in topo_order {
        let module = &graph.modules()[module_idx];
        let module_type = module.module_type();

        if module_type == "midi_input" {
            continue;
        }

        let events_in = gather_event_inputs(module_idx, module, routing, &all_outputs);

        let outputs = match module_type {
            "oscillator" => {
                let pitch_in = sum_control_input(
                    module_idx,
                    builtin_ports::PITCH,
                    routing,
                    &all_outputs,
                    frames,
                );
                process_oscillator(&mut states[module_idx], &pitch_in, frames)
            }
            "adsr" => {
                let attack_in = sum_control_input(
                    module_idx,
                    builtin_ports::ATTACK,
                    routing,
                    &all_outputs,
                    frames,
                );
                let decay_in = sum_control_input(
                    module_idx,
                    builtin_ports::DECAY,
                    routing,
                    &all_outputs,
                    frames,
                );
                let sustain_in = sum_control_input(
                    module_idx,
                    builtin_ports::SUSTAIN,
                    routing,
                    &all_outputs,
                    frames,
                );
                let release_in = sum_control_input(
                    module_idx,
                    builtin_ports::RELEASE,
                    routing,
                    &all_outputs,
                    frames,
                );
                process_adsr(
                    &mut states[module_idx],
                    &events_in,
                    &attack_in,
                    &decay_in,
                    &sustain_in,
                    &release_in,
                    block_start_frame,
                    frames,
                )
            }
            "gain" => {
                let audio_in = sum_audio_input(
                    module_idx,
                    builtin_ports::AUDIO_IN,
                    routing,
                    &all_outputs,
                    frames,
                );
                let gain_in = sum_control_input(
                    module_idx,
                    builtin_ports::GAIN,
                    routing,
                    &all_outputs,
                    frames,
                );
                process_vca(audio_in, gain_in)
            }
            "audio_output" => {
                let left = sum_audio_input(
                    module_idx,
                    builtin_ports::LEFT,
                    routing,
                    &all_outputs,
                    frames,
                );
                let right = sum_audio_input(
                    module_idx,
                    builtin_ports::RIGHT,
                    routing,
                    &all_outputs,
                    frames,
                );
                let mut m = HashMap::new();
                m.insert(builtin_ports::LEFT.to_string(), left);
                m.insert(builtin_ports::RIGHT.to_string(), right);
                ModuleOutputs {
                    audio: m,
                    control: HashMap::new(),
                    events: Vec::new(),
                }
            }
            "audio_delay" => {
                let audio_in = sum_audio_input(
                    module_idx,
                    builtin_ports::AUDIO_IN,
                    routing,
                    &all_outputs,
                    frames,
                );
                process_audio_delay(audio_in)
            }
            other => panic!("unknown module type: {other}"),
        };

        all_outputs.insert(module_idx, outputs);
    }

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

pub struct RealtimeGraphProcessor {
    graph: Graph,
    routing: Routing,
    topo_order: Vec<usize>,
    states: Vec<PerModuleState>,
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    current_frame: u64,
    pending_events: Vec<ScriptEvent>,
}

impl RealtimeGraphProcessor {
    pub fn new(graph: Graph, sample_rate: f32) -> Self {
        let routing = build_routing(&graph);
        let topo_order = topological_sort(&graph);
        let midi_idx = find_midi_input(&graph);
        let out_idx = find_audio_output(&graph);
        let states: Vec<PerModuleState> = graph
            .modules()
            .iter()
            .map(|m| PerModuleState::new(m, sample_rate))
            .collect();

        Self {
            graph,
            routing,
            topo_order,
            states,
            midi_idx,
            out_idx,
            current_frame: 0,
            pending_events: Vec::new(),
        }
    }

    pub fn note_on(&mut self, _note: u8, _velocity: u8) {
        self.pending_events.push(ScriptEvent::NoteOn {
            note: _note,
            velocity: _velocity,
        });
    }

    pub fn note_off(&mut self, _note: u8) {
        self.pending_events
            .push(ScriptEvent::NoteOff { note: _note });
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let frames = left.len().min(right.len());
        let block_start = self.current_frame;
        self.current_frame += frames as u64;

        let events = std::mem::take(&mut self.pending_events);

        let mut left_buf = Vec::new();
        let mut right_buf = Vec::new();

        process_block(
            &self.graph,
            &self.routing,
            &self.topo_order,
            &mut self.states,
            self.midi_idx,
            self.out_idx,
            block_start,
            frames,
            events,
            &mut left_buf,
            &mut right_buf,
        );

        let actual = left_buf.len().min(right_buf.len()).min(frames);
        for i in 0..actual {
            left[i] = left_buf[i];
            right[i] = right_buf[i];
        }
        for i in actual..frames {
            left[i] = 0.0;
            right[i] = 0.0;
        }

        frames
    }

    pub fn is_finished(&self) -> bool {
        if !self.pending_events.is_empty() {
            return false;
        }
        for state in &self.states {
            if let PerModuleState::Adsr {
                level, gate_active, ..
            } = state
            {
                if *gate_active || *level > 0.001 {
                    return false;
                }
            }
        }
        true
    }
}

fn process_oscillator(
    state: &mut PerModuleState,
    pitch_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (phase, sample_rate) = match state {
        PerModuleState::Oscillator { phase, sample_rate } => (phase, *sample_rate),
        _ => unreachable!(),
    };

    let mut audio = Vec::with_capacity(frames);
    for i in 0..frames {
        let semitone_offset = pitch_in.get(i).copied().unwrap_or(0.0);
        let base_hz = 220.0;
        let freq = base_hz * (2.0f32).powf(semitone_offset / 12.0);
        let phase_inc = freq / sample_rate;
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn has_signal(buf: &[f32]) -> bool {
    buf.iter().any(|&v| v != 0.0)
}

fn process_adsr(
    state: &mut PerModuleState,
    events_in: &[ScriptEvent],
    attack_in: &[f32],
    decay_in: &[f32],
    sustain_in: &[f32],
    release_in: &[f32],
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

    let has_attack = has_signal(attack_in);
    let has_decay = has_signal(decay_in);
    let has_sustain = has_signal(sustain_in);
    let has_release = has_signal(release_in);

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

        let attack_ms = if has_attack {
            lerp(2.0, 100.0, attack_in[i].clamp(0.0, 1.0))
        } else {
            5.0
        };
        let decay_ms = if has_decay {
            lerp(10.0, 1000.0, decay_in[i].clamp(0.0, 1.0))
        } else {
            30.0
        };
        let sustain = if has_sustain {
            sustain_in[i].clamp(0.0, 1.0)
        } else {
            0.7
        };
        let release_ms = if has_release {
            lerp(10.0, 3000.0, release_in[i].clamp(0.0, 1.0))
        } else {
            200.0
        };

        let attack_frames = (sample_rate * attack_ms / 1000.0) as u64;
        let decay_frames = (sample_rate * decay_ms / 1000.0) as u64;
        let release_frames = (sample_rate * release_ms / 1000.0) as u64;

        if *gate_active {
            let lifetime = absolute_frame - *release_start_frame;
            if lifetime < attack_frames {
                adsr_value.push((lifetime as f32) / (attack_frames as f32));
            } else if lifetime < attack_frames + decay_frames {
                let decay_progress = (lifetime - attack_frames) as f32 / (decay_frames as f32);
                adsr_value.push(1.0 - (1.0 - sustain) * decay_progress);
            } else {
                adsr_value.push(sustain);
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
    fn graph_processor_produces_audio_from_midi_triggered_303_chain() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: 303-style
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
    outputs:
      - name: events
        signal_type: event
  - id: osc
    type: oscillator
    inputs:
      - name: pitch
        signal_type: control
    outputs:
      - name: audio
        signal_type: audio
  - id: env
    type: adsr
    inputs:
      - name: gate
        signal_type: event
    outputs:
      - name: value
        signal_type: control
  - id: vca
    type: gain
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: control
    outputs:
      - name: audio_out
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: out.left
  - from: vca.audio_out
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
            vec![
                TimedInputEvent::new(
                    0,
                    ScriptEvent::NoteOn {
                        note: 45,
                        velocity: 100,
                    },
                ),
                TimedInputEvent::new(12000, ScriptEvent::NoteOff { note: 45 }),
            ],
        );

        let has_signal = left.iter().any(|&s| s != 0.0) || right.iter().any(|&s| s != 0.0);
        assert!(has_signal, "303-style chain should produce audio");
        assert_eq!(left.len(), 48000);
        assert_eq!(right.len(), 48000);
    }

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

        let has_signal = left.iter().any(|&s| s != 0.0) || right.iter().any(|&s| s != 0.0);
        assert!(
            has_signal,
            "graph processor should produce non-silent output when events are present"
        );
        assert!(!left.is_empty(), "left buffer should have samples");
        assert!(!right.is_empty(), "right buffer should have samples");
    }
}
