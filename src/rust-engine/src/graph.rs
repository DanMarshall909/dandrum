use std::collections::BTreeMap;
use std::fmt;

use crate::patch;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleNode {
    id: ModuleId,
    module_type: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Port {
    name: String,
    direction: PortDirection,
    signal_type: SignalType,
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
        }
    }

    pub fn with_input(mut self, name: impl Into<String>, signal_type: SignalType) -> Self {
        self.inputs.push(Port::input(name, signal_type));
        self
    }

    pub fn with_output(mut self, name: impl Into<String>, signal_type: SignalType) -> Self {
        self.outputs.push(Port::output(name, signal_type));
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
}

impl Port {
    pub fn input(name: impl Into<String>, signal_type: SignalType) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Input,
            signal_type,
        }
    }

    pub fn output(name: impl Into<String>, signal_type: SignalType) -> Self {
        Self {
            name: name.into(),
            direction: PortDirection::Output,
            signal_type,
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
        let modules = patch
            .modules
            .iter()
            .map(|module| {
                let mut node =
                    ModuleNode::new(ModuleId::new(module.id.clone()), module.module_type.clone());

                for input in &module.inputs {
                    node =
                        node.with_input(input.name.clone(), SignalType::from(&input.signal_type));
                }

                for output in &module.outputs {
                    node = node
                        .with_output(output.name.clone(), SignalType::from(&output.signal_type));
                }

                node
            })
            .collect();

        let cables = patch
            .connections
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

        for cable in &self.cables {
            *destination_counts.entry(cable.destination()).or_default() += 1;

            let source = self.resolve_port(cable.source(), PortDirection::Output, &mut diagnostics);
            let destination =
                self.resolve_port(cable.destination(), PortDirection::Input, &mut diagnostics);

            if let (Some(source), Some(destination)) = (source, destination) {
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
            }
        }

        for (destination, count) in destination_counts {
            if count > 1 {
                diagnostics.push(GraphDiagnostic::MultipleSourcesToInput {
                    destination: destination.clone(),
                });
            }
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
                write!(formatter, "incorrect port direction: {port} is not a {expected:?} port")
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

        assert!(error.diagnostics().contains(&GraphDiagnostic::MissingModule {
            module_id: ModuleId::new("osc"),
        }));
        assert!(error.diagnostics().contains(&GraphDiagnostic::MissingPort {
            port: port_ref("out", "right"),
        }));
    }

    #[test]
    fn validation_reports_incorrect_port_directions() {
        let graph = audio_graph("out", "left", "osc", "audio");

        let error = graph.validate().expect_err("wrong directions should fail");

        assert!(error
            .diagnostics()
            .contains(&GraphDiagnostic::IncorrectPortDirection {
                port: port_ref("out", "left"),
                expected: PortDirection::Output,
            }));
        assert!(error
            .diagnostics()
            .contains(&GraphDiagnostic::IncorrectPortDirection {
                port: port_ref("osc", "audio"),
                expected: PortDirection::Input,
            }));
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

        let error = graph
            .validate()
            .expect_err("audio to control should fail");

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

        let error = graph
            .validate()
            .expect_err("control to audio should fail");

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

        let error = graph
            .validate()
            .expect_err("event to control should fail");

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
