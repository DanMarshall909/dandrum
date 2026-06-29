use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::builtins::{BuiltInModuleRegistry, module_types};
use crate::patch;

#[path = "graph_composite.rs"]
mod graph_composite;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionScope {
    Voice,
    Global,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleNode {
    id: ModuleId,
    module_type: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    feedback_boundaries: Vec<SignalType>,
    execution_scope: ExecutionScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Port {
    name: String,
    direction: PortDirection,
    signal_type: SignalType,
    accepts_multiple_sources: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalType {
    Audio,
    Control,
    Event,
}

pub mod builtin_ports {
    pub const AUDIO: &str = "audio";
    pub const AUDIO_IN: &str = "audio_in";
    pub const AUDIO_OUT: &str = "audio_out";
    pub const EVENTS: &str = "events";
    pub const GAIN: &str = "gain";
    pub const PITCH: &str = "pitch";
    pub const CUTOFF: &str = "cutoff";
    pub const PAN: &str = "pan";
    pub const ATTACK: &str = "attack";
    pub const DECAY: &str = "decay";
    pub const SUSTAIN: &str = "sustain";
    pub const RELEASE: &str = "release";
    pub const LEFT: &str = "left";
    pub const RIGHT: &str = "right";
    pub const INPUTS: &str = "inputs";
    pub const MIX: &str = "mix";
    pub const SUM: &str = "sum";
    pub const GATE: &str = "gate";
    pub const TRIGGER: &str = "trigger";
    pub const RATE: &str = "rate";
    pub const VALUE: &str = "value";
    pub const START: &str = "start";
    pub const LOOP_ENABLED: &str = "loop_enabled";
    pub const LOOP_START: &str = "loop_start";
    pub const LOOP_END: &str = "loop_end";
    // Dynamics processor ports
    pub const SIDECHAIN_IN: &str = "sidechain_in";
    pub const THRESHOLD: &str = "threshold";
    pub const BELOW_RATIO: &str = "below_ratio";
    pub const ABOVE_RATIO: &str = "above_ratio";
    pub const KNEE: &str = "knee";
    pub const MAKEUP_GAIN: &str = "makeup_gain";
    pub const ATTACK_GAIN: &str = "attack_gain";
    pub const SUSTAIN_GAIN: &str = "sustain_gain";
    // Saturator ports
    pub const DRIVE: &str = "drive";
    pub const BIAS: &str = "bias";
    pub const CURVE_SELECT: &str = "curve_select";
}

impl SignalType {
    pub fn is_compatible_with(self, destination: Self) -> bool {
        matches!(
            (self, destination),
            (Self::Audio, Self::Audio)
                | (Self::Control, Self::Control)
                | (Self::Event, Self::Event)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortRef {
    module_id: ModuleId,
    port_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cable {
    source: PortRef,
    destination: PortRef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    modules: Vec<ModuleNode>,
    cables: Vec<Cable>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphValidationError {
    diagnostics: Vec<GraphDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphDiagnostic {
    MissingModule {
        module_id: ModuleId,
    },
    MissingPort {
        port: PortRef,
    },
    IncorrectPortDirection {
        port: PortRef,
        expected: PortDirection,
    },
    IncompatibleSignalTypes {
        source: PortRef,
        source_type: SignalType,
        destination: PortRef,
        destination_type: SignalType,
    },
    MultipleSourcesToInput {
        destination: PortRef,
    },
    CycleDetected {
        path: Vec<Cable>,
    },
    VoiceToGlobalDirectRouting {
        source: PortRef,
        destination: PortRef,
    },
}

impl ModuleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ModuleNode {
    pub fn new(id: ModuleId, module_type: impl Into<String>) -> Self {
        Self {
            id,
            module_type: module_type.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            feedback_boundaries: Vec::new(),
            execution_scope: ExecutionScope::Global,
        }
    }

    pub fn with_execution_scope(mut self, scope: ExecutionScope) -> Self {
        self.execution_scope = scope;
        self
    }

    pub fn execution_scope(&self) -> ExecutionScope {
        self.execution_scope
    }

    pub fn with_input(mut self, name: impl Into<String>, signal_type: SignalType) -> Self {
        self.inputs.push(Port::input(name, signal_type));
        self
    }

    pub fn with_mixing_input(mut self, name: impl Into<String>, signal_type: SignalType) -> Self {
        self.inputs.push(Port::mixing_input(name, signal_type));
        self
    }

    pub fn with_output(mut self, name: impl Into<String>, signal_type: SignalType) -> Self {
        self.outputs.push(Port::output(name, signal_type));
        self
    }

    pub fn with_feedback_boundary(mut self, signal_type: SignalType) -> Self {
        self.feedback_boundaries.push(signal_type);
        self
    }

    pub fn id(&self) -> &ModuleId {
        &self.id
    }

    pub fn module_type(&self) -> &str {
        &self.module_type
    }

    pub fn inputs(&self) -> &[Port] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[Port] {
        &self.outputs
    }

    pub fn feedback_boundaries(&self) -> &[SignalType] {
        &self.feedback_boundaries
    }
}

impl Port {
    pub fn input(name: impl Into<String>, signal_type: SignalType) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Input,
            signal_type,
            accepts_multiple_sources: false,
        }
    }

    pub fn mixing_input(name: impl Into<String>, signal_type: SignalType) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Input,
            signal_type,
            accepts_multiple_sources: true,
        }
    }

    pub fn output(name: impl Into<String>, signal_type: SignalType) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Output,
            signal_type,
            accepts_multiple_sources: false,
        }
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

    pub fn accepts_multiple_sources(&self) -> bool {
        self.accepts_multiple_sources
    }
}

impl PortRef {
    pub fn new(module_id: ModuleId, port_name: impl Into<String>) -> Self {
        Self {
            module_id,
            port_name: port_name.into(),
        }
    }

    pub fn module_id(&self) -> &ModuleId {
        &self.module_id
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }
}

impl Cable {
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

impl Graph {
    pub fn new(modules: Vec<ModuleNode>, cables: Vec<Cable>) -> Self {
        Self { modules, cables }
    }

    pub fn from_patch_declarations(patch: &patch::PatchDocument) -> Self {
        let registry = BuiltInModuleRegistry::new();
        let (module_declarations, connection_declarations) =
            graph_composite::expand_patch_declarations(patch);
        let modules = module_declarations
            .iter()
            .map(|module| {
                let mut node =
                    ModuleNode::new(ModuleId::new(module.id.clone()), module.module_type.clone());
                let definition = registry.get(&module.module_type);

                if let Some(definition) = definition {
                    node = node.with_execution_scope(definition.execution_scope());

                    for input in definition.inputs() {
                        node = if input.accepts_multiple_sources() {
                            node.with_mixing_input(input.name().to_string(), input.signal_type())
                        } else {
                            node.with_input(input.name().to_string(), input.signal_type())
                        };
                    }

                    for output in definition.outputs() {
                        node = node.with_output(output.name().to_string(), output.signal_type());
                    }

                    for boundary in definition.feedback_boundaries() {
                        node = node.with_feedback_boundary(*boundary);
                    }
                }

                for input in &module.inputs {
                    if definition.is_none() || module.module_type == module_types::SCRIPT {
                        node = node
                            .with_input(input.name.clone(), SignalType::from(&input.signal_type));
                    }
                }

                for output in &module.outputs {
                    if definition.is_none() || module.module_type == module_types::SCRIPT {
                        node = node.with_output(
                            output.name.clone(),
                            SignalType::from(&output.signal_type),
                        );
                    }
                }

                node
            })
            .collect();

        let cables = connection_declarations
            .iter()
            .map(|connection| {
                Cable::new(
                    PortRef::new(
                        ModuleId::new(connection.from.module_id.clone()),
                        connection.from.port_name.clone(),
                    ),
                    PortRef::new(
                        ModuleId::new(connection.to.module_id.clone()),
                        connection.to.port_name.clone(),
                    ),
                )
            })
            .collect();

        Self { modules, cables }
    }

    pub fn modules(&self) -> &[ModuleNode] {
        &self.modules
    }

    pub fn cables(&self) -> &[Cable] {
        &self.cables
    }

    pub fn validate(&self) -> Result<(), GraphValidationError> {
        let mut diagnostics = Vec::new();
        let mut destination_counts: BTreeMap<&PortRef, usize> = BTreeMap::new();
        let mut destination_allows_multiple_sources: BTreeMap<&PortRef, bool> = BTreeMap::new();

        for cable in &self.cables {
            *destination_counts.entry(cable.destination()).or_default() += 1;

            let source = self.resolve_port(cable.source(), PortDirection::Output, &mut diagnostics);
            let destination =
                self.resolve_port(cable.destination(), PortDirection::Input, &mut diagnostics);

            if let (Some(source), Some(destination)) = (source, destination) {
                destination_allows_multiple_sources
                    .insert(cable.destination(), destination.accepts_multiple_sources());

                if !source
                    .signal_type()
                    .is_compatible_with(destination.signal_type())
                {
                    diagnostics.push(GraphDiagnostic::IncompatibleSignalTypes {
                        source: cable.source().clone(),
                        source_type: source.signal_type(),
                        destination: cable.destination().clone(),
                        destination_type: destination.signal_type(),
                    });
                }

                let source_module = self
                    .modules
                    .iter()
                    .find(|m| m.id() == cable.source().module_id());
                let dest_module = self
                    .modules
                    .iter()
                    .find(|m| m.id() == cable.destination().module_id());

                if let (Some(source_module), Some(dest_module)) = (source_module, dest_module) {
                    if source_module.execution_scope() == ExecutionScope::Voice
                        && dest_module.execution_scope() == ExecutionScope::Global
                        && !destination.accepts_multiple_sources()
                    {
                        diagnostics.push(GraphDiagnostic::VoiceToGlobalDirectRouting {
                            source: cable.source().clone(),
                            destination: cable.destination().clone(),
                        });
                    }
                }
            }
        }

        for (destination, count) in destination_counts {
            let accepts_multiple_sources = destination_allows_multiple_sources
                .get(destination)
                .copied()
                .unwrap_or(false);

            if count > 1 && !accepts_multiple_sources {
                diagnostics.push(GraphDiagnostic::MultipleSourcesToInput {
                    destination: destination.clone(),
                });
            }
        }

        if let Some(path) = self.find_invalid_cycle() {
            diagnostics.push(GraphDiagnostic::CycleDetected { path });
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(GraphValidationError { diagnostics })
        }
    }

    fn resolve_port(
        &self,
        reference: &PortRef,
        expected_direction: PortDirection,
        diagnostics: &mut Vec<GraphDiagnostic>,
    ) -> Option<&Port> {
        let Some(module) = self
            .modules
            .iter()
            .find(|module| module.id() == reference.module_id())
        else {
            diagnostics.push(GraphDiagnostic::MissingModule {
                module_id: reference.module_id().clone(),
            });
            return None;
        };

        let expected_ports = match expected_direction {
            PortDirection::Input => module.inputs(),
            PortDirection::Output => module.outputs(),
        };

        if let Some(port) = expected_ports
            .iter()
            .find(|port| port.name() == reference.port_name())
        {
            return Some(port);
        }

        let opposite_ports = match expected_direction {
            PortDirection::Input => module.outputs(),
            PortDirection::Output => module.inputs(),
        };

        if opposite_ports
            .iter()
            .any(|port| port.name() == reference.port_name())
        {
            diagnostics.push(GraphDiagnostic::IncorrectPortDirection {
                port: reference.clone(),
                expected: expected_direction,
            });
        } else {
            diagnostics.push(GraphDiagnostic::MissingPort {
                port: reference.clone(),
            });
        }

        None
    }

    fn find_invalid_cycle(&self) -> Option<Vec<Cable>> {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut stack = Vec::new();

        for module in &self.modules {
            if let Some(path) =
                self.find_cycle_from(module.id(), &mut visiting, &mut visited, &mut stack)
            {
                let Some(signal_type) = self.cycle_signal_type(&path) else {
                    return Some(path);
                };

                if signal_type == SignalType::Event {
                    continue;
                }

                if !self.cycle_has_feedback_boundary(&path, signal_type) {
                    return Some(path);
                }
            }
        }

        None
    }

    fn cycle_signal_type(&self, path: &[Cable]) -> Option<SignalType> {
        path.iter().find_map(|cable| {
            self.port(cable.source(), PortDirection::Output)
                .map(Port::signal_type)
        })
    }

    fn cycle_has_feedback_boundary(&self, path: &[Cable], signal_type: SignalType) -> bool {
        path.iter().any(|cable| {
            self.modules
                .iter()
                .find(|module| module.id() == cable.source().module_id())
                .is_some_and(|module| module.feedback_boundaries().contains(&signal_type))
        })
    }

    fn port(&self, reference: &PortRef, direction: PortDirection) -> Option<&Port> {
        let module = self
            .modules
            .iter()
            .find(|module| module.id() == reference.module_id())?;
        let ports = match direction {
            PortDirection::Input => module.inputs(),
            PortDirection::Output => module.outputs(),
        };

        ports
            .iter()
            .find(|port| port.name() == reference.port_name())
    }

    fn find_cycle_from(
        &self,
        module_id: &ModuleId,
        visiting: &mut BTreeSet<ModuleId>,
        visited: &mut BTreeSet<ModuleId>,
        stack: &mut Vec<Cable>,
    ) -> Option<Vec<Cable>> {
        if visited.contains(module_id) {
            return None;
        }

        visiting.insert(module_id.clone());

        for cable in self
            .cables
            .iter()
            .filter(|cable| cable.source().module_id() == module_id)
        {
            let next_module = cable.destination().module_id();

            if visiting.contains(next_module) {
                let mut path = stack
                    .iter()
                    .skip_while(|stacked| stacked.source().module_id() != next_module)
                    .cloned()
                    .collect::<Vec<_>>();
                path.push(cable.clone());
                return Some(path);
            }

            stack.push(cable.clone());

            if let Some(path) = self.find_cycle_from(next_module, visiting, visited, stack) {
                return Some(path);
            }

            stack.pop();
        }

        visiting.remove(module_id);
        visited.insert(module_id.clone());
        None
    }
}

impl GraphValidationError {
    pub fn diagnostics(&self) -> &[GraphDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for GraphDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingModule { module_id } => {
                write!(formatter, "missing module: {}", module_id.as_str())
            }
            Self::MissingPort { port } => {
                write!(formatter, "missing port: {}", port)
            }
            Self::IncorrectPortDirection { port, expected } => {
                write!(
                    formatter,
                    "incorrect port direction: {port} is not a {expected:?} port"
                )
            }
            Self::IncompatibleSignalTypes {
                source,
                source_type,
                destination,
                destination_type,
            } => write!(
                formatter,
                "incompatible signal types: {source} is {source_type:?}, but {destination} is {destination_type:?}"
            ),
            Self::MultipleSourcesToInput { destination } => write!(
                formatter,
                "multiple sources connected to {destination}; use an explicit mixer or summing module"
            ),
            Self::CycleDetected { path } => {
                write!(formatter, "routing cycle detected")?;

                for cable in path {
                    write!(formatter, " {}->{}", cable.source(), cable.destination())?;
                }

                Ok(())
            }
            Self::VoiceToGlobalDirectRouting {
                source,
                destination,
            } => write!(
                formatter,
                "voice-scoped output {source} cannot route directly to global input {destination}; use an explicit mixer or summing module"
            ),
        }
    }
}

impl fmt::Display for PortRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}",
            self.module_id().as_str(),
            self.port_name()
        )
    }
}

