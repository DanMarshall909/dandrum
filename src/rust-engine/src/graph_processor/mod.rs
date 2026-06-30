mod block;
mod dispatch;
mod helpers;
mod input_provider;
mod offline;
mod outputs;
mod polyphony;
mod processing;
mod realtime_graph_processor;
mod routing;
mod state;
mod traversal;

use self::input_provider::ModuleInputProvider;
pub use self::offline::{
    render_offline, render_offline_compiled, render_offline_polyphonic,
    render_offline_with_sampler_assets, render_offline_with_sampler_assets_polyphonic,
};
use self::outputs::BlockEvent;
use self::outputs::ModuleOutputs;
#[cfg(test)]
use self::processing::{process_adsr, process_note_to_rate, process_sampler, process_vca};
pub use self::realtime_graph_processor::RealtimeGraphProcessor;
#[cfg(test)]
use self::state::PerModuleState;
#[cfg(test)]
use crate::patch::{RenderSettings, VoiceAllocation};

#[cfg(test)]
mod tests;
