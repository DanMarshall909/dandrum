use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::HashMap;

use crate::builtins::module_kind::ModuleKind;
#[cfg(test)]
use crate::compiled_patch;
use crate::compiled_patch::CompiledPatch;
#[cfg(test)]
use crate::graph::Graph;
#[cfg(test)]
use crate::patch::VoiceAllocation;
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;
#[cfg(test)]
use crate::voice_allocator::VoiceAllocator;

use super::arena_processing;
use super::audio_arena::AudioArena;
#[cfg(test)]
use super::block::{collect_audio_output, process_block_compiled};
#[cfg(test)]
use super::dispatch::process_module;
use super::event_queue::{BoundedEventQueue, PreparedEventQueues};
#[cfg(test)]
use super::input_provider::CompiledInputProvider;
use super::outputs::BlockEvent;
#[cfg(test)]
use super::outputs::ModuleOutputs;
use super::polyphony::PreparedPolyRuntimeRegion;
#[cfg(test)]
use super::polyphony::build_polyphonic_states_from_compiled;
use super::process_context::ProcessContext;
use super::render_plan::{CompiledEventEdge, RenderPlan, RenderStep};
use super::state::PerModuleState;

pub struct RealtimeGraphProcessor {
    #[cfg(test)]
    graph: Graph,
    #[cfg(test)]
    sampler_assets: PreparedSamplerAssets,
    #[cfg(test)]
    voice_allocation: VoiceAllocation,
    compiled: CompiledPatch,
    #[cfg(test)]
    states: Vec<Vec<PerModuleState>>,
    #[cfg(test)]
    midi_idx: Option<usize>,
    #[cfg(test)]
    out_idx: Option<usize>,
    current_frame: u64,
    pending_events: BoundedEventQueue,
    prepared_event_queues: PreparedEventQueues,
    events_buffer: Box<[BlockEvent]>,
    #[cfg(test)]
    allocator: VoiceAllocator,
    render_plan: RenderPlan,
    prepared_step_executors: Box<[Box<dyn PreparedStepExecutor>]>,
    can_render_root_buses: bool,
    can_render_root_buses_offline: bool,
    audio_arena: AudioArena,
    prepared_max_block_size: usize,
    last_render_chunk_count: usize,
    last_render_used_arena: bool,
    #[cfg(test)]
    scratch_left: Vec<f32>,
    #[cfg(test)]
    scratch_right: Vec<f32>,
    #[cfg(test)]
    module_outputs: HashMap<usize, ModuleOutputs>,
    #[cfg(test)]
    scratch_outputs: Option<HashMap<usize, ModuleOutputs>>,
    #[cfg(test)]
    events_scratch: Vec<BlockEvent>,
    #[cfg(test)]
    voice_event_queues: Vec<Vec<BlockEvent>>,
    #[cfg(test)]
    voice_queues: Vec<PreparedEventQueues>,
    #[cfg(test)]
    accum: Vec<Option<ModuleOutputs>>,
    prepared_poly_runtime_regions: Box<[PreparedPolyRuntimeRegion]>,
}

