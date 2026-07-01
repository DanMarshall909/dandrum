use std::collections::HashMap;

use crate::compiled_patch::{self, CompiledPatch};
use crate::graph::{ExecutionScope, Graph};
use crate::patch::VoiceAllocation;
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;
use crate::voice_allocator::VoiceAllocator;

use super::block::{process_block, process_block_polyphonic};
use super::outputs::{BlockEvent, ModuleOutputs};
use super::polyphony::build_polyphonic_states;
use super::routing::{build_routing_from_compiled, Routing};
use super::state::PerModuleState;

pub struct RealtimeGraphProcessor {
    pub(super) graph: Graph,
    compiled: CompiledPatch,
    routing: Routing,
    topo_order: Vec<usize>,
    states: Vec<Vec<PerModuleState>>,
    midi_idx: Option<usize>,
    out_idx: Option<usize>,
    current_frame: u64,
    pending_events: Vec<ScriptEvent>,
    allocator: VoiceAllocator,
    prepared_max_block_size: usize,
    last_render_chunk_count: usize,
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
        let routing = build_routing_from_compiled(&compiled);
        let topo_order = compiled.topological_order().to_vec();
        let midi_idx = compiled.midi_input_index();
        let out_idx = compiled.audio_output_index();
        let max_voices = voice_allocation.max_voices.max(1) as usize;
        let states = build_polyphonic_states(&graph, sample_rate, sampler_assets, max_voices);
        let allocator = VoiceAllocator::new(
            voice_allocation.max_voices,
            voice_allocation.stealing.clone(),
        );

        let prepared_max_block_size = prepared_max_block_size.max(1);
        let module_count = graph.modules().len();

        Self {
            graph,
            compiled,
            routing,
            topo_order,
            states,
            midi_idx,
            out_idx,
            current_frame: 0,
            pending_events: Vec::new(),
            allocator,
            prepared_max_block_size,
            last_render_chunk_count: 0,
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

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.pending_events
            .push(ScriptEvent::NoteOn { note, velocity });
    }

    pub fn note_off(&mut self, note: u8) {
        self.pending_events.push(ScriptEvent::NoteOff { note });
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let frames = left.len().min(right.len());
        if frames == 0 {
            self.last_render_chunk_count = 0;
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

        let events: Vec<BlockEvent> = std::mem::take(&mut self.pending_events)
            .into_iter()
            .map(|event| BlockEvent {
                frame_offset: 0,
                event,
            })
            .collect();

        if self.allocator.max_voices() > 1
            || self
                .graph
                .modules()
                .iter()
                .any(|m| m.execution_scope() == ExecutionScope::Voice)
        {
            self.scratch_left.clear();
            self.scratch_right.clear();

            process_block_polyphonic(
                &self.graph,
                &self.routing,
                &self.topo_order,
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

            process_block(
                &self.graph,
                &self.routing,
                &self.topo_order,
                &mut self.states[0],
                self.midi_idx,
                self.out_idx,
                block_start,
                frames,
                events,
                &mut self.scratch_outputs,
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
        }

        frames
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
