use std::collections::HashMap;

use crate::graph::Graph;

pub(super) struct Routing {
    /// For each module index, maps input port name -> (source_module_idx, source_port_name)
    pub(super) inputs: Vec<HashMap<String, Vec<(usize, String)>>>,
}

pub(super) fn build_routing(graph: &Graph) -> Routing {
    let module_count = graph.modules().len();
    let name_to_idx: HashMap<&str, usize> = graph
        .modules()
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.id().as_str(), idx))
        .collect();

    let mut inputs: Vec<HashMap<String, Vec<(usize, String)>>> =
        (0..module_count).map(|_| HashMap::new()).collect();

    for cable in graph.cables() {
        let dst_idx = name_to_idx[cable.destination().module_id().as_str()];
        let src_idx = name_to_idx[cable.source().module_id().as_str()];
        let dst_port = cable.destination().port_name().to_string();
        let src_port = cable.source().port_name().to_string();
        inputs[dst_idx]
            .entry(dst_port)
            .or_default()
            .push((src_idx, src_port));
    }

    Routing { inputs }
}
