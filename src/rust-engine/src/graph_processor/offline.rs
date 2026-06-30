use std::collections::HashMap;

use crate::compiled_patch::CompiledPatch;
use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::Graph;
use crate::patch::{RenderSettings, VoiceAllocation};
use crate::sample::PreparedSamplerAssets;
use crate::voice_allocator::VoiceAllocator;

use super::block::{process_block, process_block_compiled, process_block_polyphonic};
use super::outputs::{BlockEvent, ModuleOutputs};
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
        .map(|node| {
            let module_type = node.module_type.as_str();
            match module_type {
                "oscillator" => PerModuleState::Oscillator {
                    phase: 0.0,
                    sample_rate,
                },
                "adsr" => PerModuleState::Adsr {
                    level: 0.0,
                    gate_active: false,
                    release_start_frame: 0,
                    release_start_level: 0.0,
                    sample_rate,
                },
                "gain" => PerModuleState::Vca,
                "audio_output" => PerModuleState::AudioOutput,
                "midi_input" => PerModuleState::MidiInput,
                "note_to_rate" => PerModuleState::NoteToRate { rate: 1.0 },
                "audio_mixer" => PerModuleState::AudioMixer,
                "sampler" => PerModuleState::Sampler {
                    sample: sampler_assets.get(node.id.as_str()).cloned(),
                    position: 0.0,
                    active: false,
                },
                "echo" => PerModuleState::Echo {
                    processor: crate::echo::Echo::new(sample_rate as f64),
                    sample_rate: sample_rate as f64,
                },
                "reverb" => PerModuleState::Reverb {
                    processor: crate::reverb::Reverb::new(sample_rate as f64),
                    sample_rate: sample_rate as f64,
                },
                other => panic!("unknown module type in compiled render: {other}"),
            }
        })
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

    let mut left_buf = Vec::new();
    let mut right_buf = Vec::new();
    let mut all_outputs: HashMap<usize, ModuleOutputs> = HashMap::new();

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
            &mut all_outputs,
            &mut left_buf,
            &mut right_buf,
        );
    }

    (left_buf, right_buf)
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
