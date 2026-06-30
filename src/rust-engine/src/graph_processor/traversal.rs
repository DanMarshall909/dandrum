use std::collections::HashMap;

use crate::graph::Graph;

pub(super) fn find_audio_output(graph: &Graph) -> Option<usize> {
    graph
        .modules()
        .iter()
        .position(|module| module.module_type() == "audio_output")
}

pub(super) fn find_midi_input(graph: &Graph) -> Option<usize> {
    graph
        .modules()
        .iter()
        .position(|module| module.module_type() == "midi_input")
}

/// Topological sort using Kahn's algorithm.
pub(super) fn topological_sort(graph: &Graph) -> Vec<usize> {
    let module_count = graph.modules().len();
    let name_to_idx: HashMap<&str, usize> = graph
        .modules()
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.id().as_str(), idx))
        .collect();

    let mut in_degree = vec![0usize; module_count];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); module_count];

    for cable in graph.cables() {
        let src_idx = name_to_idx[cable.source().module_id().as_str()];
        let dst_idx = name_to_idx[cable.destination().module_id().as_str()];
        adjacency[src_idx].push(dst_idx);
        in_degree[dst_idx] += 1;
    }

    let mut queue: Vec<usize> = (0..module_count)
        .filter(|&idx| in_degree[idx] == 0)
        .collect();
    let mut order = Vec::new();

    while let Some(idx) = queue.pop() {
        order.push(idx);
        for &next in &adjacency[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }

    order
}
