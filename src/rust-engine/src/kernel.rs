//! Unified recursive graph kernel (see `unify-graph-kernel` change).
//!
//! This module introduces the kernel type model beside the legacy
//! [`crate::graph`] model. A [`GraphDefinition`] declares static parameters,
//! public ports, internal nodes, and connections; a [`Node`] is an instance of
//! another graph definition. Primitives are graph definitions implemented in
//! Rust; composites are graph definitions authored in YAML. Both expose the
//! same public interface (ports and static parameters) and are validated
//! through one path.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};
use crate::graph::{PortDirection, SignalType};

/// Definition name of the feedback-delay primitive: the only node through which
/// a routing cycle (audio or control) is legal.
pub const FEEDBACK_DELAY_DEFINITION: &str = "feedback_delay";

/// Separator between a composite instance identity and an internal node
/// identity when flattening produces namespaced atomic node ids.
pub const NAMESPACE_SEPARATOR: &str = "::";

/// Definition name of the compiler-generated node that promotes a `control`
/// signal to `audio` where a control output feeds an audio input. Flattening
/// inserts one so the promotion is a visible, inspectable node rather than an
/// implicit conversion.
pub const CONTROL_TO_AUDIO_DEFINITION: &str = crate::builtins::module_types::CONTROL_TO_AUDIO;

/// Identity prefix for compiler-generated control→audio promotion nodes.
pub const PROMOTION_NODE_PREFIX: &str = "promote";

/// Input (control) port name on a control→audio promotion node.
pub const PROMOTION_INPUT_PORT: &str = crate::graph::builtin_ports::IN;

/// Output (audio) port name on a control→audio promotion node.
pub const PROMOTION_OUTPUT_PORT: &str = crate::graph::builtin_ports::OUT;

/// Structural definition name for a polyphonic region.
pub const POLY_DEFINITION: &str = crate::builtins::module_types::POLY;
/// Static argument naming the graph definition instantiated for each voice.
pub const POLY_WRAPPED_DEFINITION_PARAM: &str = "definition";
/// Static argument setting the region's preallocated voice capacity.
pub const POLY_MAX_VOICES_PARAM: &str = "max_voices";
/// Static argument selecting the voice allocation policy.
pub const POLY_ALLOCATION_PARAM: &str = "allocation";
pub const POLY_ALLOCATION_OLDEST_STEAL: &str = "oldest-steal";
pub const POLY_ALLOCATION_REJECT_NEW: &str = "reject-new";
/// Event input receiving note-on and note-off events for allocation.
pub const POLY_NOTE_EVENTS_INPUT: &str = "notes";
/// Optional wrapped voice lifecycle output consumed by the poly region rather
/// than exposed as a summed public output.
pub const POLY_DONE_OUTPUT: &str = "done";
pub const POLY_MIN_VOICES: i64 = 1;
pub const POLY_MAX_VOICES: i64 = u32::MAX as i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyAllocationPolicy {
    OldestSteal,
    RejectNew,
}

impl PolyAllocationPolicy {
    pub fn from_static_value(value: &StaticValue) -> Option<Self> {
        match value {
            StaticValue::Enum(value) if value == POLY_ALLOCATION_OLDEST_STEAL => {
                Some(Self::OldestSteal)
            }
            StaticValue::Enum(value) if value == POLY_ALLOCATION_REJECT_NEW => {
                Some(Self::RejectNew)
            }
            _ => None,
        }
    }
}

/// Selects how a named graph definition is implemented. Ordinary definitions
/// retain graph semantics; script-backed definitions are atomic and lower to
/// the existing Rust script primitive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DefinitionImplementation {
    #[default]
    Graph,
    Script,
}

/// Input multiplicity: whether an input port accepts a single source or
/// arbitrarily many sources that the runtime sums.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Multiplicity {
    /// At most one incoming connection. The default for all inputs.
    #[default]
    SingleSource,
    /// Any number of incoming connections; the runtime sums them.
    Summing,
}

pub mod builtins;
pub mod document;
pub mod flatten;
pub mod latency;

/// The static (compile-time) type of a graph-definition static parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticType {
    /// Integer value (e.g. a channel count or voice count).
    Int,
    /// Named enumeration value.
    Enum,
    /// Arbitrary construction-time text, such as inline script source.
    String,
    /// Reference to an external resource of a declared semantic kind.
    Resource(ResourceKind),
}

/// Semantic kind of immutable resource accepted by a static parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    Sample,
    ImpulseResponse,
}

impl ResourceKind {
    pub const SAMPLE_NAME: &str = "sample";
    pub const IMPULSE_RESPONSE_NAME: &str = "impulse_response";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sample => Self::SAMPLE_NAME,
            Self::ImpulseResponse => Self::IMPULSE_RESPONSE_NAME,
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Scope in which a resource literal was authored. Preparation maps a document
/// origin to its current document root; package loading records the concrete
/// package-version root directly.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceOrigin {
    Document,
    Package(PathBuf),
}

/// Typed unresolved resource literal retained through static pass-through and
/// flattening until preparation resolves it to a shared immutable handle.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceRef {
    kind: ResourceKind,
    path: PathBuf,
    origin: ResourceOrigin,
}

impl ResourceRef {
    pub fn new(kind: ResourceKind, path: impl Into<PathBuf>, origin: ResourceOrigin) -> Self {
        Self {
            kind,
            path: path.into(),
            origin,
        }
    }

    pub fn kind(&self) -> ResourceKind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn origin(&self) -> &ResourceOrigin {
        &self.origin
    }
}

/// A resolved value supplied for a static parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StaticValue {
    Int(i64),
    Enum(String),
    String(String),
    Resource(ResourceRef),
}

impl StaticValue {
    /// The [`StaticType`] this value satisfies.
    pub fn static_type(&self) -> StaticType {
        match self {
            Self::Int(_) => StaticType::Int,
            Self::Enum(_) => StaticType::Enum,
            Self::String(_) => StaticType::String,
            Self::Resource(reference) => StaticType::Resource(reference.kind()),
        }
    }
}

/// How an atomic node's processing latency (in samples) is determined after
/// static-argument resolution. Most primitives are [`LatencySpec::Zero`];
/// lookahead, FFT, spectral, and convolution processing declare real latency.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum LatencySpec {
    #[default]
    Zero,
    /// A fixed number of samples independent of static arguments.
    Samples(u32),
    /// Latency derived from an integer static parameter minus a constant,
    /// e.g. a spectral processor's `fft_size - 1`.
    StaticParam { name: String, minus: u32 },
}