impl RealtimeGraphProcessor {
    #[cfg(test)]
    pub fn new(graph: Graph, sample_rate: f32) -> Self {
        Self::new_with_sampler_assets(graph, sample_rate, &PreparedSamplerAssets::empty())
    }

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        graph: Graph,
        compiled: CompiledPatch,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        voice_allocation: &VoiceAllocation,
        prepared_max_block_size: usize,
    ) -> Self {
        Self::initialize_compiled(
            compiled,
            sample_rate,
            sampler_assets,
            voice_allocation.max_voices.max(1) as usize,
            graph,
            voice_allocation.clone(),
            prepared_max_block_size,
        )
    }

    pub fn from_compiled_patch(
        compiled: CompiledPatch,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        prepared_max_block_size: usize,
    ) -> Self {
        Self::initialize_compiled(
            compiled,
            sample_rate,
            sampler_assets,
            1,
            #[cfg(test)]
            Graph::default(),
            #[cfg(test)]
            VoiceAllocation::default(),
            prepared_max_block_size,
        )
    }

    fn initialize_compiled(
        compiled: CompiledPatch,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
        max_voices: usize,
        #[cfg(test)] graph: Graph,
        #[cfg(test)] voice_allocation: VoiceAllocation,
        prepared_max_block_size: usize,
    ) -> Self {
        #[cfg(test)]
        let midi_idx = compiled.midi_input_index();
        #[cfg(test)]
        let out_idx = compiled.audio_output_index();
        #[cfg(test)]
        let states = build_polyphonic_states_from_compiled(
            &compiled,
            sample_rate,
            sampler_assets,
            max_voices,
        );
        #[cfg(test)]
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
        let prepared_step_executors =
            bind_prepared_step_executors(&compiled, &render_plan, sample_rate, sampler_assets);
        let can_render_root_buses =
            render_plan_supports_root_buses(&render_plan, false, max_voices);
        let can_render_root_buses_offline =
            render_plan_supports_root_buses(&render_plan, true, max_voices);
        #[cfg(test)]
        let uses_legacy_module_outputs =
            uses_legacy_module_outputs(&compiled, midi_idx, max_voices, &render_plan);
        #[cfg(test)]
        let queue_count = render_plan.event_queues.queue_count;
        #[cfg(test)]
        let queue_capacity = render_plan.event_queues.queue_capacity;
        #[cfg(test)]
        let accum_len = compiled.nodes().len();
        let audio_arena = AudioArena::new(render_plan.audio_buffers);
        let prepared_event_queues = PreparedEventQueues::new(
            render_plan.event_queues.queue_count,
            render_plan.event_queues.queue_capacity,
        );
        let prepared_poly_runtime_regions = compiled
            .poly_regions()
            .iter()
            .map(|region| PreparedPolyRuntimeRegion::new(region, sample_rate, sampler_assets))
            .collect::<Vec<_>>()
            .into_boxed_slice();

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
            #[cfg(test)]
            graph: graph.clone(),
            #[cfg(test)]
            sampler_assets: sampler_assets.clone(),
            #[cfg(test)]
            voice_allocation,
            compiled,
            #[cfg(test)]
            states,
            #[cfg(test)]
            midi_idx,
            #[cfg(test)]
            out_idx,
            current_frame: 0,
            pending_events: BoundedEventQueue::with_capacity(prepared_max_block_size),
            prepared_event_queues,
            events_buffer,
            #[cfg(test)]
            allocator,
            render_plan,
            prepared_step_executors,
            can_render_root_buses,
            can_render_root_buses_offline,
            audio_arena,
            prepared_max_block_size,
            last_render_chunk_count: 0,
            last_render_used_arena: false,
            #[cfg(test)]
            scratch_left: Vec::with_capacity(prepared_max_block_size),
            #[cfg(test)]
            scratch_right: Vec::with_capacity(prepared_max_block_size),
            #[cfg(test)]
            module_outputs: HashMap::with_capacity(graph.modules().len()),
            #[cfg(test)]
            scratch_outputs: uses_legacy_module_outputs
                .then(|| HashMap::with_capacity(graph.modules().len())),
            #[cfg(test)]
            events_scratch: Vec::with_capacity(prepared_max_block_size),
            #[cfg(test)]
            voice_event_queues: (0..max_voices)
                .map(|_| Vec::with_capacity(prepared_max_block_size))
                .collect(),
            #[cfg(test)]
            voice_queues: (0..max_voices)
                .map(|_| PreparedEventQueues::new(queue_count, queue_capacity))
                .collect(),
            #[cfg(test)]
            accum: {
                let mut accum = Vec::with_capacity(accum_len);
                accum.resize_with(accum_len, || None);
                accum
            },
            prepared_poly_runtime_regions,
        }
    }

    /// Start a fresh processing session off the audio thread. Cloning the
    /// compiled patch retains current parameter values and their slot indices.
    #[cfg(test)]
    pub(crate) fn prepare_realtime(&mut self, sample_rate: f32, max_block_size: usize) {
        *self = Self::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            self.graph.clone(),
            self.compiled.clone(),
            sample_rate,
            &self.sampler_assets,
            &self.voice_allocation,
            max_block_size,
        );
    }

    pub fn prepared_max_block_size(&self) -> usize {
        self.prepared_max_block_size
    }

    pub fn prepared_poly_runtime_regions(&self) -> &[PreparedPolyRuntimeRegion] {
        &self.prepared_poly_runtime_regions
    }

    #[cfg(test)]
    pub(crate) fn prepared_poly_runtime_regions_mut_for_test(
        &mut self,
    ) -> &mut [PreparedPolyRuntimeRegion] {
        &mut self.prepared_poly_runtime_regions
    }

    pub fn last_render_chunk_count(&self) -> usize {
        self.last_render_chunk_count
    }

    #[cfg(test)]
    pub fn top_level_scratch_capacities(&self) -> (usize, usize) {
        (self.scratch_left.capacity(), self.scratch_right.capacity())
    }

    #[cfg(test)]
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
        self.render_plan.audio_buffers.max_voices
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
        self.compiled
            .numeric_parameter_value(module_id, parameter_name)
    }

    pub fn parameter_slot_index(&self, module_id: &str, parameter_name: &str) -> Option<usize> {
        self.compiled
            .parameter_slot_index(module_id, parameter_name)
    }

    /// O(1) parameter update by a previously-resolved slot index. Safe to call
    /// from a realtime audio callback: no string comparisons, no allocation.
    pub fn set_parameter_slot(&mut self, slot_index: usize, value: f32) -> bool {
        self.compiled.set_parameter_slot(slot_index, value)
    }

    #[cfg(test)]
    pub fn last_render_used_arena(&self) -> bool {
        self.last_render_used_arena
    }

    #[cfg(test)]
    pub fn prepared_event_queue_overflow_count(&self) -> usize {
        0
    }

    #[cfg(test)]
    pub(crate) fn route_poly_note_event_for_test(
        &mut self,
        node_id: &str,
        event: ScriptEvent,
        frame_offset: u32,
    ) {
        let region = self
            .prepared_poly_runtime_regions
            .iter_mut()
            .find(|region| region.node_id() == node_id)
            .expect("test names a prepared poly region");
        region.begin_block(self.prepared_max_block_size, self.current_frame);
        region.route_note_events(
            &[BlockEvent {
                frame_offset,
                event,
            }],
            self.prepared_max_block_size,
        );
    }

    #[cfg(test)]
    pub(crate) fn set_poly_child_control_default_for_test(
        &mut self,
        poly_node_id: &str,
        child_module_id: &str,
        port_name: &str,
        value: f32,
    ) -> bool {
        self.prepared_poly_runtime_regions
            .iter_mut()
            .find(|region| region.node_id() == poly_node_id)
            .is_some_and(|region| {
                region.set_child_control_default_for_test(child_module_id, port_name, value)
            })
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.note_on_at(note, velocity, 0);
    }

    /// Stop every voice and clear prepared DSP and event state for panic or reload.
    pub fn reset(&mut self) {
        self.pending_events.clear();
        self.prepared_event_queues.clear_all();
        #[cfg(test)]
        self.allocator.reset();
        #[cfg(test)]
        for voice in &mut self.states {
            for state in voice {
                state.reset_all();
            }
        }
        for executor in self.prepared_step_executors.iter_mut() {
            executor.reset_all();
        }
        for region in self.prepared_poly_runtime_regions.iter_mut() {
            region.reset();
        }
        self.audio_arena.reset();
        #[cfg(test)]
        for queue in &mut self.voice_event_queues {
            queue.clear();
        }
        #[cfg(test)]
        for queues in &mut self.voice_queues {
            queues.clear_all();
        }
        #[cfg(test)]
        self.module_outputs.clear();
        #[cfg(test)]
        if let Some(outputs) = self.scratch_outputs.as_mut() {
            outputs.clear();
        }
        #[cfg(test)]
        self.scratch_left.clear();
        #[cfg(test)]
        self.scratch_right.clear();
        #[cfg(test)]
        self.events_scratch.clear();
        #[cfg(test)]
        self.accum.clear();
        self.current_frame = 0;
        self.last_render_chunk_count = 0;
        self.last_render_used_arena = false;
    }

    pub fn note_off(&mut self, note: u8) {
        self.note_off_at(note, 0);
    }

    pub fn note_on_at(&mut self, note: u8, velocity: u8, frame_offset: u32) {
        let _ = self.try_note_on_at(note, velocity, frame_offset);
    }

    pub fn note_off_at(&mut self, note: u8, frame_offset: u32) {
        let _ = self.try_note_off_at(note, frame_offset);
    }

    pub fn try_note_on_at(&mut self, note: u8, velocity: u8, frame_offset: u32) -> bool {
        self.pending_events
            .push_at(ScriptEvent::NoteOn { note, velocity }, frame_offset)
            .is_ok()
    }

    pub fn try_note_off_at(&mut self, note: u8, frame_offset: u32) -> bool {
        self.pending_events
            .push_at(ScriptEvent::NoteOff { note }, frame_offset)
            .is_ok()
    }

    #[cfg(test)]
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

    /// Render prepared root outputs in root-port order. Each outer entry is one
    /// named root output and contains one planar vector per channel.
    pub fn render_root_outputs(&mut self, outputs: &mut [Vec<Vec<f32>>]) -> usize {
        self.render_root_buses(&[], outputs)
    }

    /// Preparation can use this to reject schedules that the named-bus arena
    /// path cannot execute before exposing a realtime handle to a host.
    pub(crate) fn can_render_root_buses(&self) -> bool {
        self.can_render_root_buses_with_scripts(false)
    }

    pub(crate) fn can_render_root_buses_offline(&self) -> bool {
        self.can_render_root_buses_with_scripts(true)
    }

    fn can_render_root_buses_with_scripts(&self, allow_scripts: bool) -> bool {
        if allow_scripts {
            self.can_render_root_buses_offline
        } else {
            self.can_render_root_buses
        }
    }

    /// Render planar root input and output buffers in their prepared root-port
    /// order. Omitted input entries are treated as silence.
    pub fn render_root_buses(
        &mut self,
        inputs: &[Vec<Vec<f32>>],
        outputs: &mut [Vec<Vec<f32>>],
    ) -> usize {
        self.render_root_buses_with_scripts(inputs, outputs, false)
    }

    pub(crate) fn render_root_buses_offline(
        &mut self,
        inputs: &[Vec<Vec<f32>>],
        outputs: &mut [Vec<Vec<f32>>],
    ) -> usize {
        self.render_root_buses_with_scripts(inputs, outputs, true)
    }

    fn render_root_buses_with_scripts(
        &mut self,
        inputs: &[Vec<Vec<f32>>],
        outputs: &mut [Vec<Vec<f32>>],
        allow_scripts: bool,
    ) -> usize {
        let Some(frames) = outputs
            .iter()
            .flat_map(|bus| bus.iter().map(Vec::len))
            .min()
        else {
            return 0;
        };
        if !self.can_render_root_buses_with_scripts(allow_scripts) {
            return 0;
        }
        let event_count = self
            .pending_events
            .drain_into_buffer(&mut self.events_buffer);
        let segment_poly_events = !self.prepared_poly_runtime_regions.is_empty() && event_count > 0;
        let mut segment_start = 0;
        let mut chunks = 0;
        while segment_start < frames {
            let next_event = if segment_poly_events {
                self.events_buffer[..event_count]
                    .iter()
                    .map(|event| event.frame_offset as usize)
                    .filter(|offset| *offset > segment_start && *offset < frames)
                    .min()
                    .unwrap_or(frames)
            } else {
                frames
            };
            let segment_end = next_event.min(segment_start + self.prepared_max_block_size);
            self.render_root_bus_segment(
                inputs,
                outputs,
                segment_start,
                segment_end - segment_start,
                event_count,
            );
            segment_start = segment_end;
            chunks += 1;
        }
        self.last_render_chunk_count = chunks;
        frames
    }

    fn render_root_bus_segment(
        &mut self,
        inputs: &[Vec<Vec<f32>>],
        outputs: &mut [Vec<Vec<f32>>],
        segment_start: usize,
        frames: usize,
        event_count: usize,
    ) {
        for region in self.prepared_poly_runtime_regions.iter_mut() {
            region.begin_block(frames, self.current_frame);
        }
        self.prepared_event_queues.clear_all();
        if let Some(queue) = self.render_plan.midi_input {
            let destination = self
                .prepared_event_queues
                .queue_mut(queue.0)
                .expect("compiled MIDI output has a prepared event queue");
            for event in &self.events_buffer[..event_count] {
                let event_frame = event.frame_offset as usize;
                if event_frame >= segment_start && event_frame < segment_start + frames {
                    let _ = destination
                        .push_at(event.event.clone(), (event_frame - segment_start) as u32);
                }
            }
        } else {
            for event in &self.events_buffer[..event_count] {
                let event_frame = event.frame_offset as usize;
                if event_frame >= segment_start && event_frame < segment_start + frames {
                    let routed = BlockEvent {
                        frame_offset: (event_frame - segment_start) as u32,
                        event: event.event.clone(),
                    };
                    for region in self.prepared_poly_runtime_regions.iter_mut() {
                        region.route_note_events(std::slice::from_ref(&routed), frames);
                    }
                }
            }
        }

        for (input_index, planned) in self.compiled.root_bus_plan().inputs().iter().enumerate() {
            let Some(span) = planned.span() else { continue };
            for channel in 0..span.channel_count {
                let buffer = super::render_plan::BufferId(span.first_buffer + channel);
                self.audio_arena.clear(buffer, frames);
                if planned.is_bound() {
                    if let Some(source) = inputs.get(input_index).and_then(|bus| bus.get(channel)) {
                        if let Some(remaining) = source.get(segment_start..) {
                            let actual = frames.min(remaining.len());
                            self.audio_arena
                                .slice_mut(buffer, actual)
                                .copy_from_slice(&remaining[..actual]);
                        }
                    }
                }
            }
        }

        let mut step_context = PreparedStepContext {
            arena: &mut self.audio_arena,
            #[cfg(test)]
            states: &mut self.states[0],
            #[cfg(not(test))]
            states: &mut [],
            poly_regions: &mut self.prepared_poly_runtime_regions,
            event_queues: &mut self.prepared_event_queues,
            compiled: &self.compiled,
            frames,
            block_start_frame: self.current_frame,
        };
        for (step, executor) in self
            .render_plan
            .global_steps
            .iter()
            .zip(self.prepared_step_executors.iter_mut())
        {
            execute_bound_prepared_step(&mut step_context, step, executor.as_mut());
        }
        capture_bound_feedback_delays(
            &mut self.audio_arena,
            &self.render_plan.global_steps,
            &self.render_plan.feedback_step_indices,
            &mut self.prepared_step_executors,
            frames,
            &self.compiled,
        );
        for (planned, bus) in self
            .compiled
            .root_bus_plan()
            .outputs()
            .iter()
            .zip(outputs.iter_mut())
        {
            let Some(span) = planned.span() else { continue };
            for (channel, destination) in bus.iter_mut().enumerate().take(span.channel_count) {
                destination[segment_start..segment_start + frames].copy_from_slice(
                    self.audio_arena.slice(
                        super::render_plan::BufferId(span.first_buffer + channel),
                        frames,
                    ),
                );
            }
        }
        self.current_frame += frames as u64;
    }

    #[cfg(test)]
    fn render_chunk(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let frames = left.len().min(right.len());
        let block_start = self.current_frame;
        self.current_frame += frames as u64;
        for region in self.prepared_poly_runtime_regions.iter_mut() {
            region.begin_block(frames, block_start);
        }
        self.prepared_event_queues.clear_all();

        if self.pending_events.is_empty() && self.render_mono_global_arena(left, right, frames) {
            self.last_render_used_arena = true;
            return frames;
        }

        let predrained_event_count = if self.midi_idx.is_none() {
            let event_count = self.drain_and_route_poly_events(frames);
            if self.render_mono_global_arena(left, right, frames) {
                self.last_render_used_arena = true;
                return frames;
            }
            Some(event_count)
        } else {
            None
        };

        self.last_render_used_arena = false;

        let event_count =
            predrained_event_count.unwrap_or_else(|| self.drain_and_route_poly_events(frames));
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

    #[cfg(test)]
    fn drain_and_route_poly_events(&mut self, frames: usize) -> usize {
        let event_count = self
            .pending_events
            .drain_into_buffer(&mut self.events_buffer);
        let events = &self.events_buffer[..event_count];
        for region in self.prepared_poly_runtime_regions.iter_mut() {
            region.route_note_events(events, frames);
        }
        event_count
    }

    #[cfg(test)]
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
        let mut context = PreparedStepContext {
            arena: &mut self.audio_arena,
            states: &mut self.states[0],
            poly_regions: &mut self.prepared_poly_runtime_regions,
            event_queues: &mut self.prepared_event_queues,
            compiled: &self.compiled,
            frames,
            block_start_frame: self.current_frame - frames as u64,
        };
        for step in steps {
            execute_prepared_step(&mut context, step);
        }

        let output = self
            .render_plan
            .audio_output
            .expect("audio output was checked before arena render");
        self.audio_arena
            .copy_to_slices(output.left, output.right, frames, left, right);
        true
    }

    #[cfg(test)]
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
        voice_event_queues: &mut Vec<Vec<BlockEvent>>,
        voice_queues: &mut Vec<PreparedEventQueues>,
        accum: &mut Vec<Option<ModuleOutputs>>,
        events_scratch: &mut Vec<BlockEvent>,
    ) {
        global_event_queues.clear_all();
        prepare_voice_event_queues(voice_event_queues, events, allocator, states, compiled);

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
        if self
            .prepared_step_executors
            .iter()
            .any(|executor| executor.is_active())
        {
            return false;
        }
        #[cfg(test)]
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

#[cfg(test)]
fn prepare_voice_event_queues(
    voice_events: &mut Vec<Vec<BlockEvent>>,
    events: &[BlockEvent],
    allocator: &mut VoiceAllocator,
    states: &mut [Vec<PerModuleState>],
    compiled: &CompiledPatch,
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
            for &node_index in compiled.voice_node_indices() {
                states[slot][node_index].reset_voice();
            }
            voice_events[slot].push(event.clone());
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
                    voice_events[slot_idx].push(event.clone());
                }
            }
        }
    }
}

