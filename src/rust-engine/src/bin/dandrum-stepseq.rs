use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::graph::{builtin_ports, Cable, Graph, ModuleId, ModuleNode, PortRef, SignalType};
use dandrum_engine::graph_processor::render_offline;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::script::ScriptEvent;
use dandrum_engine::wav::write_wav_file;

const NOTE_NAMES: &[&str] = &[
    "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
];

fn note_name(midi: u8) -> String {
    let octave = midi / 12;
    let name = NOTE_NAMES[(midi % 12) as usize];
    format!("{}{}", name, octave.saturating_sub(1))
}

#[derive(Clone)]
struct Step {
    note: u8,
    velocity: u8,
    gate: f32,
    active: bool,
}

impl Default for Step {
    fn default() -> Self {
        Self { note: 60, velocity: 100, gate: 0.5, active: false }
    }
}

struct Sequencer {
    steps: [Step; 16],
    bpm: u32,
    cursor: usize,
    field: Field,
    dirty: bool,
}

enum Field { Note, Velocity, Gate }

impl Sequencer {
    fn new() -> Self {
        let mut steps: [Step; 16] = Default::default();
        let pattern: [(u8, u8); 16] = [
            (40, 100), (40, 100), ( 0, 0), (43, 90), (45, 95),
            (46, 90), (45, 90), (43, 85), (40, 90), (38, 95),
            (40, 90), (43, 95), (45, 90), (47, 90), (45, 85), (43, 90),
        ];
        for (i, &(note, vel)) in pattern.iter().enumerate() {
            steps[i].note = if note == 0 { 60 } else { note };
            steps[i].velocity = vel;
            steps[i].gate = 0.5;
            steps[i].active = note != 0;
        }
        Self { steps, bpm: 140, cursor: 0, field: Field::Note, dirty: true }
    }

    fn render_grid(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(" Dandrum Step Sequencer                    BPM: {}\n", self.bpm));
        s.push_str(" ┌─────────────────────────────────────────────────────────────────────┐\n");

        // Step numbers
        s.push_str(" │Step ");
        for i in 0..16 {
            let marker = if i == self.cursor && matches!(self.field, Field::Note) { '▲' } else { ' ' };
            if i == self.cursor { s.push_str("\x1b[7m"); }
            s.push_str(&format!("{:>2}{} ", i + 1, marker));
            if i == self.cursor { s.push_str("\x1b[0m"); }
        }
        s.push_str("│\n");

        // Notes
        s.push_str(" │Note ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active { note_name(st.note) } else { String::from("--") };
            if i == self.cursor && matches!(self.field, Field::Note) {
                s.push_str(&format!("\x1b[7m{:>4}\x1b[0m", label));
            } else {
                s.push_str(&format!("{:>4} ", label));
            }
        }
        s.push_str("│\n");

        // Velocity
        s.push_str(" │Vel  ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active { format!("{:>3}", st.velocity) } else { String::from(" --") };
            if i == self.cursor && matches!(self.field, Field::Velocity) {
                s.push_str(&format!("\x1b[7m{:>4}\x1b[0m", label));
            } else {
                s.push_str(&format!("{:>4} ", label));
            }
        }
        s.push_str("│\n");

        // Gate
        s.push_str(" │Gate ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active { format!("{:>4.2}", st.gate) } else { String::from(" ---") };
            if i == self.cursor && matches!(self.field, Field::Gate) {
                s.push_str(&format!("\x1b[7m{:>5}\x1b[0m", label));
            } else {
                s.push_str(&format!("{:>5} ", label));
            }
        }
        s.push_str("│\n");

        s.push_str(" └─────────────────────────────────────────────────────────────────────┘\n");
         s.push_str(" ←→ move  Tab field  ↑↓ change  Space toggle  P play  s/S save  L load  Q quit\n");
        s
    }
}

fn build_events(seq: &Sequencer) -> Vec<TimedInputEvent> {
    let bpm = seq.bpm;
    let ticks_per_step = (60.0 / bpm as f64 * 48000.0) as u64; // 48000 Hz sample rate
    let mut events = Vec::new();
    for (i, step) in seq.steps.iter().enumerate() {
        if !step.active { continue; }
        let start = i as u64 * ticks_per_step;
        let gate_frames = (ticks_per_step as f64 * step.gate as f64) as u64;
        events.push(TimedInputEvent::new(start, ScriptEvent::NoteOn { note: step.note, velocity: step.velocity }));
        if gate_frames > 0 {
            events.push(TimedInputEvent::new(start + gate_frames, ScriptEvent::NoteOff { note: step.note }));
        }
    }
    events
}

