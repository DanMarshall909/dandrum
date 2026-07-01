use crate::compiled_patch::CompiledPatch;
use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::Graph;
use crate::patch::{RenderSettings, VoiceAllocation};
use crate::sample::PreparedSamplerAssets;
use crate::voice_allocator::VoiceAllocator;

use super::block::{process_block_compiled, process_block_polyphonic};
use super::outputs::BlockEvent;
use super::polyphony::build_polyphonic_states;
use super::routing::build_routing;
use super::state::PerModuleState;
use super::traversal::{find_audio_output, find_midi_input, topological_sort};

pub fn render_offline_compiled(
    compiled: &CompiledPatch,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
) -> (Vec<f32>, Vec<f32>) {
    let settings = compiled.render_settings();
    let sample_rate = settings.sample_rate_hz as f32;

    let midi_idx = compiled
        .nodes()
        .iter()
        .position(|n| n.module_type == "midi_input");
    let out_idx = compiled
        .nodes()
        .iter()
        .position(|n| n.module_type == "audio_output");

    let mut states: Vec<PerModuleState> = compiled
        .nodes()
        .iter()
        .map(|node| PerModuleState::new_compiled(node, sample_rate, sampler_assets))
        .collect();

    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);

    let mut left_buf = Vec::new();
    let mut right_buf = Vec::new();

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

        process_block_compiled(
            compiled,
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
    let compiled = crate::compiled_patch::compile(graph, settings)
        .expect("validated graph should compile for offline rendering");

    render_offline_compiled(&compiled, events, sampler_assets)
}

pub fn render_offline_polyphonic(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
    voice_allocation: &VoiceAllocation,
) -> (Vec<f32>, Vec<f32>) {
    render_offline_with_sampler_assets_polyphonic(
        graph,
        settings,
        events,
        &PreparedSamplerAssets::empty(),
        voice_allocation,
    )
}

pub fn render_offline_with_sampler_assets_polyphonic(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
    voice_allocation: &VoiceAllocation,
) -> (Vec<f32>, Vec<f32>) {
    let sample_rate = settings.sample_rate_hz as f32;
    let routing = build_routing(graph);
    let topo_order = topological_sort(graph);

    let max_voices = voice_allocation.max_voices.max(1) as usize;
    let mut states = build_polyphonic_states(graph, sample_rate, sampler_assets, max_voices);
    let mut allocator = VoiceAllocator::new(
        voice_allocation.max_voices,
        voice_allocation.stealing.clone(),
    );

    let midi_idx = find_midi_input(graph);
    let out_idx = find_audio_output(graph);

    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);

    let mut left_buf = Vec::new();
    let mut right_buf = Vec::new();

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

        process_block_polyphonic(
            graph,
            &routing,
            &topo_order,
            &mut states,
            &mut allocator,
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
