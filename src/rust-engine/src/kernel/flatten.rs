//! Recursive flattening of a kernel graph definition into a flat graph of
//! atomic (Rust primitive) nodes (see `unify-graph-kernel` §2.1, §2.2).
//!
//! Composite nodes are expanded until only atomic nodes remain. Public ports of
//! a composite forward to/gather from internal ports via each port's
//! `maps_to`/`maps_from`, so the flattened connections join atomic ports
//! directly. Node identities are namespaced by their composite instance path,
//! making expansion deterministic.
//!
//! Expansion is cached: a definition is structurally expanded once per distinct
//! `(definition, resolved static arguments)` key, producing a [`Template`] with
//! ids relative to the definition's own frame. Each instantiation re-namespaces
//! the template under its instance path — so repeated instances share one
//! expansion structure while, at runtime, each receives disjoint state.
//! Per-instance control-default overrides are applied after instantiation
//! through the resolved boundary interface, so they never contaminate the
//! cached structure.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};
use crate::graph::{PortDirection, SignalType};

use super::{
    Connection, DefinitionRegistry, GraphDefinition, NAMESPACE_SEPARATOR, Node, NodeId, PortRef,
    ResolvedPort, StaticValue,
};

/// Maximum composite nesting depth before flattening bails out. Guards against
/// pathologically deep (non-recursive) definition chains.
pub const MAX_FLATTEN_DEPTH: usize = 64;

/// An atomic node in the flattened graph: a resolved instance of a Rust
/// primitive with a namespaced identity, resolved ports, effective control
/// defaults, and processing latency.
#[derive(Clone, Debug, PartialEq)]
pub struct AtomicNode {
    id: NodeId,
    definition: String,
    static_args: BTreeMap<String, StaticValue>,
    port_defaults: BTreeMap<String, f64>,
    ports: Vec<ResolvedPort>,
    latency: u32,
}

impl AtomicNode {
    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }

    pub fn static_args(&self) -> &BTreeMap<String, StaticValue> {
        &self.static_args
    }

    pub fn port_defaults(&self) -> &BTreeMap<String, f64> {
        &self.port_defaults
    }

    pub fn ports(&self) -> &[ResolvedPort] {
        &self.ports
    }

    pub fn latency(&self) -> u32 {
        self.latency
    }
}

/// The result of flattening: a flat list of atomic nodes, the connections
/// between their ports, and the resolved public ports of the root definition.
#[derive(Clone, Debug, PartialEq)]
pub struct FlattenedGraph {
    nodes: Vec<AtomicNode>,
    connections: Vec<Connection>,
    root_ports: Vec<ResolvedPort>,
    expansion_count: usize,
}

impl FlattenedGraph {
    pub fn nodes(&self) -> &[AtomicNode] {
        &self.nodes
    }

    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub fn root_ports(&self) -> &[ResolvedPort] {
        &self.root_ports
    }

    pub fn node(&self, id: &NodeId) -> Option<&AtomicNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// The number of distinct `(definition, static arguments)` keys structurally
    /// expanded — i.e. cache misses. Repeated identical instances do not
    /// increase this count.
    pub fn expansion_count(&self) -> usize {
        self.expansion_count
    }
}

/// The externally visible ports of an expanded subtree: each public port name
/// resolved to the atomic port(s) it forwards to (inputs) or gathers from
/// (outputs), with ids relative to the subtree's own frame.
#[derive(Clone, Debug, Default)]
struct Interface {
    inputs: BTreeMap<String, Vec<PortRef>>,
    outputs: BTreeMap<String, Vec<PortRef>>,
}

/// A cached structural expansion of one definition, with node ids and port
/// references relative to the definition's own frame (no instance prefix).
#[derive(Clone, Debug)]
struct Template {
    nodes: Vec<AtomicNode>,
    connections: Vec<Connection>,
    interface: Interface,
}

/// Compilation state threaded through the recursive expansion.
struct Compiler<'a> {
    registry: &'a DefinitionRegistry,
    diagnostics: Diagnostics,
    cache: BTreeMap<String, Template>,
    expansions: usize,
    /// Definition names currently on the expansion stack, for recursion checks.
    path: Vec<String>,
    /// Definition names whose static references have been checked already.
    checked: BTreeSet<String>,
}

impl GraphDefinition {
    /// Recursively flatten this definition (as the root) into a graph of atomic
    /// nodes, or return the accumulated diagnostics on failure.
    pub fn flatten(&self, registry: &DefinitionRegistry) -> Result<FlattenedGraph, Diagnostics> {
        let context = self.enclosing_context();
        let mut compiler = Compiler {
            registry,
            diagnostics: Diagnostics::new(),
            cache: BTreeMap::new(),
            expansions: 0,
            path: Vec::new(),
            checked: BTreeSet::new(),
        };

        let template = compiler.resolve_expansion(self, &context, 0);

        if compiler.diagnostics.has_errors() {
            return Err(compiler.diagnostics);
        }

        let template = template.expect("no errors implies a template");
        let (nodes, connections, _interface) = instantiate(&template, "");
        let root_ports = GraphDefinition::resolve_ports(self, &context);
        Ok(FlattenedGraph {
            nodes,
            connections,
            root_ports,
            expansion_count: compiler.expansions,
        })
    }
}

