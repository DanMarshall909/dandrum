use super::*;
use crate::diagnostics::error_codes;
use crate::graph::SignalType;
use crate::kernel::{
    COMPENSATION_DELAY_DEFINITION, Connection, DefinitionRegistry, FEEDBACK_DELAY_DEFINITION,
    GraphDefinition, LatencySpec, Node, NodeId, Port, PortRef, StaticParam, StaticType, StaticValue,
};

const FFT_SIZE: i64 = 8;
const FFT_LATENCY: u32 = (FFT_SIZE as u32) - 1;

fn oscillator() -> GraphDefinition {
    GraphDefinition::new("oscillator").with_port(Port::output("audio", SignalType::Audio, 1))
}

fn gain() -> GraphDefinition {
    GraphDefinition::new("gain")
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

/// A spectral processor with a fixed default FFT size, giving `fft_size - 1`
/// samples of latency.
fn spectral() -> GraphDefinition {
    GraphDefinition::new("spectral")
        .with_static_param(
            StaticParam::new("fft_size", StaticType::Int).with_default(StaticValue::Int(FFT_SIZE)),
        )
        .with_latency(LatencySpec::StaticParam {
            name: "fft_size".to_string(),
            minus: 1,
        })
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

fn mixer() -> GraphDefinition {
    GraphDefinition::new("mixer")
        .with_port(Port::input("a", SignalType::Audio, 1))
        .with_port(Port::input("b", SignalType::Audio, 1))
        .with_port(Port::output("out", SignalType::Audio, 1))
}

fn feedback_delay() -> GraphDefinition {
    GraphDefinition::new(FEEDBACK_DELAY_DEFINITION)
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

/// A registry with the atomic building blocks the balancing tests reference.
fn primitives() -> DefinitionRegistry {
    DefinitionRegistry::new()
        .with_definition(oscillator())
        .with_definition(gain())
        .with_definition(spectral())
        .with_definition(mixer())
        .with_definition(feedback_delay())
}

fn connect(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> Connection {
    Connection::new(
        PortRef::new(NodeId::new(from_node), from_port),
        PortRef::new(NodeId::new(to_node), to_port),
    )
}

fn has_connection(balanced: &BalancedGraph, from: (&str, &str), to: (&str, &str)) -> bool {
    balanced.connections().iter().any(|connection| {
        connection.source().node().as_str() == from.0
            && connection.source().port() == from.1
            && connection.destination().node().as_str() == to.0
            && connection.destination().port() == to.1
    })
}

/// osc feeds both a latency-inducing wet path (spectral -> mixer.a) and a
/// zero-latency dry path (mixer.b); the two converge at the mixer.
fn dry_wet_root() -> GraphDefinition {
    GraphDefinition::new("root")
        .with_port(Port::output("master", SignalType::Audio, 1).maps_from(PortRef::new(
            NodeId::new("mix"),
            "out",
        )))
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("fft"), "spectral"))
        .with_node(Node::new(NodeId::new("mix"), "mixer"))
        .with_connection(connect("osc", "audio", "fft", "audio_in"))
        .with_connection(connect("fft", "audio_out", "mix", "a"))
        .with_connection(connect("osc", "audio", "mix", "b"))
}

#[test]
fn unequal_parallel_paths_are_compensated_to_align() {
    let flat = dry_wet_root().flatten(&primitives()).expect("flattens");

    let balanced = flat.balance_latency().expect("balances");

    assert_eq!(
        balanced.compensations().len(),
        1,
        "only the dry path needs a compensation delay"
    );
    let insertion = &balanced.compensations()[0];
    assert_eq!(
        insertion.samples(),
        FFT_LATENCY,
        "the dry path is delayed by the wet path's latency"
    );
    assert_eq!(
        insertion.source(),
        &PortRef::new(NodeId::new("osc"), "audio"),
        "because the delayed edge is the dry osc output"
    );
    assert_eq!(
        insertion.destination(),
        &PortRef::new(NodeId::new("mix"), "b"),
        "because the dry path feeds the mixer's b input"
    );
}

#[test]
fn compensation_node_is_spliced_into_the_dry_edge() {
    let flat = dry_wet_root().flatten(&primitives()).expect("flattens");

    let balanced = flat.balance_latency().expect("balances");

    let node_id = balanced.compensations()[0].node().as_str().to_string();
    let inserted = balanced
        .nodes()
        .iter()
        .find(|node| node.id().as_str() == node_id)
        .expect("compensation node is present");
    assert_eq!(
        inserted.definition(),
        COMPENSATION_DELAY_DEFINITION,
        "because the balancer synthesises a compensation-delay node"
    );
    assert_eq!(
        inserted.latency(),
        FFT_LATENCY,
        "because the delay's own latency equals its compensation"
    );
    assert!(
        has_connection(&balanced, ("osc", "audio"), (&node_id, COMPENSATION_INPUT_PORT))
            && has_connection(&balanced, (&node_id, COMPENSATION_OUTPUT_PORT), ("mix", "b")),
        "because the dry edge is rerouted through the compensation node"
    );
    assert!(
        !has_connection(&balanced, ("osc", "audio"), ("mix", "b")),
        "because the original direct dry edge is replaced"
    );
}

#[test]
fn total_root_latency_reflects_the_deepest_path() {
    let flat = dry_wet_root().flatten(&primitives()).expect("flattens");

    let balanced = flat.balance_latency().expect("balances");

    assert_eq!(
        balanced.total_latency(),
        FFT_LATENCY,
        "because the root output carries the wet path's accumulated latency"
    );
}

#[test]
fn inserted_compensation_is_reported_as_an_info_diagnostic() {
    let flat = dry_wet_root().flatten(&primitives()).expect("flattens");

    let balanced = flat.balance_latency().expect("balances");

    assert!(
        balanced
            .diagnostics()
            .infos()
            .any(|d| d.error_code() == error_codes::KERNEL_COMPENSATION_INSERTED),
        "because each insertion is advertised for inspection, got: {:?}",
        balanced.diagnostics()
    );
}

#[test]
fn balanced_graph_carries_no_error_diagnostics() {
    let flat = dry_wet_root().flatten(&primitives()).expect("flattens");

    let balanced = flat.balance_latency().expect("balances");

    assert!(
        !balanced.diagnostics().has_errors(),
        "because a balanceable graph produces only advisory notes"
    );
}

#[test]
fn zero_latency_graph_needs_no_compensation() {
    let registry = primitives();
    let root = GraphDefinition::new("root")
        .with_port(Port::output("master", SignalType::Audio, 1).maps_from(PortRef::new(
            NodeId::new("amp"),
            "audio_out",
        )))
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(connect("osc", "audio", "amp", "audio_in"));

    let balanced = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert!(
        balanced.compensations().is_empty(),
        "because every path has equal (zero) latency"
    );
    assert_eq!(
        balanced.total_latency(),
        0,
        "because no node reports latency"
    );
}

#[test]
fn latency_accumulates_through_a_composite() {
    // Wrap the spectral processor in a composite so its latency only surfaces
    // after flattening; the dry/wet convergence must still align.
    let wet = GraphDefinition::new("wet")
        .with_port(Port::input("audio_in", SignalType::Audio, 1).maps_to(PortRef::new(
            NodeId::new("fft"),
            "audio_in",
        )))
        .with_port(Port::output("audio_out", SignalType::Audio, 1).maps_from(PortRef::new(
            NodeId::new("fft"),
            "audio_out",
        )))
        .with_node(Node::new(NodeId::new("fft"), "spectral"));
    let registry = primitives().with_definition(wet);
    let root = GraphDefinition::new("root")
        .with_port(Port::output("master", SignalType::Audio, 1).maps_from(PortRef::new(
            NodeId::new("mix"),
            "out",
        )))
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_node(Node::new(NodeId::new("w"), "wet"))
        .with_node(Node::new(NodeId::new("mix"), "mixer"))
        .with_connection(connect("osc", "audio", "w", "audio_in"))
        .with_connection(connect("w", "audio_out", "mix", "a"))
        .with_connection(connect("osc", "audio", "mix", "b"));

    let balanced = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(
        balanced.compensations().len(),
        1,
        "because the composite's internal latency reaches the convergence"
    );
    assert_eq!(balanced.compensations()[0].samples(), FFT_LATENCY);
    assert_eq!(balanced.total_latency(), FFT_LATENCY);
}

#[test]
fn latency_bearing_node_in_a_feedback_cycle_is_rejected() {
    let registry = primitives();
    // fb -> fft -> fb: a legal feedback_delay cut, but the spectral node inside
    // the loop carries latency that cannot be compensated.
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("fb"), FEEDBACK_DELAY_DEFINITION))
        .with_node(Node::new(NodeId::new("fft"), "spectral"))
        .with_connection(connect("fb", "audio_out", "fft", "audio_in"))
        .with_connection(connect("fft", "audio_out", "fb", "audio_in"));

    let diagnostics = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect_err("latency in a loop is rejected");

    let diagnostic = diagnostics
        .errors()
        .find(|d| d.error_code() == error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE)
        .expect("names the latency-in-cycle failure");
    assert_eq!(
        diagnostic.module_id(),
        Some("fft"),
        "because the diagnostic names the offending node"
    );
    assert_eq!(
        diagnostic.actual(),
        Some(FFT_LATENCY.to_string().as_str()),
        "because the diagnostic reports the node's latency"
    );
}

#[test]
fn zero_latency_feedback_cycle_is_allowed() {
    let registry = primitives();
    // fb -> gain -> fb: a legal feedback loop with no latency to compensate.
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("fb"), FEEDBACK_DELAY_DEFINITION))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(connect("fb", "audio_out", "amp", "audio_in"))
        .with_connection(connect("amp", "audio_out", "fb", "audio_in"));

    let balanced = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("a zero-latency loop balances");

    assert!(
        balanced.compensations().is_empty(),
        "because the loop carries no latency to compensate"
    );
}

#[test]
fn cycle_without_feedback_delay_is_rejected() {
    let registry = primitives();
    // A two-gain loop with no feedback_delay cut: no topological order exists.
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_connection(connect("a", "audio_out", "b", "audio_in"))
        .with_connection(connect("b", "audio_out", "a", "audio_in"));

    let diagnostics = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect_err("an uncut cycle is rejected");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY),
        "because a cycle with no feedback_delay cannot be scheduled, got: {diagnostics:?}"
    );
}
