use std::collections::HashMap;

use crate::compiled_patch::CompiledPatch;
use crate::core::{BlockScheduler, TimedInputEvent};
use crate::graph::Graph;
use crate::patch::{RenderSettings, VoiceAllocation};
use crate::preparation::PreparedKernelInstrument;
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;
use crate::voice_allocator::VoiceAllocator;

use super::RealtimeGraphProcessor;
use super::block::{process_block_compiled, process_block_compiled_polyphonic};
use super::outputs::{BlockEvent, ModuleOutputs};
use super::polyphony::build_polyphonic_states_from_compiled;
use super::state::PerModuleState;

/// Render each named root audio/control output as planar channels, in root-port order.
pub fn render_kernel_offline_named(
    prepared: &PreparedKernelInstrument,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
) -> Result<Vec<(String, Vec<Vec<f32>>)>, &'static str> {
    let compiled = prepared.compiled_patch();
    let settings = compiled.render_settings();
    if settings.block_size_frames == 0 {
        return Err("offline render block size must be positive");
    }
    let mut processor =
        RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            compiled.clone(),
            settings.sample_rate_hz as f32,
            sampler_assets,
            &VoiceAllocation::default(),
            settings.block_size_frames as usize,
        );
    if !processor.can_render_root_buses() {
        return Err("prepared graph cannot render named root buses");
    }
    let output_ports = compiled.root_bus_plan().outputs();
    if output_ports.iter().any(|port| port.span().is_none()) {
        return Err("offline render does not support event root outputs");
    }
    let mut rendered: Vec<_> = output_ports
        .iter()
        .map(|port| {
            (
                port.name().to_owned(),
                vec![Vec::with_capacity(settings.duration_frames as usize); port.channel_count()],
            )
        })
        .collect();
    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);
    for block in scheduler {
        let frames = block.frame_count() as usize;
        for event in block.input_events() {
            match event.event() {
                ScriptEvent::NoteOn { note, velocity } => {
                    processor.note_on_at(*note, *velocity, event.frame_offset());
                }
                ScriptEvent::NoteOff { note } => {
                    processor.note_off_at(*note, event.frame_offset());
                }
            }
        }
        let mut buffers: Vec<_> = output_ports
            .iter()
            .map(|port| vec![vec![0.0; frames]; port.channel_count()])
            .collect();
        if processor.render_root_buses(&[], &mut buffers) != frames {
            return Err("named root bus render failed");
        }
        for ((_, destination), bus) in rendered.iter_mut().zip(buffers) {
            for (channel, samples) in destination.iter_mut().zip(bus) {
                channel.extend(samples);
            }
        }
    }
    Ok(rendered)
}

pub fn render_offline_compiled(
    compiled: &CompiledPatch,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
) -> (Vec<f32>, Vec<f32>) {
    let settings = compiled.render_settings();
    let sample_rate = settings.sample_rate_hz as f32;

    let midi_idx = compiled.midi_input_index();
    let out_idx = compiled.audio_output_index();

    let mut states: Vec<PerModuleState> = compiled
        .nodes()
        .iter()
        .map(|node| PerModuleState::new_compiled(node, sample_rate, sampler_assets))
        .collect();

    let scheduler = BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
        .with_input_events(events);

    let mut left_buf = Vec::new();
    let mut right_buf = Vec::new();
    let mut all_outputs: HashMap<usize, ModuleOutputs> =
        HashMap::with_capacity(compiled.nodes().len());

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
            &external_events,
            &mut left_buf,
            &mut right_buf,
            &mut all_outputs,
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
    let compiled = crate::compiled_patch::compile(graph, settings)
        .expect("validated graph should compile for offline rendering");

    let max_voices = voice_allocation.max_voices.max(1) as usize;
    let mut states =
        build_polyphonic_states_from_compiled(&compiled, sample_rate, sampler_assets, max_voices);
    let mut allocator = VoiceAllocator::new(
        voice_allocation.max_voices,
        voice_allocation.stealing.clone(),
    );

    let midi_idx = compiled.midi_input_index();
    let out_idx = compiled.audio_output_index();

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

        process_block_compiled_polyphonic(
            &compiled,
            &mut states,
            &mut allocator,
            midi_idx,
            out_idx,
            block.start_frame(),
            frames,
            &external_events,
            &mut left_buf,
            &mut right_buf,
        );
    }

    (left_buf, right_buf)
}

#[cfg(test)]
mod named_bus_tests {
    use super::*;
    use crate::graph::SignalType;
    use crate::graph::builtin_ports;
    use crate::kernel::builtins::{CHANNELS_PARAM, builtin_registry};
    use crate::kernel::{GraphDefinition, Node, NodeId, Port, PortRef, StaticArg, StaticValue};
    use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses};

    #[test]
    fn kernel_offline_render_preserves_every_named_output_across_blocks() {
        let root = GraphDefinition::new("named_outputs")
            .with_port(
                Port::output("master", SignalType::Audio, 2).maps_from(PortRef::new(
                    NodeId::new("master_source"),
                    builtin_ports::OUT,
                )),
            )
            .with_port(
                Port::output("cue", SignalType::Audio, 1)
                    .maps_from(PortRef::new(NodeId::new("cue_source"), builtin_ports::OUT)),
            )
            .with_node(
                Node::new(
                    NodeId::new("master_source"),
                    crate::builtins::module_types::CONTROL_TO_AUDIO,
                )
                .with_static_arg(CHANNELS_PARAM, StaticArg::Literal(StaticValue::Int(2)))
                .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(
                Node::new(
                    NodeId::new("cue_source"),
                    crate::builtins::module_types::CONTROL_TO_AUDIO,
                )
                .with_default_override(builtin_ports::IN, -0.5),
            );
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 12,
        };
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry(),
            &settings,
            &HostBuses::new()
                .with_output("master", 2)
                .with_output("cue", 1),
        )
        .expect("named outputs prepare");

        let rendered =
            render_kernel_offline_named(&prepared, vec![], &PreparedSamplerAssets::empty())
                .expect("named outputs render");
        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].0, "master");
        assert_eq!(rendered[0].1, vec![vec![0.25; 12], vec![0.0; 12]]);
        assert_eq!(rendered[1].0, "cue");
        assert_eq!(rendered[1].1, vec![vec![-0.5; 12]]);
    }
}