#[cfg(test)]
fn has_active_voice(allocator: &VoiceAllocator) -> bool {
    (0..allocator.max_voices()).any(|i| allocator.slot(i).is_some_and(|slot| slot.active))
}

#[cfg(test)]
fn route_voice_input_events(
    midi_input: Option<super::render_plan::EventQueueId>,
    voice_events: &mut Vec<BlockEvent>,
    voice_queues: &mut PreparedEventQueues,
) {
    if let Some(midi_queue) = midi_input {
        if let Some(queue) = voice_queues.queue_mut(midi_queue.0) {
            for event in voice_events.drain(..) {
                let _ = queue.push_at(event.event, event.frame_offset);
            }
        }
    }
}

#[cfg(test)]
fn route_voice_event_edges(voice_queues: &mut PreparedEventQueues, step: &RenderStep) {
    for &edge in step.incoming_event_edges.iter() {
        let _ = voice_queues.route_event_edge(edge);
    }
}

#[cfg(test)]
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

#[cfg(test)]
fn route_global_event_edges(global_event_queues: &mut PreparedEventQueues, step: &RenderStep) {
    for &edge in step.incoming_event_edges.iter() {
        let _ = global_event_queues.route_event_edge(edge);
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

fn process_offline_script_step(
    arena: &mut AudioArena,
    runtime: &mut crate::script::RhaiScriptRuntime,
    script_state: &mut crate::script::ScriptModuleState,
    event_queues: &mut PreparedEventQueues,
    step: &RenderStep,
    frames: usize,
    compiled: &CompiledPatch,
) {
    let node = &compiled.nodes()[step.module_index];
    let mut controls = BTreeMap::new();
    let mut input_buffer = 0;
    for (name, span) in node
        .input_port_names
        .iter()
        .zip(node.input_port_spans.iter())
    {
        if span.channel_count > 0 {
            controls.insert(
                name.clone(),
                arena.sample(step.input_buffers[input_buffer], 0),
            );
            input_buffer += span.channel_count;
        }
    }
    let mut events = Vec::new();
    for queue_id in step.event_inputs.iter() {
        if let Some(queue) = event_queues.queue_ref(queue_id.0) {
            events.extend_from_slice(queue.events());
        }
    }
    let rendered =
        super::processing::process_script_state(runtime, script_state, &events, controls, frames);
    let mut output_buffer = 0;
    let mut event_output = 0;
    for (name, span) in node
        .output_port_names
        .iter()
        .zip(node.output_port_spans.iter())
    {
        if span.channel_count > 0 {
            let samples = rendered.control.get(name);
            for channel in 0..span.channel_count {
                let buffer = step.output_buffers[output_buffer + channel];
                for frame in 0..frames {
                    arena.set_sample(
                        buffer,
                        frame,
                        samples
                            .and_then(|samples| samples.get(frame))
                            .copied()
                            .unwrap_or(0.0),
                    );
                }
            }
            output_buffer += span.channel_count;
        } else {
            if let Some(output_events) = rendered.event_ports.get(name) {
                if let Some(queue) = event_queues.queue_mut(step.event_outputs[event_output].0) {
                    for event in output_events {
                        let _ = queue.push_at(event.event.clone(), event.frame_offset);
                    }
                }
            }
            event_output += 1;
        }
    }
}

pub(super) struct PreparedStepContext<'a> {
    pub(super) arena: &'a mut AudioArena,
    pub(super) states: &'a mut [PerModuleState],
    pub(super) poly_regions: &'a mut [PreparedPolyRuntimeRegion],
    pub(super) event_queues: &'a mut PreparedEventQueues,
    pub(super) compiled: &'a CompiledPatch,
    pub(super) frames: usize,
    pub(super) block_start_frame: u64,
}

pub(super) type PreparedStepProcessor = fn(&mut PreparedStepContext<'_>, &RenderStep);

#[cfg(test)]
pub(super) fn execute_prepared_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    route_prepared_event_edges(context.event_queues, step);
    clear_and_route_arena_inputs(context.arena, step, context.frames, context.compiled);
    (step.processor)(context, step);
}

pub(super) trait PreparedStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep);
    fn capture(&mut self, _: &ProcessContext<'_>) {}
    fn begin_block(&mut self) {}
    fn queue_sample_zone_choice(&mut self, _: Option<usize>) {}
    fn reset_voice(&mut self) {}
    fn is_active(&self) -> bool {
        false
    }
    fn reset_all(&mut self) {
        self.reset_voice();
    }
}

