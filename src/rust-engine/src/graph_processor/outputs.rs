use std::collections::HashMap;

use crate::script::ScriptEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BlockEvent {
    pub(super) frame_offset: u32,
    pub(super) event: ScriptEvent,
}

pub(super) struct ModuleOutputs {
    pub(super) audio: HashMap<String, Vec<f32>>,
    pub(super) control: HashMap<String, Vec<f32>>,
    pub(super) events: Vec<BlockEvent>,
}

impl ModuleOutputs {
    pub(super) fn empty() -> Self {
        Self {
            audio: HashMap::new(),
            control: HashMap::new(),
            events: Vec::new(),
        }
    }
}
