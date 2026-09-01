use super::*;
use crate::builtins::build_definition;
use crate::builtins::module_types;
use crate::graph::{Cable, Graph, ModuleId, ModuleNode, PortRef, SignalType, builtin_ports};
use crate::kernel::builtins::builtin_registry;
use crate::kernel::{
    Connection as KernelConnection, GraphDefinition, Node, NodeId, Port as KernelPort,
    PortRef as KernelPortRef, StaticArg, StaticValue,
};
use crate::patch::RenderSettings;
use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses};
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::test_allocator::count_current_thread_allocations;
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

fn gain_module_node(id: &str) -> ModuleNode {
    let def = build_definition();
    let mut node = ModuleNode::new(ModuleId::new(id), def.module_type());
    for port in def.inputs() {
        node = node.with_input(port.name(), port.signal_type());
    }
    for port in def.outputs() {
        node = node.with_output(port.name(), port.signal_type());
    }
    node
}

fn oscillator_gain_output_graph() -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            gain_module_node("gain"),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("gain"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("gain"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("gain"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    )
}

fn expanded_non_event_arena_graph() -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("follower"), module_types::ENVELOPE_FOLLOWER)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::ATTACK, SignalType::Control)
                .with_input(builtin_ports::RELEASE, SignalType::Control)
                .with_input(builtin_ports::AMOUNT, SignalType::Control)
                .with_input(builtin_ports::OFFSET, SignalType::Control)
                .with_input(builtin_ports::INVERT, SignalType::Control)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("mapper"), module_types::CURVE_MAPPER)
                .with_input(builtin_ports::VALUE, SignalType::Control)
                .with_input(builtin_ports::AMOUNT, SignalType::Control)
                .with_input(builtin_ports::BIAS, SignalType::Control)
                .with_input(builtin_ports::SCALE, SignalType::Control)
                .with_input(builtin_ports::OFFSET, SignalType::Control)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("filter"), module_types::FILTER)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::CUTOFF, SignalType::Control)
                .with_input(builtin_ports::RESONANCE, SignalType::Control)
                .with_input(builtin_ports::GAIN, SignalType::Control)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("noise"), module_types::NOISE)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("multiply"), module_types::MULTIPLY)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::GAIN, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), module_types::AUDIO_MIXER)
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("follower"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("follower"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("filter"), builtin_ports::CUTOFF),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("multiply"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("noise"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("multiply"), builtin_ports::GAIN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("multiply"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    )
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert!((actual - expected).abs() < 0.0001);
    }
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
        assert_eq!(
            processor.module_output_scratch_capacity(),
            module_output_capacity
        );
        assert_eq!(processor.pending_event_capacity(), pending_event_capacity);
        assert_eq!(processor.prepared_voice_count(), voice_count);
    }
}

#[test]
fn mono_realtime_render_allocation_count_is_zero_for_minimal_arena_path() {
    let graph = oscillator_output_graph();
    let voice_allocation = VoiceAllocation {
        max_voices: 1,
        ..VoiceAllocation::default()
    };
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &voice_allocation,
        64,
    );
    let mut left = vec![0.0; 64];
    let mut right = vec![0.0; 64];

    assert_eq!(processor.render(&mut left, &mut right), 64);

    let allocation_count = count_current_thread_allocations(|| {
        assert_eq!(processor.render(&mut left, &mut right), 64);
    });

    assert!(processor.last_render_used_arena());
    assert_eq!(allocation_count, 0);
}

#[test]
fn oscillator_gain_output_realtime_render_uses_arena_path() {
    let graph = oscillator_gain_output_graph();
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 128,
    };
    let (expected_left, expected_right) = render_offline(&graph, &settings, Vec::new());
    let voice_allocation = VoiceAllocation {
        max_voices: 1,
        ..VoiceAllocation::default()
    };
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &voice_allocation,
        64,
    );
    let mut left = vec![0.0; 64];
    let mut right = vec![0.0; 64];

    assert_eq!(processor.render(&mut left, &mut right), 64);
    assert!(processor.last_render_used_arena());
    assert_eq!(left, expected_left[..64]);
    assert_eq!(right, expected_right[..64]);

    let allocation_count = count_current_thread_allocations(|| {
        assert_eq!(processor.render(&mut left, &mut right), 64);
    });
    assert_eq!(allocation_count, 0);
    assert_eq!(left, expected_left[64..]);
    assert_eq!(right, expected_right[64..]);
}