struct GenericStepExecutor(PreparedStepProcessor);

impl PreparedStepExecutor for GenericStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        (self.0)(context, step);
    }
}

struct FeedbackDelayStepExecutor {
    samples: Box<[Box<[f32]>]>,
    position: usize,
}

impl PreparedStepExecutor for FeedbackDelayStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::emit_feedback_delay_parts(
            &self.samples,
            self.position,
            &mut process_context,
        );
    }

    fn capture(&mut self, context: &ProcessContext<'_>) {
        arena_processing::capture_feedback_delay_parts(
            &mut self.samples,
            &mut self.position,
            context,
        );
    }

    fn reset_voice(&mut self) {
        for channel in self.samples.iter_mut() {
            channel.fill(0.0);
        }
        self.position = 0;
    }
}

struct NoiseStepExecutor {
    states: Box<[u32]>,
    initial_seed: u32,
}

impl PreparedStepExecutor for NoiseStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_noise_states(&mut self.states, &mut process_context);
    }

    fn reset_voice(&mut self) {
        for (channel, state) in self.states.iter_mut().enumerate() {
            *state = self.initial_seed.wrapping_add(channel as u32);
        }
    }
}

struct OscillatorStepExecutor {
    phase: f32,
    sample_rate: f32,
    waveform: crate::oscillator::Waveform,
}

struct LfoStepExecutor {
    phase: f32,
    sample_rate: f32,
}

impl PreparedStepExecutor for LfoStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_lfo_state(
            &mut self.phase,
            self.sample_rate,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        self.phase = 0.0;
    }
}

struct SlewStepExecutor {
    current: f32,
    sample_rate: f32,
}

impl PreparedStepExecutor for SlewStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_slew_state(
            &mut self.current,
            self.sample_rate,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        self.current = 0.0;
    }
}

struct DynamicsStepExecutor {
    processors: Box<[crate::dynamics_processor::DynamicsProcessor]>,
}

impl PreparedStepExecutor for DynamicsStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_dynamics_processors(&mut self.processors, &mut process_context);
    }

    fn reset_voice(&mut self) {
        for processor in self.processors.iter_mut() {
            processor.reset();
        }
    }
}

struct SpectralStepExecutor {
    processor: crate::spectral::SpectralProcessor,
}

struct SamplerStepExecutor {
    sample: Option<crate::compiled_patch::SampleResourceHandle>,
    position: f32,
    active: bool,
}

impl PreparedStepExecutor for SamplerStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_sampler_state(
            &self.sample,
            &mut self.position,
            &mut self.active,
            &mut process_context,
            events,
        );
    }

    fn reset_voice(&mut self) {
        self.position = 0.0;
        self.active = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

struct SamplePlayerStepExecutor {
    sample: Option<crate::compiled_patch::SampleResourceHandle>,
    region: crate::kernel::document::SampleRegion,
    mode: String,
    interpolation: crate::compiled_patch::SampleInterpolation,
    position: f64,
    active: bool,
}

impl PreparedStepExecutor for SamplePlayerStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let gate_events = context
            .event_queues
            .queue_ref(step.event_inputs[1].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_sample_player_state(
            &self.sample,
            &self.region,
            &self.mode,
            self.interpolation,
            &mut self.position,
            &mut self.active,
            &mut process_context,
            events,
            gate_events,
        );
    }

    fn reset_voice(&mut self) {
        self.position = 0.0;
        self.active = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

struct SampleMapPlayerStepExecutor {
    zones: Box<[crate::compiled_patch::CompiledSampleZone]>,
    selection_mode: crate::compiled_patch::SampleSelectionMode,
    reject_new_while_active: bool,
    round_robin_counters: Box<[usize]>,
    initial_seed: u64,
    rng_state: u64,
    selected_zone: Option<usize>,
    selected_pitch_ratio: f32,
    position: f64,
    active: bool,
    queued_choices: Vec<Option<usize>>,
}

impl PreparedStepExecutor for SampleMapPlayerStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_sample_map_player_state(
            &self.zones,
            self.selection_mode,
            self.reject_new_while_active,
            &mut self.round_robin_counters,
            &mut self.rng_state,
            (!self.queued_choices.is_empty()).then_some(self.queued_choices.as_slice()),
            &mut self.selected_zone,
            &mut self.selected_pitch_ratio,
            &mut self.position,
            &mut self.active,
            &mut process_context,
            events,
        );
    }

    fn begin_block(&mut self) {
        self.queued_choices.clear();
    }

    fn queue_sample_zone_choice(&mut self, choice: Option<usize>) {
        if self.queued_choices.len() < self.queued_choices.capacity() {
            self.queued_choices.push(choice);
        }
    }

    fn reset_voice(&mut self) {
        self.round_robin_counters.fill(0);
        self.rng_state = self.initial_seed;
        self.selected_zone = None;
        self.selected_pitch_ratio = 1.0;
        self.position = 0.0;
        self.active = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset_all(&mut self) {
        self.reset_voice();
        self.queued_choices.clear();
    }
}

struct NoteToRateStepExecutor {
    rate: f32,
}

impl PreparedStepExecutor for NoteToRateStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_note_to_rate_state(&mut self.rate, &mut process_context, events);
    }

    fn reset_voice(&mut self) {
        self.rate = 1.0;
    }
}

struct EventFilterStepExecutor {
    note: Option<u8>,
}

impl PreparedStepExecutor for EventFilterStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let edge = CompiledEventEdge {
            source: step.event_inputs[0],
            destination: step.event_outputs[0],
        };
        let _ = context
            .event_queues
            .route_filtered_event_edge(edge, self.note);
    }
}

