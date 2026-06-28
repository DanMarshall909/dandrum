use std::collections::HashMap;

use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::{Graph, ModuleNode, SignalType, builtin_ports};
use crate::patch::RenderSettings;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
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
    NoteToRate {
        rate: f32,
    },
    // Intentionally monophonic until the engine has generic per-voice bus support.
    Sampler {
        sample: Option<LoadedSample>,
        position: f32,
        active: bool,
    },
}

impl PerModuleState {
    fn new(module: &ModuleNode, sample_rate: f32, sampler_assets: &PreparedSamplerAssets) -> Self {
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
            "note_to_rate" => PerModuleState::NoteToRate { rate: 1.0 },
            "sampler" => PerModuleState::Sampler {
                sample: sampler_assets.get(module.id().as_str()).cloned(),
                position: 0.0,
                active: false,
            },
            other => panic!("unknown module type: {other}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BlockEvent {
    frame_offset: u32,
    event: ScriptEvent,
}

struct ModuleOutputs {
    audio: HashMap<String, Vec<f32>>,
    control: HashMap<String, Vec<f32>>,
    events: Vec<BlockEvent>,
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

fn control_input_or_default(
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

fn gather_event_inputs(
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

pub fn render_offline(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
) -> (Vec<f32>, Vec<f32>) {
    render_offline_with_sampler_assets(graph, settings, events, &PreparedSamplerAssets::empty())
}

pub fn render_offline_with_sampler_assets(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
) -> (Vec<f32>, Vec<f32>) {
    let sample_rate = settings.sample_rate_hz as f32;
    let routing = build_routing(graph);
    let topo_order = topological_sort(graph);
    let mut states: Vec<PerModuleState> = graph
        .modules()
        .iter()
        .map(|m| PerModuleState::new(m, sample_rate, sampler_assets))
        .collect();

    let midi_idx = find_midi_input(graph);
    let out_idx = find_audio_output(graph);

    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);

    let mut left_buf: Vec<f32> = Vec::new();
    let mut right_buf: Vec<f32> = Vec::new();

    for block in scheduler {
        let frames = block.frame_count() as usize;

        let external_events: Vec<BlockEvent> = block
            .input_events()
            .iter()
            .map(|e| BlockEvent {
                frame_offset: e.frame_offset(),
                event: e.event().clone(),
            })
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
            "sampler" => {
                let rate_in = control_input_or_default(
                    module_idx,
                    builtin_ports::RATE,
                    routing,
                    &all_outputs,
                    frames,
                    1.0,
                );
                let start_in = sum_control_input(
                    module_idx,
                    builtin_ports::START,
                    routing,
                    &all_outputs,
                    frames,
                );
                let loop_enabled_in = sum_control_input(
                    module_idx,
                    builtin_ports::LOOP_ENABLED,
                    routing,
                    &all_outputs,
                    frames,
                );
                let loop_start_in = sum_control_input(
                    module_idx,
                    builtin_ports::LOOP_START,
                    routing,
                    &all_outputs,
                    frames,
                );
                let loop_end_in = sum_control_input(
                    module_idx,
                    builtin_ports::LOOP_END,
                    routing,
                    &all_outputs,
                    frames,
                );
                process_sampler(
                    &mut states[module_idx],
                    &events_in,
                    &rate_in,
                    &start_in,
                    &loop_enabled_in,
                    &loop_start_in,
                    &loop_end_in,
                    frames,
                )
            }
            "note_to_rate" => process_note_to_rate(&mut states[module_idx], &events_in, frames),
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
        Self::new_with_sampler_assets(graph, sample_rate, &PreparedSamplerAssets::empty())
    }

    pub fn new_with_sampler_assets(
        graph: Graph,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        let routing = build_routing(&graph);
        let topo_order = topological_sort(&graph);
        let midi_idx = find_midi_input(&graph);
        let out_idx = find_audio_output(&graph);
        let states: Vec<PerModuleState> = graph
            .modules()
            .iter()
            .map(|m| PerModuleState::new(m, sample_rate, sampler_assets))
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

        let events: Vec<BlockEvent> = std::mem::take(&mut self.pending_events)
            .into_iter()
            .map(|event| BlockEvent {
                frame_offset: 0,
                event,
            })
            .collect();

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
            } else if let PerModuleState::Sampler { active, .. } = state
                && *active
            {
                return false;
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
    events_in: &[BlockEvent],
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
        match &event.event {
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

fn process_sampler(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    rate_in: &[f32],
    start_in: &[f32],
    loop_enabled_in: &[f32],
    loop_start_in: &[f32],
    loop_end_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (sample, position, active) = match state {
        PerModuleState::Sampler {
            sample,
            position,
            active,
        } => (sample.clone(), position, active),
        _ => unreachable!(),
    };

    let mut audio = vec![0.0; frames];
    let Some(sample) = sample else {
        return audio_output(builtin_ports::AUDIO, audio);
    };
    let sample_frames = sample.frames();
    if sample_frames.is_empty() {
        return audio_output(builtin_ports::AUDIO, audio);
    }

    let mut events = events_in.to_vec();
    events.sort_by_key(|event| event.frame_offset);
    let mut next_event = 0usize;

    for frame in 0..frames {
        while next_event < events.len() && events[next_event].frame_offset as usize == frame {
            if matches!(events[next_event].event, ScriptEvent::NoteOn { .. }) {
                *position = normalized_position(
                    start_in.get(frame).copied().unwrap_or(0.0),
                    sample_frames.len(),
                );
                *active = true;
            }
            next_event += 1;
        }

        if !*active {
            continue;
        }

        let idx = *position as usize;
        if idx >= sample_frames.len() {
            *active = false;
            continue;
        }

        audio[frame] = sample_frames[idx];

        let rate = rate_in.get(frame).copied().unwrap_or(1.0).max(0.0);
        *position += rate;

        if loop_enabled_in.get(frame).copied().unwrap_or(0.0) > 0.5 {
            let loop_start = normalized_position(
                loop_start_in.get(frame).copied().unwrap_or(0.0),
                sample_frames.len(),
            );
            let mut loop_end = normalized_end_position(
                loop_end_in.get(frame).copied().unwrap_or(1.0),
                sample_frames.len(),
            );
            if loop_end <= loop_start {
                loop_end = sample_frames.len() as f32;
            }
            while *position >= loop_end {
                *position = loop_start + (*position - loop_end);
            }
        } else if *position >= sample_frames.len() as f32 {
            *active = false;
        }
    }

    audio_output(builtin_ports::AUDIO, audio)
}

fn process_note_to_rate(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    frames: usize,
) -> ModuleOutputs {
    let rate = match state {
        PerModuleState::NoteToRate { rate } => rate,
        _ => unreachable!(),
    };
    let mut events = events_in.to_vec();
    events.sort_by_key(|event| event.frame_offset);
    let mut next_event = 0usize;
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        while next_event < events.len() && events[next_event].frame_offset as usize == frame {
            if let ScriptEvent::NoteOn { note, .. } = events[next_event].event {
                *rate = 2.0f32.powf((note as f32 - 60.0) / 12.0);
            }
            next_event += 1;
        }
        output.push(*rate);
    }

    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs
        .control
        .insert(builtin_ports::RATE.to_string(), output);
    outputs
}

fn normalized_position(value: f32, sample_len: usize) -> f32 {
    (value.clamp(0.0, 1.0) * sample_len as f32).min(sample_len.saturating_sub(1) as f32)
}

fn normalized_end_position(value: f32, sample_len: usize) -> f32 {
    (value.clamp(0.0, 1.0) * sample_len as f32).clamp(0.0, sample_len as f32)
}

fn audio_output(port_name: &str, audio: Vec<f32>) -> ModuleOutputs {
    let mut outputs = ModuleOutputs {
        audio: HashMap::new(),
        control: HashMap::new(),
        events: Vec::new(),
    };
    outputs.audio.insert(port_name.to_string(), audio);
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
    use crate::sample::{LoadedSample, PreparedSamplerAssets};
    use crate::script::ScriptEvent;
    use std::collections::BTreeMap;

    fn sampler_assets(frames: Vec<f32>) -> PreparedSamplerAssets {
        PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
            "sampler".to_string(),
            LoadedSample::new(48_000, frames),
        )]))
    }

    fn sampler_graph(extra_modules: Vec<ModuleNode>, extra_cables: Vec<Cable>) -> Graph {
        let mut modules = vec![
            ModuleNode::new(ModuleId::new("midi"), "midi_input")
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("sampler"), "sampler")
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ];
        modules.extend(extra_modules);

        let mut cables = vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ];
        cables.extend(extra_cables);
        Graph::new(modules, cables)
    }

