use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, read};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{execute, queue};

use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::graph_processor::render_kernel_offline_named;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::prepare_kernel_patch;
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
        Self {
            note: 60,
            velocity: 100,
            gate: 0.5,
            active: false,
        }
    }
}

struct Sequencer {
    steps: [Step; 16],
    bpm: u32,
    cursor: usize,
    field: Field,
    dirty: bool,
}

enum Field {
    Note,
    Velocity,
    Gate,
}

impl Sequencer {
    fn new() -> Self {
        let mut steps: [Step; 16] = Default::default();
        let pattern: [(u8, u8); 16] = [
            (36, 108),
            (0, 0),
            (36, 96),
            (43, 104),
            (36, 100),
            (0, 0),
            (46, 106),
            (43, 102),
            (36, 108),
            (38, 98),
            (0, 0),
            (43, 104),
            (36, 100),
            (46, 106),
            (43, 102),
            (0, 0),
        ];
        for (i, &(note, vel)) in pattern.iter().enumerate() {
            steps[i].note = if note == 0 { 60 } else { note };
            steps[i].velocity = vel;
            steps[i].gate = match i {
                3 | 7 | 11 | 14 => 0.60,
                _ => 0.45,
            };
            steps[i].active = note != 0;
        }
        Self {
            steps,
            bpm: 140,
            cursor: 0,
            field: Field::Note,
            dirty: true,
        }
    }

    fn render_grid(&self) -> String {
        let inner_width = 5 + 16 * 5;
        let mut s = String::new();
        s.push_str(&format!(
            " Dandrum Step Sequencer                    BPM: {}\r\n",
            self.bpm
        ));
        s.push_str(&format!(" ┌{}┐\r\n", "─".repeat(inner_width)));

        // Step numbers
        s.push_str(" │Step ");
        for i in 0..16 {
            let marker = if i == self.cursor && matches!(self.field, Field::Note) {
                '▲'
            } else {
                ' '
            };
            if i == self.cursor {
                s.push_str("\x1b[7m");
            }
            s.push_str(&format!("{:>2}{}  ", i + 1, marker));
            if i == self.cursor {
                s.push_str("\x1b[0m");
            }
        }
        s.push_str("│\r\n");

        // Notes
        s.push_str(" │Note ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active {
                note_name(st.note)
            } else {
                String::from("--")
            };
            if i == self.cursor && matches!(self.field, Field::Note) {
                s.push_str(&format!("\x1b[7m{:>4}\x1b[0m ", label));
            } else {
                s.push_str(&format!("{:>4} ", label));
            }
        }
        s.push_str("│\r\n");

        // Velocity
        s.push_str(" │Vel  ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active {
                format!("{:>3}", st.velocity)
            } else {
                String::from("--")
            };
            if i == self.cursor && matches!(self.field, Field::Velocity) {
                s.push_str(&format!("\x1b[7m{:>4}\x1b[0m ", label));
            } else {
                s.push_str(&format!("{:>4} ", label));
            }
        }
        s.push_str("│\r\n");

        // Gate
        s.push_str(" │Gate ");
        for i in 0..16 {
            let st = &self.steps[i];
            let label = if st.active {
                format!("{:>4.2}", st.gate)
            } else {
                String::from("---")
            };
            if i == self.cursor && matches!(self.field, Field::Gate) {
                s.push_str(&format!("\x1b[7m{:>4}\x1b[0m ", label));
            } else {
                s.push_str(&format!("{:>4} ", label));
            }
        }
        s.push_str("│\r\n");

        s.push_str(&format!(" └{}┘\r\n", "─".repeat(inner_width)));
        s.push_str(
            " ←→ move  Tab field  ↑↓ change  Space toggle  P play  s/S save  L load  Q quit\r\n",
        );
        s
    }
}

fn build_events(seq: &Sequencer) -> Vec<TimedInputEvent> {
    let bpm = seq.bpm;
    let ticks_per_step = (60.0 / bpm as f64 * 48000.0 / 4.0) as u64; // Sixteenth notes at 48 kHz.
    let mut events = Vec::new();
    for (i, step) in seq.steps.iter().enumerate() {
        if !step.active {
            continue;
        }
        let start = i as u64 * ticks_per_step;
        let gate_frames = (ticks_per_step as f64 * step.gate as f64) as u64;
        events.push(TimedInputEvent::new(
            start,
            ScriptEvent::NoteOn {
                note: step.note,
                velocity: step.velocity,
            },
        ));
        if gate_frames > 0 {
            events.push(TimedInputEvent::new(
                start + gate_frames,
                ScriptEvent::NoteOff { note: step.note },
            ));
        }
    }
    events
}