impl fmt::Display for GraphValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "graph validation failed")?;

        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }

        Ok(())
    }
}

impl std::error::Error for GraphValidationError {}

impl From<&patch::SignalType> for SignalType {
    fn from(signal_type: &patch::SignalType) -> Self {
        match signal_type {
            patch::SignalType::Audio => Self::Audio,
            patch::SignalType::Control => Self::Control,
            patch::SignalType::Event => Self::Event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_expose_named_typed_input_and_output_ports() {
        let module = ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_input("audio_in", SignalType::Audio)
            .with_input("gain", SignalType::Control)
            .with_output("audio_out", SignalType::Audio);

        assert_eq!(module.id().as_str(), "vca");
        assert_eq!(module.module_type(), "gain");
        assert_eq!(module.inputs()[0].name(), "audio_in");
        assert_eq!(module.inputs()[0].direction(), PortDirection::Input);
        assert_eq!(module.inputs()[1].signal_type(), SignalType::Control);
        assert_eq!(module.outputs()[0].direction(), PortDirection::Output);
        assert_eq!(module.outputs()[0].signal_type(), SignalType::Audio);
    }

    #[test]
    fn graph_contains_modules_and_explicit_cables_between_ports() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_output("audio", SignalType::Audio),
                ModuleNode::new(ModuleId::new("out"), "audio_output")
                    .with_input("left", SignalType::Audio),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("osc"), "audio"),
                PortRef::new(ModuleId::new("out"), "left"),
            )],
        );

        assert_eq!(graph.modules().len(), 2);
        assert_eq!(graph.cables().len(), 1);
        assert_eq!(graph.cables()[0].source().module_id().as_str(), "osc");
        assert_eq!(graph.cables()[0].source().port_name(), "audio");
        assert_eq!(graph.cables()[0].destination().module_id().as_str(), "out");
        assert_eq!(graph.cables()[0].destination().port_name(), "left");
    }

    #[test]
    fn graph_is_constructed_from_validated_patch_declarations() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Graph Patch
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: osc
    type: oscillator
    outputs:
      - name: audio
        signal_type: audio
  - id: vca
    type: gain
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: control
    outputs:
      - name: audio_out
        signal_type: audio
