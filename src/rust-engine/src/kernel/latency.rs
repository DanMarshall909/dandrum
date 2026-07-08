//! Latency balancing over a flattened kernel graph (see `unify-graph-kernel`
//! §2.4b).
//!
//! Atomic nodes declare a per-instance processing latency (in samples). Where
//! parallel audio paths of unequal accumulated latency converge on a node, the
//! signals would otherwise arrive on different samples and smear. This pass:
//!
//! 1. rejects any feedback cycle that contains a nonzero-latency node —
//!    compensation cannot be inserted inside a loop;
//! 2. accumulates audio latency along the graph (cutting `feedback_delay`
//!    inputs, which the scheduler also cuts, so the accumulation graph is a
//!    DAG);
//! 3. inserts a [`COMPENSATION_DELAY_DEFINITION`] node on each shorter incoming
//!    audio edge at a convergence, delaying it to match the latest sibling;
//! 4. reports every insertion as an `Info` diagnostic and a structured
//!    [`CompensationInsertion`]; and
//! 5. computes the root graph's total latency for host plugin-latency reporting.
//!
//! Only audio connections are balanced; control and event edges pass through
//! unchanged (control-rate timing tolerates a block of skew, and the alignment
//! scenarios that matter are audio dry/wet topologies).

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};
use crate::graph::SignalType;

use super::flatten::{AtomicNode, FlattenedGraph};
use super::{Connection, FEEDBACK_DELAY_DEFINITION, NodeId, PortRef};

/// Static-parameter name carrying a compensation node's channel count.
pub const COMPENSATION_CHANNELS_PARAM: &str = "channels";
/// Static-parameter name carrying a compensation node's delay length in samples.
pub const COMPENSATION_DELAY_SAMPLES_PARAM: &str = "delay_samples";
/// Audio input port name of a compensation-delay node.
pub const COMPENSATION_INPUT_PORT: &str = "audio_in";
/// Audio output port name of a compensation-delay node.
pub const COMPENSATION_OUTPUT_PORT: &str = "audio_out";
/// Prefix of a synthesised compensation node's namespaced identity. Chosen to
/// stay clear of author-supplied node ids.
pub const COMPENSATION_NODE_PREFIX: &str = "__comp";

/// A compensation delay the balancer inserted on one connection, recorded so
/// callers and discovery output can see where alignment cost was paid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompensationInsertion {
    node: NodeId,
    source: PortRef,
    destination: PortRef,
    samples: u32,
    channels: u32,
}

impl CompensationInsertion {
    /// Identity of the inserted compensation node.
    pub fn node(&self) -> &NodeId {
        &self.node
    }

    /// The original source port whose signal is now delayed.
    pub fn source(&self) -> &PortRef {
        &self.source
    }

    /// The original destination port now fed through the delay.
    pub fn destination(&self) -> &PortRef {
        &self.destination
    }

    /// Samples of delay inserted.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Channel count of the delayed signal.
    pub fn channels(&self) -> u32 {
        self.channels
    }
}

/// A flattened graph after latency balancing: original nodes plus inserted
/// compensation nodes, rewired connections, the recorded insertions, the total
/// root latency, and the advisory diagnostics describing the insertions.
#[derive(Clone, Debug, PartialEq)]
pub struct BalancedGraph {
    nodes: Vec<AtomicNode>,
    connections: Vec<Connection>,
    compensations: Vec<CompensationInsertion>,
    total_latency: u32,
    diagnostics: Diagnostics,
}

impl BalancedGraph {
    pub fn nodes(&self) -> &[AtomicNode] {
        &self.nodes
    }

    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub fn compensations(&self) -> &[CompensationInsertion] {
        &self.compensations
    }

    /// The root graph's total processing latency in samples, for host plugin
    /// latency reporting.
    pub fn total_latency(&self) -> u32 {
        self.total_latency
    }

    /// Advisory (`Info`) diagnostics, one per inserted compensation delay.
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}