impl LatencySpec {
    /// Resolve the latency in samples against a node's resolved static arguments.
    pub fn resolve(&self, static_args: &BTreeMap<String, StaticValue>) -> u32 {
        match self {
            Self::Zero => 0,
            Self::Samples(samples) => *samples,
            Self::StaticParam { name, minus } => match static_args.get(name) {
                Some(StaticValue::Int(value)) if *value >= 0 => {
                    (*value as u32).saturating_sub(*minus)
                }
                // Unreachable in a compiled graph: `validate_static_references`
                // rejects any latency reference that is not a declared integer
                // static parameter, and `validate_resolved_static_references`
                // rejects negative values and values smaller than `minus`, so this
                // never silently reports (or saturates to) zero latency for a
                // latency-bearing node — compilation fails loudly instead.
                _ => 0,
            },
        }
    }
}

/// A static parameter declared on a graph definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticParam {
    name: String,
    static_type: StaticType,
    default: Option<StaticValue>,
    allowed_values: Vec<String>,
}

impl StaticParam {
    pub fn new(name: impl Into<String>, static_type: StaticType) -> Self {
        Self {
            name: name.into(),
            static_type,
            default: None,
            allowed_values: Vec::new(),
        }
    }

    pub fn with_default(mut self, default: StaticValue) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_allowed_values<T, I>(mut self, values: I) -> Self
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        self.allowed_values = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn static_type(&self) -> StaticType {
        self.static_type
    }

    pub fn default(&self) -> Option<&StaticValue> {
        self.default.as_ref()
    }

    pub fn allowed_values(&self) -> &[String] {
        &self.allowed_values
    }
}

/// A static argument supplied by a node for a referenced definition's static
/// parameter. Arguments flow by literal value or by name from an enclosing
/// definition's static parameters; there is no expression language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StaticArg {
    /// A concrete literal value.
    Literal(StaticValue),
    /// A pass-through reference (`$name`) to an enclosing static parameter.
    ParamRef(String),
    /// Any other form (arithmetic, conditionals, functions); always rejected.
    Expression(String),
}

/// The channel count of a port: a literal, or a reference to one of the
/// definition's static parameters resolved before connection validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelCount {
    Literal(u32),
    Param(String),
}

impl ChannelCount {
    /// The minimum legal resolved channel count. A resolved static argument of
    /// zero or a negative value is rejected at compile time rather than falling
    /// back to a plausible mono default.
    pub const MIN: i64 = 1;

    pub fn param(name: impl Into<String>) -> Self {
        Self::Param(name.into())
    }
}

impl From<u32> for ChannelCount {
    fn from(value: u32) -> Self {
        Self::Literal(value)
    }
}

/// Default value and optional range metadata for a `control` input port.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlDefault {
    default: f64,
    min: Option<f64>,
    max: Option<f64>,
    unit: Option<String>,
}

impl ControlDefault {
    pub fn new(default: f64) -> Self {
        Self {
            default,
            min: None,
            max: None,
            unit: None,
        }
    }

    pub fn with_min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn default(&self) -> f64 {
        self.default
    }

    pub fn min(&self) -> Option<f64> {
        self.min
    }

    pub fn max(&self) -> Option<f64> {
        self.max
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
}

/// A named, typed, channel-counted port on a graph definition.
///
/// On a composite definition, a public input port forwards inbound signal to
/// the internal ports named in `maps_to`, and a public output port gathers from
/// the internal ports named in `maps_from`. Primitive (atomic) ports leave both
/// empty.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    name: String,
    direction: PortDirection,
    signal_type: SignalType,
    channels: ChannelCount,
    multiplicity: Multiplicity,
    control_default: Option<ControlDefault>,
    maps_to: Vec<PortRef>,
    maps_from: Vec<PortRef>,
}

impl Port {
    pub fn input(
        name: impl Into<String>,
        signal_type: SignalType,
        channels: impl Into<ChannelCount>,
    ) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Input,
            signal_type,
            channels: channels.into(),
            multiplicity: Multiplicity::default(),
            control_default: None,
            maps_to: Vec::new(),
            maps_from: Vec::new(),
        }
    }

    pub fn output(
        name: impl Into<String>,
        signal_type: SignalType,
        channels: impl Into<ChannelCount>,
    ) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Output,
            signal_type,
            channels: channels.into(),
            multiplicity: Multiplicity::default(),
            control_default: None,
            maps_to: Vec::new(),
            maps_from: Vec::new(),
        }
    }

    pub fn with_control_default(mut self, control_default: ControlDefault) -> Self {
        self.control_default = Some(control_default);
        self
    }

    pub fn with_multiplicity(mut self, multiplicity: Multiplicity) -> Self {
        self.multiplicity = multiplicity;
        self
    }

    /// Forward this public input port to an internal node's input port.
    pub fn maps_to(mut self, internal: PortRef) -> Self {
        self.maps_to.push(internal);
        self
    }

    /// Gather this public output port from an internal node's output port.
    pub fn maps_from(mut self, internal: PortRef) -> Self {
        self.maps_from.push(internal);
        self
    }

    pub fn internal_targets(&self) -> &[PortRef] {
        &self.maps_to
    }

    pub fn internal_sources(&self) -> &[PortRef] {
        &self.maps_from
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn direction(&self) -> PortDirection {
        self.direction
    }

    pub fn signal_type(&self) -> SignalType {
        self.signal_type
    }

    pub fn channels(&self) -> &ChannelCount {
        &self.channels
    }

    pub fn control_default(&self) -> Option<&ControlDefault> {
        self.control_default.as_ref()
    }

    pub fn multiplicity(&self) -> Multiplicity {
        self.multiplicity
    }
}

/// Identity of a node instance within a graph definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A node instance: a reference to a graph definition, its static arguments,
/// and any per-instance overrides of port default values.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    id: NodeId,
    definition_ref: String,
    static_args: BTreeMap<String, StaticArg>,
    port_default_overrides: BTreeMap<String, f64>,
}

impl Node {
    pub fn new(id: NodeId, definition_ref: impl Into<String>) -> Self {
        Self {
            id,
            definition_ref: definition_ref.into(),
            static_args: BTreeMap::new(),
            port_default_overrides: BTreeMap::new(),
        }
    }

    pub fn with_static_arg(mut self, name: impl Into<String>, arg: StaticArg) -> Self {
        self.static_args.insert(name.into(), arg);
        self
    }

    pub fn with_default_override(mut self, port_name: impl Into<String>, value: f64) -> Self {
        self.port_default_overrides.insert(port_name.into(), value);
        self
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn definition_ref(&self) -> &str {
        &self.definition_ref
    }

    pub fn static_args(&self) -> &BTreeMap<String, StaticArg> {
        &self.static_args
    }

    pub fn port_default_overrides(&self) -> &BTreeMap<String, f64> {
        &self.port_default_overrides
    }
}

/// A reference to a named port on a node instance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortRef {
    node: NodeId,
    port: String,
}

