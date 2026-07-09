use super::*;
use crate::diagnostics::error_codes;
use crate::kernel::{
    CONTROL_TO_AUDIO_DEFINITION, ChannelCount, ControlDefault, LatencySpec, Port, StaticArg,
    StaticParam, StaticType,
};

fn oscillator() -> GraphDefinition {
    GraphDefinition::new("oscillator").with_port(Port::output("audio", SignalType::Audio, 1))
}

fn gain() -> GraphDefinition {
    GraphDefinition::new("gain")
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(
            Port::input("level", SignalType::Control, 1)
                .with_control_default(ControlDefault::new(1.0)),
        )
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

fn echo() -> GraphDefinition {
    GraphDefinition::new("echo")
        .with_static_param(
            StaticParam::new("channels", StaticType::Int).with_default(StaticValue::Int(2)),
        )
        .with_port(Port::input(
            "audio_in",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ))
        .with_port(Port::output(
            "audio_out",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ))
}

/// A composite exposing an `audio` output gathered from an internal node's
/// output port.
fn wrapping_composite(name: &str, child_type: &str, child_id: &str) -> GraphDefinition {
    GraphDefinition::new(name)
        .with_port(
            Port::output("audio", SignalType::Audio, 1)
                .maps_from(PortRef::new(NodeId::new(child_id), "audio")),
        )
        .with_node(Node::new(NodeId::new(child_id), child_type))
}

#[test]
fn nested_composites_flatten_to_atomic_nodes_with_namespaced_ids() {
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(wrapping_composite("inner", "oscillator", "osc"))
        .with_definition(wrapping_composite("outer", "inner", "in"));
    let root =
        GraphDefinition::new("root").with_node(Node::new(NodeId::new("o"), "outer"));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes().len(), 1);
    assert_eq!(flat.nodes()[0].id().as_str(), "o::in::osc");
    assert_eq!(flat.nodes()[0].definition(), "oscillator");
}

#[test]
fn boundary_ports_forward_connections_to_atomic_ports() {
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain())
        .with_definition(wrapping_composite("inner", "oscillator", "osc"))
        .with_definition(wrapping_composite("outer", "inner", "in"));
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("o"), "outer"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("o"), "audio"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.connections().len(), 1);
    let connection = &flat.connections()[0];
    assert_eq!(connection.source().node().as_str(), "o::in::osc");
    assert_eq!(connection.source().port(), "audio");
    assert_eq!(connection.destination().node().as_str(), "amp");
    assert_eq!(connection.destination().port(), "audio_in");
}

#[test]
fn flattening_is_deterministic_and_repeatable() {
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain())
        .with_definition(wrapping_composite("inner", "oscillator", "osc"))
        .with_definition(wrapping_composite("outer", "inner", "in"));
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("o"), "outer"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("o"), "audio"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    assert_eq!(
        root.flatten(&registry).expect("first"),
        root.flatten(&registry).expect("second")
    );
}

#[test]
fn resolved_channel_count_flows_into_atomic_ports() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
    );

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes()[0].ports()[0].channels(), 2);
}

#[test]
fn instance_override_becomes_effective_atomic_default() {
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("amp"), "gain").with_default_override("level", 0.3),
    );

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes()[0].port_defaults().get("level"), Some(&0.3));
}

#[test]
fn declared_default_becomes_effective_atomic_default_without_override() {
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes()[0].port_defaults().get("level"), Some(&1.0));
}

#[test]
fn repeated_identical_instances_reuse_one_expansion() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let mut root = GraphDefinition::new("root");
    for index in 0..5 {
        root = root.with_node(
            Node::new(NodeId::new(format!("e{index}")), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        );
    }

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes().len(), 5, "each instance is a distinct node");
    assert_eq!(
        flat.expansion_count(),
        2,
        "root plus one shared echo expansion"
    );
}

#[test]
fn distinct_static_arguments_expand_separately() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("mono"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(1))),
        )
        .with_node(
            Node::new(NodeId::new("stereo"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        );

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(
        flat.expansion_count(),
        3,
        "root plus one echo expansion per distinct channel count"
    );
    assert_eq!(
        flat.node(&NodeId::new("mono")).unwrap().ports()[0].channels(),
        1
    );
    assert_eq!(
        flat.node(&NodeId::new("stereo")).unwrap().ports()[0].channels(),
        2
    );
}

// --- 2.4 Per-node latency metadata --------------------------------------

/// A spectral processor whose latency is `fft_size - 1`, driven by a static
/// parameter.
fn spectral() -> GraphDefinition {
    GraphDefinition::new("spectral")
        .with_static_param(
            StaticParam::new("fft_size", StaticType::Int).with_default(StaticValue::Int(1024)),
        )
        .with_latency(LatencySpec::StaticParam {
            name: "fft_size".to_string(),
            minus: 1,
        })
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

#[test]
fn primitive_reports_zero_latency() {
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.nodes()[0].latency(), 0);
}

#[test]
fn latency_reflects_resolved_static_argument() {
    let registry = DefinitionRegistry::new().with_definition(spectral());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("fft"), "spectral")
            .with_static_arg("fft_size", StaticArg::Literal(StaticValue::Int(512))),
    );

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(
        flat.nodes()[0].latency(),
        511,
        "spectral latency is fft_size - 1"
    );
}

