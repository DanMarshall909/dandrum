use std::collections::BTreeMap;
use std::fmt;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptExecutionContext {
    operation_budget: u32,
    operations_spent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptExecutionError {
    OperationBudgetExceeded { budget: u32, requested: u32 },
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
        }
    }
}

impl std::error::Error for ScriptExecutionError {}

pub trait ScriptRuntime {
    fn process(
        &mut self,
        input: ScriptProcessInput,
    ) -> Result<ScriptProcessOutput, ScriptExecutionError>;
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
    fn script_process_input_exposes_only_data_budget_and_module_state() {
        let input = ScriptProcessInput::new(
            vec![ScriptEvent::NoteOff { note: 60 }],
            BTreeMap::from([("mod".to_string(), 0.25)]),
            ScriptExecutionContext::new(10),
            ScriptModuleState::default(),
        );

        assert_eq!(input.events(), &[ScriptEvent::NoteOff { note: 60 }]);
        assert_eq!(input.controls()["mod"], 0.25);
        assert_eq!(input.state().get("missing"), None);
    }
}
