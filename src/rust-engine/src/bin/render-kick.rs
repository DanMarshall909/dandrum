use std::path::Path;

use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::graph_processor::render_kernel_offline_named;
use dandrum_engine::kernel::document::load_kernel_patch_file;
use dandrum_engine::patch::{self, RenderSettings};
use dandrum_engine::preparation::prepare_kernel_patch;
use dandrum_engine::script::ScriptEvent;
use dandrum_engine::wav::write_wav_file;

fn main() {
    let patch_path = Path::new("../../examples/patches/synthetic-808-kick.yaml");
    let preset_path = Path::new("../../examples/presets/tight-808-kick.yaml");

    let patch_doc = load_kernel_patch_file(patch_path).expect("load patch");
    let preset_doc = patch::load_preset_file(preset_path).expect("load preset");
    let patch_doc = patch_doc.apply_preset(&preset_doc).expect("apply preset");

    let render_settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 128,
        duration_frames: 48_000,
    };
    let prepared = prepare_kernel_patch(&patch_doc, &render_settings).expect("prepare patch");

    let note = 36u8;
    let duration = render_settings.duration_frames;
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(duration.saturating_sub(1), ScriptEvent::NoteOff { note }),
    ];

    let buses = render_kernel_offline_named(&prepared, events, &PreparedSamplerAssets::empty())
        .expect("render kick");
    let (_, channels) = buses
        .into_iter()
        .find(|(name, _)| name == "master")
        .expect("kick exposes master bus");

    write_wav_file(
        Path::new("/tmp/dandrum-synth-kick.wav"),
        render_settings.sample_rate_hz,
        &channels[0],
        &channels[1],
    )
    .expect("write wav");

    println!(
        "Wrote /tmp/dandrum-synth-kick.wav (note {}, {} frames @ {} Hz)",
        note, render_settings.duration_frames, render_settings.sample_rate_hz
    );
}