#[test]
fn dangling_latency_reference_fails_compilation_instead_of_reporting_zero() {
    // A spectral processor whose latency references `fft_size`, but the static
    // parameter is never declared. Silently reporting zero latency here would
    // phase-smear any parallel dry/wet path; compilation must fail loudly.
    let broken = GraphDefinition::new("spectral")
        .with_latency(LatencySpec::StaticParam {
            name: "fft_size".to_string(),
            minus: 1,
        })
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1));
    let registry = DefinitionRegistry::new().with_definition(broken);
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("fft"), "spectral"));

    let diagnostics = root
        .flatten(&registry)
        .expect_err("dangling latency reference must fail compilation");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_UNRESOLVED_STATIC_REFERENCE),
        "expected unresolved-static-reference diagnostic, got: {diagnostics:?}"
    );
}

// --- Resolved static-reference value validation --------------------------

#[test]
fn negative_channel_count_static_argument_fails_compilation_instead_of_reporting_one() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(-1))),
    );

    let diagnostics = root
        .flatten(&registry)
        .expect_err("negative channel count must fail compilation, not fall back to one channel");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "expected invalid-static-reference-value diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn zero_channel_count_static_argument_fails_compilation() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(0))),
    );

    let diagnostics = root
        .flatten(&registry)
        .expect_err("zero channel count must fail compilation");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "expected invalid-static-reference-value diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn negative_latency_static_argument_fails_compilation_instead_of_reporting_zero() {
    let registry = DefinitionRegistry::new().with_definition(spectral());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("fft"), "spectral")
            .with_static_arg("fft_size", StaticArg::Literal(StaticValue::Int(-1))),
    );

    let diagnostics = root
        .flatten(&registry)
        .expect_err("negative latency must fail compilation, not fall back to zero");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "expected invalid-static-reference-value diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn latency_static_argument_smaller_than_minus_fails_compilation() {
    // `spectral` latency is `fft_size - 1`; an `fft_size` of zero would saturate
    // to zero latency, silently under-reporting a latency-bearing node.
    let registry = DefinitionRegistry::new().with_definition(spectral());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("fft"), "spectral")
            .with_static_arg("fft_size", StaticArg::Literal(StaticValue::Int(0))),
    );

    let diagnostics = root
        .flatten(&registry)
        .expect_err("latency smaller than the subtracted constant must fail compilation");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "expected invalid-static-reference-value diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn valid_resolved_channel_count_and_latency_still_flatten() {
    let registry = DefinitionRegistry::new()
        .with_definition(echo())
        .with_definition(spectral());
    let root = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("e"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        )
        .with_node(
            Node::new(NodeId::new("fft"), "spectral")
                .with_static_arg("fft_size", StaticArg::Literal(StaticValue::Int(512))),
        );

    let flat = root
        .flatten(&registry)
        .expect("valid resolved static values flatten successfully");

    assert_eq!(
        flat.node(&NodeId::new("e")).unwrap().ports()[0].channels(),
        2,
        "echo keeps its resolved channel count"
    );
    assert_eq!(
        flat.node(&NodeId::new("fft")).unwrap().latency(),
        511,
        "spectral keeps its resolved latency"
    );
}

#[test]
fn resolved_node_ports_returns_none_for_invalid_resolved_channel_count() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(0))),
    );

    assert_eq!(
        root.resolved_node_ports(&registry, &NodeId::new("e")),
        None,
        "an out-of-range resolved channel count yields no invented fallback ports"
    );
}

// --- 2.5 Control→audio promotion insertion ------------------------------

fn control_source() -> GraphDefinition {
    GraphDefinition::new("control_source").with_port(Port::output("cv", SignalType::Control, 1))
}

/// A sink with a two-channel audio input, so a promotion adopts its width.
fn audio_sink() -> GraphDefinition {
    GraphDefinition::new("audio_sink").with_port(Port::input("audio_in", SignalType::Audio, 2))
}

