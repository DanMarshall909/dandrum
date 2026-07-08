use super::*;
use crate::diagnostics::error_codes;
use crate::kernel::{ChannelCount, ControlDefault, Port, StaticArg, StaticParam, StaticType};

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
