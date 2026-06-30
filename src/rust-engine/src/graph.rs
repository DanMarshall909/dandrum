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

pub mod builtin_ports;

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
mod tests;