struct EnvelopeFollowerStepExecutor {
    detector: crate::envelope_follower::EnvelopeFollower,
    mode: crate::envelope_follower::DetectionMode,
}

impl PreparedStepExecutor for EnvelopeFollowerStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_envelope_follower_state(
            &mut self.detector,
            self.mode,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        self.detector.reset();
    }
}

struct CurveMapperStepExecutor {
    mapper: crate::curve_mapper::CurveMapper,
}

impl PreparedStepExecutor for CurveMapperStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_curve_mapper_state(&mut self.mapper, &mut process_context);
    }
}

struct FilterStepExecutor {
    filters: Box<[Box<dyn crate::filter::FilterAlgorithm>]>,
    sample_rate: f64,
}

impl PreparedStepExecutor for FilterStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_filter_states(
            &mut self.filters,
            self.sample_rate,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        for filter in self.filters.iter_mut() {
            filter.reset();
        }
    }
}

struct CompensationDelayStepExecutor {
    samples: Box<[Box<[f32]>]>,
    positions: Box<[usize]>,
}

impl PreparedStepExecutor for CompensationDelayStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_compensation_delay_states(
            &mut self.samples,
            &mut self.positions,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        for channel in self.samples.iter_mut() {
            channel.fill(0.0);
        }
        self.positions.fill(0);
    }
}

struct ConvolutionStepExecutor {
    processors: Box<[crate::convolution::Convolution]>,
}

impl PreparedStepExecutor for ConvolutionStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_convolution_processors(
            &mut self.processors,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        for processor in self.processors.iter_mut() {
            processor.reset();
        }
    }
}

struct FrequencySplitterStepExecutor {
    filters: Box<
        [(
            crate::crossover::LinkwitzRiley4,
            crate::crossover::LinkwitzRiley4,
        )],
    >,
    sample_rate: f64,
}

impl PreparedStepExecutor for FrequencySplitterStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_frequency_splitter_states(
            &mut self.filters,
            self.sample_rate,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        for (low, high) in self.filters.iter_mut() {
            low.reset();
            high.reset();
        }
    }
}

struct EchoStepExecutor {
    processor: crate::echo::Echo,
}

impl PreparedStepExecutor for EchoStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_echo_state(&mut self.processor, &mut process_context);
    }

    fn reset_voice(&mut self) {
        self.processor.reset();
    }
}

struct ReverbStepExecutor {
    processor: crate::reverb::Reverb,
}

struct AdsrStepExecutor {
    level: f32,
    gate_active: bool,
    release_start_frame: u64,
    release_start_level: f32,
    sample_rate: f32,
}

impl PreparedStepExecutor for AdsrStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_adsr_state(
            &mut self.level,
            &mut self.gate_active,
            &mut self.release_start_frame,
            &mut self.release_start_level,
            self.sample_rate,
            &mut process_context,
            events,
            context.block_start_frame,
        );
    }

    fn reset_voice(&mut self) {
        self.level = 0.0;
        self.gate_active = false;
        self.release_start_frame = 0;
        self.release_start_level = 0.0;
    }

    fn is_active(&self) -> bool {
        self.gate_active || self.level > 0.001
    }
}

struct NoteToControlStepExecutor {
    gate_active: bool,
    current_note: Option<u8>,
    current_velocity: f32,
    current_frequency: f32,
    current_pitch_ratio: f32,
    current_slide: bool,
}

impl PreparedStepExecutor for NoteToControlStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let (input, output) = context
            .event_queues
            .queue_pair(step.event_inputs[0], step.event_outputs[0])
            .expect("compiled note_to_control queues are distinct and valid");
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_note_to_control_state(
            &mut self.gate_active,
            &mut self.current_note,
            &mut self.current_velocity,
            &mut self.current_frequency,
            &mut self.current_pitch_ratio,
            &mut self.current_slide,
            &mut process_context,
            input.events(),
            output,
        );
    }

    fn reset_voice(&mut self) {
        self.gate_active = false;
        self.current_note = None;
        self.current_velocity = 0.0;
        self.current_frequency = 0.0;
        self.current_pitch_ratio = 0.0;
        self.current_slide = false;
    }
}

struct DecayStepExecutor {
    level: f32,
    triggered: bool,
    elapsed_frames: u64,
    sample_rate: f32,
    curve: crate::decay::DecayCurve,
}

struct ScriptStepExecutor {
    runtime: crate::script::RhaiScriptRuntime,
    state: crate::script::ScriptModuleState,
}

impl PreparedStepExecutor for ScriptStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        process_offline_script_step(
            context.arena,
            &mut self.runtime,
            &mut self.state,
            context.event_queues,
            step,
            context.frames,
            context.compiled,
        );
    }

    fn reset_voice(&mut self) {
        self.state = crate::script::ScriptModuleState::default();
    }
}

impl PreparedStepExecutor for DecayStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let events = context
            .event_queues
            .queue_ref(step.event_inputs[0].0)
            .map_or(&[][..], |queue| queue.events());
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_decay_state(
            &mut self.level,
            &mut self.triggered,
            &mut self.elapsed_frames,
            self.sample_rate,
            self.curve,
            &mut process_context,
            events,
        );
    }

    fn reset_voice(&mut self) {
        self.level = 0.0;
        self.triggered = false;
        self.elapsed_frames = 0;
    }
}

impl PreparedStepExecutor for ReverbStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_reverb_state(&mut self.processor, &mut process_context);
    }

    fn reset_voice(&mut self) {
        self.processor.reset();
    }
}

impl PreparedStepExecutor for SpectralStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_spectral_processor_state(
            &mut self.processor,
            &mut process_context,
        );
    }

    fn reset_voice(&mut self) {
        self.processor.reset();
    }
}

impl PreparedStepExecutor for OscillatorStepExecutor {
    fn execute(&mut self, context: &mut PreparedStepContext<'_>, step: &RenderStep) {
        let mut process_context = ProcessContext::new(
            context.arena,
            &step.input_buffers,
            &step.output_buffers,
            context.frames,
        );
        arena_processing::process_oscillator_state(
            &mut self.phase,
            self.sample_rate,
            self.waveform,
            &mut process_context,
        );
    }

    fn reset_all(&mut self) {
        self.phase = 0.0;
    }
}