fn build_live_patch_yaml() -> &'static str {
    r#"metadata:
  name: stepseq-pattern
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: mixer.mix }
modules:
  - { id: midi, type: midi_input }
  - { id: note_rate, type: note_to_rate }
  - { id: osc, type: oscillator, static: { channels: 2 } }
  - { id: env, type: adsr }
  - { id: vca, type: gain, static: { channels: 2 } }
  - { id: sat, type: saturator, static: { channels: 2 } }
  - { id: mixer, type: audio_mixer, static: { channels: 2 } }
connections:
  - { from: midi.events, to: note_rate.events }
  - { from: midi.events, to: env.gate }
  - { from: note_rate.rate, to: osc.pitch }
  - { from: osc.audio, to: vca.audio_in }
  - { from: env.value, to: vca.gain }
  - { from: vca.audio_out, to: sat.audio_in }
  - { from: sat.audio_out, to: mixer.inputs }
"#
}

fn build_patch_yaml() -> &'static str {
    r#"metadata:
  name: stepseq-pattern
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: mixer.mix }
modules:
  - { id: midi, type: midi_input }
  - { id: note_rate, type: note_to_rate }
  - { id: osc, type: oscillator, static: { channels: 2 } }
  - { id: env, type: adsr }
  - { id: filter_env, type: adsr }
  - { id: filter, type: filter, static: { channels: 2 } }
  - { id: vca, type: gain, static: { channels: 2 } }
  - { id: sat, type: saturator, static: { channels: 2 } }
  - { id: mixer, type: audio_mixer, static: { channels: 2 } }
connections:
  - { from: midi.events, to: note_rate.events }
  - { from: midi.events, to: env.gate }
  - { from: midi.events, to: filter_env.gate }
  - { from: note_rate.rate, to: osc.pitch }
  - { from: osc.audio, to: filter.audio_in }
  - { from: filter_env.value, to: filter.cutoff }
  - { from: filter.audio_out, to: vca.audio_in }
  - { from: env.value, to: vca.gain }
  - { from: vca.audio_out, to: sat.audio_in }
  - { from: sat.audio_out, to: mixer.inputs }
"#
}

fn play_pattern(seq: &Sequencer) {
    let events = build_events(seq);
    let total = if events.is_empty() {
        48_000
    } else {
        events.last().unwrap().frame() + 24_000 + 48_000
    };
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: total,
    };
    let patch = match load_kernel_patch_str(build_live_patch_yaml()) {
        Ok(patch) => patch,
        Err(error) => {
            eprintln!("Patch error: {error}");
            return;
        }
    };
    let prepared = match prepare_kernel_patch(&patch, &settings) {
        Ok(prepared) => prepared,
        Err(error) => {
            eprintln!("Preparation error: {error}");
            return;
        }
    };
    let outputs =
        match render_kernel_offline_named(&prepared, events, &PreparedSamplerAssets::empty()) {
            Ok(outputs) => outputs,
            Err(error) => {
                eprintln!("Render error: {error}");
                return;
            }
        };
    let Some((_, master)) = outputs.iter().find(|(name, _)| name == "master") else {
        eprintln!("Render error: missing master bus");
        return;
    };
    if master.len() != 2 {
        eprintln!("Render error: master bus must be stereo");
        return;
    }
    let tmp = format!("/tmp/dandrum-seq-{}.wav", std::process::id());
    if let Err(error) = write_wav_file(Path::new(&tmp), 48_000, &master[0], &master[1]) {
        eprintln!("Write error: {error}");
        return;
    }

    // Play via aplay (non-blocking).
    thread::spawn(move || {
        let _ = std::process::Command::new("aplay").arg(&tmp).status();
        let _ = std::fs::remove_file(&tmp);
    });
}