#[test]
fn control_to_audio_connection_inserts_a_visible_promotion_node() {
    let registry = DefinitionRegistry::new()
        .with_definition(control_source())
        .with_definition(audio_sink());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("lfo"), "control_source"))
        .with_node(Node::new(NodeId::new("amp"), "audio_sink"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("lfo"), "cv"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.promotions().len(), 1, "one control→audio edge is promoted");
    assert_eq!(
        flat.promotions()[0].channels(),
        2,
        "promotion adopts the audio destination's channel count"
    );

    let promotion = flat
        .nodes()
        .iter()
        .find(|node| node.definition() == CONTROL_TO_AUDIO_DEFINITION)
        .expect("a visible promotion node is inserted");
    assert_eq!(promotion.latency(), 0);

    assert!(
        flat.connections().iter().any(|c| c.source().port() == "cv"
            && c.destination().node() == promotion.id()),
        "control source now feeds the promotion node"
    );
    assert!(
        flat.connections().iter().any(|c| c.source().node() == promotion.id()
            && c.destination().port() == "audio_in"),
        "promotion node now feeds the audio destination"
    );
    assert!(
        !flat
            .connections()
            .iter()
            .any(|c| c.source().port() == "cv" && c.destination().port() == "audio_in"),
        "the direct control→audio edge is replaced, not left alongside"
    );
}

#[test]
fn matching_signal_connection_inserts_no_promotion() {
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("osc"), "audio"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert!(
        flat.promotions().is_empty(),
        "an audio→audio connection needs no promotion"
    );
    assert!(
        flat.nodes()
            .iter()
            .all(|node| node.definition() != CONTROL_TO_AUDIO_DEFINITION),
        "no promotion node is inserted for matching signals"
    );
}

// --- Composite input boundary -------------------------------------------

/// A composite that forwards a public audio input to an internal gain and
/// gathers its output back out, exercising both boundary directions.
fn amplifier() -> GraphDefinition {
    GraphDefinition::new("amplifier")
        .with_port(
            Port::input("audio_in", SignalType::Audio, 1)
                .maps_to(PortRef::new(NodeId::new("g"), "audio_in")),
        )
        .with_port(
            Port::output("audio_out", SignalType::Audio, 1)
                .maps_from(PortRef::new(NodeId::new("g"), "audio_out")),
        )
        .with_node(Node::new(NodeId::new("g"), "gain"))
}

#[test]
fn composite_input_port_forwards_incoming_connections_to_internal_ports() {
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain())
        .with_definition(amplifier());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("amp"), "amplifier"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("osc"), "audio"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(flat.connections().len(), 1);
    let connection = &flat.connections()[0];
    assert_eq!(connection.source().node().as_str(), "osc");
    assert_eq!(
        connection.destination().node().as_str(),
        "amp::g",
        "the composite's public input forwards to the internal node it maps to"
    );
    assert_eq!(connection.destination().port(), "audio_in");
}

#[test]
fn composite_input_mapped_to_an_unknown_internal_node_forwards_nothing() {
    // Validation reports the dangling `maps_to` separately; flattening must not
    // wire the incoming connection to an internal node that was never expanded.
    let dangling = GraphDefinition::new("dangling").with_port(
        Port::input("audio_in", SignalType::Audio, 1)
            .maps_to(PortRef::new(NodeId::new("ghost"), "audio_in")),
    );
    let registry = DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain())
        .with_definition(dangling.with_node(Node::new(NodeId::new("g"), "gain")));
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("d"), "dangling"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("osc"), "audio"),
            PortRef::new(NodeId::new("d"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert!(
        flat.connections().is_empty(),
        "a public input mapping to a nonexistent internal node forwards to nothing"
    );
}

#[test]
fn root_ports_expose_the_resolved_public_interface() {
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root")
        .with_port(
            Port::input("in", SignalType::Audio, 1)
                .maps_to(PortRef::new(NodeId::new("g"), "audio_in")),
        )
        .with_port(
            Port::output("out", SignalType::Audio, 2)
                .maps_from(PortRef::new(NodeId::new("g"), "audio_out")),
        )
        .with_node(Node::new(NodeId::new("g"), "gain"));

    let flat = root.flatten(&registry).expect("flattens");

    let names: Vec<&str> = flat.root_ports().iter().map(|port| port.name()).collect();
    assert_eq!(names, ["in", "out"], "both root ports are resolved");
    let output = flat
        .root_ports()
        .iter()
        .find(|port| port.direction() == PortDirection::Output)
        .expect("root declares an output port");
    assert_eq!(
        output.channels(),
        2,
        "the root output keeps its declared channel count for host bus binding"
    );
}

// --- Static arguments on atomic nodes ------------------------------------

#[test]
fn atomic_nodes_record_their_resolved_static_arguments() {
    let registry = DefinitionRegistry::new().with_definition(echo());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
    );

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(
        flat.nodes()[0].static_args().get("channels"),
        Some(&StaticValue::Int(2)),
        "the resolved static arguments travel with the atomic node"
    );
}

