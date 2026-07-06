use std::collections::HashMap;

use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::{self, CompiledPatch};
use crate::graph::Graph;
use crate::patch::VoiceAllocation;
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;
use crate::voice_allocator::VoiceAllocator;

use super::arena_processing;
use super::audio_arena::AudioArena;
use super::block::{collect_audio_output, process_block_compiled};
use super::dispatch::process_module;
use super::event_queue::{BoundedEventQueue, PreparedEventQueues};
use super::input_provider::CompiledInputProvider;
use super::outputs::{BlockEvent, ModuleOutputs};
use super::polyphony::build_polyphonic_states_from_compiled;
use super::process_context::ProcessContext;
use super::render_plan::{RenderPlan, RenderStep};
use super::state::PerModuleState;

pub struct RealtimeGraphProcessor {
    compiled: CompiledPatch,
    states: Vec<Vec<PerModuleState>>,
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    current_frame: u64,
    pending_events: BoundedEventQueue,
    prepared_event_queues: PreparedEventQueues,
    events_buffer: Box<[BlockEvent]>,
    allocator: VoiceAllocator,
    render_plan: RenderPlan,
    audio_arena: AudioArena,
    prepared_max_block_size: usize,
    last_render_chunk_count: usize,
    last_render_used_arena: bool,
    scratch_left: Vec<f32>,
    scratch_right: Vec<f32>,
    module_outputs: HashMap<usize, ModuleOutputs>,
    scratch_outputs: Option<HashMap<usize, ModuleOutputs>>,
    events_scratch: Vec<BlockEvent>,
    voice_event_queues: Vec<Vec<ScriptEvent>>,
    voice_queues: Vec<PreparedEventQueues>,
    accum: Vec<Option<ModuleOutputs>>,
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
        Self::polyphonic_with_sampler_assets(
            graph,
            sample_rate,
            sampler_assets,
            &VoiceAllocation::default(),
        )
    }

    pub fn polyphonic_with_sampler_assets(
        graph: Graph,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        voice_allocation: &VoiceAllocation,
    ) -> Self {
        Self::polyphonic_with_sampler_assets_and_max_block_size(
            graph,
            sample_rate,
            sampler_assets,
            voice_allocation,
            512,
        )
    }

    pub fn polyphonic_with_sampler_assets_and_max_block_size(
        graph: Graph,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        voice_allocation: &VoiceAllocation,
        prepared_max_block_size: usize,
    ) -> Self {
        let render_settings = crate::patch::RenderSettings {
            sample_rate_hz: sample_rate.max(1.0).round() as u32,
            block_size_frames: prepared_max_block_size.max(1) as u32,
            duration_frames: 0,
        };
        let compiled = compiled_patch::compile(&graph, &render_settings)
            .expect("validated graph should compile for realtime rendering");
        Self::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            graph,
            compiled,
            sample_rate,
            sampler_assets,
            voice_allocation,
            prepared_max_block_size,
        )
    }

    pub fn polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        graph: Graph,
        compiled: CompiledPatch,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        voice_allocation: &VoiceAllocation,
        prepared_max_block_size: usize,
    ) -> Self {
        let midi_idx = compiled.midi_input_index();
        let out_idx = compiled.audio_output_index();
        let max_voices = voice_allocation.max_voices.max(1) as usize;
        let states = build_polyphonic_states_from_compiled(
            &compiled,
            sample_rate,
            sampler_assets,
            max_voices,
        );
        let allocator = VoiceAllocator::new(
            voice_allocation.max_voices,
            voice_allocation.stealing.clone(),
        );

        let prepared_max_block_size = prepared_max_block_size.max(1);
        let render_plan = RenderPlan::from_compiled_patch(
            &compiled,
            prepared_max_block_size,
            max_voices,
            prepared_max_block_size,
        );
        let uses_legacy_module_outputs =
            uses_legacy_module_outputs(&compiled, midi_idx, max_voices, &render_plan);
        let queue_count = render_plan.event_queues.queue_count;
        let queue_capacity = render_plan.event_queues.queue_capacity;
        let accum_len = compiled.nodes().len();
        let audio_arena = AudioArena::new(render_plan.audio_buffers);
        let prepared_event_queues = PreparedEventQueues::new(
            render_plan.event_queues.queue_count,
            render_plan.event_queues.queue_capacity,
        );

        let events_buffer = vec![
            BlockEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 0,
                    velocity: 0,
                },
            };
            prepared_max_block_size
        ]
        .into_boxed_slice();

        Self {
            compiled,
            states,
            midi_idx,
            out_idx,
            current_frame: 0,
            pending_events: BoundedEventQueue::with_capacity(prepared_max_block_size),
            prepared_event_queues,
            events_buffer,
            allocator,
            render_plan,
            audio_arena,
            prepared_max_block_size,
            last_render_chunk_count: 0,
            last_render_used_arena: false,
            scratch_left: Vec::with_capacity(prepared_max_block_size),
            scratch_right: Vec::with_capacity(prepared_max_block_size),
            module_outputs: HashMap::with_capacity(graph.modules().len()),
            scratch_outputs: uses_legacy_module_outputs
                .then(|| HashMap::with_capacity(graph.modules().len())),
            events_scratch: Vec::with_capacity(prepared_max_block_size),
            voice_event_queues: (0..max_voices)
                .map(|_| Vec::with_capacity(prepared_max_block_size))
                .collect(),
            voice_queues: (0..max_voices)
                .map(|_| PreparedEventQueues::new(queue_count, queue_capacity))
                .collect(),
            accum: {
                let mut accum = Vec::with_capacity(accum_len);
                accum.resize_with(accum_len, || None);
                accum
            },
        }
    }

    pub fn prepared_max_block_size(&self) -> usize {
        self.prepared_max_block_size
    }

    pub fn last_render_chunk_count(&self) -> usize {
        self.last_render_chunk_count
    }

    pub fn top_level_scratch_capacities(&self) -> (usize, usize) {
        (self.scratch_left.capacity(), self.scratch_right.capacity())
    }

    pub fn module_output_scratch_capacity(&self) -> usize {
        self.scratch_outputs.as_ref().map_or(0, HashMap::capacity)
    }

    pub fn pending_event_capacity(&self) -> usize {
        self.pending_events.capacity()
    }

    pub fn pending_event_overflow_count(&self) -> usize {
        self.pending_events.dropped_events()
    }

    pub fn prepared_voice_count(&self) -> usize {
        self.states.len()
    }

    #[cfg(test)]
    pub fn last_render_used_arena(&self) -> bool {
        self.last_render_used_arena
    }

    #[cfg(test)]
    pub fn prepared_event_queue_overflow_count(&self) -> usize {
        0
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.note_on_at(note, velocity, 0);
    }

    pub fn note_off(&mut self, note: u8) {
        self.note_off_at(note, 0);
    }

    pub fn note_on_at(&mut self, note: u8, velocity: u8, frame_offset: u32) {
        let _ = self
            .pending_events
            .push_at(ScriptEvent::NoteOn { note, velocity }, frame_offset);
    }

    pub fn note_off_at(&mut self, note: u8, frame_offset: u32) {
        let _ = self
            .pending_events
            .push_at(ScriptEvent::NoteOff { note }, frame_offset);
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let frames = left.len().min(right.len());
        if frames == 0 {
            self.last_render_chunk_count = 0;
            self.last_render_used_arena = false;
            return 0;
        }

        if frames > self.prepared_max_block_size {
            let mut rendered = 0;
            let mut chunks = 0;

            while rendered < frames {
                let chunk_frames = self.prepared_max_block_size.min(frames - rendered);
                self.render_chunk(
                    &mut left[rendered..rendered + chunk_frames],
                    &mut right[rendered..rendered + chunk_frames],
                );
                rendered += chunk_frames;
                chunks += 1;
            }

            self.last_render_chunk_count = chunks;
            return frames;
        }

        self.last_render_chunk_count = 1;
        self.render_chunk(left, right)
    }

    fn render_chunk(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let frames = left.len().min(right.len());
        let block_start = self.current_frame;
        self.current_frame += frames as u64;

        if self.pending_events.is_empty() && self.render_mono_global_arena(left, right, frames) {
            self.last_render_used_arena = true;
            return frames;
        }

        if self.midi_idx.is_none() && self.render_mono_global_arena(left, right, frames) {
            self.pending_events
                .drain_into_buffer(&mut *self.events_buffer);
            self.last_render_used_arena = true;
            return frames;
        }

        self.last_render_used_arena = false;

        let event_count = self
            .pending_events
            .drain_into_buffer(&mut *self.events_buffer);
        let events = &self.events_buffer[..event_count];

        if self.allocator.max_voices() > 1 || !self.compiled.voice_node_indices().is_empty() {
            self.scratch_left.clear();
            self.scratch_right.clear();

            Self::render_polyphonic_from_plan(
                &self.compiled,
                &mut self.states,
                &mut self.allocator,
                self.out_idx,
                events,
                frames,
                block_start,
                &mut self.scratch_left,
                &mut self.scratch_right,
                &self.render_plan,
                &mut self.prepared_event_queues,
                &mut self.module_outputs,
                &mut self.voice_event_queues,
                &mut self.voice_queues,
                &mut self.accum,
                &mut self.events_scratch,
            );

            let actual = self
                .scratch_left
                .len()
                .min(self.scratch_right.len())
                .min(frames);
            for i in 0..actual {
                left[i] = self.scratch_left[i];
                right[i] = self.scratch_right[i];
            }
            for i in actual..frames {
                left[i] = 0.0;
                right[i] = 0.0;
            }
        } else {
            self.scratch_left.clear();
            self.scratch_right.clear();

            let scratch_outputs = self
                .scratch_outputs
                .as_mut()
                .expect("legacy realtime module output scratch should be prepared");

            process_block_compiled(
                &self.compiled,
                &mut self.states[0],
                self.midi_idx,
                self.out_idx,
                block_start,
                frames,
                events,
                &mut self.scratch_left,
                &mut self.scratch_right,
                scratch_outputs,
            );

            let actual = self
                .scratch_left
                .len()
                .min(self.scratch_right.len())
                .min(frames);
            for i in 0..actual {
                left[i] = self.scratch_left[i];
                right[i] = self.scratch_right[i];
            }
            for i in actual..frames {
                left[i] = 0.0;
                right[i] = 0.0;
            }
        }

        frames
    }

    fn render_mono_global_arena(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        frames: usize,
    ) -> bool {
        if self.allocator.max_voices() > 1
            || !self.compiled.voice_node_indices().is_empty()
            || self.midi_idx.is_some()
            || self.render_plan.audio_output.is_none()
            || self
                .render_plan
                .global_steps
                .iter()
                .any(|step| !is_mono_global_arena_supported(step))
        {
            return false;
        }

        let steps = self.render_plan.global_steps.as_ref();
        let arena = &mut self.audio_arena;
        let states = &mut self.states[0];
        for step in steps {
            clear_and_route_arena_inputs(arena, step, frames);
            process_mono_global_arena_step(arena, states, step, frames);
            route_prepared_event_edges(&mut self.prepared_event_queues, step);
        }

        let output = self
            .render_plan
            .audio_output
            .expect("audio output was checked before arena render");
        self.audio_arena
            .copy_to_slices(output.left, output.right, frames, left, right);
        true
    }

    fn render_polyphonic_from_plan(
        compiled: &CompiledPatch,
        states: &mut [Vec<PerModuleState>],
        allocator: &mut VoiceAllocator,
        out_idx: Option<usize>,
        events: &[BlockEvent],
        frames: usize,
        block_start_frame: u64,
        left_out: &mut Vec<f32>,
        right_out: &mut Vec<f32>,
        render_plan: &RenderPlan,
        global_event_queues: &mut PreparedEventQueues,
        all_outputs: &mut HashMap<usize, ModuleOutputs>,
        voice_event_queues: &mut Vec<Vec<ScriptEvent>>,
        voice_queues: &mut Vec<PreparedEventQueues>,
        accum: &mut Vec<Option<ModuleOutputs>>,
        events_scratch: &mut Vec<BlockEvent>,
    ) {
        global_event_queues.clear_all();
        prepare_voice_event_queues(voice_event_queues, events, allocator);

        if !has_active_voice(allocator) {
            left_out.extend(std::iter::repeat_n(0.0, frames));
            right_out.extend(std::iter::repeat_n(0.0, frames));
            return;
        }

        all_outputs.clear();
        accum.clear();
        accum.resize_with(compiled.nodes().len(), || None);
        let input_provider = CompiledInputProvider { compiled };
        for queues in voice_queues.iter_mut() {
            queues.clear_all();
        }

        for voice_idx in 0..allocator.max_voices() {
            if allocator.slot(voice_idx).is_none_or(|slot| !slot.active) {
                continue;
            }

            let voice_events = &mut voice_event_queues[voice_idx];
            let voice_queues = &mut voice_queues[voice_idx];
            route_voice_input_events(render_plan.midi_input, voice_events, voice_queues);

            let voice_states = &mut states[voice_idx];

            for step in render_plan.voice_steps.iter() {
                if step.module_kind == ModuleKind::MidiInput {
                    continue;
                }

                route_voice_event_edges(voice_queues, step);
                gather_step_events(voice_queues, step, events_scratch);

                let outputs = process_module(
                    step.module_index,
                    step.module_kind,
                    &events_scratch,
                    voice_states,
                    &input_provider,
                    &all_outputs,
                    frames,
                    block_start_frame,
                );

                route_step_outputs_to_event_queues(
                    step,
                    &outputs,
                    voice_queues,
                    global_event_queues,
                );
                all_outputs.insert(step.module_index, outputs);
            }

            accumulate_voice_outputs(accum, all_outputs, compiled, frames);
        }

        collect_accumulated_outputs(accum, all_outputs);

        for step in render_plan.global_steps.iter() {
            if step.module_kind == ModuleKind::MidiInput {
                continue;
            }

            route_global_event_edges(global_event_queues, step);

            events_scratch.clear();
            for &qid in step.event_inputs.iter() {
                if let Some(q) = global_event_queues.queue_mut(qid.0) {
                    q.drain_into_vec(events_scratch);
                }
            }

            let outputs = process_module(
                step.module_index,
                step.module_kind,
                &events_scratch,
                &mut states[0],
                &input_provider,
                &all_outputs,
                frames,
                block_start_frame,
            );

            route_step_outputs_to_global_event_queues(step, &outputs, global_event_queues);

            all_outputs.insert(step.module_index, outputs);
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

    pub fn is_finished(&self) -> bool {
        if !self.pending_events.is_empty() {
            return false;
        }
        for voice_state in &self.states {
            for state in voice_state {
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
        }
        true
    }
}

fn prepare_voice_event_queues(
    voice_events: &mut Vec<Vec<ScriptEvent>>,
    events: &[BlockEvent],
    allocator: &mut VoiceAllocator,
) {
    let max_voices = allocator.max_voices();
    while voice_events.len() < max_voices {
        voice_events.push(Vec::with_capacity(events.len()));
    }
    for events in voice_events.iter_mut().take(max_voices) {
        events.clear();
    }

    for event in events {
        if let ScriptEvent::NoteOn { note, velocity } = &event.event
            && let Some(slot) = allocator.note_on(*note, *velocity)
        {
            voice_events[slot].push(event.event.clone());
        }
    }

    for event in events {
        if let ScriptEvent::NoteOff { note } = &event.event {
            for slot_idx in 0..max_voices {
                if allocator
                    .slot(slot_idx)
                    .filter(|slot| slot.active)
                    .map(|slot| slot.note)
                    == Some(*note)
                {
                    voice_events[slot_idx].push(event.event.clone());
                }
            }
        }
    }
}

fn has_active_voice(allocator: &VoiceAllocator) -> bool {
    (0..allocator.max_voices()).any(|i| allocator.slot(i).is_some_and(|slot| slot.active))
}

fn route_voice_input_events(
    midi_input: Option<super::render_plan::EventQueueId>,
    voice_events: &mut Vec<ScriptEvent>,
    voice_queues: &mut PreparedEventQueues,
) {
    if let Some(midi_queue) = midi_input {
        if let Some(queue) = voice_queues.queue_mut(midi_queue.0) {
            for event in voice_events.drain(..) {
                let _ = queue.push(event);
            }
        }
    }
}

fn route_voice_event_edges(voice_queues: &mut PreparedEventQueues, step: &RenderStep) {
    for &edge in step.incoming_event_edges.iter() {
        let _ = voice_queues.route_event_edge(edge);
    }
}

fn gather_step_events(
    voice_queues: &mut PreparedEventQueues,
    step: &RenderStep,
    events_scratch: &mut Vec<BlockEvent>,
) {
    events_scratch.clear();
    for &qid in step.event_inputs.iter() {
        if let Some(q) = voice_queues.queue_mut(qid.0) {
            q.drain_into_vec(events_scratch);
        }
    }
}

fn route_global_event_edges(global_event_queues: &mut PreparedEventQueues, step: &RenderStep) {
    for &edge in step.incoming_event_edges.iter() {
        let _ = global_event_queues.route_event_edge(edge);
    }
}

fn route_step_outputs_to_event_queues(
    step: &RenderStep,
    outputs: &ModuleOutputs,
    voice_queues: &mut PreparedEventQueues,
    global_event_queues: &mut PreparedEventQueues,
) {
    for be in &outputs.events {
        for &eq_id in step.event_outputs.iter() {
            let _ = voice_queues
                .queue_mut(eq_id.0)
                .map(|q| q.push(be.event.clone()));
            let _ = global_event_queues
                .queue_mut(eq_id.0)
                .map(|q| q.push(be.event.clone()));
        }
    }
}

fn accumulate_voice_outputs(
    accum: &mut [Option<ModuleOutputs>],
    all_outputs: &mut HashMap<usize, ModuleOutputs>,
    compiled: &CompiledPatch,
    frames: usize,
) {
    for &idx in compiled.voice_node_indices() {
        if let Some(outputs) = all_outputs.remove(&idx) {
            let entry = accum[idx].get_or_insert_with(ModuleOutputs::empty);
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
        }
    }
}

fn collect_accumulated_outputs(
    accum: &mut Vec<Option<ModuleOutputs>>,
    all_outputs: &mut HashMap<usize, ModuleOutputs>,
) {
    all_outputs.clear();
    for (idx, output) in accum.iter_mut().enumerate() {
        if let Some(output) = output.take() {
            all_outputs.insert(idx, output);
        }
    }
}

fn route_step_outputs_to_global_event_queues(
    step: &RenderStep,
    outputs: &ModuleOutputs,
    global_event_queues: &mut PreparedEventQueues,
) {
    for be in &outputs.events {
        for &eq_id in step.event_outputs.iter() {
            let _ = global_event_queues
                .queue_mut(eq_id.0)
                .map(|q| q.push(be.event.clone()));
        }
    }
}

fn clear_and_route_arena_inputs(arena: &mut AudioArena, step: &RenderStep, frames: usize) {
    for &buffer in step.input_buffers.iter() {
        arena.clear(buffer, frames);
    }
    for default in step.control_defaults.iter() {
        arena.fill(default.buffer, frames, default.value);
    }
    for &edge in step.incoming_edges.iter() {
        arena.add_edge(edge, frames);
    }
}

fn process_mono_global_arena_step(
    arena: &mut AudioArena,
    states: &mut [PerModuleState],
    step: &RenderStep,
    frames: usize,
) {
    let mut context = ProcessContext::new(arena, &step.input_buffers, &step.output_buffers, frames);

    match step.module_kind {
        ModuleKind::AudioMixer => arena_processing::process_audio_mixer(&mut context),
        ModuleKind::Noise => {
            arena_processing::process_noise(&mut states[step.module_index], &mut context)
        }
        ModuleKind::Oscillator => {
            arena_processing::process_oscillator(&mut states[step.module_index], &mut context)
        }
        ModuleKind::Gain | ModuleKind::Multiply => arena_processing::process_gain(&mut context),
        ModuleKind::EnvelopeFollower => arena_processing::process_envelope_follower(
            &mut states[step.module_index],
            &mut context,
        ),
        ModuleKind::CurveMapper => {
            arena_processing::process_curve_mapper(&mut states[step.module_index], &mut context)
        }
        ModuleKind::Filter => {
            arena_processing::process_filter(&mut states[step.module_index], &mut context)
        }
        ModuleKind::AudioOutput => {}
        _ => unreachable!(),
    }
}

fn route_prepared_event_edges(queues: &mut PreparedEventQueues, step: &RenderStep) {
    for edge in step.incoming_event_edges.iter().copied() {
        let _ = queues.route_event_edge(edge);
    }
}

fn is_mono_global_arena_supported(step: &RenderStep) -> bool {
    match step.module_kind {
        ModuleKind::AudioOutput => step.input_buffers.len() >= 2,
        ModuleKind::AudioMixer => step.input_buffers.len() == 1 && step.output_buffers.len() == 1,
        ModuleKind::Noise => step.input_buffers.is_empty() && step.output_buffers.len() == 1,
        ModuleKind::Oscillator => step.input_buffers.len() <= 1 && step.output_buffers.len() == 1,
        ModuleKind::Gain | ModuleKind::Multiply => {
            step.input_buffers.len() == 2 && step.output_buffers.len() == 1
        }
        ModuleKind::EnvelopeFollower => {
            step.input_buffers.len() == 6 && step.output_buffers.len() == 1
        }
        ModuleKind::CurveMapper => step.input_buffers.len() == 5 && step.output_buffers.len() == 1,
        ModuleKind::Filter => step.input_buffers.len() == 4 && step.output_buffers.len() == 1,
        _ => false,
    }
}

fn uses_legacy_module_outputs(
    compiled: &CompiledPatch,
    midi_idx: Option<usize>,
    max_voices: usize,
    render_plan: &RenderPlan,
) -> bool {
    max_voices <= 1
        && compiled.voice_node_indices().is_empty()
        && (midi_idx.is_some()
            || render_plan.audio_output.is_none()
            || render_plan
                .global_steps
                .iter()
                .any(|step| !is_mono_global_arena_supported(step)))
}