impl Compiler<'_> {
    /// Expand `definition` with the given resolved static context into a cached
    /// template. Returns `None` when a guard (recursion, depth) fires.
    fn resolve_expansion(
        &mut self,
        definition: &GraphDefinition,
        static_context: &BTreeMap<String, StaticValue>,
        depth: usize,
    ) -> Option<Template> {
        let key = cache_key(definition.name(), static_context);
        if let Some(template) = self.cache.get(&key) {
            return Some(template.clone());
        }

        // Reject dangling channel/latency static references loudly, before any
        // resolution can silently fall back to a default value.
        if self.checked.insert(definition.name().to_string()) {
            definition.validate_static_references(&mut self.diagnostics);
        }

        // A definition with no internal nodes is an atomic primitive.
        if definition.nodes().is_empty() {
            self.expansions += 1;
            let template = definition.atomic_template(static_context);
            self.cache.insert(key, template.clone());
            return Some(template);
        }

        if depth > MAX_FLATTEN_DEPTH {
            self.diagnostics.push(Diagnostic::new(
                error_codes::KERNEL_MAX_DEPTH_EXCEEDED,
                Severity::Error,
                format!(
                    "definition '{}' exceeds the maximum composite nesting depth of {MAX_FLATTEN_DEPTH}",
                    definition.name()
                ),
            ));
            return None;
        }

        if self.path.iter().any(|name| name == definition.name()) {
            self.diagnostics.push(Diagnostic::new(
                error_codes::KERNEL_RECURSIVE_DEFINITION,
                Severity::Error,
                format!(
                    "definition '{}' is recursive: it instantiates itself directly or transitively",
                    definition.name()
                ),
            ));
            return None;
        }

        self.expansions += 1;
        self.path.push(definition.name().to_string());
        let template = self.expand_body(definition, static_context, depth);
        self.path.pop();

        if let Some(template) = &template {
            self.cache.insert(key, template.clone());
        }
        template
    }

    /// Expand a definition's internal nodes and connections into a relative-frame
    /// template.
    fn expand_body(
        &mut self,
        definition: &GraphDefinition,
        static_context: &BTreeMap<String, StaticValue>,
        depth: usize,
    ) -> Option<Template> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();
        let mut child_interfaces: BTreeMap<NodeId, Interface> = BTreeMap::new();

        for node in definition.nodes() {
            let Some(referenced) = self.registry.get(node.definition_ref()) else {
                self.diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_UNKNOWN_DEFINITION,
                        Severity::Error,
                        format!(
                            "node '{}' references unknown definition '{}'",
                            node.id().as_str(),
                            node.definition_ref()
                        ),
                    )
                    .with_module_id(node.id().as_str()),
                );
                continue;
            };

            let Some(child_args) = definition.resolve_static_args(
                node,
                referenced,
                static_context,
                &mut self.diagnostics,
            ) else {
                continue;
            };

            let Some(child_template) = self.resolve_expansion(referenced, &child_args, depth + 1)
            else {
                continue;
            };

            let (mut child_nodes, child_connections, child_interface) =
                instantiate(&child_template, node.id().as_str());
            apply_overrides(node, &child_interface, &mut child_nodes);

            nodes.extend(child_nodes);
            connections.extend(child_connections);
            child_interfaces.insert(node.id().clone(), child_interface);
        }

        for connection in definition.connections() {
            let sources = child_interfaces
                .get(connection.source().node())
                .and_then(|interface| interface.outputs.get(connection.source().port()));
            let destinations = child_interfaces
                .get(connection.destination().node())
                .and_then(|interface| interface.inputs.get(connection.destination().port()));

            if let (Some(sources), Some(destinations)) = (sources, destinations) {
                for source in sources {
                    for destination in destinations {
                        connections.push(Connection::new(source.clone(), destination.clone()));
                    }
                }
            }
        }

        let interface = compose_interface(definition, &child_interfaces);
        Some(Template {
            nodes,
            connections,
            interface,
        })
    }
}

/// Re-namespace a template under an instance `prefix`, yielding the concrete
/// atomic nodes, connections, and boundary interface for that instance.
fn instantiate(
    template: &Template,
    prefix: &str,
) -> (Vec<AtomicNode>, Vec<Connection>, Interface) {
    let nodes = template
        .nodes
        .iter()
        .map(|node| AtomicNode {
            id: NodeId::new(namespaced(prefix, node.id.as_str())),
            definition: node.definition.clone(),
            static_args: node.static_args.clone(),
            port_defaults: node.port_defaults.clone(),
            ports: node.ports.clone(),
            latency: node.latency,
        })
        .collect();

    let connections = template
        .connections
        .iter()
        .map(|connection| {
            Connection::new(
                renamespace(connection.source(), prefix),
                renamespace(connection.destination(), prefix),
            )
        })
        .collect();

    let interface = Interface {
        inputs: renamespace_map(&template.interface.inputs, prefix),
        outputs: renamespace_map(&template.interface.outputs, prefix),
    };

    (nodes, connections, interface)
}