connections:
  - from: osc.audio
    to: vca.audio_in
"#,
        )
        .expect("patch should parse");

        patch::validate_patch_schema(&patch).expect("patch schema should be valid");

        let graph = Graph::from_patch_declarations(&patch);

        assert_eq!(graph.modules().len(), 2);
        assert_eq!(graph.modules()[0].id().as_str(), "osc");
        assert_eq!(
            graph.modules()[0].outputs()[0].signal_type(),
            SignalType::Audio
        );
        assert_eq!(graph.modules()[1].inputs()[1].name(), "gain");
        assert_eq!(
            graph.modules()[1].inputs()[1].signal_type(),
            SignalType::Control
        );
        assert_eq!(graph.cables()[0].source().module_id().as_str(), "osc");
        assert_eq!(graph.cables()[0].destination().port_name(), "audio_in");
    }

    #[test]
    fn composite_instance_expands_to_namespaced_internal_modules_and_cables() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Composite Expansion
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
module_definitions:
  - type: drum_voice
    inputs:
      - name: pitch
        signal_type: control
        maps_to:
          - osc.pitch
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    modules:
      - id: osc
        type: oscillator
      - id: vca
        type: gain
    connections:
      - from: osc.audio
        to: vca.audio_in
modules:
  - id: pitch
    type: lfo
  - id: voice
    type: drum_voice
  - id: out
    type: audio_output