#[test]
fn expanded_non_event_realtime_render_uses_arena_path_without_allocations() {
    let graph = expanded_non_event_arena_graph();
    let voice_allocation = VoiceAllocation {
        max_voices: 1,
        ..VoiceAllocation::default()
    };
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 128,
    };
    let (expected_left, expected_right) = render_offline(&graph, &settings, Vec::new());
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &voice_allocation,
        64,
    );
    let mut left = vec![0.0; 64];
    let mut right = vec![0.0; 64];

    assert_eq!(processor.render(&mut left, &mut right), 64);
    assert!(processor.last_render_used_arena());
    assert_close(&left, &expected_left[..64]);
    assert_close(&right, &expected_right[..64]);

    let allocation_count = count_current_thread_allocations(|| {
        assert_eq!(processor.render(&mut left, &mut right), 64);
    });

    assert!(processor.last_render_used_arena());
    assert_eq!(allocation_count, 0);
    assert_close(&left, &expected_left[64..]);
    assert_close(&right, &expected_right[64..]);
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
    assert_eq!(
        processor.module_output_scratch_capacity(),
        module_output_capacity
    );
    assert_eq!(processor.pending_event_capacity(), pending_event_capacity);
    assert_eq!(processor.prepared_voice_count(), voice_count);
}

#[test]
fn prepared_poly_note_routing_performs_no_realtime_allocations() {
    let voice = GraphDefinition::new("allocation_voice")
        .with_port(
            KernelPort::output("level", SignalType::Control, 1).maps_from(KernelPortRef::new(
                NodeId::new("envelope"),
                builtin_ports::VALUE,
            )),
        )
        .with_node(Node::new(NodeId::new("envelope"), module_types::ADSR))
        .with_connection(KernelConnection::new(
            KernelPortRef::new(
                NodeId::new(crate::kernel::VOICE_INTRINSIC_NODE),
                crate::kernel::VOICE_GATE_OUTPUT,
            ),
            KernelPortRef::new(NodeId::new("envelope"), builtin_ports::GATE),
        ));
    let root = GraphDefinition::new("root")
        .with_port(
            KernelPort::output("level", SignalType::Control, 1)
                .maps_from(KernelPortRef::new(NodeId::new("voices"), "level")),
        )
        .with_node(
            Node::new(NodeId::new("voices"), crate::kernel::POLY_DEFINITION)
                .with_static_arg(
                    crate::kernel::POLY_WRAPPED_DEFINITION_PARAM,
                    StaticArg::Literal(StaticValue::String("allocation_voice".to_string())),
                )
                .with_static_arg(
                    crate::kernel::POLY_MAX_VOICES_PARAM,
                    StaticArg::Literal(StaticValue::Int(2)),
                )
                .with_static_arg(
                    crate::kernel::POLY_ALLOCATION_PARAM,
                    StaticArg::Literal(StaticValue::Enum(
                        crate::kernel::POLY_ALLOCATION_OLDEST_STEAL.to_string(),
                    )),
                ),
        );
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 64,
    };
    let prepared = prepare_kernel_graph_with_buses(
        &root,
        &builtin_registry().with_definition(voice),
        &settings,
        &HostBuses::new().with_output("level", 1),
    )
    .expect("fixed-capacity poly graph prepares");
    let mut processor = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        prepared.graph().clone(),
        prepared.compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    );

    processor.route_poly_note_event_for_test(
        "voices",
        crate::script::ScriptEvent::NoteOn {
            note: 60,
            velocity: 100,
        },
        0,
    );
    let allocation_count = count_current_thread_allocations(|| {
        for note in 61..=72 {
            processor.route_poly_note_event_for_test(
                "voices",
                crate::script::ScriptEvent::NoteOn {
                    note,
                    velocity: 100,
                },
                0,
            );
            processor.route_poly_note_event_for_test(
                "voices",
                crate::script::ScriptEvent::NoteOff { note },
                63,
            );
        }
    });

    assert_eq!(allocation_count, 0);
}