pub(super) fn bind_prepared_step_executors(
    compiled: &CompiledPatch,
    plan: &RenderPlan,
    sample_rate: f32,
    sampler_assets: &PreparedSamplerAssets,
) -> Box<[Box<dyn PreparedStepExecutor>]> {
    plan.global_steps
        .iter()
        .map(|step| {
            // Legacy delay kinds are rejected by preparation. Keep their
            // diagnostic callable available to tests that construct a plan.
            if matches!(
                step.module_kind,
                ModuleKind::AudioDelayOneSample | ModuleKind::BlockDelay | ModuleKind::ControlDelay
            ) {
                return Box::new(GenericStepExecutor(step.processor))
                    as Box<dyn PreparedStepExecutor>;
            }
            // This exhaustive state match runs during preparation. Each arm
            // moves concrete state into one executor, so render blocks never
            // inspect the state enum.
            let state = PerModuleState::new_compiled(
                &compiled.nodes()[step.module_index],
                sample_rate,
                sampler_assets,
            );
            let executor: Box<dyn PreparedStepExecutor> = match state {
                PerModuleState::FeedbackDelay { samples, position } => {
                    Box::new(FeedbackDelayStepExecutor { samples, position })
                }
                PerModuleState::Noise {
                    states,
                    initial_seed,
                } => Box::new(NoiseStepExecutor {
                    states,
                    initial_seed,
                }),
                PerModuleState::Oscillator {
                    phase,
                    sample_rate,
                    waveform,
                } => Box::new(OscillatorStepExecutor {
                    phase,
                    sample_rate,
                    waveform,
                }),
                PerModuleState::Lfo { phase, sample_rate } => {
                    Box::new(LfoStepExecutor { phase, sample_rate })
                }
                PerModuleState::Slew {
                    current,
                    sample_rate,
                } => Box::new(SlewStepExecutor {
                    current,
                    sample_rate,
                }),
                PerModuleState::DynamicsProcessor { processors } => {
                    Box::new(DynamicsStepExecutor { processors })
                }
                PerModuleState::SpectralProcessor { processor } => {
                    Box::new(SpectralStepExecutor { processor })
                }
                PerModuleState::Sampler {
                    sample,
                    position,
                    active,
                } => Box::new(SamplerStepExecutor {
                    sample,
                    position,
                    active,
                }),
                PerModuleState::SamplePlayer {
                    sample,
                    region,
                    mode,
                    interpolation,
                    position,
                    active,
                } => Box::new(SamplePlayerStepExecutor {
                    sample,
                    region,
                    mode,
                    interpolation,
                    position,
                    active,
                }),
                PerModuleState::SampleMapPlayer {
                    zones,
                    selection_mode,
                    reject_new_while_active,
                    round_robin_counters,
                    initial_seed,
                    rng_state,
                    selected_zone,
                    selected_pitch_ratio,
                    position,
                    active,
                } => Box::new(SampleMapPlayerStepExecutor {
                    zones,
                    selection_mode,
                    reject_new_while_active,
                    round_robin_counters,
                    initial_seed,
                    rng_state,
                    selected_zone,
                    selected_pitch_ratio,
                    position,
                    active,
                    queued_choices: Vec::with_capacity(plan.event_queues.queue_capacity),
                }),
                PerModuleState::NoteToRate { rate } => Box::new(NoteToRateStepExecutor { rate }),
                PerModuleState::EventFilter { note } => Box::new(EventFilterStepExecutor { note }),
                PerModuleState::EnvelopeFollower { detector, mode } => {
                    Box::new(EnvelopeFollowerStepExecutor { detector, mode })
                }
                PerModuleState::CurveMapper { mapper } => {
                    Box::new(CurveMapperStepExecutor { mapper })
                }
                PerModuleState::Filter {
                    filters,
                    sample_rate,
                } => Box::new(FilterStepExecutor {
                    filters,
                    sample_rate,
                }),
                PerModuleState::CompensationDelay { samples, positions } => {
                    Box::new(CompensationDelayStepExecutor { samples, positions })
                }
                PerModuleState::Convolution { processors } => {
                    Box::new(ConvolutionStepExecutor { processors })
                }
                PerModuleState::FrequencySplitter {
                    filters,
                    sample_rate,
                } => Box::new(FrequencySplitterStepExecutor {
                    filters,
                    sample_rate,
                }),
                PerModuleState::Echo { processor, .. } => Box::new(EchoStepExecutor { processor }),
                PerModuleState::Reverb { processor, .. } => {
                    Box::new(ReverbStepExecutor { processor })
                }
                PerModuleState::Adsr {
                    level,
                    gate_active,
                    release_start_frame,
                    release_start_level,
                    sample_rate,
                } => Box::new(AdsrStepExecutor {
                    level,
                    gate_active,
                    release_start_frame,
                    release_start_level,
                    sample_rate,
                }),
                PerModuleState::NoteToControl {
                    gate_active,
                    current_note,
                    current_velocity,
                    current_frequency,
                    current_pitch_ratio,
                    current_slide,
                } => Box::new(NoteToControlStepExecutor {
                    gate_active,
                    current_note,
                    current_velocity,
                    current_frequency,
                    current_pitch_ratio,
                    current_slide,
                }),
                PerModuleState::Decay {
                    level,
                    triggered,
                    elapsed_frames,
                    sample_rate,
                    curve,
                } => Box::new(DecayStepExecutor {
                    level,
                    triggered,
                    elapsed_frames,
                    sample_rate,
                    curve,
                }),
                PerModuleState::Script { runtime, state, .. } => {
                    Box::new(ScriptStepExecutor { runtime, state })
                }
                PerModuleState::Vca
                | PerModuleState::ControlToAudio
                | PerModuleState::Poly
                | PerModuleState::VoiceIntrinsics
                | PerModuleState::MidiInput
                | PerModuleState::AudioMixer
                | PerModuleState::Saturator { .. }
                | PerModuleState::Impulse
                | PerModuleState::Multiply => Box::new(GenericStepExecutor(step.processor)),
                #[cfg(test)]
                PerModuleState::AudioOutput => Box::new(GenericStepExecutor(step.processor)),
            };
            executor
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

pub(super) fn execute_bound_prepared_step(
    context: &mut PreparedStepContext<'_>,
    step: &RenderStep,
    executor: &mut dyn PreparedStepExecutor,
) {
    route_prepared_event_edges(context.event_queues, step);
    clear_and_route_arena_inputs(context.arena, step, context.frames, context.compiled);
    executor.execute(context, step);
}

pub(super) fn capture_bound_feedback_delays(
    arena: &mut AudioArena,
    steps: &[RenderStep],
    feedback_step_indices: &[usize],
    executors: &mut [Box<dyn PreparedStepExecutor>],
    frames: usize,
    compiled: &CompiledPatch,
) {
    for &step_index in feedback_step_indices {
        let step = &steps[step_index];
        clear_and_route_arena_inputs(arena, step, frames, compiled);
        let context = ProcessContext::new(arena, &step.input_buffers, &step.output_buffers, frames);
        executors[step_index].capture(&context);
    }
}

/// The only module-kind dispatch for a kernel render step happens while its
/// render plan is built, before the callback can execute it.
pub(super) fn resolve_step_processor(kind: ModuleKind) -> PreparedStepProcessor {
    match kind {
        ModuleKind::MidiInput | ModuleKind::VoiceIntrinsics => execute_noop_step,
        #[cfg(test)]
        ModuleKind::AudioOutput => execute_noop_step,
        ModuleKind::AudioMixer | ModuleKind::ControlMixer => execute_mixer_step,
        ModuleKind::Noise => execute_noise_step,
        ModuleKind::Oscillator => execute_oscillator_step,
        ModuleKind::Lfo => execute_lfo_step,
        ModuleKind::Slew => execute_slew_step,
        ModuleKind::DynamicsProcessor => execute_dynamics_step,
        ModuleKind::Gain | ModuleKind::Multiply => execute_gain_step,
        ModuleKind::ControlToAudio => execute_control_to_audio_step,
        ModuleKind::CompensationDelay => execute_compensation_delay_step,
        ModuleKind::FeedbackDelay => execute_feedback_delay_step,
        ModuleKind::Convolution => execute_convolution_step,
        ModuleKind::SpectralProcessor => execute_spectral_step,
        ModuleKind::FrequencySplitter => execute_splitter_step,
        ModuleKind::EnvelopeFollower => execute_envelope_follower_step,
        ModuleKind::CurveMapper => execute_curve_mapper_step,
        ModuleKind::Filter => execute_filter_step,
        ModuleKind::Saturator => execute_saturator_step,
        ModuleKind::Echo => execute_echo_step,
        ModuleKind::Reverb => execute_reverb_step,
        ModuleKind::EventFilter => execute_event_filter_step,
        ModuleKind::Adsr => execute_adsr_step,
        ModuleKind::NoteToControl => execute_note_to_control_step,
        ModuleKind::Sampler => execute_sampler_step,
        ModuleKind::SamplePlayer => execute_sample_player_step,
        ModuleKind::SampleMapPlayer => execute_sample_map_player_step,
        ModuleKind::NoteToRate => execute_note_to_rate_step,
        ModuleKind::Impulse => execute_impulse_step,
        ModuleKind::Decay => execute_decay_step,
        ModuleKind::Script => execute_script_step,
        ModuleKind::Poly => execute_poly_step,
        ModuleKind::AudioDelayOneSample | ModuleKind::BlockDelay | ModuleKind::ControlDelay => {
            execute_unsupported_step
        }
    }
}

macro_rules! stateful_step_processor {
    ($name:ident, $processor:path) => {
        fn $name(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
            let mut process_context = ProcessContext::new(
                context.arena,
                &step.input_buffers,
                &step.output_buffers,
                context.frames,
            );
            $processor(&mut context.states[step.module_index], &mut process_context);
        }
    };
}

macro_rules! stateless_step_processor {
    ($name:ident, $processor:path) => {
        fn $name(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
            let mut process_context = ProcessContext::new(
                context.arena,
                &step.input_buffers,
                &step.output_buffers,
                context.frames,
            );
            $processor(&mut process_context);
        }
    };
}

stateful_step_processor!(execute_noise_step, arena_processing::process_noise);
stateful_step_processor!(
    execute_oscillator_step,
    arena_processing::process_oscillator
);
stateful_step_processor!(execute_lfo_step, arena_processing::process_lfo);
stateful_step_processor!(execute_slew_step, arena_processing::process_slew);
stateful_step_processor!(execute_dynamics_step, arena_processing::process_dynamics);
stateful_step_processor!(
    execute_compensation_delay_step,
    arena_processing::process_compensation_delay
);
stateful_step_processor!(
    execute_convolution_step,
    arena_processing::process_convolution
);
stateful_step_processor!(
    execute_spectral_step,
    arena_processing::process_spectral_processor
);
stateful_step_processor!(
    execute_splitter_step,
    arena_processing::process_frequency_splitter
);
stateful_step_processor!(
    execute_envelope_follower_step,
    arena_processing::process_envelope_follower
);
stateful_step_processor!(
    execute_curve_mapper_step,
    arena_processing::process_curve_mapper
);
stateful_step_processor!(execute_filter_step, arena_processing::process_filter);
stateful_step_processor!(execute_echo_step, arena_processing::process_echo);
stateful_step_processor!(execute_reverb_step, arena_processing::process_reverb);
stateless_step_processor!(execute_mixer_step, arena_processing::process_audio_mixer);
stateless_step_processor!(execute_gain_step, arena_processing::process_gain);
stateless_step_processor!(
    execute_control_to_audio_step,
    arena_processing::process_control_to_audio
);
stateless_step_processor!(execute_saturator_step, arena_processing::process_saturator);

fn execute_noop_step(_: &mut PreparedStepContext<'_>, _: &RenderStep) {}

fn execute_unsupported_step(_: &mut PreparedStepContext<'_>, step: &RenderStep) {
    panic!("unsupported kernel render step: {:?}", step.module_kind);
}

fn execute_feedback_delay_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::emit_feedback_delay(&context.states[step.module_index], &mut process_context);
}

fn execute_event_filter_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let PerModuleState::EventFilter { note } = &context.states[step.module_index] else {
        unreachable!()
    };
    let edge = CompiledEventEdge {
        source: step.event_inputs[0],
        destination: step.event_outputs[0],
    };
    let _ = context.event_queues.route_filtered_event_edge(edge, *note);
}

fn execute_adsr_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_adsr(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
        context.block_start_frame,
    );
}

fn execute_note_to_control_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let (input, output) = context
        .event_queues
        .queue_pair(step.event_inputs[0], step.event_outputs[0])
        .expect("compiled note_to_control queues are distinct and valid");
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_note_to_control(
        &mut context.states[step.module_index],
        &mut process_context,
        input.events(),
        output,
    );
}

