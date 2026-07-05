mod audio_arena;
mod block;
mod dispatch;
mod helpers;
mod input_provider;
mod offline;
mod outputs;
mod polyphony;
mod process_context;
mod processing;
mod realtime_graph_processor;
mod render_plan;
mod state;

use self::input_provider::ModuleInputProvider;
pub use self::offline::{
    render_offline, render_offline_compiled, render_offline_polyphonic,
    render_offline_with_sampler_assets, render_offline_with_sampler_assets_polyphonic,
};
use self::outputs::BlockEvent;
use self::outputs::ModuleOutputs;
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

#[doc(hidden)]
pub fn exercise_active_process_context_surface_for_compilation() {
    let plan = render_plan::AudioBufferPlan {
        buffer_count: 2,
        max_block_frames: 1,
        max_voices: 1,
    };
    let mut arena = audio_arena::AudioArena::new(plan);
    let input_buffers = [render_plan::BufferId(0)];
    let output_buffers = [render_plan::BufferId(1)];
    let mut context = process_context::ProcessContext::new(
        &mut arena,
        &input_buffers,
        &output_buffers,
        1,
    );

    let _ = context.frames();
    let _ = context.input(0);
    let _ = context.output(0);
}

#[cfg(test)]
mod realtime_allocation_tests;
#[cfg(test)]
mod tests;
