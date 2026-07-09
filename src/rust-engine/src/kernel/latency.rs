//! Latency balancing over a flattened kernel graph (see `unify-graph-kernel`
//! §2.4b).
//!
//! Each atomic node declares its processing latency in samples. Where paths of
//! unequal accumulated latency converge on a node, the shorter paths must be
//! delayed so every input arrives sample-aligned; otherwise parallel dry/wet
//! topologies phase-smear. This pass accumulates declared latency along the
//! graph, produces a [`LatencyPlan`] of the compensation delays to preallocate
//! (the render pipeline inserts the actual buffers at preparation), and reports
//! the total root latency the host must advertise.
//!
//! Feedback cycles are legal only through a `feedback_delay` node, whose
//! feedback edge is cut for accumulation (its output starts a fresh latency
//! origin). Latency inside such a cycle cannot be compensated, so a cycle that
//! contains any non-`feedback_delay` node with nonzero latency is rejected with
//! a structured diagnostic.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};

use super::flatten::FlattenedGraph;
use super::{Connection, FEEDBACK_DELAY_DEFINITION, NodeId};

/// A compensation delay the compiler inserts on one connection so that its
/// destination's inputs arrive sample-aligned.
#[derive(Clone, Debug, PartialEq)]
pub struct Compensation {
    connection: Connection,
    samples: u32,
    channels: u32,
}

impl Compensation {
    /// The connection whose signal is delayed.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The delay length in samples.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// The channel count of the delayed signal (buffer width to preallocate).
    pub fn channels(&self) -> u32 {
        self.channels
    }
}

/// The outcome of balancing: the compensation delays to preallocate and the
/// total latency observed at the graph's outputs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LatencyPlan {
    compensations: Vec<Compensation>,
    root_latency: u32,
}

impl LatencyPlan {
    pub fn compensations(&self) -> &[Compensation] {
        &self.compensations
    }

    /// The latency in samples the host must report (the longest path to any
    /// graph output).
    pub fn root_latency(&self) -> u32 {
        self.root_latency
    }
}

