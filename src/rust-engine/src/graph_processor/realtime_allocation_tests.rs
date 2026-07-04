use super::*;
use crate::builtins::module_types;
use crate::graph::{builtin_ports, Cable, Graph, ModuleId, ModuleNode, PortRef, SignalType};
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use std::collections::BTreeMap;

fn oscillator_output_graph() -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    )
}

fn sampler_output_graph() -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("sampler"), module_types::SAMPLER)
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    )
}

fn sampler_assets() -> PreparedSamplerAssets {
    PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
        "sampler".to_string(),
        LoadedSample::new(48_000, vec![0.25; 128]),
    )]))
}

#[test]
fn mono_realtime_render_reuses_prepared_capacity_for_repeated_prepared_size_blocks() {
    let graph = oscillator_output_graph();
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    );
    let mut left = vec![0.0; 64];
    let mut right = vec![0.0; 64];

    assert_eq!(processor.render(&mut left, &mut right), 64);
    let top_level_capacity = processor.top_level_scratch_capacities();
    let module_output_capacity = processor.module_output_scratch_capacity();
    let pending_event_capacity = processor.pending_event_capacity();
    let voice_count = processor.prepared_voice_count();

    for _ in 0..8 {
        assert_eq!(processor.render(&mut left, &mut right), 64);
        assert_eq!(processor.top_level_scratch_capacities(), top_level_capacity);
        assert_eq!(processor.module_output_scratch_capacity(), module_output_capacity);
        assert_eq!(processor.pending_event_capacity(), pending_event_capacity);
        assert_eq!(processor.prepared_voice_count(), voice_count);
    }
}

#[test]
fn event_driven_realtime_render_reuses_prepared_capacity_for_repeated_prepared_size_blocks() {
    let graph = sampler_output_graph();
    let assets = sampler_assets();
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &assets,
        &VoiceAllocation::default(),
        16,
    );
    let mut left = vec![0.0; 16];
    let mut right = vec![0.0; 16];

    for note in 48..64 {
        processor.note_on(note, 100);
    }
    assert_eq!(processor.render(&mut left, &mut right), 16);
    let top_level_capacity = processor.top_level_scratch_capacities();
    let module_output_capacity = processor.module_output_scratch_capacity();
    let pending_event_capacity = processor.pending_event_capacity();
    let voice_count = processor.prepared_voice_count();

    for note in 48..64 {
        processor.note_on(note, 100);
    }
    assert_eq!(processor.render(&mut left, &mut right), 16);

    assert_eq!(processor.top_level_scratch_capacities(), top_level_capacity);
    assert_eq!(processor.module_output_scratch_capacity(), module_output_capacity);
    assert_eq!(processor.pending_event_capacity(), pending_event_capacity);
    assert_eq!(processor.prepared_voice_count(), voice_count);
}
