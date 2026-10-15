#![allow(dead_code)]
use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph::PortDirection;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::{ChannelCount, document::load_kernel_patch_str};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, prepare_kernel_graph_with_buses};

pub const GAIN: &str = include_str!("../../fixtures/signed-gain.yaml");
pub const TB303: &str = include_str!("../../../../examples/patches/tb303-acid.yaml");

pub fn direct_engine(
    document: &str,
    frames: usize,
    sample_rate: u32,
    notes: &[(u32, [u8; 3])],
    changes: &[(usize, usize, f64)],
) -> Vec<Vec<f32>> {
    direct_engine_at_rate(document, frames, f64::from(sample_rate), notes, changes)
}

pub fn direct_engine_at_rate(
    document: &str,
    frames: usize,
    sample_rate: f64,
    notes: &[(u32, [u8; 3])],
    changes: &[(usize, usize, f64)],
) -> Vec<Vec<f32>> {
    let patch = load_kernel_patch_str(document).unwrap();
    let settings = RenderSettings {
        sample_rate_hz: sample_rate.round() as u32,
        block_size_frames: frames as u32,
        duration_frames: frames as u64,
    };
    let mut buses = HostBuses::new();
    for port in patch.root().ports() {
        if port.direction() == PortDirection::Output {
            buses = buses.with_output(port.name(), 2);
        } else if port.direction() == PortDirection::Input {
            let ChannelCount::Literal(channels) = port.channels() else {
                panic!("literal channels")
            };
            buses = buses.with_input(port.name(), *channels as usize);
        }
    }
    let prepared =
        prepare_kernel_graph_with_buses(patch.root(), patch.registry(), &settings, &buses).unwrap();
    let metadata = prepared.root_port_metadata();
    let mut inputs: Vec<Vec<Vec<f32>>> = metadata
        .iter()
        .filter(|p| p.direction() == PortDirection::Input)
        .map(|p| {
            let ChannelCount::Literal(n) = p.channels() else {
                panic!("literal channels")
            };
            vec![vec![0.0; frames]; *n as usize]
        })
        .collect();
    let names: Vec<_> = metadata
        .iter()
        .filter(|p| p.direction() == PortDirection::Input)
        .map(|p| p.name())
        .collect();
    if document == GAIN {
        inputs[0][0].fill(-0.5);
        inputs[0][1].fill(0.25);
    }
    for (parameter, alias) in patch.preset_surface().parameters().iter().enumerate() {
        let index = names.iter().position(|n| *n == alias.port_name()).unwrap();
        inputs[index][0].fill(alias.control_default().default() as f32);
        for &(offset, changed_parameter, value) in changes {
            if changed_parameter == parameter {
                inputs[index][0][offset..].fill(value as f32);
            }
        }
    }
    let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
        prepared.compiled_patch().clone(),
        sample_rate as f32,
        &PreparedSamplerAssets::empty(),
        frames,
    );
    for &(offset, [status, note, velocity]) in notes {
        if status & 0xf0 == 0x90 && velocity != 0 {
            assert!(runtime.try_note_on_at(note, velocity, offset));
        } else {
            assert!(runtime.try_note_off_at(note, offset));
        }
    }
    let mut output = vec![vec![vec![0.0; frames]; 2]];
    assert_eq!(runtime.render_root_buses(&inputs, &mut output), frames);
    output.remove(0)
}
