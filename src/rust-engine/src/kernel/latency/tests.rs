use crate::builtins::module_types as names;
use crate::convolution::Convolution;
use crate::diagnostics::error_codes;
use crate::graph::SignalType;
use crate::kernel::builtins::{SPECTRAL_FFT_SIZE_PARAM, builtin_registry};
use crate::kernel::{
    Connection, DefinitionRegistry, FEEDBACK_DELAY_DEFINITION, GraphDefinition, LatencySpec, Node,
    NodeId, Port, PortRef, StaticArg, StaticValue,
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

/// A root output port that gathers from an internal node's output port.
fn root_output(from_node: &str, from_port: &str) -> Port {
    Port::output("out", SignalType::Audio, 1)
        .maps_from(PortRef::new(NodeId::new(from_node), from_port))
}

fn named_root_output(name: &str, from_node: &str, from_port: &str) -> Port {
    Port::output(name, SignalType::Audio, 1)
        .maps_from(PortRef::new(NodeId::new(from_node), from_port))
}

#[test]
fn unequal_parallel_paths_receive_a_compensating_delay() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("proc", 64))
        .with_definition(adder());
    let root = GraphDefinition::new("root")
        .with_port(root_output("m", "out"))
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
    assert_eq!(
        compensation.channels(),
        1,
        "delay buffer width matches the port"
    );
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
        .with_port(root_output("m", "out"))
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
        .with_port(root_output("second", "audio_out"))
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
fn dead_unconnected_latency_branch_does_not_inflate_root_latency() {
    // The real output path is zero-latency; a separate latency-bearing branch
    // is fed but its output goes nowhere. It must not inflate reported latency.
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(gain())
        .with_definition(delayed("dead", 512));
    let root = GraphDefinition::new("root")
        .with_port(root_output("g", "audio_out"))
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("g"), "gain"))
        .with_node(Node::new(NodeId::new("dp"), "dead"))
        .with_connection(cable("osc", "audio", "g", "audio_in"))
        .with_connection(cable("osc", "audio", "dp", "audio_in"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(
        plan.root_latency(),
        0,
        "the dead 512-sample branch does not feed a root output, so it is excluded"
    );
}

#[test]
fn root_latency_uses_root_output_sources_not_terminal_nodes() {
    // The root output gathers from `a` (64 samples). `a` also feeds a deeper
    // terminal node `c` (a further 128). A terminal-node computation would
    // report 192; the correct answer is the root output source's 64.
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("a_proc", 64))
        .with_definition(delayed("c_proc", 128));
    let root = GraphDefinition::new("root")
        .with_port(root_output("a", "audio_out"))
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("a"), "a_proc"))
        .with_node(Node::new(NodeId::new("c"), "c_proc"))
        .with_connection(cable("osc", "audio", "a", "audio_in"))
        .with_connection(cable("a", "audio_out", "c", "audio_in"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(
        plan.root_latency(),
        64,
        "root latency follows the root output source, not the deeper terminal node"
    );
}

#[test]
fn root_audio_outputs_are_compensated_to_the_reported_latency() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(delayed("proc", 64));
    let root = GraphDefinition::new("root")
        .with_port(named_root_output("dry", "osc", "audio"))
        .with_port(named_root_output("wet", "wet", "audio_out"))
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("wet"), "proc"))
        .with_connection(cable("osc", "audio", "wet", "audio_in"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert_eq!(plan.root_latency(), 64);
    assert_eq!(plan.root_compensations().len(), 1);
    let compensation = &plan.root_compensations()[0];
    assert_eq!(compensation.root_port(), "dry");
    assert_eq!(compensation.source().node().as_str(), "osc");
    assert_eq!(compensation.source().port(), "audio");
    assert_eq!(compensation.samples(), 64);
    assert_eq!(compensation.channels(), 1);
}

#[test]
fn control_edge_is_not_given_an_audio_compensation_delay() {
    let control_source = GraphDefinition::new("control_source").with_port(Port::output(
        "value",
        SignalType::Control,
        1,
    ));
    let controlled = GraphDefinition::new("controlled")
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(Port::input("amount", SignalType::Control, 1))
        .with_port(Port::output("audio_out", SignalType::Audio, 1));
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(control_source)
        .with_definition(delayed("proc", 64))
        .with_definition(controlled);
    let root = GraphDefinition::new("root")
        .with_port(root_output("target", "audio_out"))
        .with_node(Node::new(NodeId::new("osc"), "source"))
        .with_node(Node::new(NodeId::new("wet"), "proc"))
        .with_node(Node::new(NodeId::new("control"), "control_source"))
        .with_node(Node::new(NodeId::new("target"), "controlled"))
        .with_connection(cable("osc", "audio", "wet", "audio_in"))
        .with_connection(cable("wet", "audio_out", "target", "audio_in"))
        .with_connection(cable("control", "value", "target", "amount"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    assert!(
        plan.compensations().is_empty(),
        "a block-rate control edge is not delayed to match an audio input"
    );
    assert_eq!(plan.root_latency(), 64);
}

#[test]
fn balance_latency_rejects_remaining_non_feedback_cycle_instead_of_returning_default_plan() {
    // A zero-latency cycle with no feedback_delay: no latency offender, but the
    // graph is not a DAG. balance_latency must reject, not return a plan built
    // from an incomplete ordering.
    let registry = DefinitionRegistry::new().with_definition(gain());
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("g1"), "gain"))
        .with_node(Node::new(NodeId::new("g2"), "gain"))
        .with_connection(cable("g1", "audio_out", "g2", "audio_in"))
        .with_connection(cable("g2", "audio_out", "g1", "audio_in"));

    let diagnostics = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect_err("a cycle without feedback_delay must be rejected");

    assert!(
        diagnostics
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY),
        "expected cycle-without-feedback-delay diagnostic, got: {diagnostics:?}"
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
    assert_eq!(
        diagnostics.errors().count(),
        1,
        "one cycle yields exactly one diagnostic, not one per back edge walked"
    );
}

#[test]
fn each_distinct_feedback_cycle_through_a_latency_node_is_reported_once() {
    // `p` sits in two distinct cycles: the outer m -> p -> fb -> m, and the
    // inner p -> fb -> p (fb's output also feeds p directly). Each cycle is a
    // separate uncompensatable path, so each is reported — but neither twice.
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
        .with_connection(cable("fb", "audio_out", "m", "b"))
        .with_connection(cable("fb", "audio_out", "p", "audio_in"));

    let diagnostics = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect_err("latency inside a feedback cycle cannot be compensated");

    assert_eq!(
        diagnostics.errors().count(),
        2,
        "one diagnostic per distinct cycle carrying the latency-bearing node"
    );
    assert!(
        diagnostics
            .errors()
            .all(|d| d.error_code() == error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE),
        "both diagnostics name the uncompensatable cycle, got: {diagnostics:?}"
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

// --- 2.4c (compile-time half) Dry/wet alignment with real builtin latencies --
//
// These prove that the latencies declared in the kernel builtin registry (§2.4),
// placed in the canonical dry-plus-processed topology, drive the balancer to
// compensate the dry path so both arrive aligned. The sample-accurate render
// assertion ("arrives time-aligned at the mix") lands with the pipeline (§2.6).

/// The builtin's real declared definition, ports and latency as registered.
fn builtin(name: &str) -> GraphDefinition {
    builtin_registry()
        .get(name)
        .unwrap_or_else(|| panic!("builtin '{name}' is registered"))
        .clone()
}

#[test]
fn convolution_dry_and_wet_paths_are_compensated_to_alignment() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(builtin(names::CONVOLUTION))
        .with_definition(adder());
    let root = GraphDefinition::new("root")
        .with_port(root_output("mix", "out"))
        .with_node(Node::new(NodeId::new("imp"), "source"))
        .with_node(Node::new(NodeId::new("wet"), names::CONVOLUTION))
        .with_node(Node::new(NodeId::new("mix"), "adder"))
        // dry path straight to the mix; wet path through the convolution
        .with_connection(cable("imp", "audio", "mix", "a"))
        .with_connection(cable("imp", "audio", "wet", "audio_in"))
        .with_connection(cable("wet", "audio_out", "mix", "b"));

    let plan = root
        .flatten(&registry)
        .expect("flattens")
        .balance_latency()
        .expect("balances");

    let block = Convolution::BLOCK_SIZE as u32;
    assert_eq!(
        plan.compensations().len(),
        1,
        "only the dry path needs compensation"
    );
    let compensation = &plan.compensations()[0];
    assert_eq!(
        compensation.connection().destination().port(),
        "a",
        "the dry input is the one delayed"
    );
    assert_eq!(
        compensation.samples(),
        block,
        "the dry path is delayed by exactly the convolution's one-block latency"
    );
    assert_eq!(
        plan.root_latency(),
        block,
        "the aligned mix presents one block of latency to the host"
    );
}

#[test]
fn spectral_dry_and_wet_paths_align_per_resolved_fft_size() {
    let registry = DefinitionRegistry::new()
        .with_definition(source())
        .with_definition(builtin(names::SPECTRAL_PROCESSOR))
        .with_definition(adder());

    for (fft_size, expected_latency) in [(512_i64, 511_u32), (1024, 1023)] {
        let root = GraphDefinition::new("root")
            .with_port(root_output("mix", "out"))
            .with_node(Node::new(NodeId::new("imp"), "source"))
            .with_node(
                Node::new(NodeId::new("wet"), names::SPECTRAL_PROCESSOR).with_static_arg(
                    SPECTRAL_FFT_SIZE_PARAM,
                    StaticArg::Literal(StaticValue::Int(fft_size)),
                ),
            )
            .with_node(Node::new(NodeId::new("mix"), "adder"))
            .with_connection(cable("imp", "audio", "mix", "a"))
            .with_connection(cable("imp", "audio", "wet", "audio_in"))
            .with_connection(cable("wet", "audio_out", "mix", "b"));

        let plan = root
            .flatten(&registry)
            .expect("flattens")
            .balance_latency()
            .expect("balances");

        let compensation = &plan.compensations()[0];
        assert_eq!(
            compensation.connection().destination().port(),
            "a",
            "the dry input is delayed for fft_size {fft_size}"
        );
        assert_eq!(
            compensation.samples(),
            expected_latency,
            "dry path compensation is fft_size - 1 for fft_size {fft_size}"
        );
        assert_eq!(
            plan.root_latency(),
            expected_latency,
            "aligned mix latency tracks the resolved fft_size {fft_size}"
        );
    }
}
