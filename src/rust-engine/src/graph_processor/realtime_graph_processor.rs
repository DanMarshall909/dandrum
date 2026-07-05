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
use super::block::{process_block_compiled, process_block_compiled_polyphonic};
use super::event_queue::BoundedEventQueue;
use super::outputs::ModuleOutputs;
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
    events_buffer: Box<[super::outputs::BlockEvent]>,
    allocator: VoiceAllocator,
    render_plan: RenderPlan,
    audio_arena: AudioArena,
    prepared_max_block_size: usize,
    last_render_chunk_count: usize,
    last_render_used_arena: bool,
    scratch_left: Vec<f32>,
    scratch_right: Vec<f32>,
    scratch_outputs: HashMap<usize, ModuleOutputs>,
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
        let module_count = graph.modules().len();
        let render_plan = RenderPlan::from_compiled_patch(
            &compiled,
            prepared_max_block_size,
            max_voices,
            prepared_max_block_size,
        );
        let audio_arena = AudioArena::new(render_plan.audio_buffers);

        let events_buffer = vec![
            super::outputs::BlockEvent {
                frame_offset: 0,
                event: crate::script::ScriptEvent::NoteOn {
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
            events_buffer,
            allocator,
            render_plan,
            audio_arena,
            prepared_max_block_size,
            last_render_chunk_count: 0,
            last_render_used_arena: false,
            scratch_left: Vec::with_capacity(prepared_max_block_size),
            scratch_right: Vec::with_capacity(prepared_max_block_size),
            scratch_outputs: HashMap::with_capacity(module_count),
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
        self.scratch_outputs.capacity()
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
        let _ = self
            .pending_events
            .push(ScriptEvent::NoteOn { note, velocity });
    }

    pub fn note_off(&mut self, note: u8) {
        let _ = self.pending_events.push(ScriptEvent::NoteOff { note });
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

        self.last_render_used_arena = false;

        let event_count = self
            .pending_events
            .drain_into_buffer(&mut *self.events_buffer);
        let events = &self.events_buffer[..event_count];

        if self.allocator.max_voices() > 1 || !self.compiled.voice_node_indices().is_empty() {
            self.scratch_left.clear();
            self.scratch_right.clear();

            process_block_compiled_polyphonic(
                &self.compiled,
                &mut self.states,
                &mut self.allocator,
                self.midi_idx,
                self.out_idx,
                block_start,
                frames,
                events,
                &mut self.scratch_left,
                &mut self.scratch_right,
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
                &mut self.scratch_outputs,
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
        }

        let output = self
            .render_plan
            .audio_output
            .expect("audio output was checked before arena render");
        self.audio_arena
            .copy_to_slices(output.left, output.right, frames, left, right);
        true
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