fn execute_sampler_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_sampler(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
    );
}

fn execute_sample_player_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let gate_events = context
        .event_queues
        .queue_ref(step.event_inputs[1].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_sample_player(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
        gate_events,
    );
}

fn execute_sample_map_player_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_sample_map_player(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
    );
}

fn execute_note_to_rate_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_note_to_rate(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
    );
}

fn execute_impulse_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_impulse(&mut process_context, events);
}

fn execute_decay_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = context
        .event_queues
        .queue_ref(step.event_inputs[0].0)
        .map_or(&[][..], |queue| queue.events());
    let mut process_context = ProcessContext::new(
        context.arena,
        &step.input_buffers,
        &step.output_buffers,
        context.frames,
    );
    arena_processing::process_decay(
        &mut context.states[step.module_index],
        &mut process_context,
        events,
    );
}

fn execute_script_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let PerModuleState::Script { runtime, state, .. } = &mut context.states[step.module_index]
    else {
        unreachable!()
    };
    process_offline_script_step(
        context.arena,
        runtime,
        state,
        context.event_queues,
        step,
        context.frames,
        context.compiled,
    );
}

fn execute_poly_step(context: &mut PreparedStepContext<'_>, step: &RenderStep) {
    let events = step
        .event_inputs
        .first()
        .and_then(|queue| context.event_queues.queue_ref(queue.0))
        .map_or(&[][..], |queue| queue.events());
    let region = context
        .poly_regions
        .get_mut(
            step.poly_region_index
                .expect("poly step has a region index"),
        )
        .expect("prepared poly region exists");
    region.route_note_events(events, context.frames);
    region.render_into(
        context.arena,
        &step.input_buffers,
        &step.event_inputs,
        context.event_queues,
        &step.output_buffers,
        context.frames,
    );
}

pub(super) fn clear_and_route_arena_inputs(
    arena: &mut AudioArena,
    step: &RenderStep,
    frames: usize,
    compiled: &CompiledPatch,
) {
    for &buffer in step.input_buffers.iter() {
        arena.clear(buffer, frames);
    }
    for default in step.control_defaults.iter() {
        let value = compiled
            .parameter_slot_value(default.slot.index())
            .expect("render-plan control slot was compiled");
        arena.fill(default.buffer, frames, value);
    }
    for &edge in step.incoming_edges.iter() {
        arena.add_edge(edge, frames);
    }
}

fn route_prepared_event_edges(queues: &mut PreparedEventQueues, step: &RenderStep) {
    for edge in step.incoming_event_edges.iter().copied() {
        let _ = queues.route_event_edge(edge);
    }
}

fn render_plan_supports_root_buses(
    render_plan: &RenderPlan,
    allow_scripts: bool,
    max_voices: usize,
) -> bool {
    if cfg!(test) && max_voices > 1 {
        return false;
    }
    render_plan
        .global_steps
        .iter()
        .all(|step| match step.module_kind {
            ModuleKind::MidiInput => {
                step.input_buffers.is_empty()
                    && step.output_buffers.is_empty()
                    && step.event_outputs.len() == 1
            }
            ModuleKind::EventFilter => {
                step.input_buffers.is_empty()
                    && step.output_buffers.is_empty()
                    && step.event_inputs.len() == 1
                    && step.event_outputs.len() == 1
            }
            ModuleKind::Adsr => {
                step.input_buffers.len() == 4
                    && step.output_buffers.len() == 1
                    && step.event_inputs.len() == 1
            }
            ModuleKind::NoteToControl => {
                step.input_buffers.is_empty()
                    && step.output_buffers.len() == 4
                    && step.event_inputs.len() == 1
                    && step.event_outputs.len() == 1
            }
            ModuleKind::Sampler => {
                step.input_buffers.len() == 5
                    && !step.output_buffers.is_empty()
                    && step.event_inputs.len() == 1
            }
            ModuleKind::SamplePlayer => {
                step.input_buffers.len() == 4
                    && !step.output_buffers.is_empty()
                    && step.event_inputs.len() == 2
            }
            ModuleKind::SampleMapPlayer => {
                step.input_buffers.len() == 5
                    && !step.output_buffers.is_empty()
                    && step.event_inputs.len() == 1
            }
            ModuleKind::NoteToRate => {
                step.input_buffers.is_empty()
                    && step.output_buffers.len() == 1
                    && step.event_inputs.len() == 1
            }
            ModuleKind::Impulse => {
                step.input_buffers.is_empty()
                    && step.output_buffers.len() == 1
                    && step.event_inputs.len() == 1
            }
            ModuleKind::Script => allow_scripts,
            _ => is_channel_arena_supported(step),
        })
}

#[cfg(test)]
fn is_mono_global_arena_supported(step: &RenderStep) -> bool {
    is_channel_arena_supported(step)
}