fn build_patch_yaml(seq: &Sequencer) -> String {
    let bpm = seq.bpm;
    let ticks_per_step = (60.0 / bpm as f64 * 48000.0) as u64;
    let total = ticks_per_step * 16 + 48000;

    let mut yaml = String::new();
    yaml.push_str("metadata:\n");
    yaml.push_str("  name: stepseq-pattern\n");
    yaml.push_str("render:\n");
    yaml.push_str("  sample_rate_hz: 48000\n");
    yaml.push_str("  block_size_frames: 64\n");
    yaml.push_str(&format!("  duration_frames: {}\n", total));
    yaml.push_str("modules:\n");
    yaml.push_str("  - id: midi\n");
    yaml.push_str("    type: midi_input\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: events\n");
    yaml.push_str("        signal_type: event\n");
    yaml.push_str("  - id: note_rate\n");
    yaml.push_str("    type: note_to_rate\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: events\n");
    yaml.push_str("        signal_type: event\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: rate\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("  - id: osc\n");
    yaml.push_str("    type: oscillator\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: pitch\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: audio\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("  - id: env\n");
    yaml.push_str("    type: adsr\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: gate\n");
    yaml.push_str("        signal_type: event\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: value\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("  - id: filter_env\n");
    yaml.push_str("    type: adsr\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: gate\n");
    yaml.push_str("        signal_type: event\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: value\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("  - id: filter\n");
    yaml.push_str("    type: filter\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: audio_in\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("      - name: cutoff\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: audio_out\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("  - id: vca\n");
    yaml.push_str("    type: gain\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: audio_in\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("      - name: gain\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: audio_out\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("  - id: sat\n");
    yaml.push_str("    type: saturator\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: audio_in\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("      - name: drive\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("      - name: bias\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("      - name: curve_select\n");
    yaml.push_str("        signal_type: control\n");
    yaml.push_str("    outputs:\n");
    yaml.push_str("      - name: audio_out\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("  - id: mixer\n");
    yaml.push_str("    type: audio_mixer\n");
    yaml.push_str("  - id: out\n");
    yaml.push_str("    type: audio_output\n");
    yaml.push_str("    inputs:\n");
    yaml.push_str("      - name: left\n");
    yaml.push_str("        signal_type: audio\n");
    yaml.push_str("      - name: right\n");
    yaml.push_str("        signal_type: audio\n");

    // Connections
    yaml.push_str("connections:\n");
    yaml.push_str("  - from: midi.events\n");
    yaml.push_str("    to: note_rate.events\n");
    yaml.push_str("  - from: midi.events\n");
    yaml.push_str("    to: env.gate\n");
    yaml.push_str("  - from: midi.events\n");
    yaml.push_str("    to: filter_env.gate\n");
    yaml.push_str("  - from: note_rate.rate\n");
    yaml.push_str("    to: osc.pitch\n");
    yaml.push_str("  - from: osc.audio\n");
    yaml.push_str("    to: filter.audio_in\n");
    yaml.push_str("  - from: filter_env.value\n");
    yaml.push_str("    to: filter.cutoff\n");
    yaml.push_str("  - from: filter.audio_out\n");
    yaml.push_str("    to: vca.audio_in\n");
    yaml.push_str("  - from: env.value\n");
    yaml.push_str("    to: vca.gain\n");
    yaml.push_str("  - from: vca.audio_out\n");
    yaml.push_str("    to: sat.audio_in\n");
    yaml.push_str("  - from: sat.audio_out\n");
    yaml.push_str("    to: mixer.inputs\n");
    yaml.push_str("  - from: mixer.mix\n");
    yaml.push_str("    to: out.left\n");
    yaml.push_str("  - from: mixer.mix\n");
    yaml.push_str("    to: out.right\n");

    yaml
}

fn play_pattern(seq: &Sequencer) {
    let events = build_events(seq);
    let total = if events.is_empty() {
        48000
    } else {
        let last_event = events.last().unwrap().frame() + 24000;
        last_event + 48000
    };

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
        Cable::new(PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS), PortRef::new(ModuleId::new("note_rate"), builtin_ports::EVENTS)),
        Cable::new(PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS), PortRef::new(ModuleId::new("env"), builtin_ports::GATE)),
        Cable::new(PortRef::new(ModuleId::new("note_rate"), builtin_ports::RATE), PortRef::new(ModuleId::new("osc"), builtin_ports::PITCH)),
        Cable::new(PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO), PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN)),
        Cable::new(PortRef::new(ModuleId::new("env"), builtin_ports::VALUE), PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN)),
        Cable::new(PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT), PortRef::new(ModuleId::new("sat"), builtin_ports::AUDIO_IN)),
        Cable::new(PortRef::new(ModuleId::new("sat"), builtin_ports::AUDIO_OUT), PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS)),
        Cable::new(PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX), PortRef::new(ModuleId::new("out"), builtin_ports::LEFT)),
        Cable::new(PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX), PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT)),
    ];

    let graph = Graph::new(modules, cables);
    if let Err(e) = graph.validate() {
        eprintln!("Graph validation error: {e}");
        return;
    }

    let settings = RenderSettings {
        sample_rate_hz: 48000,
        block_size_frames: 64,
        duration_frames: total as u64,
    };

    let (left, right) = render_offline(&graph, &settings, events);
    let tmp = format!("/tmp/dandrum-seq-{}.wav", std::process::id());
    if let Err(e) = write_wav_file(Path::new(&tmp), 48000, &left, &right) {
        eprintln!("Write error: {e}");
        return;
    }

    // Play via aplay (non-blocking)
    let tmp2 = tmp.clone();
    thread::spawn(move || {
        let _ = std::process::Command::new("aplay")
            .arg(&tmp2)
            .status();
        let _ = std::fs::remove_file(&tmp2);
    });
}

