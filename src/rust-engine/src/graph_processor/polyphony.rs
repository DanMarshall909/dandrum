use crate::graph::Graph;
use crate::sample::PreparedSamplerAssets;

use super::state::PerModuleState;

pub(super) fn build_polyphonic_states(
    graph: &Graph,
    sample_rate: f32,
    sampler_assets: &PreparedSamplerAssets,
    max_voices: usize,
) -> Vec<Vec<PerModuleState>> {
    (0..max_voices)
        .map(|_| {
            graph
                .modules()
                .iter()
                .map(|m| PerModuleState::new(m, sample_rate, sampler_assets))
                .collect::<Vec<_>>()
        })
        .collect()
}