fn load_yaml(path: &str) -> Result<Sequencer, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let steps_val = doc
        .get("steps")
        .and_then(|v| v.as_sequence())
        .ok_or("missing steps")?;
    let bpm = doc.get("bpm").and_then(|v| v.as_u64()).unwrap_or(140) as u32;
    let mut seq = Sequencer::new();
    seq.bpm = bpm;
    for (i, step_val) in steps_val.iter().enumerate() {
        if i >= 16 {
            break;
        }
        let map = step_val.as_mapping().ok_or("step not a map")?;
        let note = map
            .get(&serde_yaml::Value::String("note".into()))
            .and_then(|v| v.as_u64())
            .unwrap_or(60) as u8;
        let vel = map
            .get(&serde_yaml::Value::String("velocity".into()))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as u8;
        let gate = map
            .get(&serde_yaml::Value::String("gate".into()))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        let active = map
            .get(&serde_yaml::Value::String("active".into()))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        seq.steps[i] = Step {
            note,
            velocity: vel,
            gate,
            active,
        };
    }
    Ok(seq)
}

fn save_yaml(seq: &Sequencer, path: &str) -> Result<(), String> {
    use serde::Serialize;
    #[derive(Serialize)]
    struct StepData {
        note: u8,
        velocity: u8,
        gate: f32,
        active: bool,
    }
    #[derive(Serialize)]
    struct SeqData {
        bpm: u32,
        steps: Vec<StepData>,
    }

    let data = SeqData {
        bpm: seq.bpm,
        steps: seq
            .steps
            .iter()
            .map(|s| StepData {
                note: s.note,
                velocity: s.velocity,
                gate: s.gate,
                active: s.active,
            })
            .collect(),
    };
    let yaml = serde_yaml::to_string(&data).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, &yaml).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