fn load_yaml(path: &str) -> Result<Sequencer, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let steps_val = doc.get("steps").and_then(|v| v.as_sequence()).ok_or("missing steps")?;
    let bpm = doc.get("bpm").and_then(|v| v.as_u64()).unwrap_or(140) as u32;
    let mut seq = Sequencer::new();
    seq.bpm = bpm;
    for (i, step_val) in steps_val.iter().enumerate() {
        if i >= 16 { break; }
        let map = step_val.as_mapping().ok_or("step not a map")?;
        let note = map.get(&serde_yaml::Value::String("note".into())).and_then(|v| v.as_u64()).unwrap_or(60) as u8;
        let vel = map.get(&serde_yaml::Value::String("velocity".into())).and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        let gate = map.get(&serde_yaml::Value::String("gate".into())).and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
        let active = map.get(&serde_yaml::Value::String("active".into())).and_then(|v| v.as_bool()).unwrap_or(true);
        seq.steps[i] = Step { note, velocity: vel, gate, active };
    }
    Ok(seq)
}

fn save_yaml(seq: &Sequencer, path: &str) -> Result<(), String> {
    use serde::Serialize;
    #[derive(Serialize)]
    struct StepData { note: u8, velocity: u8, gate: f32, active: bool }
    #[derive(Serialize)]
    struct SeqData { bpm: u32, steps: Vec<StepData> }

    let data = SeqData {
        bpm: seq.bpm,
        steps: seq.steps.iter().map(|s| StepData {
            note: s.note,
            velocity: s.velocity,
            gate: s.gate,
            active: s.active,
        }).collect(),
    };
    let yaml = serde_yaml::to_string(&data).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, &yaml).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;

    let mut seq = Sequencer::new();
    let (tx, rx) = mpsc::channel();

    // Spawn input thread
    let tx2 = tx.clone();
    thread::spawn(move || loop {
        match read() {
            Ok(Event::Key(k)) => {
                let _ = tx2.send(k);
                if k.code == KeyCode::Char('q') && k.modifiers == KeyModifiers::NONE { break; }
            }
            Ok(Event::Resize(_w, _h)) => {
                let _ = tx2.send(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
            }
            _ => {}
        }
    });

    let render = |seq: &Sequencer, stdout: &mut io::Stdout| -> io::Result<()> {
        queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
        let grid = seq.render_grid();
        print!("{}", grid);
        stdout.flush()?;
        Ok(())
    };

    render(&seq, &mut stdout)?;

    loop {
        let key = match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(k) => k,
            Err(_) => {
                // Tick - redraw if dirty
                if seq.dirty {
                    let _ = render(&seq, &mut stdout);
                    seq.dirty = false;
                }
                continue;
            }
        };

        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Char('p') => {
                queue!(stdout, MoveTo(0, 10), Print("Rendering...                 "))?;
                stdout.flush()?;
                play_pattern(&seq);
                queue!(stdout, MoveTo(0, 10), Print("Playing...                   "))?;
                stdout.flush()?;
            }
            KeyCode::Char('s') => {
                queue!(stdout, MoveTo(0, 10), Print("Save path: /tmp/pattern.yaml   "))?;
                stdout.flush()?;
                match save_yaml(&seq, "/tmp/pattern.yaml") {
                    Ok(_) => queue!(stdout, MoveTo(0, 10), Print("Saved steps to /tmp/pattern.yaml"))?,
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            KeyCode::Char('S') => {
                let patch_yaml = build_patch_yaml(&seq);
                match std::fs::write("/tmp/pattern-patch.yaml", &patch_yaml) {
                    Ok(_) => queue!(stdout, MoveTo(0, 10), Print("Saved patch to /tmp/pattern-patch.yaml"))?,
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            KeyCode::Char('l') => {
                queue!(stdout, MoveTo(0, 10), Print("Loading /tmp/pattern.yaml...   "))?;
                stdout.flush()?;
                match load_yaml("/tmp/pattern.yaml") {
                    Ok(loaded) => { seq = loaded; seq.dirty = true; seq.cursor = 0; seq.field = Field::Note; }
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            KeyCode::Char(' ') => {
                seq.steps[seq.cursor].active = !seq.steps[seq.cursor].active;
                seq.dirty = true;
            }
            KeyCode::Right => {
                seq.cursor = (seq.cursor + 1) % 16;
                seq.dirty = true;
            }
            KeyCode::Left => {
                seq.cursor = if seq.cursor == 0 { 15 } else { seq.cursor - 1 };
                seq.dirty = true;
            }
            KeyCode::Up | KeyCode::Char('+') => {
                let st = &mut seq.steps[seq.cursor];
                match seq.field {
                    Field::Note => if st.note < 127 { st.note += 1; },
                    Field::Velocity => if st.velocity < 127 { st.velocity += 1; },
                    Field::Gate => st.gate = (st.gate + 0.05).min(1.0),
                }
                seq.dirty = true;
            }
            KeyCode::Down | KeyCode::Char('-') => {
                let st = &mut seq.steps[seq.cursor];
                match seq.field {
                    Field::Note => if st.note > 0 { st.note -= 1; },
                    Field::Velocity => if st.velocity > 0 { st.velocity -= 1; },
                    Field::Gate => st.gate = (st.gate - 0.05).max(0.01),
                }
                seq.dirty = true;
            }
            KeyCode::Tab => {
                seq.field = match seq.field {
                    Field::Note => Field::Velocity,
                    Field::Velocity => Field::Gate,
                    Field::Gate => Field::Note,
                };
                seq.dirty = true;
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                if let Some(digit) = ch.to_digit(10).map(|d| d as u8) {
                    let st = &mut seq.steps[seq.cursor];
                    match seq.field {
                        Field::Note => st.note = (st.note / 10) * 10 + digit.min(2),
                        Field::Velocity => st.velocity = (st.velocity / 10) * 10 + digit.min(7),
                        Field::Gate => {}
                    }
                    seq.dirty = true;
                }
            }
            _ => {}
        }

        let _ = render(&seq, &mut stdout);
    }

    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--render" {
        let path = if args.len() > 2 { &args[2] } else { "/tmp/pattern.yaml" };
        match load_yaml(path) {
            Ok(seq) => {
                play_pattern(&seq);
                println!("Rendering {}...", path);
                thread::sleep(Duration::from_secs(1));
            }
            Err(e) => eprintln!("Error loading {}: {e}", path),
        }
        return Ok(());
    }
    if args.len() > 1 && args[1] == "--export" {
        let path = if args.len() > 2 { &args[2] } else { "/tmp/pattern.yaml" };
        match load_yaml(path) {
            Ok(seq) => {
                let patch_yaml = build_patch_yaml(&seq);
                let out_path = path.replace(".yaml", "-patch.yaml");
                std::fs::write(&out_path, &patch_yaml)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                println!("Exported patch to {}", out_path);
            }
            Err(e) => eprintln!("Error loading {}: {e}", path),
        }
        return Ok(());
    }
    run_tui()
}
