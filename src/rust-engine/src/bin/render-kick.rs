use std::path::Path;

use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::graph::Graph;
use dandrum_engine::graph_processor::render_offline_polyphonic;
use dandrum_engine::patch;
use dandrum_engine::script::ScriptEvent;
use dandrum_engine::wav::write_wav_file;

fn main() {
    let patch_path = Path::new("../../examples/patches/synthetic-808-kick.yaml");
    let preset_path = Path::new("../../examples/presets/tight-808-kick.yaml");

    let mut patch_doc = patch::load_patch_file(patch_path).expect("load patch");
    let preset_doc = patch::load_preset_file(preset_path).expect("load preset");
    patch_doc = patch::apply_preset(&patch_doc, &preset_doc).expect("apply preset");

    let graph = Graph::from_patch_declarations(&patch_doc);
    let render_settings = &patch_doc.render;
    let voice_allocation = &patch_doc.voice_allocation;

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

    let (left, right) =
        render_offline_polyphonic(&graph, render_settings, events, voice_allocation);

    write_wav_file(
        Path::new("/tmp/dandrum-synth-kick.wav"),
        render_settings.sample_rate_hz,
        &left,
        &right,
    )
    .expect("write wav");

    println!(
        "Wrote /tmp/dandrum-synth-kick.wav (note {}, {} frames @ {} Hz)",
        note, render_settings.duration_frames, render_settings.sample_rate_hz
    );
}
