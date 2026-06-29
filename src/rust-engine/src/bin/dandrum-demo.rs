use std::path::Path;

use dandrum_engine::graph::{Cable, Graph, ModuleId, ModuleNode, PortRef};
use dandrum_engine::graph::builtin_ports;
use dandrum_engine::graph::SignalType;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::graph_processor::render_offline;
use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::script::ScriptEvent;
use dandrum_engine::wav::write_wav_file;

fn note_on(frame: u64, note: u8) -> TimedInputEvent {
    TimedInputEvent::new(frame, ScriptEvent::NoteOn { note, velocity: 100 })
}

fn note_off(frame: u64, note: u8) -> TimedInputEvent {
    TimedInputEvent::new(frame, ScriptEvent::NoteOff { note })
}

fn main() {
    let fs = 48000;

    // 303-style acid line: saw osc -> moog filter (envelope modulated) -> vca -> saturator -> out
    let modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("note_rate"), "note_to_rate")
            .with_input(builtin_ports::EVENTS, SignalType::Event)
            .with_output(builtin_ports::RATE, SignalType::Control),
        ModuleNode::new(ModuleId::new("osc"), "oscillator")
            .with_input(builtin_ports::PITCH, SignalType::Control)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("env"), "adsr")
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("filter_env"), "adsr")
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("filter"), "filter")
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::CUTOFF, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("sat"), "saturator")
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::DRIVE, SignalType::Control)
            .with_input(builtin_ports::BIAS, SignalType::Control)
            .with_input(builtin_ports::CURVE_SELECT, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
            .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
            .with_output(builtin_ports::MIX, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];

    let cables = vec![
        // MIDI fans out to env, filter_env, and note_to_rate
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("env"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("filter_env"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("note_rate"), builtin_ports::EVENTS),
        ),
        // Note rate drives oscillator pitch
        Cable::new(
            PortRef::new(ModuleId::new("note_rate"), builtin_ports::RATE),
            PortRef::new(ModuleId::new("osc"), builtin_ports::PITCH),
        ),
        // Oscillator -> filter -> VCA
        Cable::new(
            PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("filter_env"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("filter"), builtin_ports::CUTOFF),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("env"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("sat"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
        ),
    ];

    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");

    // 303-style acid bass sequence (E minor blues)
    let notes: [(u8, u64); 16] = [
        (40, 0),    // E2
        (40, 3000), // E2
        (43, 6000), // G2
        (45, 9000), // A2
        (46, 12000), // Bb2
        (45, 15000), // A2
        (43, 18000), // G2
        (40, 21000), // E2
        (38, 24000), // D2
        (40, 27000), // E2
        (43, 30000), // G2
        (45, 33000), // A2
        (47, 36000), // B2
        (45, 39000), // A2
        (43, 42000), // G2
        (40, 45000), // E2
    ];

    let mut events = Vec::new();
    for &(note, start_frame) in &notes {
        events.push(note_on(start_frame, note));
        events.push(note_off(start_frame + 2400, note));
    }

    let settings = RenderSettings {
        sample_rate_hz: fs,
        block_size_frames: 64,
        duration_frames: 48000 * 4, // 4 seconds
    };

    let (left, right) = render_offline(&graph, &settings, events);

    write_wav_file(Path::new("/tmp/dandrum-acid.wav"), fs, &left, &right)
        .expect("write wav");
    println!("Wrote 4s 303 acid line to /tmp/dandrum-acid.wav");
}