connections:
  - from: pitch.value
    to: voice.pitch
  - from: voice.audio
    to: out.left
"#,
        )
        .expect("patch should parse");
        patch::validate_patch_schema(&patch).expect("patch schema should validate");

        let graph = Graph::from_patch_declarations(&patch);

        let module_ids = graph
            .modules()
            .iter()
            .map(|module| module.id().as_str())
            .collect::<Vec<_>>();
        assert_eq!(module_ids, ["pitch", "voice::osc", "voice::vca", "out"]);
        let cable_pairs = graph
            .cables()
            .iter()
            .map(|cable| format!("{}->{}", cable.source(), cable.destination()))
            .collect::<Vec<_>>();
        assert_eq!(
            cable_pairs,
            [
                "voice::osc.audio->voice::vca.audio_in",
                "pitch.value->voice::osc.pitch",
                "voice::vca.audio_out->out.left"
            ]
        );
    }

    #[test]
    fn multiple_composite_instances_expand_without_id_collisions() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Composite Expansion
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
module_definitions:
  - type: drum_voice
    inputs:
      - name: pitch
        signal_type: control
        maps_to:
          - osc.pitch
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - osc.audio
    modules:
      - id: osc
        type: oscillator
modules:
  - id: voice_a
    type: drum_voice
  - id: voice_b
    type: drum_voice
  - id: out
    type: audio_output
connections:
  - from: voice_a.audio
    to: out.left
  - from: voice_b.audio
    to: out.right
"#,
        )
        .expect("patch should parse");
        patch::validate_patch_schema(&patch).expect("patch schema should validate");

        let graph = Graph::from_patch_declarations(&patch);

        let module_ids = graph
            .modules()
            .iter()
            .map(|module| module.id().as_str())
            .collect::<Vec<_>>();
        assert_eq!(module_ids, ["voice_a::osc", "voice_b::osc", "out"]);
        let mut unique = BTreeSet::new();
        for module_id in module_ids {
            assert!(
                unique.insert(module_id),
                "duplicate expanded ID: {module_id}"
            );
        }
        let cable_pairs = graph
            .cables()
            .iter()
            .map(|cable| format!("{}->{}", cable.source(), cable.destination()))
            .collect::<Vec<_>>();
        assert_eq!(
            cable_pairs,
            [
                "voice_a::osc.audio->out.left",
                "voice_b::osc.audio->out.right"
            ]
        );
    }

    #[test]
    fn expanded_composite_diagnostics_include_instance_and_internal_module_path() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Composite Diagnostics
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
module_definitions:
  - type: drum_voice
    inputs: []
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    modules:
      - id: osc
        type: oscillator
      - id: vca
        type: gain
    connections:
      - from: osc.audio
        to: vca.missing
modules:
  - id: voice
    type: drum_voice
  - id: out
    type: audio_output
connections:
  - from: voice.audio
    to: out.left
