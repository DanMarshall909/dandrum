use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::diagnostics::{Diagnostic, Severity, error_codes};

pub const DEFAULT_SCRIPT_MAX_OPERATIONS: u64 = 10_000;
pub const DEFAULT_SCRIPT_MAX_CALL_DEPTH: usize = 16;
pub const DEFAULT_SCRIPT_MAX_INPUT_EVENTS: usize = 256;
pub const DEFAULT_SCRIPT_MAX_EMITTED_EVENTS_PER_PORT: usize = 256;
pub const DEFAULT_SCRIPT_MAX_CONTROL_OUTPUTS: usize = 64;
pub const DEFAULT_SCRIPT_MAX_STATE_ENTRIES: usize = 128;
pub const DEFAULT_SCRIPT_MAX_KEY_LENGTH: usize = 64;
pub const DEFAULT_SCRIPT_MAX_DYNAMIC_VALUE_SIZE: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptProcessInput {
    events: Vec<ScriptEvent>,
    controls: BTreeMap<String, f32>,
    context: ScriptExecutionContext,
    state: ScriptModuleState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptProcessOutput {
    pub events: BTreeMap<String, Vec<ScriptEvent>>,
    pub controls: BTreeMap<String, f32>,
    pub state: ScriptModuleState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptModuleState {
    values: BTreeMap<String, ScriptValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRuntimeLimits {
    pub max_operations: u64,
    pub max_call_depth: usize,
    pub max_input_events: usize,
    pub max_emitted_events_per_port: usize,
    pub max_control_outputs: usize,
    pub max_state_entries: usize,
    pub max_key_length: usize,
    pub max_dynamic_value_size: usize,
}

pub struct RhaiScriptRuntime {
    engine: rhai::Engine,
    ast: rhai::AST,
    limits: ScriptRuntimeLimits,
    event_outputs: BTreeSet<String>,
    control_outputs: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct DandrumScriptContext {
    data: Arc<Mutex<DandrumScriptContextData>>,
}

#[derive(Clone, Debug)]
struct DandrumScriptContextData {
    input_events: Vec<ScriptEvent>,
    input_controls: BTreeMap<String, f32>,
    output_events: BTreeMap<String, Vec<ScriptEvent>>,
    output_controls: BTreeMap<String, f32>,
    state: ScriptModuleState,
    event_outputs: BTreeSet<String>,
    control_outputs: BTreeSet<String>,
    limits: ScriptRuntimeLimits,
    error: Option<ScriptExecutionError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptPrepareError {
    Parse { message: String },
    MissingEntryPoint,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptFeedbackScheduler {
    current_block: BTreeMap<String, Vec<ScriptEvent>>,
    next_block: BTreeMap<String, Vec<ScriptEvent>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptValue {
    Number(f32),
}

impl ScriptModuleState {
    pub fn get(&self, key: &str) -> Option<ScriptValue> {
        self.values.get(key).copied()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: ScriptValue) {
        self.values.insert(key.into(), value);
    }
}

impl Default for ScriptRuntimeLimits {
    fn default() -> Self {
        Self {
            max_operations: DEFAULT_SCRIPT_MAX_OPERATIONS,
            max_call_depth: DEFAULT_SCRIPT_MAX_CALL_DEPTH,
            max_input_events: DEFAULT_SCRIPT_MAX_INPUT_EVENTS,
            max_emitted_events_per_port: DEFAULT_SCRIPT_MAX_EMITTED_EVENTS_PER_PORT,
            max_control_outputs: DEFAULT_SCRIPT_MAX_CONTROL_OUTPUTS,
            max_state_entries: DEFAULT_SCRIPT_MAX_STATE_ENTRIES,
            max_key_length: DEFAULT_SCRIPT_MAX_KEY_LENGTH,
            max_dynamic_value_size: DEFAULT_SCRIPT_MAX_DYNAMIC_VALUE_SIZE,
        }
    }
}

impl RhaiScriptRuntime {
    pub fn compile(source: &str, limits: ScriptRuntimeLimits) -> Result<Self, ScriptPrepareError> {
        Self::compile_with_output_ports(source, limits, Vec::new(), Vec::new())
    }

    pub fn compile_with_output_ports(
        source: &str,
        limits: ScriptRuntimeLimits,
        event_outputs: Vec<String>,
        control_outputs: Vec<String>,
    ) -> Result<Self, ScriptPrepareError> {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(limits.max_operations);
        engine.set_max_call_levels(limits.max_call_depth);
        register_script_context_api(&mut engine);

        let ast = engine
            .compile(source)
            .map_err(|error| ScriptPrepareError::Parse {
                message: error.to_string(),
            })?;

        if !ast
            .iter_functions()
            .any(|function| function.name == "process" && function.params.len() == 1)
        {
            return Err(ScriptPrepareError::MissingEntryPoint);
        }

        Ok(Self {
            engine,
            ast,
            limits,
            event_outputs: event_outputs.into_iter().collect(),
            control_outputs: control_outputs.into_iter().collect(),
        })
    }

    pub fn limits(&self) -> ScriptRuntimeLimits {
        self.limits
    }
}

impl DandrumScriptContext {
    fn new(
        input: ScriptProcessInput,
        event_outputs: BTreeSet<String>,
        control_outputs: BTreeSet<String>,
        limits: ScriptRuntimeLimits,
    ) -> Self {
        let input_events = input
            .events
            .into_iter()
            .take(limits.max_input_events)
            .collect();

        Self {
            data: Arc::new(Mutex::new(DandrumScriptContextData {
                input_events,
                input_controls: input.controls,
                output_events: BTreeMap::new(),
                output_controls: BTreeMap::new(),
                state: input.state,
                event_outputs,
                control_outputs,
                limits,
                error: None,
            })),
        }
    }

    fn events(&mut self) -> rhai::Array {
        self.data
            .lock()
            .expect("script context mutex should not be poisoned")
            .input_events
            .iter()
            .map(script_event_to_dynamic)
            .collect()
    }

    fn controls(&mut self) -> rhai::Map {
        self.data
            .lock()
            .expect("script context mutex should not be poisoned")
            .input_controls
            .iter()
            .map(|(key, value)| (key.as_str().into(), f64::from(*value).into()))
            .collect()
    }

    fn emit(&mut self, port: &str, event: rhai::Map) {
        let mut data = self
            .data
            .lock()
            .expect("script context mutex should not be poisoned");

        if !data.event_outputs.contains(port) {
            data.error = Some(ScriptExecutionError::UndeclaredOutputPort {
                port: port.to_string(),
            });
            return;
        }

        let limit = data.limits.max_emitted_events_per_port;
        let events = data.output_events.entry(port.to_string()).or_default();
        if events.len() >= limit {
            data.error = Some(ScriptExecutionError::OutputLimitExceeded {
                port: port.to_string(),
                limit,
            });
            return;
        }

        if let Some(event) = dynamic_map_to_script_event(&event) {
            events.push(event);
        }
    }

    fn control(&mut self, port: &str, value: rhai::FLOAT) {
        let mut data = self
            .data
            .lock()
            .expect("script context mutex should not be poisoned");

        if !data.control_outputs.contains(port) {
            data.error = Some(ScriptExecutionError::UndeclaredOutputPort {
                port: port.to_string(),
            });
            return;
        }

        if !data.output_controls.contains_key(port)
            && data.output_controls.len() >= data.limits.max_control_outputs
        {
            data.error = Some(ScriptExecutionError::OutputLimitExceeded {
                port: port.to_string(),
                limit: data.limits.max_control_outputs,
            });
            return;
        }

        data.output_controls.insert(port.to_string(), value as f32);
    }

    fn state_get(&mut self, key: &str) -> rhai::FLOAT {
        let data = self
            .data
            .lock()
            .expect("script context mutex should not be poisoned");

        match data.state.get(key) {
            Some(ScriptValue::Number(value)) => rhai::FLOAT::from(value),
            None => 0.0,
        }
    }

    fn state_set(&mut self, key: &str, value: rhai::FLOAT) {
        let mut data = self
            .data
            .lock()
            .expect("script context mutex should not be poisoned");

        if key.len() > data.limits.max_key_length {
            data.error = Some(ScriptExecutionError::KeyLengthExceeded {
                key: key.to_string(),
                limit: data.limits.max_key_length,
            });
            return;
        }

        if data.state.get(key).is_none() && data.state.values.len() >= data.limits.max_state_entries
        {
            data.error = Some(ScriptExecutionError::StateLimitExceeded {
                limit: data.limits.max_state_entries,
            });
            return;
        }

        data.state.insert(key, ScriptValue::Number(value as f32));
    }

    fn into_output(self) -> Result<ScriptProcessOutput, ScriptExecutionError> {
        let data = self
            .data
            .lock()
            .expect("script context mutex should not be poisoned");

        if let Some(error) = data.error.clone() {
            return Err(error);
        }

        Ok(ScriptProcessOutput {
            events: data.output_events.clone(),
            controls: data.output_controls.clone(),
            state: data.state.clone(),
        })
    }
}

fn register_script_context_api(engine: &mut rhai::Engine) {
    engine.register_type::<DandrumScriptContext>();
    engine.register_get("events", DandrumScriptContext::events);
    engine.register_get("controls", DandrumScriptContext::controls);
    engine.register_fn("emit", DandrumScriptContext::emit);
    engine.register_fn("control", DandrumScriptContext::control);
    engine.register_fn("state_get", DandrumScriptContext::state_get);
    engine.register_fn("state_set", DandrumScriptContext::state_set);
}

fn script_event_to_dynamic(event: &ScriptEvent) -> rhai::Dynamic {
    let mut map = rhai::Map::new();
    match event {
        ScriptEvent::NoteOn { note, velocity } => {
            map.insert("type".into(), "note_on".into());
            map.insert("note".into(), i64::from(*note).into());
            map.insert("velocity".into(), i64::from(*velocity).into());
        }
        ScriptEvent::NoteOff { note } => {
            map.insert("type".into(), "note_off".into());
            map.insert("note".into(), i64::from(*note).into());
        }
    }
    map.into()
}

fn dynamic_map_to_script_event(map: &rhai::Map) -> Option<ScriptEvent> {
    let event_type = map.get("type")?.clone().try_cast::<String>()?;
    let note = map.get("note")?.clone().try_cast::<i64>()? as u8;

    match event_type.as_str() {
        "note_on" => Some(ScriptEvent::NoteOn {
            note,
            velocity: map.get("velocity")?.clone().try_cast::<i64>()? as u8,
        }),
        "note_off" => Some(ScriptEvent::NoteOff { note }),
        _ => None,
    }
}

impl ScriptFeedbackScheduler {
    pub fn queue_for_next_block(
        &mut self,
        destination: impl Into<String>,
        events: Vec<ScriptEvent>,
    ) {
        self.next_block
            .entry(destination.into())
            .or_default()
            .extend(events);
    }

    pub fn events_for_current_block(&self, destination: &str) -> &[ScriptEvent] {
        self.current_block
            .get(destination)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn advance_block(&mut self) {
        self.current_block = std::mem::take(&mut self.next_block);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptExecutionContext {
    operation_budget: u32,
    operations_spent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptExecutionError {
    OperationBudgetExceeded { budget: u32, requested: u32 },
    CallDepthExceeded { limit: usize },
    RuntimeFailed { message: String },
    UndeclaredOutputPort { port: String },
    OutputLimitExceeded { port: String, limit: usize },
    StateLimitExceeded { limit: usize },
    KeyLengthExceeded { key: String, limit: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

impl ScriptProcessInput {
    pub fn new(
        events: Vec<ScriptEvent>,
        controls: BTreeMap<String, f32>,
        context: ScriptExecutionContext,
        state: ScriptModuleState,
    ) -> Self {
        Self {
            events,
            controls,
            context,
            state,
        }
    }

    pub fn events(&self) -> &[ScriptEvent] {
        &self.events
    }

    pub fn controls(&self) -> &BTreeMap<String, f32> {
        &self.controls
    }

    pub fn context_mut(&mut self) -> &mut ScriptExecutionContext {
        &mut self.context
    }

    pub fn state(&self) -> &ScriptModuleState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ScriptModuleState {
        &mut self.state
    }

    pub fn into_state(self) -> ScriptModuleState {
        self.state
    }
}

impl ScriptExecutionContext {
    pub fn new(operation_budget: u32) -> Self {
        Self {
            operation_budget,
            operations_spent: 0,
        }
    }

    pub fn spend(&mut self, operations: u32) -> Result<(), ScriptExecutionError> {
        let requested = self.operations_spent.saturating_add(operations);

        if requested > self.operation_budget {
            return Err(ScriptExecutionError::OperationBudgetExceeded {
                budget: self.operation_budget,
                requested,
            });
        }

        self.operations_spent = requested;
        Ok(())
    }
}

impl fmt::Display for ScriptExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationBudgetExceeded { budget, requested } => write!(
                formatter,
                "script operation budget exceeded: budget {budget}, requested {requested}"
            ),
            Self::CallDepthExceeded { limit } => {
                write!(formatter, "script call depth limit {limit} exceeded")
            }
            Self::RuntimeFailed { message } => {
                write!(formatter, "script runtime failed: {message}")
            }
            Self::UndeclaredOutputPort { port } => {
                write!(formatter, "script output port {port} is not declared")
            }
            Self::OutputLimitExceeded { port, limit } => write!(
                formatter,
                "script output port {port} exceeded limit {limit}"
            ),
            Self::StateLimitExceeded { limit } => {
                write!(formatter, "script state entry limit {limit} exceeded")
            }
            Self::KeyLengthExceeded { key, limit } => write!(
                formatter,
                "script state key {key} exceeded length limit {limit}"
            ),
        }
    }
}

impl fmt::Display for ScriptPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { message } => write!(formatter, "script parse failed: {message}"),
            Self::MissingEntryPoint => write!(formatter, "script missing process(ctx) entry point"),
        }
    }
}

impl std::error::Error for ScriptExecutionError {}

impl std::error::Error for ScriptPrepareError {}

impl ScriptExecutionError {
    pub fn to_diagnostic(&self, module_id: impl Into<String>) -> Diagnostic {
        match self {
            Self::OperationBudgetExceeded { budget, requested } => Diagnostic::new(
                error_codes::SCRIPT_BUDGET_EXCEEDED,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_expected(format!("<= {budget} operations"))
            .with_actual(format!("{requested} operations"))
            .with_suggested_fix("reduce script work or increase the prepared script budget"),
            Self::CallDepthExceeded { limit } => Diagnostic::new(
                error_codes::SCRIPT_BUDGET_EXCEEDED,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_expected(format!("<= {limit} call levels")),
            Self::RuntimeFailed { message } => Diagnostic::new(
                error_codes::SCRIPT_VALIDATION,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_actual(message),
            Self::UndeclaredOutputPort { port } => Diagnostic::new(
                error_codes::SCRIPT_VALIDATION,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_actual(port),
            Self::OutputLimitExceeded { port, limit } => Diagnostic::new(
                error_codes::SCRIPT_BUDGET_EXCEEDED,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_expected(format!("<= {limit} outputs"))
            .with_actual(port),
            Self::StateLimitExceeded { limit } => Diagnostic::new(
                error_codes::SCRIPT_BUDGET_EXCEEDED,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_expected(format!("<= {limit} state entries")),
            Self::KeyLengthExceeded { key, limit } => Diagnostic::new(
                error_codes::SCRIPT_BUDGET_EXCEEDED,
                Severity::Error,
                self.to_string(),
            )
            .with_module_id(module_id)
            .with_expected(format!("<= {limit} characters"))
            .with_actual(key),
        }
    }
}

pub trait ScriptRuntime {
    fn process(
        &mut self,
        input: ScriptProcessInput,
    ) -> Result<ScriptProcessOutput, ScriptExecutionError>;
}

impl ScriptRuntime for RhaiScriptRuntime {
    fn process(
        &mut self,
        input: ScriptProcessInput,
    ) -> Result<ScriptProcessOutput, ScriptExecutionError> {
        let mut scope = rhai::Scope::new();
        let ctx = DandrumScriptContext::new(
            input,
            self.event_outputs.clone(),
            self.control_outputs.clone(),
            self.limits,
        );

        self.engine
            .call_fn::<()>(&mut scope, &self.ast, "process", (ctx.clone(),))
            .map_err(|error| rhai_error_to_script_error(error.to_string(), self.limits))?;

        ctx.into_output()
    }
}

fn rhai_error_to_script_error(
    message: String,
    limits: ScriptRuntimeLimits,
) -> ScriptExecutionError {
    if message.contains("Too many operations") || message.contains("maximum operations") {
        return ScriptExecutionError::OperationBudgetExceeded {
            budget: limits.max_operations as u32,
            requested: limits.max_operations.saturating_add(1) as u32,
        };
    }

    if message.contains("Stack overflow")
        || message.contains("call stack")
        || message.contains("call depth")
        || message.contains("recursion")
    {
        return ScriptExecutionError::CallDepthExceeded {
            limit: limits.max_call_depth,
        };
    }

    ScriptExecutionError::RuntimeFailed { message }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AccentRuntime;

    impl ScriptRuntime for AccentRuntime {
        fn process(
            &mut self,
            mut input: ScriptProcessInput,
        ) -> Result<ScriptProcessOutput, ScriptExecutionError> {
            input.context_mut().spend(1)?;

            let accent = input.events().iter().any(
                |event| matches!(event, ScriptEvent::NoteOn { velocity, .. } if *velocity > 100),
            );

            Ok(ScriptProcessOutput {
                events: BTreeMap::new(),
                controls: BTreeMap::from([("accent".to_string(), if accent { 1.0 } else { 0.0 })]),
                state: input.into_state(),
            })
        }
    }

    #[test]
    fn script_runtime_processes_events_and_controls_with_explicit_budget() {
        let mut runtime = AccentRuntime;

        let output = runtime
            .process(ScriptProcessInput::new(
                vec![ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 127,
                }],
                BTreeMap::from([("threshold".to_string(), 0.75)]),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("script should stay within budget");

        assert_eq!(output.controls["accent"], 1.0);
    }

    #[test]
    fn rhai_runtime_compiles_source_before_render_processing() {
        let mut runtime = RhaiScriptRuntime::compile(
            "fn process(ctx) { let events = ctx.events; }",
            ScriptRuntimeLimits::default(),
        )
        .expect("valid Rhai script should compile during preparation");

        let output = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("render should execute the prepared AST");

        assert_eq!(output, ScriptProcessOutput::default());
    }

    #[test]
    fn rhai_runtime_rejects_malformed_source_before_render_processing() {
        let result =
            RhaiScriptRuntime::compile("fn process(ctx) {", ScriptRuntimeLimits::default());

        let Err(error) = result else {
            panic!("malformed Rhai should fail during preparation");
        };

        assert!(error.to_string().contains("script parse failed"));
    }

    #[test]
    fn script_runtime_limits_have_engine_level_maximums() {
        let limits = ScriptRuntimeLimits::default();

        assert_eq!(limits.max_operations, DEFAULT_SCRIPT_MAX_OPERATIONS);
        assert_eq!(limits.max_call_depth, DEFAULT_SCRIPT_MAX_CALL_DEPTH);
        assert_eq!(limits.max_input_events, DEFAULT_SCRIPT_MAX_INPUT_EVENTS);
        assert_eq!(
            limits.max_emitted_events_per_port,
            DEFAULT_SCRIPT_MAX_EMITTED_EVENTS_PER_PORT
        );
        assert_eq!(
            limits.max_control_outputs,
            DEFAULT_SCRIPT_MAX_CONTROL_OUTPUTS
        );
        assert_eq!(limits.max_state_entries, DEFAULT_SCRIPT_MAX_STATE_ENTRIES);
        assert_eq!(limits.max_key_length, DEFAULT_SCRIPT_MAX_KEY_LENGTH);
        assert_eq!(
            limits.max_dynamic_value_size,
            DEFAULT_SCRIPT_MAX_DYNAMIC_VALUE_SIZE
        );
    }

    #[test]
    fn rhai_scripts_read_events_and_emit_to_declared_event_ports() {
        let mut runtime = RhaiScriptRuntime::compile_with_output_ports(
            r#"
            fn process(ctx) {
                for event in ctx.events {
                    if event.type == "note_on" && event.note == 36 {
                        ctx.emit("kick", event);
                    }
                }
            }
            "#,
            ScriptRuntimeLimits::default(),
            vec!["kick".to_string()],
            Vec::new(),
        )
        .expect("event router script should compile");

        let output = runtime
            .process(ScriptProcessInput::new(
                vec![
                    ScriptEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                    ScriptEvent::NoteOn {
                        note: 38,
                        velocity: 100,
                    },
                ],
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("event router should execute");

        assert_eq!(
            output.events["kick"],
            vec![ScriptEvent::NoteOn {
                note: 36,
                velocity: 100,
            }]
        );
    }

    #[test]
    fn rhai_scripts_read_controls_and_write_declared_control_outputs() {
        let mut runtime = RhaiScriptRuntime::compile_with_output_ports(
            r#"
            fn process(ctx) {
                ctx.control("accent", ctx.controls.velocity / 127.0);
            }
            "#,
            ScriptRuntimeLimits::default(),
            Vec::new(),
            vec!["accent".to_string()],
        )
        .expect("control mapper script should compile");

        let output = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::from([("velocity".to_string(), 63.5)]),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("control mapper should execute");

        assert_eq!(output.controls["accent"], 0.5);
    }

    #[test]
    fn rhai_scripts_persist_numeric_state_between_process_calls() {
        let mut runtime = RhaiScriptRuntime::compile_with_output_ports(
            r#"
            fn process(ctx) {
                let count = ctx.state_get("count") + 1.0;
                ctx.state_set("count", count);
                ctx.control("count", count);
            }
            "#,
            ScriptRuntimeLimits::default(),
            Vec::new(),
            vec!["count".to_string()],
        )
        .expect("stateful script should compile");

        let first = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("first process call should execute");

        let second = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                first.state,
            ))
            .expect("second process call should execute");

        assert_eq!(second.controls["count"], 2.0);
        assert_eq!(second.state.get("count"), Some(ScriptValue::Number(2.0)));
    }

    #[test]
    fn rhai_scripts_report_undeclared_output_ports_as_structured_diagnostics() {
        let mut runtime = RhaiScriptRuntime::compile_with_output_ports(
            r#"
            fn process(ctx) {
                ctx.control("missing", 1.0);
            }
            "#,
            ScriptRuntimeLimits::default(),
            Vec::new(),
            vec!["declared".to_string()],
        )
        .expect("script should compile");

        let error = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("undeclared output should fail deterministically");

        assert_eq!(
            error,
            ScriptExecutionError::UndeclaredOutputPort {
                port: "missing".to_string(),
            }
        );

        let diagnostic = error.to_diagnostic("mapper");
        assert_eq!(diagnostic.error_code(), error_codes::SCRIPT_VALIDATION);
        assert_eq!(diagnostic.module_id(), Some("mapper"));
        assert_eq!(diagnostic.actual(), Some("missing"));
    }

    #[test]
    fn rhai_scripts_report_operation_budget_exhaustion() {
        let limits = ScriptRuntimeLimits {
            max_operations: 32,
            ..ScriptRuntimeLimits::default()
        };
        let mut runtime = RhaiScriptRuntime::compile(
            r#"
            fn process(ctx) {
                loop {}
            }
            "#,
            limits,
        )
        .expect("loop script should compile");

        let error = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("infinite loop should exceed operation budget");

        assert_eq!(
            error,
            ScriptExecutionError::OperationBudgetExceeded {
                budget: 32,
                requested: 33,
            }
        );
        assert_eq!(
            error.to_diagnostic("mapper").error_code(),
            error_codes::SCRIPT_BUDGET_EXCEEDED
        );
    }

    #[test]
    fn rhai_scripts_report_call_depth_exhaustion() {
        let limits = ScriptRuntimeLimits {
            max_call_depth: 4,
            ..ScriptRuntimeLimits::default()
        };
        let mut runtime = RhaiScriptRuntime::compile(
            r#"
            fn recurse() { recurse(); }
            fn process(ctx) { recurse(); }
            "#,
            limits,
        )
        .expect("recursive script should compile");

        let error = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("recursive script should exceed call depth");

        assert_eq!(error, ScriptExecutionError::CallDepthExceeded { limit: 4 });
    }

    #[test]
    fn rhai_scripts_cap_emitted_events_per_output_port() {
        let limits = ScriptRuntimeLimits {
            max_emitted_events_per_port: 1,
            ..ScriptRuntimeLimits::default()
        };
        let mut runtime = RhaiScriptRuntime::compile_with_output_ports(
            r#"
            fn process(ctx) {
                for event in ctx.events {
                    ctx.emit("out", event);
                }
            }
            "#,
            limits,
            vec!["out".to_string()],
            Vec::new(),
        )
        .expect("event cap script should compile");

        let error = runtime
            .process(ScriptProcessInput::new(
                vec![
                    ScriptEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                    ScriptEvent::NoteOn {
                        note: 38,
                        velocity: 100,
                    },
                ],
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("second emitted event should exceed cap");

        assert_eq!(
            error,
            ScriptExecutionError::OutputLimitExceeded {
                port: "out".to_string(),
                limit: 1,
            }
        );
    }

    #[test]
    fn rhai_scripts_cap_state_entries_and_key_lengths() {
        let limits = ScriptRuntimeLimits {
            max_state_entries: 1,
            max_key_length: 10,
            ..ScriptRuntimeLimits::default()
        };
        let mut state_runtime = RhaiScriptRuntime::compile(
            r#"
            fn process(ctx) {
                ctx.state_set("first", 1.0);
                ctx.state_set("second", 2.0);
            }
            "#,
            limits,
        )
        .expect("state cap script should compile");

        let state_error = state_runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("second state entry should exceed cap");

        assert_eq!(
            state_error,
            ScriptExecutionError::StateLimitExceeded { limit: 1 }
        );

        let key_limits = ScriptRuntimeLimits {
            max_key_length: 5,
            ..ScriptRuntimeLimits::default()
        };
        let mut key_runtime = RhaiScriptRuntime::compile(
            r#"
            fn process(ctx) {
                ctx.state_set("too_long", 1.0);
            }
            "#,
            key_limits,
        )
        .expect("key cap script should compile");

        let key_error = key_runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect_err("long key should exceed cap");

        assert_eq!(
            key_error,
            ScriptExecutionError::KeyLengthExceeded {
                key: "too_long".to_string(),
                limit: 5,
            }
        );
    }

    #[test]
    fn rhai_script_failure_returns_error_without_panicking() {
        let mut runtime = RhaiScriptRuntime::compile(
            r#"
            fn process(ctx) {
                let value = 1 / 0;
            }
            "#,
            ScriptRuntimeLimits::default(),
        )
        .expect("runtime failure script should compile");

        let result = runtime.process(ScriptProcessInput::new(
            Vec::new(),
            BTreeMap::new(),
            ScriptExecutionContext::new(1_000),
            ScriptModuleState::default(),
        ));

        assert!(matches!(
            result,
            Err(ScriptExecutionError::RuntimeFailed { .. })
        ));
    }

    #[test]
    fn script_process_input_exposes_original_control_map() {
        let input = ScriptProcessInput::new(
            Vec::new(),
            BTreeMap::from([("cutoff".to_string(), 0.75)]),
            ScriptExecutionContext::new(1_000),
            ScriptModuleState::default(),
        );

        assert_eq!(input.controls().get("cutoff"), Some(&0.75));
        assert_eq!(input.controls().len(), 1);
    }

    #[test]
    fn script_execution_context_rejects_work_after_budget_is_exhausted() {
        let mut context = ScriptExecutionContext::new(2);

        context.spend(1).expect("first operation should fit");
        context.spend(1).expect("second operation should fit");

        let error = context
            .spend(1)
            .expect_err("third operation should exceed budget");

        assert_eq!(
            error,
            ScriptExecutionError::OperationBudgetExceeded {
                budget: 2,
                requested: 3,
            }
        );
        assert_eq!(
            error.to_string(),
            "script operation budget exceeded: budget 2, requested 3"
        );

        let diagnostic = error.to_diagnostic("mapper");
        assert_eq!(diagnostic.error_code(), error_codes::SCRIPT_BUDGET_EXCEEDED);
        assert_eq!(diagnostic.severity(), Severity::Error);
        assert_eq!(diagnostic.module_id(), Some("mapper"));
        assert_eq!(diagnostic.expected(), Some("<= 2 operations"));
        assert_eq!(diagnostic.actual(), Some("3 operations"));
    }

    struct LastNoteRuntime;

    impl ScriptRuntime for LastNoteRuntime {
        fn process(
            &mut self,
            mut input: ScriptProcessInput,
        ) -> Result<ScriptProcessOutput, ScriptExecutionError> {
            input.context_mut().spend(1)?;

            let mut controls = BTreeMap::new();

            if let Some(ScriptValue::Number(note)) = input.state().get("last_note") {
                controls.insert("previous_note".to_string(), note);
            }

            let events: Vec<ScriptEvent> = input.events().to_vec();
            for event in events {
                if let ScriptEvent::NoteOn { note, .. } = event {
                    input
                        .state_mut()
                        .insert("last_note", ScriptValue::Number(f32::from(note)));
                }
            }

            Ok(ScriptProcessOutput {
                events: BTreeMap::new(),
                controls,
                state: input.into_state(),
            })
        }
    }

    #[test]
    fn script_module_state_is_returned_and_can_be_used_by_later_process_calls() {
        let mut runtime = LastNoteRuntime;

        let first_output = runtime
            .process(ScriptProcessInput::new(
                vec![ScriptEvent::NoteOn {
                    note: 64,
                    velocity: 100,
                }],
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                ScriptModuleState::default(),
            ))
            .expect("first call should stay within budget");

        let second_output = runtime
            .process(ScriptProcessInput::new(
                Vec::new(),
                BTreeMap::new(),
                ScriptExecutionContext::new(1_000),
                first_output.state,
            ))
            .expect("second call should stay within budget");

        assert_eq!(second_output.controls["previous_note"], 64.0);
    }

    #[test]
    fn script_feedback_events_are_queued_for_a_future_block() {
        let mut scheduler = ScriptFeedbackScheduler::default();

        scheduler.queue_for_next_block(
            "script.notes",
            vec![ScriptEvent::NoteOn {
                note: 72,
                velocity: 110,
            }],
        );

        assert_eq!(scheduler.events_for_current_block("script.notes"), &[]);

        scheduler.advance_block();

        assert_eq!(
            scheduler.events_for_current_block("script.notes"),
            &[ScriptEvent::NoteOn {
                note: 72,
                velocity: 110,
            }]
        );
    }
}