impl FlattenedGraph {
    /// Balance latency across this flattened graph: insert compensation delays
    /// where unequal paths converge and compute total root latency, or return
    /// diagnostics when a feedback cycle contains an uncompensatable
    /// latency-bearing node.
    pub fn balance_latency(&self) -> Result<LatencyPlan, Diagnostics> {
        // A remaining forward cycle means a routing cycle that no feedback_delay
        // breaks. `validate()` also catches this, but `balance_latency` is public
        // and may run without it, so fail loudly rather than returning a plan
        // computed from an incomplete ordering.
        let order = match self.topological_order() {
            Ok(order) => order,
            Err(unordered) => {
                let mut diagnostics = Diagnostics::new();
                let printable = unordered
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY,
                        Severity::Error,
                        format!(
                            "latency balancing found a routing cycle with no '{FEEDBACK_DELAY_DEFINITION}' node among nodes: {printable}; every feedback cycle must pass through a '{FEEDBACK_DELAY_DEFINITION}' primitive"
                        ),
                    )
                    .with_suggested_fix(format!(
                        "insert a '{FEEDBACK_DELAY_DEFINITION}' node into the cycle"
                    )),
                );
                return Err(diagnostics);
            }
        };

        let mut diagnostics = Diagnostics::new();
        self.reject_latency_in_feedback_cycles(&mut diagnostics);
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }

        // Accumulate: a node's inputs arrive at the max output-latency of its
        // sources; its own output latency is that arrival plus its declared
        // latency. A feedback_delay node starts a fresh origin at zero.
        let mut arrival: BTreeMap<NodeId, u32> = BTreeMap::new();
        let mut output_latency: BTreeMap<NodeId, u32> = BTreeMap::new();
        for node_id in &order {
            if self.is_feedback_delay(node_id) {
                arrival.insert(node_id.clone(), 0);
                output_latency.insert(node_id.clone(), 0);
                continue;
            }
            let arr = self
                .incoming(node_id)
                .filter_map(|connection| output_latency.get(connection.source().node()).copied())
                .max()
                .unwrap_or(0);
            let own = self.node(node_id).map(|node| node.latency()).unwrap_or(0);
            arrival.insert(node_id.clone(), arr);
            output_latency.insert(node_id.clone(), arr + own);
        }

        // Any edge whose source arrives earlier than its destination's aligned
        // arrival needs a compensating delay of the difference.
        let mut compensations = Vec::new();
        for connection in self.connections() {
            if self.is_feedback_delay(connection.destination().node()) {
                continue; // feedback tap: not part of forward accumulation
            }
            let dest_arrival = arrival
                .get(connection.destination().node())
                .copied()
                .unwrap_or(0);
            let source_output = output_latency
                .get(connection.source().node())
                .copied()
                .unwrap_or(0);
            let samples = dest_arrival.saturating_sub(source_output);
            if samples > 0 {
                compensations.push(Compensation {
                    connection: connection.clone(),
                    samples,
                    channels: self.edge_channels(connection),
                });
            }
        }

        Ok(LatencyPlan {
            compensations,
            root_latency: self.root_latency(&output_latency),
        })
    }

    /// Total latency the host must report: the max output latency across the
    /// atomic nodes that actually feed the root output ports. Computed from the
    /// root-output interface, not terminal nodes, so a dead or unconnected
    /// latency-bearing branch never inflates it.
    fn root_latency(&self, output_latency: &BTreeMap<NodeId, u32>) -> u32 {
        self.root_output_sources()
            .values()
            .flatten()
            .filter_map(|source| output_latency.get(source.node()).copied())
            .max()
            .unwrap_or(0)
    }

    /// Reject cycles that carry uncompensatable latency: any cycle containing a
    /// non-`feedback_delay` node whose declared latency is nonzero.
    fn reject_latency_in_feedback_cycles(&self, diagnostics: &mut Diagnostics) {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut stack: Vec<NodeId> = Vec::new();
        let mut reported: BTreeSet<Vec<NodeId>> = BTreeSet::new();
        let ids: Vec<NodeId> = self.nodes().iter().map(|node| node.id().clone()).collect();
        for id in &ids {
            self.walk_for_latency_cycle(
                id,
                &mut visiting,
                &mut visited,
                &mut stack,
                &mut reported,
                diagnostics,
            );
        }
    }

    fn walk_for_latency_cycle(
        &self,
        node_id: &NodeId,
        visiting: &mut BTreeSet<NodeId>,
        visited: &mut BTreeSet<NodeId>,
        stack: &mut Vec<NodeId>,
        reported: &mut BTreeSet<Vec<NodeId>>,
        diagnostics: &mut Diagnostics,
    ) {
        if visited.contains(node_id) {
            return;
        }
        visiting.insert(node_id.clone());
        stack.push(node_id.clone());

        for successor in self.full_successors(node_id) {
            if visiting.contains(&successor) {
                let cycle: Vec<NodeId> = stack
                    .iter()
                    .skip_while(|id| **id != successor)
                    .cloned()
                    .collect();
                self.report_latency_cycle(&cycle, reported, diagnostics);
            } else if !visited.contains(&successor) {
                self.walk_for_latency_cycle(
                    &successor,
                    visiting,
                    visited,
                    stack,
                    reported,
                    diagnostics,
                );
            }
        }

        stack.pop();
        visiting.remove(node_id);
        visited.insert(node_id.clone());
    }

    fn report_latency_cycle(
        &self,
        cycle: &[NodeId],
        reported: &mut BTreeSet<Vec<NodeId>>,
        diagnostics: &mut Diagnostics,
    ) {
        let Some(offender) = cycle
            .iter()
            .find(|id| !self.is_feedback_delay(id) && self.latency_of(id) > 0)
        else {
            return; // a legal cycle: only zero-latency nodes plus feedback_delay
        };

        let mut key = cycle.to_vec();
        key.sort();
        if !reported.insert(key) {
            return; // already reported this cycle
        }

        let printable = cycle
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        diagnostics.push(
            Diagnostic::new(
                error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE,
                Severity::Error,
                format!(
                    "feedback cycle {printable} contains latency-bearing node '{}' ({} samples); latency inside a feedback cycle cannot be compensated",
                    offender.as_str(),
                    self.latency_of(offender)
                ),
            )
            .with_module_id(offender.as_str())
            .with_suggested_fix(format!(
                "move the latency-bearing node out of the feedback cycle, or keep only the '{FEEDBACK_DELAY_DEFINITION}' node's delay inside it"
            )),
        );
    }

    fn latency_of(&self, node_id: &NodeId) -> u32 {
        self.node(node_id).map(|node| node.latency()).unwrap_or(0)
    }

    fn is_feedback_delay(&self, node_id: &NodeId) -> bool {
        self.node(node_id)
            .is_some_and(|node| node.definition() == FEEDBACK_DELAY_DEFINITION)
    }

    /// Connections arriving at `node_id`, excluding feedback taps.
    fn incoming(&self, node_id: &NodeId) -> impl Iterator<Item = &Connection> {
        self.connections()
            .iter()
            .filter(move |connection| connection.destination().node() == node_id)
    }

    /// Distinct destination nodes reachable by one connection from `node_id`,
    /// including feedback edges (used for cycle detection).
    fn full_successors(&self, node_id: &NodeId) -> Vec<NodeId> {
        let mut seen = BTreeSet::new();
        self.connections()
            .iter()
            .filter(|connection| connection.source().node() == node_id)
            .filter_map(|connection| {
                let destination = connection.destination().node().clone();
                seen.insert(destination.clone()).then_some(destination)
            })
            .collect()
    }

    /// Channel count of a connection's source port (the compensation buffer
    /// width), or zero when the port is unresolved.
    fn edge_channels(&self, connection: &Connection) -> u32 {
        self.node(connection.source().node())
            .and_then(|node| {
                node.ports()
                    .iter()
                    .find(|port| port.name() == connection.source().port())
            })
            .map(|port| port.channels())
            .unwrap_or(0)
    }

    /// A deterministic topological order over forward edges (edges into a
    /// `feedback_delay` node are cut). Returns `Err` with the nodes that could
    /// not be ordered when a non-feedback cycle remains.
    fn topological_order(&self) -> Result<Vec<NodeId>, Vec<NodeId>> {
        let mut indegree: BTreeMap<NodeId, usize> =
            self.nodes().iter().map(|node| (node.id().clone(), 0)).collect();
        for connection in self.connections() {
            if self.is_feedback_delay(connection.destination().node()) {
                continue;
            }
            if let Some(degree) = indegree.get_mut(connection.destination().node()) {
                *degree += 1;
            }
        }

        // BTreeMap iteration is sorted, so the seed order (and thus the whole
        // traversal) is deterministic.
        let mut queue: Vec<NodeId> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut order = Vec::new();
        let mut index = 0;
        while index < queue.len() {
            let id = queue[index].clone();
            index += 1;
            order.push(id.clone());
            for connection in self.connections() {
                if self.is_feedback_delay(connection.destination().node()) {
                    continue;
                }
                if connection.source().node() != &id {
                    continue;
                }
                if let Some(degree) = indegree.get_mut(connection.destination().node()) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(connection.destination().node().clone());
                    }
                }
            }
        }

        if order.len() == indegree.len() {
            return Ok(order);
        }
        let ordered: BTreeSet<&NodeId> = order.iter().collect();
        let unordered = self
            .nodes()
            .iter()
            .map(|node| node.id().clone())
            .filter(|id| !ordered.contains(id))
            .collect();
        Err(unordered)
    }
}

#[cfg(test)]
mod tests;