"#,
        )
        .expect("patch should parse");

        let graph = Graph::from_patch_declarations(&patch);
        let error = graph
            .validate()
            .expect_err("invalid expanded internal route should fail");

        assert!(error.to_string().contains("voice::vca.missing"));
    }

    #[test]
    fn composite_cannot_hide_implicit_many_to_one_internal_route() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Composite Many To One
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
module_definitions:
  - type: bad_voice
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    modules:
      - id: osc_a
        type: oscillator
      - id: osc_b
        type: oscillator
      - id: vca
        type: gain
    connections:
      - from: osc_a.audio
        to: vca.audio_in
      - from: osc_b.audio
        to: vca.audio_in
modules:
  - id: voice
    type: bad_voice
  - id: out
    type: audio_output
connections:
  - from: voice.audio
    to: out.left
"#,
        )
        .expect("patch should parse");

        let graph = Graph::from_patch_declarations(&patch);
        let error = graph
            .validate()
            .expect_err("hidden many-to-one route should fail");

        assert!(error.to_string().contains("voice::vca.audio_in"));
        assert!(error.to_string().contains("explicit mixer"));
    }

    #[test]
    fn composite_cannot_hide_instantaneous_internal_audio_feedback() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Composite Feedback
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
module_definitions:
  - type: bad_voice
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - a.audio_out
    modules:
      - id: a
        type: gain
      - id: b
        type: gain
    connections:
      - from: a.audio_out
        to: b.audio_in
      - from: b.audio_out
        to: a.audio_in
modules:
  - id: voice
    type: bad_voice
  - id: out
    type: audio_output
connections:
  - from: voice.audio
    to: out.left
"#,
        )
        .expect("patch should parse");

        let graph = Graph::from_patch_declarations(&patch);
        let error = graph
            .validate()
            .expect_err("hidden instantaneous feedback should fail");

        assert!(error.to_string().contains("routing cycle detected"));
        assert!(
            error
                .to_string()
                .contains("voice::a.audio_out->voice::b.audio_in")
        );
    }

    #[test]
    fn script_module_ports_declared_in_yaml_are_loaded_into_the_graph() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Script Ports
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
    outputs:
      - name: events
        signal_type: event
  - id: accent_script
    type: script
    inputs:
      - name: notes
        signal_type: event
    outputs:
      - name: accent
        signal_type: control
  - id: vca
    type: gain
    inputs:
      - name: gain
        signal_type: control
connections:
  - from: midi.events
    to: accent_script.notes
  - from: accent_script.accent
    to: vca.gain
"#,
        )
        .expect("patch should parse");

        patch::validate_patch_schema(&patch).expect("patch schema should be valid");

        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("script custom ports should validate as graph ports");
        let script = graph
            .modules()
            .iter()
            .find(|module| module.id().as_str() == "accent_script")
            .expect("script node should be loaded");
        assert_eq!(script.inputs()[0].name(), "notes");
        assert_eq!(script.inputs()[0].signal_type(), SignalType::Event);
        assert_eq!(script.outputs()[0].name(), "accent");
        assert_eq!(script.outputs()[0].signal_type(), SignalType::Control);
    }

    #[test]
    fn validation_accepts_compatible_output_to_input_route() {
        let graph = audio_graph("osc", "audio", "out", "left");

        graph.validate().expect("compatible route should validate");
    }

    #[test]
    fn signal_compatibility_accepts_only_matching_signal_families() {
        let signal_types = [SignalType::Audio, SignalType::Control, SignalType::Event];

        for source in signal_types {
            for destination in signal_types {
                assert_eq!(
                    source.is_compatible_with(destination),
                    source == destination,
                    "{source:?} -> {destination:?} compatibility mismatch"
                );
            }
        }
    }

    #[test]
    fn validation_accepts_compatible_control_route() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("env"), "adsr")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input("gain", SignalType::Control),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("env"), "value"),
                PortRef::new(ModuleId::new("vca"), "gain"),
            )],
        );

        graph.validate().expect("control route should validate");
    }

    #[test]
    fn validation_accepts_event_control_and_audio_routes_for_built_in_sampler() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Sampler Routing
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
assets:
  - id: hit
    kind: sample
    path: hit.wav
modules:
  - id: midi
    type: midi_input
  - id: sampler
    type: sampler
    parameters:
      asset: hit
  - id: lfo
    type: lfo
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: sampler.trigger
  - from: lfo.value
    to: sampler.rate
  - from: lfo.value
    to: sampler.start
  - from: sampler.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
        )
        .expect("patch should parse");

        patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("sampler built-in ports should validate compatible routes");
    }

    #[test]
    fn validation_rejects_ad_hoc_event_ports_on_pure_built_in_generators() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Pure Generators
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
assets:
  - id: hit
    kind: sample
    path: hit.wav
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
    inputs:
      - name: gate
        signal_type: event
  - id: sampler
    type: sampler
    inputs:
      - name: gate
        signal_type: event
    parameters:
      asset: hit
connections:
  - from: midi.events
    to: osc.gate
  - from: midi.events
    to: sampler.gate