impl FlattenedGraph {
    /// Balance parallel-path latency, inserting compensation delays and
    /// computing total root latency. Fails with structured diagnostics when a
    /// feedback cycle contains a nonzero-latency node, or when a residual cycle
    /// with no `feedback_delay` cut remains.
    pub fn balance_latency(&self) -> Result<BalancedGraph, Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        self.reject_latency_in_feedback_cycles(&mut diagnostics);
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }

        let Some(order) = self.dependency_topo_order() else {
            diagnostics.push(Diagnostic::new(
                error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY,
                Severity::Error,
                format!(
                    "flattened graph contains a routing cycle with no '{FEEDBACK_DELAY_DEFINITION}' node; every feedback cycle must pass through a '{FEEDBACK_DELAY_DEFINITION}' primitive"
                ),
            ));
            return Err(diagnostics);
        };

        // Accumulate audio latency in dependency order. `arrival` is the aligned
        // time all of a node's audio inputs are compensated to; `output_latency`
        // is that plus the node's own processing latency.
        let mut arrival: BTreeMap<NodeId, u32> = BTreeMap::new();
        let mut output_latency: BTreeMap<NodeId, u32> = BTreeMap::new();
        for node_id in &order {
            let node = self.node(node_id).expect("ordered id resolves to a node");
            if node.definition() == FEEDBACK_DELAY_DEFINITION {
                // A feedback_delay emits stored samples at block start: its
                // output does not depend on this block's input, so it is a
                // zero-latency source that cuts the loop.
                output_latency.insert(node_id.clone(), 0);
                continue;
            }
            let node_arrival = self
                .audio_inputs(node_id)
                .filter_map(|connection| output_latency.get(connection.source().node()).copied())
                .max()
                .unwrap_or(0);
            arrival.insert(node_id.clone(), node_arrival);
            output_latency.insert(node_id.clone(), node_arrival + node.latency());
        }

        let (nodes, connections, compensations) =
            self.insert_compensations(&arrival, &output_latency, &mut diagnostics);
        let total_latency = self.total_root_latency(&output_latency);

        Ok(BalancedGraph {
            nodes,
            connections,
            compensations,
            total_latency,
            diagnostics,
        })
    }

    /// Rebuild the node and connection lists, splicing a compensation delay into
    /// each shorter incoming audio edge at a convergence.
    fn insert_compensations(
        &self,
        arrival: &BTreeMap<NodeId, u32>,
        output_latency: &BTreeMap<NodeId, u32>,
        diagnostics: &mut Diagnostics,
    ) -> (Vec<AtomicNode>, Vec<Connection>, Vec<CompensationInsertion>) {
        let mut nodes = self.nodes().to_vec();
        let mut connections = Vec::with_capacity(self.connections().len());
        let mut compensations = Vec::new();

        for connection in self.connections() {
            let destination = connection.destination().node();
            let target = arrival.get(destination).copied();
            let source_latency = output_latency
                .get(connection.source().node())
                .copied()
                .unwrap_or(0);
            let is_audio = self.port_signal_type(connection.source()) == Some(SignalType::Audio);

            let deficit = match target {
                Some(target) if is_audio && target > source_latency => target - source_latency,
                _ => {
                    connections.push(connection.clone());
                    continue;
                }
            };

            let channels = self.port_channels(connection.source()).unwrap_or(1);
            let node_id = NodeId::new(format!("{COMPENSATION_NODE_PREFIX}{}", compensations.len()));

            nodes.push(AtomicNode::compensation(node_id.clone(), channels, deficit));
            connections.push(Connection::new(
                connection.source().clone(),
                PortRef::new(node_id.clone(), COMPENSATION_INPUT_PORT),
            ));
            connections.push(Connection::new(
                PortRef::new(node_id.clone(), COMPENSATION_OUTPUT_PORT),
                connection.destination().clone(),
            ));

            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_COMPENSATION_INSERTED,
                    Severity::Info,
                    format!(
                        "inserted {deficit}-sample latency compensation on {}.{} -> {}.{} ({channels} channel(s))",
                        connection.source().node().as_str(),
                        connection.source().port(),
                        connection.destination().node().as_str(),
                        connection.destination().port(),
                    ),
                )
                .with_module_id(node_id.as_str()),
            );

            compensations.push(CompensationInsertion {
                node: node_id,
                source: connection.source().clone(),
                destination: connection.destination().clone(),
                samples: deficit,
                channels,
            });
        }

        (nodes, connections, compensations)
    }

    /// The total accumulated latency at the root output ports: the maximum
    /// output latency across every atomic source feeding any root output.
    fn total_root_latency(&self, output_latency: &BTreeMap<NodeId, u32>) -> u32 {
        self.root_output_sources()
            .values()
            .flatten()
            .filter_map(|source| output_latency.get(source.node()).copied())
            .max()
            .unwrap_or(0)
    }

    /// Order nodes so that every audio-carrying dependency precedes its
    /// dependants, cutting `feedback_delay` inputs. Returns `None` when a cycle
    /// remains after the cuts (a cycle with no `feedback_delay`).
    fn dependency_topo_order(&self) -> Option<Vec<NodeId>> {
        let mut successors: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        let mut indegree: BTreeMap<NodeId, usize> = BTreeMap::new();
        for node in self.nodes() {
            indegree.entry(node.id().clone()).or_insert(0);
            successors.entry(node.id().clone()).or_default();
        }

        for connection in self.dependency_edges() {
            let source = connection.source().node().clone();
            let destination = connection.destination().node().clone();
            if successors
                .entry(source)
                .or_default()
                .insert(destination.clone())
            {
                *indegree.entry(destination).or_insert(0) += 1;
            }
        }

        let mut ready: Vec<NodeId> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut order = Vec::with_capacity(self.nodes().len());
        while let Some(node_id) = ready.pop() {
            order.push(node_id.clone());
            for successor in &successors[&node_id] {
                let degree = indegree
                    .get_mut(successor)
                    .expect("successor has an indegree entry");
                *degree -= 1;
                if *degree == 0 {
                    ready.push(successor.clone());
                }
            }
        }

        (order.len() == self.nodes().len()).then_some(order)
    }

    /// Fail compilation when a feedback cycle contains any nonzero-latency node:
    /// compensation cannot be inserted inside a loop, so such a topology would
    /// smear rather than delay.
    fn reject_latency_in_feedback_cycles(&self, diagnostics: &mut Diagnostics) {
        for component in self.strongly_connected_components() {
            if component.len() < 2 {
                continue;
            }
            let has_feedback_delay = component
                .iter()
                .any(|id| self.definition_of(id) == Some(FEEDBACK_DELAY_DEFINITION));
            if !has_feedback_delay {
                // A cycle with no feedback_delay is illegal for a different
                // reason; `dependency_topo_order` reports it.
                continue;
            }
            for node_id in &component {
                let node = self.node(node_id).expect("component id resolves to a node");
                if node.definition() != FEEDBACK_DELAY_DEFINITION && node.latency() > 0 {
                    let printable = component
                        .iter()
                        .map(NodeId::as_str)
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE,
                            Severity::Error,
                            format!(
                                "feedback cycle {printable} contains node '{}' with {} sample(s) of latency; latency cannot be compensated inside a feedback loop",
                                node_id.as_str(),
                                node.latency()
                            ),
                        )
                        .with_module_id(node_id.as_str())
                        .with_actual(node.latency().to_string()),
                    );
                }
            }
        }
    }

    /// Connections that carry a scheduling dependency: every connection except
    /// those feeding a `feedback_delay` input, which the scheduler cuts.
    fn dependency_edges(&self) -> impl Iterator<Item = &Connection> {
        self.connections().iter().filter(|connection| {
            self.definition_of(connection.destination().node()) != Some(FEEDBACK_DELAY_DEFINITION)
        })
    }

    /// The incoming audio connections of a node (source port carries audio).
    fn audio_inputs<'a>(&'a self, node_id: &'a NodeId) -> impl Iterator<Item = &'a Connection> {
        self.connections().iter().filter(move |connection| {
            connection.destination().node() == node_id
                && self.port_signal_type(connection.source()) == Some(SignalType::Audio)
        })
    }

    fn definition_of(&self, node_id: &NodeId) -> Option<&str> {
        self.node(node_id).map(AtomicNode::definition)
    }

    fn port_signal_type(&self, reference: &PortRef) -> Option<SignalType> {
        self.resolved_port(reference).map(|port| port.signal_type())
    }

    fn port_channels(&self, reference: &PortRef) -> Option<u32> {
        self.resolved_port(reference).map(|port| port.channels())
    }

    fn resolved_port(&self, reference: &PortRef) -> Option<&super::ResolvedPort> {
        self.node(reference.node())?
            .ports()
            .iter()
            .find(|port| port.name() == reference.port())
    }

    /// Tarjan's strongly-connected components over the full connection graph
    /// (all edges, including `feedback_delay` inputs). Each SCC of size ≥ 2 is a
    /// routing cycle.
    fn strongly_connected_components(&self) -> Vec<Vec<NodeId>> {
        let mut successors: BTreeMap<&NodeId, BTreeSet<&NodeId>> = BTreeMap::new();
        for node in self.nodes() {
            successors.entry(node.id()).or_default();
        }
        for connection in self.connections() {
            successors
                .entry(connection.source().node())
                .or_default()
                .insert(connection.destination().node());
        }

        let mut state = TarjanState {
            successors: &successors,
            index: 0,
            indices: BTreeMap::new(),
            lowlink: BTreeMap::new(),
            on_stack: BTreeSet::new(),
            stack: Vec::new(),
            components: Vec::new(),
        };
        for node in self.nodes() {
            if !state.indices.contains_key(node.id()) {
                state.connect(node.id());
            }
        }
        state.components
    }
}

