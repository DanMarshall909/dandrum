mod arena_processing;
mod audio_arena;
#[cfg(test)]
mod block;
#[cfg(test)]
mod dispatch;
mod event_queue;
mod helpers;
#[cfg(test)]
mod input_provider;
mod offline;
mod outputs;
mod polyphony;
mod process_context;
mod processing;
mod realtime_graph_processor;
mod render_plan;
mod state;

#[cfg(test)]
use self::input_provider::ModuleInputProvider;
pub use self::offline::{render_kernel_offline_named, render_kernel_offline_named_with_inputs};
#[cfg(test)]
pub use self::offline::{
    render_offline, render_offline_compiled, render_offline_polyphonic,
    render_offline_with_sampler_assets, render_offline_with_sampler_assets_polyphonic,
};
#[cfg(test)]
use self::outputs::BlockEvent;
#[cfg(test)]
use self::outputs::ModuleOutputs;
pub use self::polyphony::PreparedPolyRuntimeRegion;
pub(crate) use self::polyphony::first_unrenderable_poly_child;
#[cfg(test)]
use self::processing::{
    process_adsr, process_curve_mapper, process_envelope_follower, process_filter,
    process_note_to_rate, process_sampler, process_vca,
};
pub use self::realtime_graph_processor::RealtimeGraphProcessor;
#[cfg(test)]
use self::state::PerModuleState;
#[cfg(test)]
use crate::patch::{RenderSettings, VoiceAllocation};

#[cfg(test)]
mod realtime_allocation_tests;
#[cfg(test)]
mod tests;