/// A primitive keyed by a non-integer static parameter of each remaining type.
fn sampler() -> GraphDefinition {
    GraphDefinition::new("sampler")
        .with_static_param(StaticParam::new("mode", StaticType::Enum))
        .with_static_param(StaticParam::new("sample", StaticType::Resource))
        .with_port(Port::output("audio", SignalType::Audio, 1))
}

fn sampler_node(id: &str, mode: &str, sample: &str) -> Node {
    Node::new(NodeId::new(id), "sampler")
        .with_static_arg("mode", StaticArg::Literal(StaticValue::Enum(mode.into())))
        .with_static_arg(
            "sample",
            StaticArg::Literal(StaticValue::Resource(sample.into())),
        )
}

#[test]
fn enum_and_resource_static_arguments_key_the_expansion_cache() {
    let registry = DefinitionRegistry::new().with_definition(sampler());
    let root = GraphDefinition::new("root")
        .with_node(sampler_node("a", "loop", "kick"))
        .with_node(sampler_node("b", "loop", "kick"))
        .with_node(sampler_node("c", "loop", "snare"))
        .with_node(sampler_node("d", "one_shot", "kick"));

    let flat = root.flatten(&registry).expect("flattens");

    assert_eq!(
        flat.expansion_count(),
        4,
        "root plus one expansion per distinct (mode, sample) pair: 'a' and 'b' share one"
    );
}

// --- Flattening in the presence of invalid structure ---------------------

#[test]
fn unknown_definition_reference_inside_a_composite_is_rejected() {
    let broken =
        GraphDefinition::new("broken").with_node(Node::new(NodeId::new("x"), "ghost"));
    let registry = DefinitionRegistry::new().with_definition(broken);
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("b"), "broken"));

    let diagnostics = root
        .flatten(&registry)
        .expect_err("a node referencing an undeclared definition must fail compilation");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_UNKNOWN_DEFINITION),
        "expected unknown-definition diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn unresolvable_static_arguments_fail_compilation_without_expanding_the_node() {
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("amp"), "gain")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
    );

    let diagnostics = root
        .flatten(&registry)
        .expect_err("an unknown static argument must fail compilation");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT),
        "expected unknown-static-argument diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn connection_referencing_an_unknown_node_is_dropped_rather_than_wired() {
    // Validation reports dangling connections separately; flattening must not
    // invent an edge to a node that does not exist.
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("ghost"), "audio_out"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let flat = root.flatten(&registry).expect("flattens");

    assert!(
        flat.connections().is_empty(),
        "no edge is wired for a connection whose source node does not exist"
    );
}

#[test]
fn override_of_an_unknown_port_leaves_declared_defaults_intact() {
    // Validation rejects unknown-port overrides; flattening must ignore them
    // rather than fabricate a default for a port the node does not have.
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("amp"), "gain").with_default_override("bogus", 0.3),
    );

    let flat = root.flatten(&registry).expect("flattens");

    let defaults = flat.nodes()[0].port_defaults();
    assert_eq!(defaults.get("bogus"), None, "the unknown port gains no default");
    assert_eq!(
        defaults.get("level"),
        Some(&1.0),
        "the declared default of the real control port is untouched"
    );
}

#[test]
fn recursive_definition_is_rejected() {
    let recursive = GraphDefinition::new("loop").with_node(Node::new(NodeId::new("inner"), "loop"));
    let registry = DefinitionRegistry::new().with_definition(recursive);
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("l"), "loop"));

    let diagnostics = root.flatten(&registry).expect_err("recursion rejected");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_RECURSIVE_DEFINITION),
        "expected recursion diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn excessive_nesting_depth_is_rejected() {
    // Build a chain of distinct composites c0 -> c1 -> ... -> leaf, deep enough
    // to exceed the flatten depth guard without any recursion.
    let mut registry = DefinitionRegistry::new().with_definition(oscillator());
    let chain_length = MAX_FLATTEN_DEPTH + 3;
    for index in 0..chain_length {
        let child = if index + 1 == chain_length {
            "oscillator".to_string()
        } else {
            format!("c{}", index + 1)
        };
        registry = registry.with_definition(wrapping_composite(
            &format!("c{index}"),
            &child,
            "next",
        ));
    }
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("c"), "c0"));

    let diagnostics = root.flatten(&registry).expect_err("depth rejected");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_MAX_DEPTH_EXCEEDED),
        "expected max-depth diagnostic, got: {diagnostics:?}"
    );
}