/// Working state for a single Tarjan SCC traversal.
struct TarjanState<'a> {
    successors: &'a BTreeMap<&'a NodeId, BTreeSet<&'a NodeId>>,
    index: usize,
    indices: BTreeMap<&'a NodeId, usize>,
    lowlink: BTreeMap<&'a NodeId, usize>,
    on_stack: BTreeSet<&'a NodeId>,
    stack: Vec<&'a NodeId>,
    components: Vec<Vec<NodeId>>,
}

impl<'a> TarjanState<'a> {
    fn connect(&mut self, node: &'a NodeId) {
        self.indices.insert(node, self.index);
        self.lowlink.insert(node, self.index);
        self.index += 1;
        self.stack.push(node);
        self.on_stack.insert(node);

        for successor in &self.successors[node] {
            if !self.indices.contains_key(successor) {
                self.connect(successor);
                let low = self.lowlink[successor];
                let entry = self.lowlink.get_mut(node).expect("node has a lowlink");
                *entry = (*entry).min(low);
            } else if self.on_stack.contains(successor) {
                let successor_index = self.indices[successor];
                let entry = self.lowlink.get_mut(node).expect("node has a lowlink");
                *entry = (*entry).min(successor_index);
            }
        }

        if self.lowlink[node] == self.indices[node] {
            let mut component = Vec::new();
            while let Some(member) = self.stack.pop() {
                self.on_stack.remove(member);
                component.push(member.clone());
                if member == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}

#[cfg(test)]
mod tests;