impl PortRef {
    pub fn new(node: NodeId, port: impl Into<String>) -> Self {
        Self {
            node,
            port: port.into(),
        }
    }

    pub fn node(&self) -> &NodeId {
        &self.node
    }

    pub fn port(&self) -> &str {
        &self.port
    }
}

/// A directed connection between an output port and an input port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    source: PortRef,
    destination: PortRef,
}

impl Connection {
    pub fn new(source: PortRef, destination: PortRef) -> Self {
        Self {
            source,
            destination,
        }
    }

    pub fn source(&self) -> &PortRef {
        &self.source
    }

    pub fn destination(&self) -> &PortRef {
        &self.destination
    }
}

/// A graph definition: the single recursive unit that describes primitives,
/// composites, and complete patches alike.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphDefinition {
    name: String,
    implementation: DefinitionImplementation,
    static_params: Vec<StaticParam>,
    ports: Vec<Port>,
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    latency: LatencySpec,
}

impl GraphDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn with_static_param(mut self, param: StaticParam) -> Self {
        self.static_params.push(param);
        self
    }

    pub fn with_implementation(mut self, implementation: DefinitionImplementation) -> Self {
        self.implementation = implementation;
        self
    }

    pub fn with_port(mut self, port: Port) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_node(mut self, node: Node) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn with_connection(mut self, connection: Connection) -> Self {
        self.connections.push(connection);
        self
    }

    /// Declare this (atomic) definition's processing latency.
    pub fn with_latency(mut self, latency: LatencySpec) -> Self {
        self.latency = latency;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn implementation(&self) -> DefinitionImplementation {
        self.implementation
    }

    pub fn latency(&self) -> &LatencySpec {
        &self.latency
    }

    pub fn static_params(&self) -> &[StaticParam] {
        &self.static_params
    }

    pub fn ports(&self) -> &[Port] {
        &self.ports
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    fn static_param(&self, name: &str) -> Option<&StaticParam> {
        self.static_params.iter().find(|p| p.name() == name)
    }

    fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id() == id)
    }
}

/// A port whose channel count (and thus routing shape) has been resolved
/// against a node's static arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPort {
    name: String,
    direction: PortDirection,
    signal_type: SignalType,
    channels: u32,
    multiplicity: Multiplicity,
    control_default: Option<ControlDefault>,
}

impl ResolvedPort {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn direction(&self) -> PortDirection {
        self.direction
    }

    pub fn signal_type(&self) -> SignalType {
        self.signal_type
    }

    pub fn channels(&self) -> u32 {
        self.channels
    }

    pub fn control_default(&self) -> Option<&ControlDefault> {
        self.control_default.as_ref()
    }

    pub fn multiplicity(&self) -> Multiplicity {
        self.multiplicity
    }
}

/// A compiler-recorded step promoting a `control` output to an `audio` input.
#[derive(Clone, Debug, PartialEq)]
pub struct PromotionStep {
    source: PortRef,
    destination: PortRef,
    channels: u32,
}

impl PromotionStep {
    pub fn source(&self) -> &PortRef {
        &self.source
    }

    pub fn destination(&self) -> &PortRef {
        &self.destination
    }

    pub fn channels(&self) -> u32 {
        self.channels
    }
}

/// A node whose definition and static arguments both resolved during
/// validation. Holding the definition alongside its resolved ports means later
/// checks never have to look the definition up again — a lookup that could only
/// fail in a state this value's existence already rules out.
struct ResolvedNode<'a> {
    definition: &'a GraphDefinition,
    ports: Vec<ResolvedPort>,
}

/// The effective source of a control input port after default/override/cable
/// resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectiveInput {
    /// An incoming cable drives the port; it takes precedence over any default.
    Connected,
    /// No cable; the port reads this resolved value.
    Value(f64),
}

/// A registry of graph definitions (primitives and composites) available for
/// instantiation. Primitives are represented by their public interface
/// (ports and static parameters) with an empty body.
#[derive(Clone, Debug, Default)]
pub struct DefinitionRegistry {
    definitions: BTreeMap<String, GraphDefinition>,
}

impl DefinitionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_definition(mut self, definition: GraphDefinition) -> Self {
        self.definitions
            .insert(definition.name().to_string(), definition);
        self
    }

    pub fn get(&self, name: &str) -> Option<&GraphDefinition> {
        self.definitions.get(name)
    }

    /// Every registered definition, in name order. Capability discovery and
    /// latency audits enumerate the registry through this.
    pub fn definitions(&self) -> impl Iterator<Item = &GraphDefinition> {
        self.definitions.values()
    }
}

/// The outcome of validating a graph definition: accumulated diagnostics plus
/// the control→audio promotion steps the compiler must insert.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KernelValidation {
    diagnostics: Diagnostics,
    promotions: Vec<PromotionStep>,
}

impl KernelValidation {
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    pub fn promotions(&self) -> &[PromotionStep] {
        &self.promotions
    }

    /// True when no error-severity diagnostics were produced.
    pub fn is_ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

/// Compatibility classification for a source→destination signal-type pair.
enum SignalCompatibility {
    Same,
    PromoteControlToAudio,
    Incompatible,
}

fn classify_signal(source: SignalType, destination: SignalType) -> SignalCompatibility {
    match (source, destination) {
        (SignalType::Audio, SignalType::Audio)
        | (SignalType::Control, SignalType::Control)
        | (SignalType::Event, SignalType::Event) => SignalCompatibility::Same,
        (SignalType::Control, SignalType::Audio) => SignalCompatibility::PromoteControlToAudio,
        _ => SignalCompatibility::Incompatible,
    }
}

impl GraphDefinition {
    /// Resolve this definition's own static parameters to concrete values using
    /// their declared defaults. This is the enclosing context against which
    /// node `ParamRef` arguments resolve during validation.
    fn enclosing_context(&self) -> BTreeMap<String, StaticValue> {
        self.static_params
            .iter()
            .filter_map(|param| {
                param
                    .default()
                    .map(|v| (param.name().to_string(), v.clone()))
            })
            .collect()
    }