fn apply_edit_key(seq: &mut Sequencer, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(' ') => {
            seq.steps[seq.cursor].active = !seq.steps[seq.cursor].active;
            seq.dirty = true;
            true
        }
        KeyCode::Right => {
            seq.cursor = (seq.cursor + 1) % 16;
            seq.dirty = true;
            true
        }
        KeyCode::Left => {
            seq.cursor = if seq.cursor == 0 { 15 } else { seq.cursor - 1 };
            seq.dirty = true;
            true
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
            let st = &mut seq.steps[seq.cursor];
            st.note = st.note.saturating_add(12).min(127);
            seq.dirty = true;
            true
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            let st = &mut seq.steps[seq.cursor];
            st.note = st.note.saturating_sub(12);
            seq.dirty = true;
            true
        }
        KeyCode::PageUp => {
            let st = &mut seq.steps[seq.cursor];
            st.velocity = st.velocity.saturating_add(1).min(127);
            seq.dirty = true;
            true
        }
        KeyCode::PageDown => {
            let st = &mut seq.steps[seq.cursor];
            st.velocity = st.velocity.saturating_sub(1);
            seq.dirty = true;
            true
        }
        KeyCode::Up | KeyCode::Char('+') => {
            let st = &mut seq.steps[seq.cursor];
            match seq.field {
                Field::Note => {
                    if st.note < 127 {
                        st.note += 1;
                    }
                }
                Field::Velocity => {
                    if st.velocity < 127 {
                        st.velocity += 1;
                    }
                }
                Field::Gate => st.gate = (st.gate + 0.05).min(1.0),
            }
            seq.dirty = true;
            true
        }
        KeyCode::Down | KeyCode::Char('-') => {
            let st = &mut seq.steps[seq.cursor];
            match seq.field {
                Field::Note => {
                    if st.note > 0 {
                        st.note -= 1;
                    }
                }
                Field::Velocity => {
                    if st.velocity > 0 {
                        st.velocity -= 1;
                    }
                }
                Field::Gate => st.gate = (st.gate - 0.05).max(0.01),
            }
            seq.dirty = true;
            true
        }
        KeyCode::Tab => {
            seq.field = match seq.field {
                Field::Note => Field::Velocity,
                Field::Velocity => Field::Gate,
                Field::Gate => Field::Note,
            };
            seq.dirty = true;
            true
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
            true
        }
        _ => false,
    }
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;

    let mut seq = Sequencer::new();
    let (tx, rx) = mpsc::channel();

    // Spawn input thread
    let tx2 = tx.clone();
    thread::spawn(move || {
        loop {
            match read() {
                Ok(Event::Key(k)) => {
                    let _ = tx2.send(k);
                    if k.code == KeyCode::Char('q') && k.modifiers == KeyModifiers::NONE {
                        break;
                    }
                }
                Ok(Event::Resize(_w, _h)) => {
                    let _ = tx2.send(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
                }
                _ => {}
            }
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
                queue!(
                    stdout,
                    MoveTo(0, 10),
                    Print("Rendering...                 ")
                )?;
                stdout.flush()?;
                play_pattern(&seq);
                queue!(
                    stdout,
                    MoveTo(0, 10),
                    Print("Playing...                   ")
                )?;
                stdout.flush()?;
            }
            KeyCode::Char('s') => {
                queue!(
                    stdout,
                    MoveTo(0, 10),
                    Print("Save path: /tmp/pattern.yaml   ")
                )?;
                stdout.flush()?;
                match save_yaml(&seq, "/tmp/pattern.yaml") {
                    Ok(_) => queue!(
                        stdout,
                        MoveTo(0, 10),
                        Print("Saved steps to /tmp/pattern.yaml")
                    )?,
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            KeyCode::Char('S') => {
                let patch_yaml = build_patch_yaml();
                match std::fs::write("/tmp/pattern-patch.yaml", &patch_yaml) {
                    Ok(_) => queue!(
                        stdout,
                        MoveTo(0, 10),
                        Print("Saved patch to /tmp/pattern-patch.yaml")
                    )?,
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            KeyCode::Char('l') => {
                queue!(
                    stdout,
                    MoveTo(0, 10),
                    Print("Loading /tmp/pattern.yaml...   ")
                )?;
                stdout.flush()?;
                match load_yaml("/tmp/pattern.yaml") {
                    Ok(loaded) => {
                        seq = loaded;
                        seq.dirty = true;
                        seq.cursor = 0;
                        seq.field = Field::Note;
                    }
                    Err(e) => queue!(stdout, MoveTo(0, 10), Print(format!("Error: {e:<30}")))?,
                }
                stdout.flush()?;
            }
            _ => {
                let _ = apply_edit_key(&mut seq, key);
            }
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
        let path = if args.len() > 2 {
            &args[2]
        } else {
            "/tmp/pattern.yaml"
        };
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
        let path = if args.len() > 2 {
            &args[2]
        } else {
            "/tmp/pattern.yaml"
        };
        match load_yaml(path) {
            Ok(_) => {
                let patch_yaml = build_patch_yaml();
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_and_exported_stepseq_patches_render_on_stereo_master_buses() {
        let seq = Sequencer::new();
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 64,
            duration_frames: 1_024,
        };
        for yaml in [build_live_patch_yaml(), build_patch_yaml()] {
            let patch = load_kernel_patch_str(yaml).expect("stepseq patch uses kernel YAML");
            let prepared = prepare_kernel_patch(&patch, &settings).expect("stepseq patch prepares");
            let outputs = render_kernel_offline_named(
                &prepared,
                build_events(&seq),
                &PreparedSamplerAssets::empty(),
            )
            .expect("stepseq patch renders the pattern");
            assert_eq!(outputs.len(), 1);
            assert_eq!(outputs[0].0, "master");
            assert_eq!(outputs[0].1.len(), 2);
            assert_eq!(outputs[0].1[0], outputs[0].1[1]);
            assert!(outputs[0].1[0].iter().any(|sample| sample.abs() > 0.001));
        }
    }

    #[test]
    fn shift_up_and_down_change_note_by_octaves() {
        let mut seq = Sequencer::new();
        seq.steps[0].note = 60;

        assert!(apply_edit_key(
            &mut seq,
            KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT)
        ));
        assert_eq!(seq.steps[0].note, 72);

        assert!(apply_edit_key(
            &mut seq,
            KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT)
        ));
        assert_eq!(seq.steps[0].note, 60);
    }

    #[test]
    fn page_up_and_down_change_velocity() {
        let mut seq = Sequencer::new();
        seq.steps[0].velocity = 100;

        assert!(apply_edit_key(
            &mut seq,
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)
        ));
        assert_eq!(seq.steps[0].velocity, 101);

        assert!(apply_edit_key(
            &mut seq,
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)
        ));
        assert_eq!(seq.steps[0].velocity, 100);
    }
}
