use super::*;
use crate::builtins::build_definition;
use crate::builtins::module_types;
use crate::graph::{Cable, Graph, ModuleId, ModuleNode, PortRef, SignalType, builtin_ports};
use crate::kernel::builtins::builtin_registry;
use crate::kernel::document::load_kernel_patch_str;
use crate::kernel::{
    Connection as KernelConnection, GraphDefinition, Node, NodeId, Port as KernelPort,
    PortRef as KernelPortRef, StaticArg, StaticValue,
};
use crate::patch::RenderSettings;
use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses, prepare_kernel_patch};
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

fn prepared_constant_poly(allocation: &str, done_on_gate_event: bool) -> RealtimeGraphProcessor {
    let mut voice =
        GraphDefinition::new("constant_voice")
            .with_port(KernelPort::output("audio", SignalType::Audio, 1).maps_from(
                KernelPortRef::new(NodeId::new("constant"), builtin_ports::OUT),
            ))
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            );
    if done_on_gate_event {
        voice = voice.with_port(
            KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Event, 1).maps_from(
                KernelPortRef::new(
                    NodeId::new(crate::kernel::VOICE_INTRINSIC_NODE),
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
            ),
        );
    }
    let root = GraphDefinition::new("root")
        .with_port(
            KernelPort::output("left", SignalType::Audio, 1)
                .maps_from(KernelPortRef::new(NodeId::new("voices"), "audio")),
        )
        .with_port(
            KernelPort::output("right", SignalType::Audio, 1)
                .maps_from(KernelPortRef::new(NodeId::new("voices"), "audio")),
        )
        .with_node(
            Node::new(NodeId::new("voices"), crate::kernel::POLY_DEFINITION)
                .with_static_arg(
                    crate::kernel::POLY_WRAPPED_DEFINITION_PARAM,
                    StaticArg::Literal(StaticValue::String("constant_voice".to_string())),
                )
                .with_static_arg(
                    crate::kernel::POLY_MAX_VOICES_PARAM,
                    StaticArg::Literal(StaticValue::Int(2)),
                )
                .with_static_arg(
                    crate::kernel::POLY_ALLOCATION_PARAM,
                    StaticArg::Literal(StaticValue::Enum(allocation.to_string())),
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
        &HostBuses::new()
            .with_output("left", 1)
            .with_output("right", 1),
    )
    .expect("constant poly graph prepares");
    RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        prepared.graph().clone(),
        prepared.compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    )
}

#[test]
fn full_capacity_poly_activation_mix_and_done_retirement_do_not_allocate() {
    let mut processor = prepared_constant_poly(crate::kernel::POLY_ALLOCATION_REJECT_NEW, true);
    let mut outputs = vec![vec![vec![0.0; 64]]; 2];

    let allocation_count = count_current_thread_allocations(|| {
        processor.note_on(60, 100);
        processor.note_on(64, 100);
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
    });
    assert_eq!(allocation_count, 0);
    assert!(outputs[0][0].iter().all(|sample| *sample == 0.5));
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(
        processor.prepared_poly_runtime_regions()[0].active_voice_count(),
        0
    );

    let reuse_allocations = count_current_thread_allocations(|| {
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
        processor.note_on(67, 100);
        processor.note_on(72, 100);
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
    });
    assert_eq!(reuse_allocations, 0);
    assert!(outputs[0][0].iter().all(|sample| *sample == 0.5));
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(
        processor.prepared_poly_runtime_regions()[0].active_voice_count(),
        0
    );

    assert_eq!(processor.render_root_outputs(&mut outputs), 64);
    assert!(
        outputs
            .iter()
            .flatten()
            .flatten()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn poly_note_to_control_renders_and_releases_without_allocation() {
    let patch = load_kernel_patch_str(include_str!(
        "../../../../examples/patches/module-impulse-tone.yaml"
    ))
    .expect("impulse tone example loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 128,
    };
    let prepared = prepare_kernel_patch(&patch, &settings).expect("impulse tone prepares");
    let mut processor = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        prepared.graph().clone(),
        prepared.compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    );
    let mut outputs = vec![vec![vec![0.0; 64]; 2]];
    let mut note_was_audible = false;

    let allocation_count = count_current_thread_allocations(|| {
        processor.note_on(60, 100);
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
        note_was_audible = outputs[0][0].iter().any(|sample| sample.abs() > 0.001);
        processor.note_off(60);
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
    });
    assert_eq!(allocation_count, 0);
    assert!(note_was_audible);
    assert!(
        outputs[0]
            .iter()
            .all(|channel| channel.iter().all(|sample| *sample == 0.0))
    );
}

#[test]
fn poly_drum_decay_and_stereo_impulse_render_without_allocation() {
    let patch = load_kernel_patch_str(include_str!(
        "../../../../examples/patches/drums/drum-909-kick.yaml"
    ))
    .expect("909 kick example loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 64,
    };
    let prepared = prepare_kernel_patch(&patch, &settings).expect("909 kick prepares");
    let mut processor = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
        prepared.graph().clone(),
        prepared.compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    );
    let mut outputs = vec![vec![vec![0.0; 64]; 2]];

    let allocation_count = count_current_thread_allocations(|| {
        processor.note_on(36, 100);
        assert_eq!(processor.render_root_outputs(&mut outputs), 64);
    });

    assert_eq!(allocation_count, 0);
    assert!(outputs[0][0].iter().any(|sample| sample.abs() > 0.001));
    assert_eq!(outputs[0][0], outputs[0][1]);
}

#[test]
fn full_capacity_poly_stealing_and_rejection_render_without_allocation() {
    for (allocation, expected_notes) in [
        (crate::kernel::POLY_ALLOCATION_OLDEST_STEAL, [67, 64]),
        (crate::kernel::POLY_ALLOCATION_REJECT_NEW, [60, 64]),
    ] {
        let mut processor = prepared_constant_poly(allocation, false);
        let mut outputs = vec![vec![vec![0.0; 64]]; 2];

        let allocation_count = count_current_thread_allocations(|| {
            processor.note_on(60, 100);
            processor.note_on(64, 100);
            assert_eq!(processor.render_root_outputs(&mut outputs), 64);
            processor.note_on(67, 100);
            assert_eq!(processor.render_root_outputs(&mut outputs), 64);
        });
        assert_eq!(allocation_count, 0, "allocation policy: {allocation}");
        assert!(outputs[0][0].iter().all(|sample| *sample == 0.5));
        assert_eq!(outputs[0], outputs[1]);
        let region = &processor.prepared_poly_runtime_regions()[0];
        assert_eq!(region.active_voice_count(), 2);
        assert_eq!(
            [region.voice_note(0).unwrap(), region.voice_note(1).unwrap()],
            expected_notes,
            "allocation policy: {allocation}"
        );
    }
}