    /// Resolve the static arguments a node supplies for its referenced
    /// definition, validating presence, names, types, and forbidden
    /// expressions. Returns `None` (with diagnostics pushed) if resolution
    /// fails.
    fn resolve_static_args(
        &self,
        node: &Node,
        referenced: &GraphDefinition,
        enclosing: &BTreeMap<String, StaticValue>,
        diagnostics: &mut Diagnostics,
    ) -> Option<BTreeMap<String, StaticValue>> {
        let mut ok = true;

        for (name, _) in node.static_args() {
            if referenced.static_param(name).is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT,
                        Severity::Error,
                        format!(
                            "node '{}' supplies unknown static argument '{name}' for definition '{}'",
                            node.id().as_str(),
                            referenced.name()
                        ),
                    )
                    .with_module_id(node.id().as_str()),
                );
                ok = false;
            }
        }

        let mut resolved = BTreeMap::new();
        for param in referenced.static_params() {
            let value = match node.static_args().get(param.name()) {
                Some(StaticArg::Literal(value)) => Some(value.clone()),
                Some(StaticArg::ParamRef(referenced_param)) => {
                    match enclosing.get(referenced_param) {
                        Some(value) => Some(value.clone()),
                        None => {
                            diagnostics.push(
                                Diagnostic::new(
                                    error_codes::KERNEL_UNKNOWN_STATIC_PARAM_REFERENCE,
                                    Severity::Error,
                                    format!(
                                        "node '{}' references unknown static parameter '${referenced_param}' for argument '{}'",
                                        node.id().as_str(),
                                        param.name()
                                    ),
                                )
                                .with_module_id(node.id().as_str()),
                            );
                            ok = false;
                            None
                        }
                    }
                }
                Some(StaticArg::Expression(expression)) => {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_STATIC_ARGUMENT_EXPRESSION,
                            Severity::Error,
                            format!(
                                "node '{}' uses unsupported static expression '{expression}' for argument '{}'; only literals and name pass-through are allowed",
                                node.id().as_str(),
                                param.name()
                            ),
                        )
                        .with_module_id(node.id().as_str()),
                    );
                    ok = false;
                    None
                }
                None => match param.default() {
                    Some(value) => Some(value.clone()),
                    None => {
                        diagnostics.push(
                            Diagnostic::new(
                                error_codes::KERNEL_MISSING_STATIC_ARGUMENT,
                                Severity::Error,
                                format!(
                                    "node '{}' omits required static argument '{}' for definition '{}'",
                                    node.id().as_str(),
                                    param.name(),
                                    referenced.name()
                                ),
                            )
                            .with_module_id(node.id().as_str()),
                        );
                        ok = false;
                        None
                    }
                },
            };

            if let Some(value) = value {
                if let (StaticType::Resource(expected_kind), StaticType::Resource(actual_kind)) =
                    (param.static_type(), value.static_type())
                    && expected_kind != actual_kind
                {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_RESOURCE_KIND_MISMATCH,
                            Severity::Error,
                            format!(
                                "node '{}' supplies a {actual_kind} resource for static parameter '{}', which requires {expected_kind}",
                                node.id().as_str(),
                                param.name()
                            ),
                        )
                        .with_module_id(node.id().as_str())
                        .with_expected(expected_kind.as_str())
                        .with_actual(actual_kind.as_str()),
                    );
                    ok = false;
                } else if value.static_type() != param.static_type() {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_STATIC_ARGUMENT_TYPE_MISMATCH,
                            Severity::Error,
                            format!(
                                "node '{}' supplies a {:?} value for static parameter '{}' of type {:?}",
                                node.id().as_str(),
                                value.static_type(),
                                param.name(),
                                param.static_type()
                            ),
                        )
                        .with_module_id(node.id().as_str())
                        .with_expected(format!("{:?}", param.static_type()))
                        .with_actual(format!("{:?}", value.static_type())),
                    );
                    ok = false;
                } else if let StaticValue::Enum(enum_value) = &value
                    && !param.allowed_values().is_empty()
                    && !param
                        .allowed_values()
                        .iter()
                        .any(|allowed| allowed == enum_value)
                {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_STATIC_ARGUMENT_INVALID_ENUM_VALUE,
                            Severity::Error,
                            format!(
                                "node '{}' supplies invalid value '{enum_value}' for enum static parameter '{}'",
                                node.id().as_str(),
                                param.name()
                            ),
                        )
                        .with_module_id(node.id().as_str())
                        .with_expected(param.allowed_values().join(", "))
                        .with_actual(enum_value),
                    );
                    ok = false;
                } else if let StaticValue::Int(int_value) = &value
                    && !param.allowed_values().is_empty()
                    && !param
                        .allowed_values()
                        .iter()
                        .any(|allowed| allowed == &int_value.to_string())
                {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_STATIC_ARGUMENT_UNSUPPORTED_VALUE,
                            Severity::Error,
                            format!(
                                "node '{}' supplies unsupported value '{int_value}' for integer static parameter '{}'",
                                node.id().as_str(),
                                param.name()
                            ),
                        )
                        .with_module_id(node.id().as_str())
                        .with_expected(param.allowed_values().join(", "))
                        .with_actual(int_value.to_string()),
                    );
                    ok = false;
                } else {
                    resolved.insert(param.name().to_string(), value);
                }
            }
        }

        ok.then_some(resolved)
    }

    /// Resolve the ports a node exposes, substituting its resolved static
    /// arguments into any `channels` references.
    fn resolve_ports(
        referenced: &GraphDefinition,
        static_args: &BTreeMap<String, StaticValue>,
    ) -> Vec<ResolvedPort> {
        referenced
            .ports()
            .iter()
            .map(|port| {
                let channels = match port.channels() {
                    ChannelCount::Literal(count) => *count,
                    ChannelCount::Param(name) => match static_args.get(name) {
                        Some(StaticValue::Int(value)) if *value >= ChannelCount::MIN => {
                            *value as u32
                        }
                        // Unreachable in a compiled graph: `validate_static_references`
                        // rejects any channel reference that is not a declared integer
                        // static parameter, and `validate_resolved_static_references`
                        // rejects any resolved value below `ChannelCount::MIN`, so a
                        // successfully-compiled arg is always in range here. Kept only
                        // for totality; the value is never observed because compilation
                        // has already failed.
                        _ => ChannelCount::MIN as u32,
                    },
                };
                ResolvedPort {
                    name: port.name().to_string(),
                    direction: port.direction(),
                    signal_type: port.signal_type(),
                    channels,
                    multiplicity: port.multiplicity(),
                    control_default: port.control_default().cloned(),
                }
            })
            .collect()
    }

    fn resolve_node_ports(
        &self,
        node: &Node,
        referenced: &GraphDefinition,
        static_args: &BTreeMap<String, StaticValue>,
        registry: &DefinitionRegistry,
        diagnostics: &mut Diagnostics,
    ) -> Option<Vec<ResolvedPort>> {
        if referenced.name() != POLY_DEFINITION {
            return Some(Self::resolve_ports(referenced, static_args));
        }
        self.resolve_poly_ports(node, static_args, registry, diagnostics)
    }

    fn resolve_poly_ports(
        &self,
        node: &Node,
        static_args: &BTreeMap<String, StaticValue>,
        registry: &DefinitionRegistry,
        diagnostics: &mut Diagnostics,
    ) -> Option<Vec<ResolvedPort>> {
        let mut valid = true;
        let max_voices = match static_args.get(POLY_MAX_VOICES_PARAM) {
            Some(StaticValue::Int(value)) => *value,
            _ => return None,
        };
        if !(POLY_MIN_VOICES..=POLY_MAX_VOICES).contains(&max_voices) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_POLY_INVALID_MAX_VOICES,
                    Severity::Error,
                    format!(
                        "poly node '{}' supplies max_voices {max_voices}, but it must be positive and convertible to a 32-bit voice count",
                        node.id().as_str()
                    ),
                )
                .with_module_id(node.id().as_str())
                .with_expected(format!("{POLY_MIN_VOICES}..={POLY_MAX_VOICES}"))
                .with_actual(max_voices.to_string()),
            );
            valid = false;
        }

        let wrapped_name = match static_args.get(POLY_WRAPPED_DEFINITION_PARAM) {
            Some(StaticValue::String(name)) => name,
            _ => return None,
        };
        let Some(wrapped) = registry.get(wrapped_name) else {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_POLY_UNKNOWN_WRAPPED_DEFINITION,
                    Severity::Error,
                    format!(
                        "poly node '{}' wraps unknown definition '{wrapped_name}'",
                        node.id().as_str()
                    ),
                )
                .with_module_id(node.id().as_str())
                .with_actual(wrapped_name),
            );
            return None;
        };

        wrapped.validate_static_references(diagnostics);
        wrapped.validate_definition_structure(diagnostics);
        let wrapped_args = wrapped.enclosing_context();
        for param in wrapped.static_params() {
            if param.default().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_POLY_MALFORMED_INTERFACE,
                        Severity::Error,
                        format!(
                            "poly node '{}' cannot resolve wrapped definition '{}' interface because static parameter '{}' has no default",
                            node.id().as_str(),
                            wrapped.name(),
                            param.name()
                        ),
                    )
                    .with_module_id(node.id().as_str())
                    .with_expected(format!("default for {}", param.name())),
                );
                valid = false;
            }
        }
        if !wrapped.validate_resolved_static_references(&wrapped_args, diagnostics) {
            valid = false;
        }

        let mut seen = BTreeSet::new();
        let mut ports = vec![ResolvedPort {
            name: POLY_NOTE_EVENTS_INPUT.to_string(),
            direction: PortDirection::Input,
            signal_type: SignalType::Event,
            channels: 1,
            multiplicity: Multiplicity::SingleSource,
            control_default: None,
        }];
        seen.insert((true, POLY_NOTE_EVENTS_INPUT.to_string()));

        for port in Self::resolve_ports(wrapped, &wrapped_args) {
            let key = (
                port.direction() == PortDirection::Input,
                port.name().to_string(),
            );
            if port.name() == POLY_NOTE_EVENTS_INPUT || !seen.insert(key) {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_POLY_MALFORMED_INTERFACE,
                        Severity::Error,
                        format!(
                            "poly node '{}' cannot synthesize wrapped definition '{}' port '{}': the public poly interface would contain a duplicate port",
                            node.id().as_str(),
                            wrapped.name(),
                            port.name()
                        ),
                    )
                    .with_module_id(node.id().as_str())
                    .with_port_name(port.name()),
                );
                valid = false;
                continue;
            }
            if port.direction() == PortDirection::Output && port.name() == POLY_DONE_OUTPUT {
                if matches!(port.signal_type(), SignalType::Event | SignalType::Control) {
                    continue;
                }
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_POLY_MALFORMED_INTERFACE,
                        Severity::Error,
                        format!(
                            "poly node '{}' requires '{}.{}' to be an event or control lifecycle output",
                            node.id().as_str(),
                            wrapped.name(),
                            port.name()
                        ),
                    )
                    .with_module_id(node.id().as_str())
                    .with_port_name(port.name())
                    .with_expected("event or control output")
                    .with_actual("audio output"),
                );
                valid = false;
                continue;
            }
            if port.direction() == PortDirection::Output
                && !matches!(port.signal_type(), SignalType::Audio | SignalType::Control)
            {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::KERNEL_POLY_MALFORMED_INTERFACE,
                        Severity::Error,
                        format!(
                            "poly node '{}' cannot expose event output '{}.{}'; wrapped outputs must be audio or control",
                            node.id().as_str(),
                            wrapped.name(),
                            port.name()
                        ),
                    )
                    .with_module_id(node.id().as_str())
                    .with_port_name(port.name())
                    .with_expected("audio or control output")
                    .with_actual("event output"),
                );
                valid = false;
                continue;
            }
            ports.push(port);
        }

        valid.then_some(ports)
    }

    fn validate_nested_poly_interfaces(
        &self,
        registry: &DefinitionRegistry,
        diagnostics: &mut Diagnostics,
    ) {
        let mut visited = BTreeSet::new();
        visited.insert(self.name().to_string());
        for node in self.nodes() {
            let Some(referenced) = registry.get(node.definition_ref()) else {
                continue;
            };
            if referenced.name() == POLY_DEFINITION {
                let mut sink = Diagnostics::new();
                let Some(args) = self.resolve_static_args(
                    node,
                    referenced,
                    &self.enclosing_context(),
                    &mut sink,
                ) else {
                    continue;
                };
                if let Some(StaticValue::String(name)) = args.get(POLY_WRAPPED_DEFINITION_PARAM)
                    && let Some(wrapped) = registry.get(name)
                {
                    Self::validate_poly_interfaces_in_definition(
                        wrapped,
                        registry,
                        &mut visited,
                        diagnostics,
                    );
                }
            } else {
                Self::validate_poly_interfaces_in_definition(
                    referenced,
                    registry,
                    &mut visited,
                    diagnostics,
                );
            }
        }
    }

    fn validate_poly_interfaces_in_definition(
        definition: &GraphDefinition,
        registry: &DefinitionRegistry,
        visited: &mut BTreeSet<String>,
        diagnostics: &mut Diagnostics,
    ) {
        if !visited.insert(definition.name().to_string()) {
            return;
        }
        let enclosing = definition.enclosing_context();
        for node in definition.nodes() {
            let Some(referenced) = registry.get(node.definition_ref()) else {
                continue;
            };
            if referenced.name() == POLY_DEFINITION {
                let Some(args) =
                    definition.resolve_static_args(node, referenced, &enclosing, diagnostics)
                else {
                    continue;
                };
                definition.resolve_poly_ports(node, &args, registry, diagnostics);
                if let Some(StaticValue::String(name)) = args.get(POLY_WRAPPED_DEFINITION_PARAM)
                    && let Some(wrapped) = registry.get(name)
                {
                    Self::validate_poly_interfaces_in_definition(
                        wrapped,
                        registry,
                        visited,
                        diagnostics,
                    );
                }
            } else {
                Self::validate_poly_interfaces_in_definition(
                    referenced,
                    registry,
                    visited,
                    diagnostics,
                );
            }
        }
    }

    /// Reject, loudly, any port channel count or latency spec on this definition
    /// that references a static parameter which is not a declared integer static
    /// parameter. Without this gate a dangling or mistyped reference would
    /// silently resolve to a plausible default (one channel, zero latency),
    /// producing e.g. a latency-bearing node that reports zero and phase-smears
    /// parallel dry/wet paths.
    fn validate_static_references(&self, diagnostics: &mut Diagnostics) {
        for port in &self.ports {
            if let ChannelCount::Param(name) = port.channels() {
                self.require_integer_static_param(
                    name,
                    &format!("channel count of port '{}'", port.name()),
                    diagnostics,
                );
            }
        }

        if let LatencySpec::StaticParam { name, .. } = &self.latency {
            self.require_integer_static_param(name, "processing latency", diagnostics);
        }
    }

    pub(crate) fn validate_definition_structure(&self, diagnostics: &mut Diagnostics) {
        if self.implementation != DefinitionImplementation::Script {
            return;
        }

        let invalid = |message: String| {
            Diagnostic::new(
                error_codes::KERNEL_SCRIPT_DEFINITION_INVALID,
                Severity::Error,
                message,
            )
            .with_module_id(self.name())
        };
        if !self.nodes.is_empty() {
            diagnostics.push(invalid(format!(
                "script-backed definition '{}' cannot declare internal modules",
                self.name()
            )));
        }
        if !self.connections.is_empty() {
            diagnostics.push(invalid(format!(
                "script-backed definition '{}' cannot declare internal connections",
                self.name()
            )));
        }
        for port in &self.ports {
            if port.signal_type() == SignalType::Audio {
                diagnostics.push(
                    invalid(format!(
                        "script-backed definition '{}' declares unsupported audio port '{}'",
                        self.name(),
                        port.name()
                    ))
                    .with_port_name(port.name()),
                );
            }
            if !port.internal_targets().is_empty() || !port.internal_sources().is_empty() {
                diagnostics.push(
                    invalid(format!(
                        "script-backed definition '{}' port '{}' cannot map to internal ports",
                        self.name(),
                        port.name()
                    ))
                    .with_port_name(port.name()),
                );
            }
        }

        self.require_script_static_param(
            crate::builtins::SCRIPT_LANGUAGE_PARAMETER,
            StaticType::Enum,
            diagnostics,
        );
        self.require_script_static_param(
            crate::builtins::SCRIPT_SOURCE_PARAMETER,
            StaticType::String,
            diagnostics,
        );
    }

    fn require_script_static_param(
        &self,
        name: &str,
        expected: StaticType,
        diagnostics: &mut Diagnostics,
    ) {
        if self
            .static_param(name)
            .is_some_and(|param| param.static_type() == expected)
        {
            return;
        }
        diagnostics.push(
            Diagnostic::new(
                error_codes::KERNEL_SCRIPT_DEFINITION_INVALID,
                Severity::Error,
                format!(
                    "script-backed definition '{}' must declare '{name}' as {expected:?}",
                    self.name()
                ),
            )
            .with_module_id(self.name())
            .with_expected(format!("{name}: {expected:?}")),
        );
    }

    fn require_integer_static_param(
        &self,
        name: &str,
        context: &str,
        diagnostics: &mut Diagnostics,
    ) {
        let message = match self.static_param(name) {
            Some(param) if param.static_type() == StaticType::Int => return,
            Some(param) => format!(
                "definition '{}' resolves its {context} from static parameter '{name}', which is declared {:?}, not an integer",
                self.name(),
                param.static_type()
            ),
            None => format!(
                "definition '{}' resolves its {context} from undeclared static parameter '{name}'",
                self.name()
            ),
        };
        diagnostics.push(
            Diagnostic::new(
                error_codes::KERNEL_UNRESOLVED_STATIC_REFERENCE,
                Severity::Error,
                message,
            )
            .with_module_id(self.name())
            .with_expected("integer static parameter"),
        );
    }

    /// Reject, loudly, any resolved channel-count or latency static value that is
    /// out of range for a specific node instance: a channel count below
    /// [`ChannelCount::MIN`], a negative latency, or a latency that would
    /// saturate to zero because the resolved value is smaller than the subtracted
    /// constant. [`Self::validate_static_references`] already guarantees the
    /// referenced parameter is a declared integer; this guards the concrete value
    /// so a bad static argument cannot fall back to one channel or zero latency
    /// (which would phase-smear parallel dry/wet paths). Returns `true` when every
    /// resolved reference is in range.
    fn validate_resolved_static_references(
        &self,
        static_args: &BTreeMap<String, StaticValue>,
        diagnostics: &mut Diagnostics,
    ) -> bool {
        let mut ok = true;

        for port in &self.ports {
            if let ChannelCount::Param(name) = port.channels() {
                if let Some(StaticValue::Int(value)) = static_args.get(name) {
                    if *value < ChannelCount::MIN {
                        diagnostics.push(
                            Diagnostic::new(
                                error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE,
                                Severity::Error,
                                format!(
                                    "definition '{}' resolves the channel count of port '{}' from static parameter '{name}' to {value}, but a channel count must be at least {}",
                                    self.name(),
                                    port.name(),
                                    ChannelCount::MIN
                                ),
                            )
                            .with_module_id(self.name())
                            .with_port_name(port.name())
                            .with_expected(format!("channel count >= {}", ChannelCount::MIN))
                            .with_actual(value.to_string()),
                        );
                        ok = false;
                    }
                }
            }
        }

        if let LatencySpec::StaticParam { name, minus } = &self.latency {
            if let Some(StaticValue::Int(value)) = static_args.get(name) {
                if *value < 0 {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE,
                            Severity::Error,
                            format!(
                                "definition '{}' resolves its processing latency from static parameter '{name}' to {value}, but latency cannot be negative",
                                self.name()
                            ),
                        )
                        .with_module_id(self.name())
                        .with_expected("latency >= 0")
                        .with_actual(value.to_string()),
                    );
                    ok = false;
                } else if (*value as u32) < *minus {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE,
                            Severity::Error,
                            format!(
                                "definition '{}' resolves its processing latency from static parameter '{name}' to {value}, which is smaller than the subtracted constant {minus}; latency would silently saturate to zero",
                                self.name()
                            ),
                        )
                        .with_module_id(self.name())
                        .with_expected(format!("latency static value >= {minus}"))
                        .with_actual(value.to_string()),
                    );
                    ok = false;
                }
            }
        }

        ok
    }

    /// Validate this definition against the referenced definitions in
    /// `registry`, resolving static arguments and channel counts, checking
    /// connections for signal-type and channel-count compatibility, and
    /// recording control→audio promotions.
    pub fn validate(&self, registry: &DefinitionRegistry) -> KernelValidation {
        let mut diagnostics = Diagnostics::new();
        let mut promotions = Vec::new();
        let enclosing = self.enclosing_context();

        // Reject dangling channel/latency static references before anything can
        // silently fall back to a default value.
        let mut checked: BTreeSet<String> = BTreeSet::new();
        self.validate_static_references(&mut diagnostics);
        self.validate_definition_structure(&mut diagnostics);
        checked.insert(self.name().to_string());

        // Resolve every node's ports up front so connection checks share them.
        let mut resolved_nodes: BTreeMap<&NodeId, ResolvedNode<'_>> = BTreeMap::new();
        for node in &self.nodes {
            let Some(referenced) = registry.get(node.definition_ref()) else {
                diagnostics.push(
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

            if checked.insert(referenced.name().to_string()) {
                referenced.validate_static_references(&mut diagnostics);
                referenced.validate_definition_structure(&mut diagnostics);
            }

            let Some(static_args) =
                self.resolve_static_args(node, referenced, &enclosing, &mut diagnostics)
            else {
                continue;
            };

            // Guard the resolved values before ports are built so an out-of-range
            // channel count cannot fall back to one channel here.
            if !referenced.validate_resolved_static_references(&static_args, &mut diagnostics) {
                continue;
            }

            let Some(ports) =
                self.resolve_node_ports(node, referenced, &static_args, registry, &mut diagnostics)
            else {
                continue;
            };

            // Overrides must target ports the referenced definition declares.
            for port_name in node.port_default_overrides().keys() {
                if !ports.iter().any(|port| port.name() == port_name) {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_OVERRIDE_UNKNOWN_PORT,
                            Severity::Error,
                            format!(
                                "node '{}' overrides default of unknown port '{port_name}' on definition '{}'",
                                node.id().as_str(),
                                referenced.name()
                            ),
                        )
                        .with_module_id(node.id().as_str())
                        .with_port_name(port_name),
                    );
                }
            }

            resolved_nodes.insert(
                node.id(),
                ResolvedNode {
                    definition: referenced,
                    ports,
                },
            );
        }

        for connection in &self.connections {
            let source = self.resolve_endpoint(
                connection.source(),
                PortDirection::Output,
                &resolved_nodes,
                &mut diagnostics,
            );
            let destination = self.resolve_endpoint(
                connection.destination(),
                PortDirection::Input,
                &resolved_nodes,
                &mut diagnostics,
            );

            let (Some(source), Some(destination)) = (source, destination) else {
                continue;
            };

            match classify_signal(source.signal_type(), destination.signal_type()) {
                SignalCompatibility::Same => {
                    self.check_channel_counts(connection, &source, &destination, &mut diagnostics);
                }
                SignalCompatibility::PromoteControlToAudio => {
                    if self.check_channel_counts(
                        connection,
                        &source,
                        &destination,
                        &mut diagnostics,
                    ) {
                        promotions.push(PromotionStep {
                            source: connection.source().clone(),
                            destination: connection.destination().clone(),
                            channels: destination.channels(),
                        });
                    }
                }
                SignalCompatibility::Incompatible => {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::KERNEL_INCOMPATIBLE_SIGNAL_TYPES,
                            Severity::Error,
                            format!(
                                "incompatible signal types: {} is {:?}, but {} is {:?}",
                                connection.source().port(),
                                source.signal_type(),
                                connection.destination().port(),
                                destination.signal_type()
                            ),
                        )
                        .with_module_id(connection.destination().node().as_str())
                        .with_port_name(connection.destination().port())
                        .with_expected(format!("{:?}", source.signal_type()))
                        .with_actual(format!("{:?}", destination.signal_type())),
                    );
                }
            }
        }

        // Check input multiplicity: single-source inputs may receive at most
        // one connection; summing inputs accept any number.
        let mut destination_counts: BTreeMap<PortRef, usize> = BTreeMap::new();
        for connection in &self.connections {
            *destination_counts
                .entry(connection.destination().clone())
                .or_insert(0) += 1;
        }
        for (destination, count) in &destination_counts {
            if *count > 1 {
                if let Some(node) = resolved_nodes.get(destination.node()) {
                    if let Some(port) = node.ports.iter().find(|p| {
                        p.name() == destination.port() && p.direction() == PortDirection::Input
                    }) {
                        if port.multiplicity() == Multiplicity::SingleSource {
                            diagnostics.push(
                                Diagnostic::new(
                                    error_codes::KERNEL_MULTIPLE_SOURCES,
                                    Severity::Error,
                                    format!(
                                        "input port '{}' on node '{}' receives {count} connections, but is declared single-source; use a summing input or explicit mixer",
                                        destination.port(),
                                        destination.node().as_str()
                                    ),
                                )
                                .with_module_id(destination.node().as_str())
                                .with_port_name(destination.port()),
                            );
                        }
                    }
                }
            }
        }

        self.validate_nested_poly_interfaces(registry, &mut diagnostics);

        if let Some(path) = self.find_illegal_cycle(registry) {
            let printable = path
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(" -> ");
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY,
                    Severity::Error,
                    format!(
                        "routing cycle {printable} has no '{FEEDBACK_DELAY_DEFINITION}' node; every feedback cycle must pass through a '{FEEDBACK_DELAY_DEFINITION}' primitive"
                    ),
                )
                .with_suggested_fix(format!(
                    "insert a '{FEEDBACK_DELAY_DEFINITION}' node into the cycle"
                )),
            );
        }

        KernelValidation {
            diagnostics,
            promotions,
        }
    }

    /// Find a routing cycle that does not pass through any `feedback_delay`
    /// node, returning the cycle's node path. Cycles through a `feedback_delay`
    /// node are legal (the scheduler cuts them there) and are not reported.
    fn find_illegal_cycle(&self, registry: &DefinitionRegistry) -> Option<Vec<NodeId>> {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut stack: Vec<NodeId> = Vec::new();

        for node in &self.nodes {
            if let Some(path) =
                self.walk_for_cycle(node.id(), registry, &mut visiting, &mut visited, &mut stack)
            {
                return Some(path);
            }
        }
        None
    }

    fn walk_for_cycle(
        &self,
        node_id: &NodeId,
        registry: &DefinitionRegistry,
        visiting: &mut BTreeSet<NodeId>,
        visited: &mut BTreeSet<NodeId>,
        stack: &mut Vec<NodeId>,
    ) -> Option<Vec<NodeId>> {
        if visited.contains(node_id) {
            return None;
        }
        visiting.insert(node_id.clone());
        stack.push(node_id.clone());

        for successor in self.successors(node_id) {
            if visiting.contains(&successor) {
                let cycle: Vec<NodeId> = stack
                    .iter()
                    .skip_while(|id| **id != successor)
                    .cloned()
                    .collect();
                if !cycle.iter().any(|id| self.is_feedback_delay(id, registry)) {
                    return Some(cycle);
                }
            } else if let Some(path) =
                self.walk_for_cycle(&successor, registry, visiting, visited, stack)
            {
                return Some(path);
            }
        }

        stack.pop();
        visiting.remove(node_id);
        visited.insert(node_id.clone());
        None
    }

    /// The distinct destination nodes reachable by one cable from `node_id`.
    fn successors(&self, node_id: &NodeId) -> Vec<NodeId> {
        let mut seen = BTreeSet::new();
        self.connections
            .iter()
            .filter(|connection| connection.source().node() == node_id)
            .filter_map(|connection| {
                let destination = connection.destination().node().clone();
                seen.insert(destination.clone()).then_some(destination)
            })
            .collect()
    }

    fn is_feedback_delay(&self, node_id: &NodeId, registry: &DefinitionRegistry) -> bool {
        self.node(node_id).is_some_and(|node| {
            node.definition_ref() == FEEDBACK_DELAY_DEFINITION
                || registry
                    .get(node.definition_ref())
                    .is_some_and(|definition| definition.name() == FEEDBACK_DELAY_DEFINITION)
        })
    }

    /// Returns `true` when source and destination resolved channel counts match.
    fn check_channel_counts(
        &self,
        connection: &Connection,
        source: &ResolvedPort,
        destination: &ResolvedPort,
        diagnostics: &mut Diagnostics,
    ) -> bool {
        if source.channels() == destination.channels() {
            return true;
        }
        diagnostics.push(
            Diagnostic::new(
                error_codes::KERNEL_CHANNEL_COUNT_MISMATCH,
                Severity::Error,
                format!(
                    "channel count mismatch: {} has {} channel(s), but {} has {} channel(s)",
                    connection.source().port(),
                    source.channels(),
                    connection.destination().port(),
                    destination.channels()
                ),
            )
            .with_module_id(connection.destination().node().as_str())
            .with_port_name(connection.destination().port())
            .with_expected(source.channels().to_string())
            .with_actual(destination.channels().to_string()),
        );
        false
    }

    /// Resolve a connection endpoint to its resolved port, emitting a
    /// diagnostic (and returning `None`) when the node, definition, or port is
    /// unresolved, wrong-direction, or names a static parameter.
    fn resolve_endpoint(
        &self,
        reference: &PortRef,
        expected: PortDirection,
        resolved_nodes: &BTreeMap<&NodeId, ResolvedNode<'_>>,
        diagnostics: &mut Diagnostics,
    ) -> Option<ResolvedPort> {
        if self.node(reference.node()).is_none() {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_MISSING_NODE,
                    Severity::Error,
                    format!("missing node: {}", reference.node().as_str()),
                )
                .with_module_id(reference.node().as_str()),
            );
            return None;
        }

        // If the node's definition or static args failed to resolve, the entry
        // is absent; the earlier diagnostic already explains why.
        let resolved = resolved_nodes.get(reference.node())?;
        let ports = &resolved.ports;

        if let Some(port) = ports
            .iter()
            .find(|port| port.name() == reference.port() && port.direction() == expected)
        {
            return Some(port.clone());
        }

        if ports.iter().any(|port| port.name() == reference.port()) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_INCORRECT_PORT_DIRECTION,
                    Severity::Error,
                    format!(
                        "incorrect port direction: {}.{} is not a {expected:?} port",
                        reference.node().as_str(),
                        reference.port()
                    ),
                )
                .with_module_id(reference.node().as_str())
                .with_port_name(reference.port()),
            );
            return None;
        }

        // A connection that targets a static parameter name gets a dedicated
        // diagnostic rather than a generic missing-port error.
        if resolved.definition.static_param(reference.port()).is_some() {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::KERNEL_STATIC_PARAM_NOT_A_PORT,
                    Severity::Error,
                    format!(
                        "'{}' is a compile-time static parameter of '{}', not a port; static parameters cannot be connected",
                        reference.port(),
                        resolved.definition.name()
                    ),
                )
                .with_module_id(reference.node().as_str())
                .with_port_name(reference.port()),
            );
            return None;
        }

        diagnostics.push(
            Diagnostic::new(
                error_codes::KERNEL_MISSING_PORT,
                Severity::Error,
                format!(
                    "missing port: {}.{}",
                    reference.node().as_str(),
                    reference.port()
                ),
            )
            .with_module_id(reference.node().as_str())
            .with_port_name(reference.port()),
        );
        None
    }

    /// Resolve the ports a node exposes after substituting its static
    /// arguments, or `None` when the node's definition or static arguments do
    /// not resolve. Discovery and channel-count inspection use this.
    pub fn resolved_node_ports(
        &self,
        registry: &DefinitionRegistry,
        node_id: &NodeId,
    ) -> Option<Vec<ResolvedPort>> {
        let node = self.node(node_id)?;
        let referenced = registry.get(node.definition_ref())?;
        let mut sink = Diagnostics::new();
        // Don't hand back ports built from a dangling channel reference.
        referenced.validate_static_references(&mut sink);
        if sink.has_errors() {
            return None;
        }
        let enclosing = self.enclosing_context();
        let static_args = self.resolve_static_args(node, referenced, &enclosing, &mut sink)?;
        // Don't hand back ports built from an out-of-range resolved channel count.
        if !referenced.validate_resolved_static_references(&static_args, &mut sink) {
            return None;
        }
        self.resolve_node_ports(node, referenced, &static_args, registry, &mut sink)
    }

    /// Resolve the effective source of a node's control input port, applying
    /// the precedence: incoming cable > instance override > declared default.
    /// Returns `None` when the node or a control input port of that name is
    /// unresolved.
    pub fn effective_control_input(
        &self,
        registry: &DefinitionRegistry,
        node_id: &NodeId,
        port_name: &str,
    ) -> Option<EffectiveInput> {
        let node = self.node(node_id)?;
        let ports = self.resolved_node_ports(registry, node_id)?;
        let port = ports.iter().find(|port| {
            port.name() == port_name
                && port.direction() == PortDirection::Input
                && port.signal_type() == SignalType::Control
        })?;

        let is_connected = self.connections.iter().any(|connection| {
            connection.destination().node() == node_id
                && connection.destination().port() == port_name
        });
        if is_connected {
            return Some(EffectiveInput::Connected);
        }

        if let Some(override_value) = node.port_default_overrides().get(port_name) {
            return Some(EffectiveInput::Value(*override_value));
        }

        port.control_default()
            .map(|control_default| EffectiveInput::Value(control_default.default()))
    }
}

#[cfg(test)]
mod tests;