    fn sampler_settings(duration_frames: u64) -> RenderSettings {
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 4,
            duration_frames,
        }
    }

    fn note_on(frame: u64, velocity: u8) -> TimedInputEvent {
        TimedInputEvent::new(frame, ScriptEvent::NoteOn { note: 60, velocity })
    }

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

    #[test]
    fn sampler_trigger_event_starts_sample_playback_at_event_frame() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

        let (left, right) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(6),
            vec![note_on(2, 100)],
            &assets,
        );

        assert_eq!(left, vec![0.0, 0.0, 0.25, 0.5, 0.75, 0.0]);
        assert_eq!(right, vec![0.0; 6]);
    }

    #[test]
    fn sampler_ignores_trigger_velocity_payload_for_amplitude() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

        let low_velocity = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![note_on(0, 1)],
            &assets,
        );
        let high_velocity = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![note_on(0, 127)],
            &assets,
        );

        assert_eq!(low_velocity, high_velocity);
        assert_eq!(low_velocity.0, vec![0.25, 0.5, 0.75, 0.0]);
    }

    #[test]
    fn sampler_ignores_midi_note_payload_for_playback_rate() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

        let low_note = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            )],
            &assets,
        );
        let high_note = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 84,
                    velocity: 100,
                },
            )],
            &assets,
        );

        assert_eq!(low_note, high_note);
    }

    #[test]
    fn routed_rate_control_changes_sampler_playback_speed() {
        let graph = sampler_graph(
            vec![
                ModuleNode::new(ModuleId::new("rate"), "adsr")
                    .with_output(builtin_ports::VALUE, SignalType::Control),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("rate"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::RATE),
            )],
        );
        let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

        let (default_rate, _) = render_offline_with_sampler_assets(
            &sampler_graph(Vec::new(), Vec::new()),
            &sampler_settings(4),
            vec![note_on(0, 100)],
            &assets,
        );
        let (routed_rate, _) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![note_on(0, 100)],
            &assets,
        );

        assert_ne!(routed_rate, default_rate);
        assert_eq!(default_rate, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn downstream_gain_can_apply_amplitude_policy_outside_sampler() {
        let output = process_vca(vec![0.25, 0.5, 0.75], vec![0.5, 0.5, 0.5]);

        assert_eq!(
            output.audio[builtin_ports::AUDIO_OUT],
            vec![0.125, 0.25, 0.375]
        );
    }

    #[test]
    fn note_to_rate_converts_midi_notes_to_equal_tempered_playback_rates() {
        let mut state = PerModuleState::NoteToRate { rate: 1.0 };

        let output = process_note_to_rate(
            &mut state,
            &[
                BlockEvent {
                    frame_offset: 0,
                    event: ScriptEvent::NoteOn {
                        note: 60,
                        velocity: 100,
                    },
                },
                BlockEvent {
                    frame_offset: 2,
                    event: ScriptEvent::NoteOn {
                        note: 72,
                        velocity: 100,
                    },
                },
            ],
            4,
        );

        assert_eq!(
            output.control[builtin_ports::RATE],
            vec![1.0, 1.0, 2.0, 2.0]
        );
    }

    #[test]
    fn routed_note_to_rate_changes_sampler_pitch_from_midi_note() {
        let graph = sampler_graph(
            vec![
                ModuleNode::new(ModuleId::new("note_rate"), "note_to_rate")
                    .with_input(builtin_ports::EVENTS, SignalType::Event)
                    .with_output(builtin_ports::RATE, SignalType::Control),
            ],
            vec![
                Cable::new(
                    PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                    PortRef::new(ModuleId::new("note_rate"), builtin_ports::EVENTS),
                ),
                Cable::new(
                    PortRef::new(ModuleId::new("note_rate"), builtin_ports::RATE),
                    PortRef::new(ModuleId::new("sampler"), builtin_ports::RATE),
                ),
            ],
        );
        graph
            .validate()
            .expect("note_to_rate should route event input to sampler rate");
        let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

        let (middle_c, _) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            )],
            &assets,
        );
        let (octave_up, _) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(4),
            vec![TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 72,
                    velocity: 100,
                },
            )],
            &assets,
        );

        assert_eq!(middle_c, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(octave_up, vec![1.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn later_trigger_replaces_monophonic_sampler_playback() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

        let (left, _) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(5),
            vec![note_on(0, 100), note_on(2, 100)],
            &assets,
        );

        assert_eq!(left, vec![1.0, 2.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn sampler_outputs_silence_after_sample_completion() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let assets = sampler_assets(vec![0.5, 0.25]);

        let (left, _) = render_offline_with_sampler_assets(
            &graph,
            &sampler_settings(5),
            vec![note_on(0, 100)],
            &assets,
        );

        assert_eq!(left, vec![0.5, 0.25, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn start_control_changes_sampler_playback_position_before_trigger() {
        let mut state = PerModuleState::Sampler {
            sample: Some(LoadedSample::new(
                48_000,
                vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            )),
            position: 0.0,
            active: false,
        };

        let output = process_sampler(
            &mut state,
            &[BlockEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }],
            &[1.0; 4],
            &[0.75; 4],
            &[0.0; 4],
            &[0.0; 4],
            &[0.0; 4],
            4,
        );

        assert_eq!(output.audio[builtin_ports::AUDIO], vec![6.0, 7.0, 0.0, 0.0]);
    }

    #[test]
    fn loop_control_wraps_active_sampler_playback() {
        let mut state = PerModuleState::Sampler {
            sample: Some(LoadedSample::new(48_000, vec![1.0, 2.0, 3.0])),
            position: 0.0,
            active: false,
        };

        let output = process_sampler(
            &mut state,
            &[BlockEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }],
            &[1.0; 7],
            &[0.0; 7],
            &[1.0; 7],
            &[0.0; 7],
            &[1.0; 7],
            7,
        );

        assert_eq!(
            output.audio[builtin_ports::AUDIO],
            vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]
        );
    }

    #[test]
    fn offline_graph_processor_handles_sampler_modules_without_panics() {
        let graph = sampler_graph(Vec::new(), Vec::new());

        let (left, right) = render_offline(&graph, &sampler_settings(4), vec![note_on(0, 100)]);

        assert_eq!(left, vec![0.0; 4]);
        assert_eq!(right, vec![0.0; 4]);
    }

    #[test]
    fn realtime_graph_processor_handles_sampler_modules_without_panics() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let mut processor = RealtimeGraphProcessor::new(graph, 48_000.0);
        let mut left = vec![1.0; 4];
        let mut right = vec![1.0; 4];

        processor.note_on(60, 100);
        let rendered = processor.render(&mut left, &mut right);

        assert_eq!(rendered, 4);
        assert_eq!(left, vec![0.0; 4]);
        assert_eq!(right, vec![0.0; 4]);
    }

    #[test]
    fn sampler_render_repeats_exactly_for_same_inputs() {
        let graph = sampler_graph(Vec::new(), Vec::new());
        let settings = sampler_settings(8);
        let assets = sampler_assets(vec![0.1, 0.2, 0.3, 0.4]);
        let events = vec![note_on(1, 64), note_on(5, 127)];

        let first = render_offline_with_sampler_assets(&graph, &settings, events.clone(), &assets);
        let second = render_offline_with_sampler_assets(&graph, &settings, events, &assets);

        assert_eq!(first, second);
    }
}
