use crate::diagnostics::error_codes;
use crate::graph::SignalType;
use crate::kernel::{
    Connection, DefinitionRegistry, FEEDBACK_DELAY_DEFINITION, GraphDefinition, LatencySpec, Node,
    NodeId, Port, PortRef,
};

fn source() -> GraphDefinition {
    GraphDefinition::new("source").with_port(Port::output("audio", SignalType::Audio, 1))
}

/// A processor that declares a fixed processing latency.
fn delayed(name: &str, samples: u32) -> GraphDefinition {
    GraphDefinition::new(name)
        .with_latency(LatencySpec::Samples(samples))
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

/// A zero-latency two-input summing node where paths converge.
fn adder() -> GraphDefinition {
    GraphDefinition::new("adder")
        .with_port(Port::input("a", SignalType::Audio, 1))
        .with_port(Port::input("b", SignalType::Audio, 1))
        .with_port(Port::output("out", SignalType::Audio, 1))
}

fn gain() -> GraphDefinition {
    GraphDefinition::new("gain")
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

fn feedback_delay() -> GraphDefinition {
    GraphDefinition::new(FEEDBACK_DELAY_DEFINITION)
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

fn cable(from: &str, from_port: &str, to: &str, to_port: &str) -> Connection {
    Connection::new(
        PortRef::new(NodeId::new(from), from_port),
        PortRef::new(NodeId::new(to), to_port),
    )
}

#[test]
fn unequal_parallel_paths_receive_a_compensating_delay() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("proc", 64))
        .with_definition(adder());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("p"), "proc"))
        .with_node(Node::new(NodeId::new("m"), "adder"))
        .with_connection(cable("osc", "audio", "p", "audio_in"))
        .with_connection(cable("osc", "audio", "m", "a"))
        .with_connection(cable("p", "audio_out", "m", "b"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(
        plan.compensations().len(),
        1,
        "one convergence needs a single compensating delay"
    );
    let compensation = &plan.compensations()[0];
    assert_eq!(
        compensation.samples(),
        64,
        "the direct path is delayed to match the 64-sample processing path"
    );
    assert_eq!(compensation.channels(), 1, "delay buffer width matches the port");
    assert_eq!(
        compensation.connection().destination().port(),
        "a",
        "the shorter (direct) input is the one delayed"
    );
    assert_eq!(plan.root_latency(), 64);
}

#[test]
fn balanced_parallel_paths_need_no_compensation() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("p1", 64))
        .with_definition(delayed("p2", 64))
        .with_definition(adder());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("x"), "p1"))
        .with_node(Node::new(NodeId::new("y"), "p2"))
        .with_node(Node::new(NodeId::new("m"), "adder"))
        .with_connection(cable("osc", "audio", "x", "audio_in"))
        .with_connection(cable("osc", "audio", "y", "audio_in"))
        .with_connection(cable("x", "audio_out", "m", "a"))
        .with_connection(cable("y", "audio_out", "m", "b"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert!(
        plan.compensations().is_empty(),
        "equal-latency paths align without compensation"
    );
    assert_eq!(plan.root_latency(), 64);
}

#[test]
fn root_latency_is_the_longest_path_latency() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("a", 64))
        .with_definition(delayed("b", 128));
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("first"), "a"))
        .with_node(Node::new(NodeId::new("second"), "b"))
        .with_connection(cable("osc", "audio", "first", "audio_in"))
        .with_connection(cable("first", "audio_out", "second", "audio_in"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(
        plan.root_latency(),
        192,
        "accumulated latency of the chain (64 + 128)"
    );
}

#[test]
fn latency_bearing_node_in_feedback_cycle_is_rejected() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("proc", 64))
        .with_definition(adder())
        .with_definition(feedback_delay());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("m"), "adder"))
        .with_node(Node::new(NodeId::new("p"), "proc"))
        .with_node(Node::new(NodeId::new("fb"), FEEDBACK_DELAY_DEFINITION))
        .with_connection(cable("osc", "audio", "m", "a"))
        .with_connection(cable("m", "out", "p", "audio_in"))
        .with_connection(cable("p", "audio_out", "fb", "audio_in"))
        .with_connection(cable("fb", "audio_out", "m", "b"));

    let diagnostics = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect_err("latency inside a feedback cycle cannot be compensated");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE),
        "expected latency-in-feedback-cycle diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn zero_latency_feedback_cycle_is_allowed() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(gain())
        .with_definition(adder())
        .with_definition(feedback_delay());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("m"), "adder"))
        .with_node(Node::new(NodeId::new("g"), "gain"))
        .with_node(Node::new(NodeId::new("fb"), FEEDBACK_DELAY_DEFINITION))
        .with_connection(cable("osc", "audio", "m", "a"))
        .with_connection(cable("m", "out", "g", "audio_in"))
        .with_connection(cable("g", "audio_out", "fb", "audio_in"))
        .with_connection(cable("fb", "audio_out", "m", "b"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("a zero-latency feedback cycle is legal");

    assert!(plan.compensations().is_empty());
    assert_eq!(plan.root_latency(), 0);
}
