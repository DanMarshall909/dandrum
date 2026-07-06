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

    pub fn set_numeric_parameter_by_target(
        &mut self,
        module_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> bool {
        self.compiled
            .set_numeric_parameter_by_target(module_id, parameter_name, value)
    }

    pub fn numeric_parameter_value(&self, module_id: &str, parameter_name: &str) -> Option<f32> {
        self.compiled.numeric_parameter_value(module_id, parameter_name)
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
        block_start: u64,
        scratch_left: &mut Vec<f32>,
        scratch_right: &mut Vec<f32>,
        render_plan: &RenderPlan,
        prepared_event_queues: &mut PreparedEventQueues,
        module_outputs: &mut HashMap<usize, ModuleOutputs>,
        voice_event_queues: &mut [Vec<ScriptEvent>],
        voice_queues: &mut [PreparedEventQueues],
        accum: &mut [Option<ModuleOutputs>],
        events_scratch: &mut Vec<BlockEvent>,
    ) {
        block_start: {
            let _ = block_start;
        }

        if let Some(midi_idx) = compiled.midi_input_index() {
            module_outputs.insert(midi_idx, ModuleOutputs::from_events(events.to_vec()));
        }

        Self::render_global_steps_from_plan(
            compiled,
            &mut states[0],
            render_plan,
            prepared_event_queues,
            module_outputs,
            frames,
        );

        render_voice_steps_from_plan(
            compiled,
            states,
            allocator,
            events,
            frames,
            block_start,
            render_plan,
            module_outputs,
            voice_event_queues,
            voice_queues,
            accum,
            events_scratch,
        );

        collect_audio_output(out_idx, module_outputs, scratch_left, scratch_right);
    }

    fn render_global_steps_from_plan(
        compiled: &CompiledPatch,
        state: &mut [PerModuleState],
        render_plan: &RenderPlan,
        prepared_event_queues: &mut PreparedEventQueues,
        module_outputs: &mut HashMap<usize, ModuleOutputs>,
        frames: usize,
    ) {
        let provider = CompiledInputProvider { compiled };
        let context = ProcessContext::new(0, 0);
        for step in &render_plan.global_steps {
            process_module(
                step.module_index,
                &mut state[step.module_index],
                &provider,
                module_outputs,
                frames,
                &context,
                &mut module_outputs.entry(step.module_index).or_default().event_ports,
            );
            route_prepared_event_edges(prepared_event_queues, step);
        }
    }
}

fn render_voice_steps_from_plan(
    compiled: &CompiledPatch,
    states: &mut [Vec<PerModuleState>],
    allocator: &mut VoiceAllocator,
    events: &[BlockEvent],
    frames: usize,
    block_start: u64,
    render_plan: &RenderPlan,
    module_outputs: &mut HashMap<usize, ModuleOutputs>,
    voice_event_queues: &mut [Vec<ScriptEvent>],
    voice_queues: &mut [PreparedEventQueues],
    accum: &mut [Option<ModuleOutputs>],
    events_scratch: &mut Vec<BlockEvent>,
) {
    if events.is_empty() && allocator.active_voice_indices().is_empty() {
        return;
    }

    for event in events {
        match event.event {
            ScriptEvent::NoteOn { note, velocity } => {
                allocator.note_on(note, velocity, block_start + event.frame_offset as u64);
            }
            ScriptEvent::NoteOff { note } => {
                allocator.note_off(note);
            }
        }
    }

    for voice_index in allocator.active_voice_indices().to_vec() {
        let voice_context = ProcessContext::new(voice_index, block_start);
        let voice_outputs = arena_processing::render_voice_plan_to_accum(
            compiled,
            &mut states[voice_index],
            frames,
            &voice_context,
            render_plan,
            module_outputs,
            &mut voice_event_queues[voice_index],
            &mut voice_queues[voice_index],
            accum,
            events_scratch,
        );
        for (module_index, output) in voice_outputs.into_iter().enumerate() {
            if let Some(output) = output {
                module_outputs.insert(module_index, output);
            }
        }
    }
}

fn route_prepared_event_edges(queues: &mut PreparedEventQueues, step: &RenderStep) {
    for route in &step.event_routes {
        let Some(output) = queues.queue_output(route.source_queue) else {
            continue;
        };
        queues.route_event_edge(output, route.destination_queue);
    }
}

fn clear_and_route_arena_inputs(arena: &mut AudioArena, step: &RenderStep, frames: usize) {
    arena.clear_inputs(step, frames);
    arena.route_inputs(step, frames);
}

fn process_mono_global_arena_step(
    arena: &mut AudioArena,
    states: &mut [PerModuleState],
    step: &RenderStep,
    frames: usize,
) {
    arena_processing::process_mono_global_step(arena, states, step, frames);
}

fn is_mono_global_arena_supported(step: &RenderStep) -> bool {
    matches!(
        step.module_kind,
        ModuleKind::Oscillator | ModuleKind::Gain | ModuleKind::AudioMixer | ModuleKind::AudioOutput
    )
}