pub(super) fn is_channel_arena_supported(step: &RenderStep) -> bool {
    match step.module_kind {
        #[cfg(test)]
        ModuleKind::AudioOutput => step.input_buffers.len() >= 2,
        ModuleKind::Poly => true,
        ModuleKind::VoiceIntrinsics => {
            step.input_buffers.is_empty() && step.output_buffers.len() == 2
        }
        ModuleKind::AudioMixer | ModuleKind::ControlMixer => {
            step.input_buffers.len() == step.output_buffers.len()
        }
        ModuleKind::Noise => step.input_buffers.is_empty() && !step.output_buffers.is_empty(),
        ModuleKind::Oscillator => step.input_buffers.len() <= 1 && !step.output_buffers.is_empty(),
        ModuleKind::Lfo => step.input_buffers.len() == 1 && step.output_buffers.len() == 1,
        ModuleKind::Slew => step.input_buffers.len() == 3 && step.output_buffers.len() == 1,
        ModuleKind::DynamicsProcessor => step.input_buffers.len() == step.output_buffers.len() + 10,
        ModuleKind::Gain => step.input_buffers.len() == step.output_buffers.len() + 1,
        ModuleKind::Multiply => step.input_buffers.len() == step.output_buffers.len() * 2,
        ModuleKind::EnvelopeFollower => {
            step.input_buffers.len() == 6 && step.output_buffers.len() == 1
        }
        ModuleKind::CurveMapper => step.input_buffers.len() == 5 && step.output_buffers.len() == 1,
        ModuleKind::Filter => step.input_buffers.len() == step.output_buffers.len() + 3,
        ModuleKind::Saturator => step.input_buffers.len() == step.output_buffers.len() + 3,
        ModuleKind::ControlToAudio | ModuleKind::CompensationDelay => {
            step.input_buffers.len() == step.output_buffers.len()
        }
        ModuleKind::FeedbackDelay => {
            step.input_buffers.len() == step.output_buffers.len() && step.output_buffers.len() >= 2
        }
        ModuleKind::Convolution => step.input_buffers.len() == step.output_buffers.len() + 1,
        ModuleKind::SpectralProcessor => {
            step.input_buffers.len() == 3 && step.output_buffers.len() == 1
        }
        ModuleKind::FrequencySplitter => {
            step.output_buffers.len() % 3 == 0
                && step.input_buffers.len() == step.output_buffers.len() / 3 + 1
        }
        ModuleKind::Echo => {
            matches!(step.output_buffers.len(), 1 | 2)
                && step.input_buffers.len() == step.output_buffers.len() + 8
        }
        ModuleKind::Reverb => {
            matches!(step.output_buffers.len(), 1 | 2)
                && step.input_buffers.len() == step.output_buffers.len() + 8
        }
        _ => false,
    }
}

#[cfg(test)]
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

#[cfg(test)]
mod prepared_dispatch_tests {
    use super::*;
    use crate::kernel::document::load_kernel_patch_str;
    use crate::patch::RenderSettings;

    #[test]
    fn prepared_processor_executes_without_rechecking_module_kind() {
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: source.out }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: 0.25 } }\nconnections: []\n",
        )
        .expect("constant patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 8,
        };
        let prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
            .expect("constant patch prepares");
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            8,
        );
        assert_eq!(runtime.render_plan.global_steps.len(), 1);
        assert_eq!(
            runtime.render_plan.global_steps[0].module_kind,
            ModuleKind::ControlToAudio
        );

        // After plan construction, the kind is metadata: the prepared callable
        // must remain the render path's sole processor selection.
        runtime.render_plan.global_steps[0].module_kind = ModuleKind::AudioDelayOneSample;
        let mut output = vec![vec![vec![0.0; 8]]];
        assert_eq!(runtime.render_root_outputs(&mut output), 8);
        assert_eq!(output[0][0], [0.25; 8]);
    }

    #[test]
    fn prepared_poly_child_executes_without_rechecking_module_kind() {
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voices.audio }\nmodule_definitions:\n  - type: hit_voice\n    ports:\n      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: hit.audio }\n    modules:\n      - { id: hit, type: impulse }\n    connections:\n      - { from: voice.gate, to: hit.trigger }\nmodules:\n  - { id: midi, type: midi_input }\n  - { id: voices, type: poly, static: { definition: hit_voice, max_voices: 1, allocation: reject-new } }\nconnections:\n  - { from: midi.events, to: voices.notes }\n",
        )
        .expect("poly impulse patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 4,
            duration_frames: 4,
        };
        let prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
            .expect("poly impulse patch prepares");
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            4,
        );
        runtime.prepared_poly_runtime_regions[0]
            .replace_child_step_kind_for_test(ModuleKind::Impulse, ModuleKind::AudioDelayOneSample);
        runtime.note_on(60, 100);
        let mut output = vec![vec![vec![0.0; 4]]];
        assert_eq!(runtime.render_root_outputs(&mut output), 4);
        assert_eq!(output[0][0], [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn prepared_stateful_root_executes_without_rechecking_module_kind() {
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: delay.audio_out }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: -0.25 } }\n  - { id: delay, type: feedback_delay, static: { delay_samples: 8 } }\nconnections:\n  - { from: source.out, to: delay.audio_in }\n",
        )
        .expect("delayed constant patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 16,
        };
        let prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
            .expect("delayed constant patch prepares");
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            8,
        );
        let delay = runtime
            .render_plan
            .global_steps
            .iter_mut()
            .find(|step| step.module_kind == ModuleKind::FeedbackDelay)
            .expect("prepared delay step");
        delay.module_kind = ModuleKind::AudioDelayOneSample;
        let mut output = vec![vec![vec![0.0; 8]]];
        assert_eq!(runtime.render_root_outputs(&mut output), 8);
        assert_eq!(output[0][0], [0.0; 8]);
        assert_eq!(runtime.render_root_outputs(&mut output), 8);
        assert_eq!(output[0][0], [-0.25; 8]);
    }

    #[test]
    fn prepared_stateful_poly_child_executes_without_rechecking_module_kind() {
        let patch = load_kernel_patch_str(
            r#"
ports:
  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voices.audio }
module_definitions:
  - type: echo_voice
    ports:
      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: sum.mix }
    modules:
      - { id: hit, type: impulse }
      - { id: sum, type: audio_mixer }
      - { id: feedback, type: feedback_delay, static: { delay_samples: 4 } }
      - { id: half, type: gain, defaults: { gain: 0.5 } }
    connections:
      - { from: voice.gate, to: hit.trigger }
      - { from: hit.audio, to: sum.inputs }
      - { from: sum.mix, to: feedback.audio_in }
      - { from: feedback.audio_out, to: half.audio_in }
      - { from: half.audio_out, to: sum.inputs }
modules:
  - { id: midi, type: midi_input }
  - { id: voices, type: poly, static: { definition: echo_voice, max_voices: 1, allocation: reject-new } }
connections:
  - { from: midi.events, to: voices.notes }
"#,
        )
        .expect("poly feedback patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 4,
            duration_frames: 12,
        };
        let prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
            .expect("poly feedback patch prepares");
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            4,
        );
        runtime.prepared_poly_runtime_regions[0].replace_child_step_kind_for_test(
            ModuleKind::FeedbackDelay,
            ModuleKind::AudioDelayOneSample,
        );
        runtime.note_on(60, 100);
        let mut output = vec![vec![vec![0.0; 4]]];
        let mut samples = Vec::new();
        for _ in 0..3 {
            assert_eq!(runtime.render_root_outputs(&mut output), 4);
            samples.extend_from_slice(&output[0][0]);
        }
        assert_eq!(
            samples,
            [1.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn prepared_splitter_processor_reconstructs_a_signed_constant() {
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: low, direction: output, signal: audio, channels: 1, maps_from: splitter.low }\n  - { name: mid, direction: output, signal: audio, channels: 1, maps_from: splitter.mid }\n  - { name: high, direction: output, signal: audio, channels: 1, maps_from: splitter.high }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: -0.25 } }\n  - { id: splitter, type: frequency_splitter }\nconnections:\n  - { from: source.out, to: splitter.audio_in }\n",
        )
        .expect("splitter patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 128,
            duration_frames: 12_800,
        };
        let prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
            .expect("splitter patch prepares");
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            128,
        );
        let mut outputs = vec![vec![vec![0.0; 128]]; 3];
        for _ in 0..100 {
            assert_eq!(runtime.render_root_outputs(&mut outputs), 128);
        }
        let reconstructed = outputs.iter().map(|bus| bus[0][127]).sum::<f32>();
        assert!((reconstructed + 0.25).abs() < 0.005, "{reconstructed}");
    }
}