/// Apply a node instance's control-default overrides onto the atomic nodes its
/// public input ports resolve to. Overrides of unknown ports are ignored here
/// (validation reports them separately).
fn apply_overrides(node: &Node, interface: &Interface, nodes: &mut [AtomicNode]) {
    for (port_name, value) in node.port_default_overrides() {
        let Some(targets) = interface.inputs.get(port_name) else {
            continue;
        };
        for target in targets {
            if let Some(atomic) = nodes.iter_mut().find(|atomic| atomic.id == *target.node()) {
                atomic
                    .port_defaults
                    .insert(target.port().to_string(), *value);
            }
        }
    }
}

fn compose_interface(
    definition: &GraphDefinition,
    child_interfaces: &BTreeMap<NodeId, Interface>,
) -> Interface {
    let mut interface = Interface::default();
    for port in definition.ports() {
        match port.direction() {
            PortDirection::Input => {
                let mut targets = Vec::new();
                for internal in port.internal_targets() {
                    if let Some(child) = child_interfaces.get(internal.node()) {
                        if let Some(refs) = child.inputs.get(internal.port()) {
                            targets.extend(refs.iter().cloned());
                        }
                    }
                }
                interface.inputs.insert(port.name().to_string(), targets);
            }
            PortDirection::Output => {
                let mut sources = Vec::new();
                for internal in port.internal_sources() {
                    if let Some(child) = child_interfaces.get(internal.node()) {
                        if let Some(refs) = child.outputs.get(internal.port()) {
                            sources.extend(refs.iter().cloned());
                        }
                    }
                }
                interface.outputs.insert(port.name().to_string(), sources);
            }
        }
    }
    interface
}

impl GraphDefinition {
    /// Build the template for a single atomic (primitive) definition: one node
    /// whose id is empty relative to its own instance frame, carrying declared
    /// control defaults only.
    fn atomic_template(&self, static_args: &BTreeMap<String, StaticValue>) -> Template {
        let ports = GraphDefinition::resolve_ports(self, static_args);
        let id = NodeId::new("");
        let interface = atomic_interface(&id, &ports);
        Template {
            nodes: vec![AtomicNode {
                id,
                definition: self.name().to_string(),
                static_args: static_args.clone(),
                port_defaults: declared_control_defaults(&ports),
                ports,
                latency: self.latency().resolve(static_args),
            }],
            connections: Vec::new(),
            interface,
        }
    }
}

fn namespaced(prefix: &str, relative: &str) -> String {
    match (prefix.is_empty(), relative.is_empty()) {
        (_, true) => prefix.to_string(),
        (true, false) => relative.to_string(),
        (false, false) => format!("{prefix}{NAMESPACE_SEPARATOR}{relative}"),
    }
}

fn renamespace(reference: &PortRef, prefix: &str) -> PortRef {
    PortRef::new(
        NodeId::new(namespaced(prefix, reference.node().as_str())),
        reference.port(),
    )
}

fn renamespace_map(
    map: &BTreeMap<String, Vec<PortRef>>,
    prefix: &str,
) -> BTreeMap<String, Vec<PortRef>> {
    map.iter()
        .map(|(name, refs)| {
            (
                name.clone(),
                refs.iter().map(|r| renamespace(r, prefix)).collect(),
            )
        })
        .collect()
}

fn atomic_interface(node_id: &NodeId, ports: &[ResolvedPort]) -> Interface {
    let mut interface = Interface::default();
    for port in ports {
        let reference = vec![PortRef::new(node_id.clone(), port.name())];
        match port.direction() {
            PortDirection::Input => {
                interface.inputs.insert(port.name().to_string(), reference);
            }
            PortDirection::Output => {
                interface.outputs.insert(port.name().to_string(), reference);
            }
        }
    }
    interface
}

fn declared_control_defaults(ports: &[ResolvedPort]) -> BTreeMap<String, f64> {
    ports
        .iter()
        .filter(|port| {
            port.direction() == PortDirection::Input && port.signal_type() == SignalType::Control
        })
        .filter_map(|port| {
            port.control_default()
                .map(|control| (port.name().to_string(), control.default()))
        })
        .collect()
}

fn cache_key(name: &str, static_args: &BTreeMap<String, StaticValue>) -> String {
    let mut key = name.to_string();
    for (parameter, value) in static_args {
        key.push('|');
        key.push_str(parameter);
        key.push('=');
        match value {
            StaticValue::Int(number) => key.push_str(&number.to_string()),
            StaticValue::Enum(text) => {
                key.push_str("enum:");
                key.push_str(text);
            }
            StaticValue::Resource(text) => {
                key.push_str("res:");
                key.push_str(text);
            }
        }
    }
    key
}

#[cfg(test)]
mod tests;