"#,
        )
        .expect("patch should parse");

        patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);
        let error = graph
            .validate()
            .expect_err("built-in generators should not accept ad hoc MIDI event ports");

        assert!(error.diagnostics().contains(&GraphDiagnostic::MissingPort {
            port: port_ref("osc", "gate"),
        }));
        assert!(error.diagnostics().contains(&GraphDiagnostic::MissingPort {
            port: port_ref("sampler", "gate"),
        }));
    }

    #[test]
    fn modulatable_destinations_are_represented_as_control_input_ports() {
        let module = ModuleNode::new(ModuleId::new("voice"), "synth_voice")
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_input(builtin_ports::PITCH, SignalType::Control)
            .with_input(builtin_ports::CUTOFF, SignalType::Control)
            .with_input(builtin_ports::PAN, SignalType::Control)
            .with_input(builtin_ports::ATTACK, SignalType::Control)
            .with_input(builtin_ports::DECAY, SignalType::Control)
            .with_input(builtin_ports::SUSTAIN, SignalType::Control)
            .with_input(builtin_ports::RELEASE, SignalType::Control);

        let ports: Vec<(&str, SignalType)> = module
            .inputs()
            .iter()
            .map(|port| (port.name(), port.signal_type()))
            .collect();

        assert_eq!(
            ports,
            vec![
                ("gain", SignalType::Control),
                ("pitch", SignalType::Control),
                ("cutoff", SignalType::Control),
                ("pan", SignalType::Control),
                ("attack", SignalType::Control),
                ("decay", SignalType::Control),
                ("sustain", SignalType::Control),
                ("release", SignalType::Control),
            ]
        );
    }

    #[test]
    fn validation_routes_control_sources_to_modulatable_destination_ports() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("lfo"), "lfo")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("filter"), "filter")
                    .with_input(builtin_ports::CUTOFF, SignalType::Control),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("lfo"), "value"),
                PortRef::new(ModuleId::new("filter"), "cutoff"),
            )],
        );

        graph
            .validate()
            .expect("cutoff modulation should be routed through a control port");
    }

    #[test]
    fn validation_accepts_any_control_source_for_any_modulatable_destination() {
        let sources = [("env", "adsr"), ("lfo", "lfo"), ("script", "script")];
        let destinations = [
            builtin_ports::GAIN,
            builtin_ports::PITCH,
            builtin_ports::CUTOFF,
            builtin_ports::PAN,
            builtin_ports::ATTACK,
            builtin_ports::DECAY,
            builtin_ports::SUSTAIN,
            builtin_ports::RELEASE,
        ];

        for (source_id, source_type) in sources {
            for destination in destinations {
                let graph = Graph::new(
                    vec![
                        ModuleNode::new(ModuleId::new(source_id), source_type)
                            .with_output("value", SignalType::Control),
                        ModuleNode::new(ModuleId::new("target"), "modulatable")
                            .with_input(destination, SignalType::Control),
                    ],
                    vec![Cable::new(
                        PortRef::new(ModuleId::new(source_id), "value"),
                        PortRef::new(ModuleId::new("target"), destination),
                    )],
                );

                graph.validate().unwrap_or_else(|error| {
                    panic!("{source_type} should route to {destination}: {error}")
                });
            }
        }
    }

    #[test]
    fn validation_reports_missing_module_or_port_references() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("out"), "audio_output")
                    .with_input("left", SignalType::Audio),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("osc"), "audio"),
                PortRef::new(ModuleId::new("out"), "right"),
            )],
        );

        let error = graph
            .validate()
            .expect_err("missing references should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::MissingModule {
                    module_id: ModuleId::new("osc"),
                })
        );
        assert!(error.diagnostics().contains(&GraphDiagnostic::MissingPort {
            port: port_ref("out", "right"),
        }));
    }

    #[test]
    fn validation_reports_incorrect_port_directions() {
        let graph = audio_graph("out", "left", "osc", "audio");

        let error = graph.validate().expect_err("wrong directions should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::IncorrectPortDirection {
                    port: port_ref("out", "left"),
                    expected: PortDirection::Output,
                })
        );
        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::IncorrectPortDirection {
                    port: port_ref("osc", "audio"),
                    expected: PortDirection::Input,
                })
        );
    }

    #[test]
    fn validation_reports_incompatible_signal_types() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_output("audio", SignalType::Audio),
                ModuleNode::new(ModuleId::new("script"), "script")
                    .with_input("notes", SignalType::Event),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("osc"), "audio"),
                PortRef::new(ModuleId::new("script"), "notes"),
            )],
        );

        let error = graph
            .validate()
            .expect_err("incompatible types should fail");

        assert_eq!(
            error.diagnostics()[0],
            GraphDiagnostic::IncompatibleSignalTypes {
                source: port_ref("osc", "audio"),
                source_type: SignalType::Audio,
                destination: port_ref("script", "notes"),
                destination_type: SignalType::Event,
            }
        );
    }

    #[test]
    fn validation_reports_audio_to_control_incompatibility() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_output("audio", SignalType::Audio),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input("gain", SignalType::Control),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("osc"), "audio"),
                PortRef::new(ModuleId::new("vca"), "gain"),
            )],
        );

        let error = graph.validate().expect_err("audio to control should fail");

        assert_eq!(
            error.diagnostics()[0],
            GraphDiagnostic::IncompatibleSignalTypes {
                source: port_ref("osc", "audio"),
                source_type: SignalType::Audio,
                destination: port_ref("vca", "gain"),
                destination_type: SignalType::Control,
            }
        );
    }

    #[test]
    fn validation_reports_control_to_audio_incompatibility() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("env"), "adsr")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("out"), "audio_output")
                    .with_input("left", SignalType::Audio),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("env"), "value"),
                PortRef::new(ModuleId::new("out"), "left"),
            )],
        );

        let error = graph.validate().expect_err("control to audio should fail");

        assert_eq!(
            error.diagnostics()[0],
            GraphDiagnostic::IncompatibleSignalTypes {
                source: port_ref("env", "value"),
                source_type: SignalType::Control,
                destination: port_ref("out", "left"),
                destination_type: SignalType::Audio,
            }
        );
    }

    #[test]
    fn validation_reports_event_to_control_incompatibility() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("midi"), "midi_input")
                    .with_output("notes", SignalType::Event),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input("gain", SignalType::Control),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("midi"), "notes"),
                PortRef::new(ModuleId::new("vca"), "gain"),
            )],
        );

        let error = graph.validate().expect_err("event to control should fail");

        assert_eq!(
            error.diagnostics()[0],
            GraphDiagnostic::IncompatibleSignalTypes {
                source: port_ref("midi", "notes"),
                source_type: SignalType::Event,
                destination: port_ref("vca", "gain"),
                destination_type: SignalType::Control,
            }
        );
    }

    #[test]
    fn validation_reports_unsupported_implicit_many_to_one_routes() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("env"), "adsr")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("lfo"), "lfo")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input("gain", SignalType::Control),
            ],
            vec![
                Cable::new(
                    PortRef::new(ModuleId::new("env"), "value"),
                    PortRef::new(ModuleId::new("vca"), "gain"),
                ),
                Cable::new(
                    PortRef::new(ModuleId::new("lfo"), "value"),
                    PortRef::new(ModuleId::new("vca"), "gain"),
                ),
            ],
        );

        let error = graph.validate().expect_err("many-to-one route should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::MultipleSourcesToInput {
                    destination: port_ref("vca", "gain"),
                })
        );
    }

    #[test]
    fn validation_accepts_multiple_control_sources_through_explicit_mixer() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("env"), "adsr")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("lfo"), "lfo")
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("control_mix"), "control_mixer")
                    .with_mixing_input("inputs", SignalType::Control)
                    .with_output("sum", SignalType::Control),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input(builtin_ports::GAIN, SignalType::Control),
            ],
            vec![
                Cable::new(
                    PortRef::new(ModuleId::new("env"), "value"),
                    PortRef::new(ModuleId::new("control_mix"), "inputs"),
                ),
                Cable::new(
                    PortRef::new(ModuleId::new("lfo"), "value"),
                    PortRef::new(ModuleId::new("control_mix"), "inputs"),
                ),
                Cable::new(
                    PortRef::new(ModuleId::new("control_mix"), "sum"),
                    PortRef::new(ModuleId::new("vca"), "gain"),
                ),
            ],
        );

        graph
            .validate()
            .expect("explicit mixer input should accept multiple control sources");
    }

    #[test]
    fn validation_reports_cycle_path_with_participating_ports() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("left"), "gain")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
                ModuleNode::new(ModuleId::new("right"), "gain")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
            ],
            vec![
                Cable::new(port_ref("left", "audio_out"), port_ref("right", "audio_in")),
                Cable::new(port_ref("right", "audio_out"), port_ref("left", "audio_in")),
            ],
        );

        let error = graph.validate().expect_err("cycle should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::CycleDetected {
                    path: vec![
                        Cable::new(port_ref("left", "audio_out"), port_ref("right", "audio_in")),
                        Cable::new(port_ref("right", "audio_out"), port_ref("left", "audio_in")),
                    ],
                })
        );
    }

    #[test]
    fn validation_accepts_audio_feedback_cycle_with_explicit_audio_delay_boundary() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("gain"), "gain")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
                ModuleNode::new(ModuleId::new("delay"), "audio_delay_one_sample")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio)
                    .with_feedback_boundary(SignalType::Audio),
            ],
            vec![
                Cable::new(port_ref("gain", "audio_out"), port_ref("delay", "audio_in")),
                Cable::new(port_ref("delay", "audio_out"), port_ref("gain", "audio_in")),
            ],
        );

        graph
            .validate()
            .expect("audio feedback through explicit audio delay should validate");
    }

    #[test]
    fn validation_rejects_instantaneous_audio_feedback_cycle() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("left"), "gain")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
                ModuleNode::new(ModuleId::new("right"), "gain")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
            ],
            vec![
                Cable::new(port_ref("left", "audio_out"), port_ref("right", "audio_in")),
                Cable::new(port_ref("right", "audio_out"), port_ref("left", "audio_in")),
            ],
        );

        let error = graph
            .validate()
            .expect_err("instantaneous audio feedback should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::CycleDetected {
                    path: vec![
                        Cable::new(port_ref("left", "audio_out"), port_ref("right", "audio_in")),
                        Cable::new(port_ref("right", "audio_out"), port_ref("left", "audio_in")),
                    ],
                })
        );
    }

    #[test]
    fn validation_accepts_control_feedback_cycle_with_explicit_control_delay_boundary() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("scale"), "control_scale")
                    .with_input("value", SignalType::Control)
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("delay"), "control_delay")
                    .with_input("value", SignalType::Control)
                    .with_output("value", SignalType::Control)
                    .with_feedback_boundary(SignalType::Control),
            ],
            vec![
                Cable::new(port_ref("scale", "value"), port_ref("delay", "value")),
                Cable::new(port_ref("delay", "value"), port_ref("scale", "value")),
            ],
        );

        graph
            .validate()
            .expect("control feedback through explicit control delay should validate");
    }

    #[test]
    fn validation_rejects_instantaneous_control_feedback_cycle() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("left"), "control_scale")
                    .with_input("value", SignalType::Control)
                    .with_output("value", SignalType::Control),
                ModuleNode::new(ModuleId::new("right"), "control_scale")
                    .with_input("value", SignalType::Control)
                    .with_output("value", SignalType::Control),
            ],
            vec![
                Cable::new(port_ref("left", "value"), port_ref("right", "value")),
                Cable::new(port_ref("right", "value"), port_ref("left", "value")),
            ],
        );

        let error = graph
            .validate()
            .expect_err("instantaneous control feedback should fail");

        assert!(
            error
                .diagnostics()
                .contains(&GraphDiagnostic::CycleDetected {
                    path: vec![
                        Cable::new(port_ref("left", "value"), port_ref("right", "value")),
                        Cable::new(port_ref("right", "value"), port_ref("left", "value")),
                    ],
                })
        );
    }

    #[test]
    fn validation_accepts_event_feedback_cycle_for_future_scheduling() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("first"), "script")
                    .with_input("events", SignalType::Event)
                    .with_output("events", SignalType::Event),
                ModuleNode::new(ModuleId::new("second"), "script")
                    .with_input("events", SignalType::Event)
                    .with_output("events", SignalType::Event),
            ],
            vec![
                Cable::new(port_ref("first", "events"), port_ref("second", "events")),
                Cable::new(port_ref("second", "events"), port_ref("first", "events")),
            ],
        );

        graph
            .validate()
            .expect("event feedback should be handled by future-block scheduling");
    }

    // --- Section 3: Voice sub-synth scope and routing validation ---

    #[test]
    fn built_in_voice_modules_are_voice_scope() {
        let registry = crate::builtins::BuiltInModuleRegistry::new();

        for module_type in [
            "oscillator",
            "gain",
            "filter",
            "adsr",
            "sampler",
            "script",
            "note_to_rate",
        ] {
            let definition = registry.get(module_type).unwrap_or_else(|| {
                panic!("{module_type} should be built in");
            });
            assert_eq!(
                definition.execution_scope(),
                ExecutionScope::Voice,
                "{module_type} should be Voice scope"
            );
        }
    }

    #[test]
    fn built_in_global_modules_are_global_scope() {
        let registry = crate::builtins::BuiltInModuleRegistry::new();

        for module_type in [
            "midi_input",
            "audio_output",
            "audio_mixer",
            "control_mixer",
            "lfo",
            "audio_delay_one_sample",
            "block_delay",
            "control_delay",
        ] {
            let definition = registry.get(module_type).unwrap_or_else(|| {
                panic!("{module_type} should be built in");
            });
            assert_eq!(
                definition.execution_scope(),
                ExecutionScope::Global,
                "{module_type} should be Global scope"
            );
        }
    }

    #[test]
    fn voice_local_sub_synth_chain_through_mixer_validates() {
        let patch = crate::patch::load_patch_str(
            r#"
metadata:
  name: Voice Sub-Synth
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: osc
    type: oscillator
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: osc.audio
    to: vca.audio_in
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
        )
        .expect("patch should parse");

        crate::patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("voice sub-synth chain through mixer should validate");
    }

    #[test]
    fn voice_local_sub_synth_with_control_and_output_shaping_validates() {
        let patch = crate::patch::load_patch_str(
            r#"
metadata:
  name: Voice Sub-Synth With ADSR
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: osc
    type: oscillator
  - id: adsr
    type: adsr
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: osc.audio
    to: vca.audio_in
  - from: adsr.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
        )
        .expect("patch should parse");

        crate::patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("voice sub-synth with ADSR control routing should validate");
    }

    #[test]
    fn voice_to_global_direct_routing_without_mixer_is_rejected() {
        let patch = crate::patch::load_patch_str(
            r#"
metadata:
  name: Direct Voice To Output
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: osc
    type: oscillator
  - id: out
    type: audio_output
connections:
  - from: osc.audio
    to: out.left
"#,
        )
        .expect("patch should parse");

        crate::patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);
        let error = graph
            .validate()
            .expect_err("voice-to-global direct routing should fail");

        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| matches!(d, GraphDiagnostic::VoiceToGlobalDirectRouting { .. }))
        );
    }

    #[test]
    fn explicit_audio_mixer_accepts_multiple_voice_sources() {
        let patch = crate::patch::load_patch_str(
            r#"
metadata:
  name: Two Voices To Mixer
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: osc1
    type: oscillator
  - id: osc2
    type: oscillator
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: osc1.audio
    to: mixer.inputs
  - from: osc2.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
        )
        .expect("patch should parse");

        crate::patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("explicit audio mixer should accept multiple voice sources");
    }

    #[test]
    fn explicit_note_to_rate_converter_accepts_event_input_and_control_output() {
        let patch = crate::patch::load_patch_str(
            r#"
metadata:
  name: Note To Rate Converter
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
  - id: note_rate
    type: note_to_rate
  - id: osc
    type: oscillator
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: note_rate.events
  - from: note_rate.rate
    to: osc.pitch
  - from: osc.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
        )
        .expect("patch should parse");

        crate::patch::validate_patch_schema(&patch).expect("patch schema should be valid");
        let graph = Graph::from_patch_declarations(&patch);

        graph
            .validate()
            .expect("note_to_rate converter should accept event input and emit control output");
    }

    #[test]
    fn implicit_audio_to_control_routing_is_rejected() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_output("audio", SignalType::Audio),
                ModuleNode::new(ModuleId::new("vca"), "gain")
                    .with_input("gain", SignalType::Control),
            ],
            vec![Cable::new(
                port_ref("osc", "audio"),
                port_ref("vca", "gain"),
            )],
        );

        let error = graph
            .validate()
            .expect_err("implicit audio-to-control routing should fail");

        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| matches!(d, GraphDiagnostic::IncompatibleSignalTypes { .. }))
        );
    }

    fn port_ref(module_id: &str, port_name: &str) -> PortRef {
        PortRef::new(ModuleId::new(module_id), port_name)
    }

    fn audio_graph(
        source_module: &str,
        source_port: &str,
        destination_module: &str,
        destination_port: &str,
    ) -> Graph {
        Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_output("audio", SignalType::Audio),
                ModuleNode::new(ModuleId::new("out"), "audio_output")
                    .with_input("left", SignalType::Audio),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new(source_module), source_port),
                PortRef::new(ModuleId::new(destination_module), destination_port),
            )],
        )
    }
}
