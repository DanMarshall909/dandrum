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
