//! Same-artifact independent instances, canonical PCM, and native editor lifetimes.
use super::*;

const KICK_FRAMES: usize = 12_000;

// Parse the copied fixture independently of the engine WAV reader. This pins
// the oracle's PCM format and region, not the implementation's decoding path.
fn canonical_kick() -> Vec<f32> {
    let path = artifact()
        .parent()
        .unwrap()
        .join("DandrumTrigger.resources")
        .join(dandrum_trigger_host::ADVANCED_DRUM_KIT_SAMPLE);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let u16_at = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    assert_eq!(u32_at(4) as usize + 8, bytes.len());
    let mut cursor = 12;
    let mut format = false;
    let mut data = None;
    while cursor < bytes.len() {
        assert!(cursor + 8 <= bytes.len());
        let size = u32_at(cursor + 4) as usize;
        let begin = cursor + 8;
        let end = begin.checked_add(size).unwrap();
        assert!(end <= bytes.len());
        match &bytes[cursor..cursor + 4] {
            b"fmt " => {
                assert!(!format);
                assert_eq!(size, 16);
                assert_eq!((u16_at(begin), u16_at(begin + 2)), (1, 1));
                assert_eq!((u32_at(begin + 4), u32_at(begin + 8)), (48_000, 96_000));
                assert_eq!((u16_at(begin + 12), u16_at(begin + 14)), (2, 16));
                format = true;
            }
            b"data" => {
                assert!(data.is_none());
                data = Some(&bytes[begin..end]);
            }
            _ => {}
        }
        cursor = end + size % 2;
    }
    assert_eq!(cursor, bytes.len());
    assert!(format);
    let data = data.unwrap();
    assert!(data.len().is_multiple_of(2) && data.len() >= KICK_FRAMES * 2);
    let samples: Vec<_> = data
        .chunks_exact(2)
        .take(KICK_FRAMES)
        .map(|s| i16::from_le_bytes(s.try_into().unwrap()))
        .collect();
    assert_eq!(&samples[..2], &[-16_384, -358]);
    samples
        .into_iter()
        .map(|s| f32::from(s) / 32768.0)
        .collect()
}

use clack_host::process::StoppedPluginAudioProcessor;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

const MAX_BLOCKS: usize = 2_000;
const WATCHDOG: Duration = Duration::from_millis(500);
#[derive(Clone, Copy, Debug)]
enum Command {
    Note { key: u8, audible: bool },
    Reset,
    Autonomous,
}
#[derive(Default)]
struct Progress {
    stop: AtomicBool,
    finished: AtomicBool,
    blocks: AtomicUsize,
    ack: AtomicUsize,
    ack_block: AtomicUsize,
    kick_block: AtomicUsize,
}
#[derive(Debug, Default)]
struct Receipt {
    first_frame: usize,
    blocks: usize,
    frames: usize,
    max_duration: Duration,
    max_gap: Duration,
    failure: Option<&'static str>,
    mismatch: Option<(usize, usize, f32, f32)>,
    intervals: Vec<(u64, u64)>,
    commands: Vec<(usize, usize, Command)>,
    lifecycle: Vec<(&'static str, u64, u64)>,
}
struct Worker {
    progress: Arc<Progress>,
    commands: mpsc::SyncSender<(usize, Command)>,
    thread: std::thread::JoinHandle<(StoppedPluginAudioProcessor<GuiHost>, Receipt)>,
    sequence: usize,
}
struct WorkerSetup {
    wave: Arc<Vec<f32>>,
    frames: usize,
    gain: f32,
    offset: usize,
    consumed_blocks: usize,
    first_frame: usize,
    origin: Instant,
    deadline: Instant,
}
fn worker_loop(
    stopped: StoppedPluginAudioProcessor<GuiHost>,
    progress: Arc<Progress>,
    commands: mpsc::Receiver<(usize, Command)>,
    setup: WorkerSetup,
) -> (StoppedPluginAudioProcessor<GuiHost>, Receipt) {
    let mut receipt = Receipt {
        first_frame: setup.first_frame,
        intervals: Vec::with_capacity(MAX_BLOCKS),
        commands: Vec::with_capacity(32),
        lifecycle: Vec::with_capacity(34),
        ..Receipt::default()
    };
    let start = Instant::now();
    let mut audio = match stopped.start_processing() {
        Ok(audio) => audio,
        Err(error) => {
            receipt.failure = Some("start_processing failed");
            progress.finished.store(true, Ordering::Release);
            return (error.into_stopped_processor(), receipt);
        }
    };
    receipt.lifecycle.push((
        "start_processing",
        start.duration_since(setup.origin).as_micros() as u64,
        setup.origin.elapsed().as_micros() as u64,
    ));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut pcm = [[123.0; 512]; 2];
        let mut inputs = AudioPorts::with_capacity(0, 0);
        let input = inputs.with_input_buffers(std::iter::empty::<
            AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
        >());
        let mut ports = AudioPorts::with_capacity(2, 1);
        let mut events = EventBuffer::new();
        // Reserve the single event before entering the bounded callback loop.
        events.push(&MidiEvent::new(setup.offset as u32, 0, [0x90, 60, 127]));
        let mut output = Recorder::new(64);
        let mut onset_frame = None;
        let mut autonomous_start = None;
        let mut previous_end = None;
        while !progress.stop.load(Ordering::Acquire) {
            if Instant::now() >= setup.deadline
                || receipt.blocks + setup.consumed_blocks >= MAX_BLOCKS
            {
                receipt.failure = Some("shared interval or callback budget exhausted");
                break;
            }
            events.clear();
            let command = commands.try_recv().ok();
            if let Some((sequence, command)) = command {
                assert_eq!(sequence, receipt.commands.len() + 1);
                assert!(sequence <= 32);
                receipt.commands.push((sequence, receipt.blocks, command));
                match command {
                    Command::Reset => {
                        let start = Instant::now();
                        audio.reset();
                        receipt.lifecycle.push((
                            "reset",
                            start.duration_since(setup.origin).as_micros() as u64,
                            setup.origin.elapsed().as_micros() as u64,
                        ));
                        onset_frame = None;
                    }
                    Command::Note { key, audible } => {
                        assert!(onset_frame.is_none(), "probe begins in a silent window");
                        events.push(&MidiEvent::new(setup.offset as u32, 0, [0x90, key, 127]));
                        if audible {
                            onset_frame = Some(receipt.frames + setup.offset);
                        }
                    }
                    Command::Autonomous => {
                        assert!(autonomous_start.is_none());
                        assert!(onset_frame.is_none());
                        autonomous_start = Some(receipt.blocks);
                    }
                }
            }
            if autonomous_start.is_some_and(|first| (receipt.blocks - first).is_multiple_of(64)) {
                assert!(
                    onset_frame.is_none(),
                    "canonical kick must end before next autonomous onset"
                );
                events.push(&MidiEvent::new(3, 0, [0x90, 61, 127]));
                onset_frame = Some(receipt.frames + 3);
                progress
                    .kick_block
                    .store(receipt.blocks + 1, Ordering::Release);
            }
            pcm.fill([123.0; 512]);
            output.attempts = 0;
            let mut buffers = ports.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only(
                    pcm.iter_mut().map(|c| &mut c[..setup.frames]),
                ),
            }]);
            let start = Instant::now();
            let status = audio.process(
                &input,
                &mut buffers,
                &events.as_input(),
                &mut OutputEvents::from_buffer(&mut output),
                Some((receipt.first_frame + receipt.frames) as u64),
                None,
            );
            let end = Instant::now();
            receipt.intervals.push((
                start.duration_since(setup.origin).as_micros() as u64,
                end.duration_since(setup.origin).as_micros() as u64,
            ));
            receipt.max_duration = receipt.max_duration.max(end.duration_since(start));
            if let Some(previous) = previous_end {
                receipt.max_gap = receipt.max_gap.max(end.duration_since(previous));
            }
            previous_end = Some(end);
            if !matches!(status, Ok(ProcessStatus::Continue)) {
                receipt.failure = Some("process must Continue");
                break;
            }
            if output.len != 0 {
                receipt.failure = Some("unexpected automation after completed setup gesture");
                break;
            }
            if receipt.max_duration > WATCHDOG || receipt.max_gap > WATCHDOG {
                receipt.failure = Some("synthetic callback watchdog exceeded");
                break;
            }
            for (channel, samples) in pcm.iter().enumerate() {
                for (index, actual) in samples[..setup.frames].iter().copied().enumerate() {
                    let frame = receipt.frames + index;
                    let expected = onset_frame
                        .and_then(|onset| frame.checked_sub(onset))
                        .and_then(|i| setup.wave.get(i))
                        .copied()
                        .unwrap_or(0.0)
                        * setup.gain;
                    if actual != expected {
                        receipt.failure = Some("canonical advancing stereo waveform mismatch");
                        receipt.mismatch = Some((frame, channel, expected, actual));
                        return;
                    }
                    if onset_frame.is_some_and(|onset| frame == onset || frame == onset + 1) {
                        let anchor = if setup.gain == 0.5 {
                            [-0.25, -179.0 / 32768.0]
                        } else {
                            [-1.0, -716.0 / 32768.0]
                        };
                        assert_eq!(actual, anchor[frame - onset_frame.unwrap()]);
                    }
                }
            }
            receipt.blocks += 1;
            receipt.frames += setup.frames;
            if onset_frame.is_some_and(|onset| receipt.frames >= onset + KICK_FRAMES) {
                onset_frame = None;
            }
            progress.blocks.store(receipt.blocks, Ordering::Release);
            if let Some((sequence, _)) = command {
                progress.ack_block.store(receipt.blocks, Ordering::Release);
                progress.ack.store(sequence, Ordering::Release);
            }
            // No GUI phase barrier or catch-up burst; each endpoint advances alone.
            std::thread::sleep(Duration::from_millis(10));
        }
    }));
    if outcome.is_err() {
        receipt.failure = Some("worker setup or loop panicked");
    }
    let start = Instant::now();
    let stopped = audio.stop_processing();
    receipt.lifecycle.push((
        "stop_processing",
        start.duration_since(setup.origin).as_micros() as u64,
        setup.origin.elapsed().as_micros() as u64,
    ));
    progress.finished.store(true, Ordering::Release);
    (stopped, receipt)
}

// GTK, Rc, browser objects and plugin handles remain on this one host thread.
// Only the owned stopped processor moves into each worker.
struct Pair {
    plugins: [PluginInstance<GuiHost>; 2],
    parents: [Parent; 2],
    views: [Option<webkit2gtk::WebView>; 2],
    ledgers: [Rc<RefCell<TimerLedger>>; 2],
    workers: [Option<Worker>; 2],
    consumed: [usize; 2],
    consumed_frames: [usize; 2],
    command_count: usize,
    origin: Instant,
    deadline: Instant,
    operations: Vec<(&'static str, usize, u64, u64)>,
    receipts: Vec<(usize, Receipt)>,
}
impl Pair {
    fn pump(&mut self) {
        assert!(Instant::now() < self.deadline, "shared UI deadline");
        for plugin in &mut self.plugins {
            dispatch_context(plugin);
        }
        for worker in self.workers.iter().flatten() {
            assert!(
                !worker.progress.finished.load(Ordering::Acquire),
                "audio worker finished before requested stop; receipt follows in cleanup"
            );
        }
    }
    fn js(&mut self, side: usize, script: &str) -> String {
        let result = Rc::new(RefCell::new(None));
        let received = Rc::clone(&result);
        self.views[side].as_ref().unwrap().run_javascript(
            script,
            None::<&gtk::gio::Cancellable>,
            move |value| {
                *received.borrow_mut() = Some(
                    value
                        .map(|v| v.js_value().unwrap().to_string())
                        .map_err(|e| e.to_string()),
                );
            },
        );
        loop {
            self.pump();
            if let Some(value) = result.borrow_mut().take() {
                return value.expect("actual WebKit evaluation");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn dom(&mut self, side: usize, expression: &str) {
        while self.js(side, &format!("String(Boolean({expression}))")) != "true" {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn invoke(&mut self, side: usize, expression: &str) -> serde_json::Value {
        self.js(side, &format!("window.__PAIR_REPLY__=null;({expression}).then(result=>window.__PAIR_REPLY__={{status:'accepted',result}},error=>window.__PAIR_REPLY__={{status:'rejected',code:error.code}});'started'"));
        loop {
            let reply = self.js(side, "JSON.stringify(window.__PAIR_REPLY__)");
            if reply != "null" {
                let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
                assert_eq!(
                    reply["status"], "accepted",
                    "native side{side} reply {reply}"
                );
                return reply["result"].clone();
            }
        }
    }
    fn snapshot(&mut self, side: usize) -> serde_json::Value {
        self.invoke(
            side,
            "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
        )
    }
    fn feedback(&mut self, side: usize) -> serde_json::Value {
        self.invoke(
            side,
            "window.__DANDRUM__.backend.getNativeFunction('getParameterState')()",
        )
    }
    fn operation(&mut self, side: usize, label: &'static str, start: Instant) {
        assert!(self.operations.len() < 64);
        self.operations.push((
            label,
            side,
            start.duration_since(self.origin).as_micros() as u64,
            self.origin.elapsed().as_micros() as u64,
        ));
    }
    fn open(&mut self, side: usize) {
        let gui = self.plugins[side]
            .plugin_handle()
            .get_extension::<PluginGui>()
            .unwrap();
        let start = Instant::now();
        gui.create(
            &self.plugins[side].plugin_handle(),
            GuiConfiguration {
                api_type: GuiApiType::X11,
                is_floating: false,
            },
        )
        .unwrap();
        self.operation(side, "gui.create", start);
        let start = Instant::now();
        unsafe {
            gui.set_parent(
                &self.plugins[side].plugin_handle(),
                Window::from_x11_handle(self.parents[side].id.into()),
            )
        }
        .unwrap();
        self.operation(side, "gui.set_parent", start);
        gui.show(&self.plugins[side].plugin_handle()).unwrap();
        let children = self.parents[side].children();
        assert_eq!(children.len(), 1);
        let geometry = self.parents[side]
            .x
            .get_geometry(children[0])
            .unwrap()
            .reply()
            .unwrap();
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (0, 0, 1200, 800)
        );
        self.views[side] = Some(
            weak_browser_in_child(children[0])
                .upgrade()
                .unwrap()
                .downcast::<webkit2gtk::WebView>()
                .unwrap(),
        );
        self.dom(
            side,
            "document.querySelector('[data-testid=trigger-editor]')",
        );
    }
    fn close(&mut self, side: usize) {
        let child = self.parents[side].children()[0];
        let weak = weak_browser_in_child(child);
        let gui = self.plugins[side]
            .plugin_handle()
            .get_extension::<PluginGui>()
            .unwrap();
        let start = Instant::now();
        gui.destroy(&self.plugins[side].plugin_handle());
        self.operation(side, "gui.destroy", start);
        drop(self.views[side].take());
        assert!(
            weak.upgrade().is_none(),
            "side{side} WebView owner released"
        );
        assert!(self.parents[side].children().is_empty());
        assert!(
            self.parents[side]
                .x
                .get_geometry(child)
                .unwrap()
                .reply()
                .is_err()
        );
    }
    fn edit_level(&mut self, side: usize, gain: f64) {
        self.js(side, "(()=>{const tab=Array.from(document.querySelectorAll('[role=tab]')).find(t=>t.textContent.trim()==='Voice');tab.click();return 'clicked';})()");
        self.dom(
            side,
            "document.querySelector('[data-knob=\"drums.level\"] [role=slider]')",
        );
        self.js(side, "(()=>{const s=document.querySelector('[data-knob=\"drums.level\"] [role=slider]');s.focus();s.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'editing';})()");
        self.dom(
            side,
            "document.querySelector('[data-knob=\"drums.level\"] input')",
        );
        self.js(side, &format!("(()=>{{const input=document.querySelector('[data-knob=\"drums.level\"] input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'{gain}');input.dispatchEvent(new Event('input',{{bubbles:true}}));return input.value;}})()"));
        self.js(side, "(()=>{document.querySelector('[data-knob=\"drums.level\"] input').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'committed';})()");
        self.dom(
            side,
            "!document.querySelector('[data-knob=\"drums.level\"] input')",
        );
        self.dom(side, &format!("document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='{}'", gain / 4.0));
        let plugin = &mut self.plugins[side];
        let params = plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap();
        let timer = plugin
            .plugin_handle()
            .get_extension::<clack_extensions::timer::PluginTimer>()
            .unwrap();
        let ids = self.ledgers[side].borrow().live.clone();
        assert_eq!(ids.len(), 1);
        timer.on_timer(&plugin.plugin_handle(), ids[0]);
        let mut output = Recorder::new(64);
        params.flush(
            &mut plugin.inactive_plugin_handle().unwrap(),
            &InputEvents::empty(),
            &mut OutputEvents::from_buffer(&mut output),
        );
        assert_eq!(
            &output.accepted[..output.len],
            &[
                Some(Recorded::Begin(3)),
                Some(Recorded::Value(3, gain)),
                Some(Recorded::End(3))
            ]
        );
        timer.on_timer(&plugin.plugin_handle(), ids[0]);
        self.assert_level(side, gain);
        assert_eq!(
            self.snapshot(side)["connection"]["history"],
            serde_json::json!({"undoCount":0,"redoCount":0})
        );
    }
    fn assert_level(&mut self, side: usize, gain: f64) -> serde_json::Value {
        let plugin = &mut self.plugins[side];
        let params = plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap();
        assert_eq!(
            params.get_value(&plugin.plugin_handle(), ClapId::new(3)),
            Some(gain)
        );
        let feedback = self.feedback(side);
        let level = feedback["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "drums.level")
            .unwrap();
        assert_eq!(
            level,
            &serde_json::json!({"id":"drums.level","name":"drums.level","value":gain / 4.0})
        );
        feedback
    }
    fn mapping(&mut self, side: usize, request: &str, key: u8) -> serde_json::Value {
        let before = self.snapshot(side);
        let revision = before["connection"]["revision"].as_str().unwrap();
        let reply = self.invoke(side, &format!("(()=>{{const c=window.__DANDRUM_NATIVE__;return window.__DANDRUM__.backend.getNativeFunction('setInstrumentZoneKeyRange')(c.instanceId,c.attachmentId,'{request}','{revision}','kit','kick',{key},{key});}})()"));
        assert_eq!(
            reply,
            serde_json::json!({"status":"accepted","job_id":request})
        );
        self.job(side, request);
        let accepted = self.snapshot(side);
        assert_eq!(
            accepted["connection"]["revision"],
            (revision.parse::<u64>().unwrap() + 1).to_string()
        );
        assert_map(&accepted, key);
        accepted
    }
    fn job(&mut self, side: usize, request: &str) -> serde_json::Value {
        loop {
            let job = self.invoke(side, &format!("window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('{request}')"));
            if job["state"] == "completed" {
                return job;
            }
            assert_ne!(job["state"], "failed", "side{side}: {job}");
        }
    }
    fn start(&mut self, side: usize, frames: usize, wave: Arc<Vec<f32>>) {
        assert!(self.workers[side].is_none());
        let start = Instant::now();
        let stopped = self.plugins[side]
            .activate(
                |_, _| (),
                PluginAudioConfiguration {
                    sample_rate: 48_000.0,
                    min_frames_count: 1,
                    max_frames_count: frames as u32,
                },
            )
            .unwrap();
        self.operation(side, "activate", start);
        let progress = Arc::new(Progress::default());
        let shared = Arc::clone(&progress);
        let (send, receive) = mpsc::sync_channel(1);
        let setup = WorkerSetup {
            wave,
            frames,
            gain: [0.5, 2.0][side],
            offset: [5, 3][side],
            consumed_blocks: self.consumed[side],
            first_frame: self.consumed_frames[side],
            origin: self.origin,
            deadline: self.deadline,
        };
        self.workers[side] = Some(Worker {
            progress,
            commands: send,
            thread: std::thread::spawn(move || worker_loop(stopped, shared, receive, setup)),
            sequence: 0,
        });
    }
    fn command(&mut self, side: usize, command: Command) -> usize {
        self.command_count += 1;
        assert!(self.command_count <= 32, "whole-case command bound");
        let start = Instant::now();
        let worker = self.workers[side].as_mut().unwrap();
        worker.sequence += 1;
        assert!(worker.sequence <= 32);
        let sequence = worker.sequence;
        worker.commands.try_send((sequence, command)).unwrap();
        loop {
            self.pump();
            let progress = &self.workers[side].as_ref().unwrap().progress;
            if progress.ack.load(Ordering::Acquire) == sequence {
                let block = progress.ack_block.load(Ordering::Acquire);
                self.operation(side, "command/ack", start);
                return block;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn wait_blocks(&mut self, side: usize, count: usize) {
        loop {
            self.pump();
            if self.workers[side]
                .as_ref()
                .unwrap()
                .progress
                .blocks
                .load(Ordering::Acquire)
                >= count
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn probe(&mut self, side: usize, key: u8, audible: bool, frames: usize) {
        let block = self.command(side, Command::Note { key, audible });
        self.wait_blocks(side, block + (KICK_FRAMES + 5).div_ceil(frames));
    }
    fn b_audible(&mut self) {
        loop {
            self.pump();
            let progress = &self.workers[1].as_ref().unwrap().progress;
            let kick = progress.kick_block.load(Ordering::Acquire);
            let block = progress.blocks.load(Ordering::Acquire);
            if kick > 0 && block >= kick && block - kick <= 8 {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn finish(&mut self, side: usize) {
        if let Some(worker) = self.workers[side].take() {
            worker.progress.stop.store(true, Ordering::Release);
            let start = Instant::now();
            let (stopped, receipt) = worker
                .thread
                .join()
                .expect("caught worker returns endpoint");
            self.operation(side, "stop/join", start);
            assert!(
                stopped.matches(&self.plugins[side]),
                "actual returned endpoint belongs to side{side}"
            );
            let start = Instant::now();
            self.plugins[side].deactivate(stopped);
            self.operation(side, "deactivate", start);
            self.consumed[side] += receipt.blocks;
            assert_eq!(receipt.first_frame, self.consumed_frames[side]);
            self.consumed_frames[side] += receipt.frames;
            self.receipts.push((side, receipt));
        }
    }
}
fn assert_map(snapshot: &serde_json::Value, key: u8) {
    assert_eq!(snapshot["document"]["maps"][0]["zones"][0]["keyLow"], key);
    assert_eq!(snapshot["document"]["maps"][0]["zones"][0]["keyHigh"], key);
}

struct Baseline {
    snapshot: serde_json::Value,
    feedback: serde_json::Value,
    saved: Vec<u8>,
    child: u32,
    weak: gtk::glib::WeakRef<gtk::glib::Object>,
}
impl Baseline {
    fn capture(pair: &mut Pair, side: usize) -> Self {
        let child = pair.parents[side].children()[0];
        Self {
            snapshot: pair.snapshot(side),
            feedback: pair.assert_level(side, [0.5, 2.0][side]),
            saved: saved_state(&mut pair.plugins[side]),
            child,
            weak: weak_browser_in_child(child),
        }
    }
    fn check(&self, pair: &mut Pair, side: usize) {
        assert_eq!(
            pair.snapshot(side),
            self.snapshot,
            "side{side} full accepted snapshot"
        );
        assert_eq!(
            pair.assert_level(side, [0.5, 2.0][side]),
            self.feedback,
            "side{side} full feedback DTO"
        );
        assert_eq!(
            saved_state(&mut pair.plugins[side]),
            self.saved,
            "side{side} exact saved bytes"
        );
        assert_eq!(pair.parents[side].children(), vec![self.child]);
        assert!(self.weak.upgrade().is_some());
        assert_eq!(
            pair.parents[side]
                .x
                .get_window_attributes(self.child)
                .unwrap()
                .reply()
                .unwrap()
                .map_state,
            x11rb::protocol::xproto::MapState::VIEWABLE
        );
    }
}

#[test]
#[ignore = "two-instance native boundary: inherited X11 and copied release"]
fn copied_native_two_instances_keep_edits_audio_and_editors_independent() {
    std::thread::spawn(two_instances)
        .join()
        .expect("two-instance main owner completed");
}
fn two_instances() {
    let wave = Arc::new(canonical_kick());
    let entry = copied_entry();
    let ledgers = std::array::from_fn(|_| Rc::new(RefCell::new(TimerLedger::default())));
    let plugins =
        std::array::from_fn(|i| instantiate_options(&entry, true, Rc::clone(&ledgers[i])));
    let origin = Instant::now();
    let mut pair = Pair {
        plugins,
        parents: [Parent::new(), Parent::new()],
        views: [None, None],
        ledgers,
        workers: [None, None],
        consumed: [0, 0],
        consumed_frames: [0, 0],
        command_count: 0,
        origin,
        deadline: origin + Duration::from_secs(15),
        operations: Vec::with_capacity(64),
        receipts: Vec::with_capacity(3),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for side in 0..2 {
            pair.open(side);
            pair.edit_level(side, [0.5, 2.0][side]);
        }
        let a_initial = pair.snapshot(0);
        let b_initial = pair.snapshot(1);
        assert_ne!(
            a_initial["connection"]["instanceId"],
            b_initial["connection"]["instanceId"]
        );
        let a60 = pair.mapping(0, "9101", 60);
        let a60_saved = saved_state(&mut pair.plugins[0]);
        let a = pair.mapping(0, "9102", 62);
        let b = pair.mapping(1, "9201", 61);
        assert_eq!(
            a["connection"]["history"],
            serde_json::json!({"undoCount":2,"redoCount":0})
        );
        assert_eq!(
            b["connection"]["history"],
            serde_json::json!({"undoCount":1,"redoCount":0})
        );
        // These setup edits used authenticated native APIs directly. React's
        // command owner normally refreshes local history itself; reopen here to
        // initialize each real view from the accepted session before pinning B.
        for (side, before) in [(0, &a), (1, &b)] {
            let saved = saved_state(&mut pair.plugins[side]);
            let feedback = pair.assert_level(side, [0.5, 2.0][side]);
            pair.close(side);
            pair.open(side);
            let current = pair.snapshot(side);
            assert_eq!(current["document"], before["document"]);
            assert_eq!(current["yaml"], before["yaml"]);
            for field in ["instanceId", "revision", "history"] {
                assert_eq!(current["connection"][field], before["connection"][field]);
            }
            assert_ne!(
                current["connection"]["attachmentId"],
                before["connection"]["attachmentId"]
            );
            assert_eq!(saved_state(&mut pair.plugins[side]), saved);
            assert_eq!(pair.assert_level(side, [0.5, 2.0][side]), feedback);
            pair.dom(
                side,
                "document.querySelector('header button[aria-label^=\"Undo \"]:not(:disabled)')",
            );
        }
        let setup_end = Instant::now();
        pair.operation(0, "setup complete", setup_end);
        pair.deadline = setup_end + Duration::from_secs(20);
        pair.start(0, 256, Arc::clone(&wave));
        pair.start(1, 256, Arc::clone(&wave));
        // Wrong keys are probed only while the prior full kick has ended.
        for (side, mapped, foreign) in [(0, 62, 61), (1, 61, 62)] {
            pair.probe(side, mapped, true, 256);
            pair.probe(side, foreign, false, 256);
            pair.probe(side, 36, false, 256);
        }
        let b = Baseline::capture(&mut pair, 1);
        pair.command(1, Command::Autonomous);
        let b_worker = pair.workers[1].as_ref().unwrap().thread.thread().id();
        pair.dom(
            0,
            "document.querySelector('header button[aria-label^=\"Undo \"]:not(:disabled)')",
        );
        let undo_start = Instant::now();
        pair.js(0, r#"(()=>{window.__PAIR_UNDO__=null;const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{const promise=get(method)(...args);if(method==='undoInstrumentSession')promise.then(result=>window.__PAIR_UNDO__={status:'accepted',result},error=>window.__PAIR_UNDO__={status:'rejected',code:error.code});return promise;};document.querySelector('header button[aria-label^="Undo "]').click();return 'clicked';})()"#);
        let undo: serde_json::Value = loop {
            let reply = pair.js(0, "JSON.stringify(window.__PAIR_UNDO__)");
            if reply != "null" {
                break serde_json::from_str(&reply).unwrap();
            }
        };
        assert_eq!(undo["status"], "accepted");
        pair.job(0, undo["result"]["job_id"].as_str().unwrap());
        pair.operation(0, "header Undo accepted", undo_start);
        let a_undo = pair.snapshot(0);
        assert_map(&a_undo, 60);
        assert_eq!(
            a_undo["yaml"], a60["yaml"],
            "Undo restores the exact earlier accepted document"
        );
        assert_eq!(saved_state(&mut pair.plugins[0]), a60_saved);
        assert_eq!(
            a_undo["connection"]["history"],
            serde_json::json!({"undoCount":1,"redoCount":1})
        );
        assert_eq!(a_undo["connection"]["revision"], "3");
        pair.assert_level(0, 0.5);
        b.check(&mut pair, 1);
        pair.probe(0, 60, true, 256);
        pair.probe(0, 62, false, 256);
        let a_saved = saved_state(&mut pair.plugins[0]);
        let a_feedback = pair.feedback(0);
        pair.b_audible();
        pair.close(0);
        b.check(&mut pair, 1);
        pair.open(0);
        let reopened = pair.snapshot(0);
        assert_eq!(reopened["document"], a_undo["document"]);
        assert_eq!(
            reopened["connection"]["instanceId"],
            a_undo["connection"]["instanceId"]
        );
        assert_ne!(
            reopened["connection"]["attachmentId"],
            a_undo["connection"]["attachmentId"]
        );
        assert_eq!(
            reopened["connection"]["history"],
            a_undo["connection"]["history"]
        );
        assert_eq!(
            reopened["connection"]["revision"],
            a_undo["connection"]["revision"]
        );
        assert_eq!(saved_state(&mut pair.plugins[0]), a_saved);
        assert_eq!(pair.assert_level(0, 0.5), a_feedback);
        b.check(&mut pair, 1);
        let a_reopened = Baseline::capture(&mut pair, 0);
        pair.b_audible();
        pair.command(
            0,
            Command::Note {
                key: 60,
                audible: true,
            },
        );
        let reset = pair.command(0, Command::Reset);
        pair.wait_blocks(0, reset + 3);
        a_reopened.check(&mut pair, 0);
        b.check(&mut pair, 1);
        pair.probe(0, 60, true, 256);
        pair.b_audible();
        pair.command(
            0,
            Command::Note {
                key: 60,
                audible: true,
            },
        );
        pair.finish(0);
        b.check(&mut pair, 1);
        pair.start(0, 512, Arc::clone(&wave));
        pair.wait_blocks(0, 3);
        a_reopened.check(&mut pair, 0);
        b.check(&mut pair, 1);
        pair.probe(0, 60, true, 512);
        a_reopened.check(&mut pair, 0);
        b.check(&mut pair, 1);
        assert_eq!(
            pair.workers[1].as_ref().unwrap().thread.thread().id(),
            b_worker
        );
        pair.close(0);
        b.check(&mut pair, 1);
        pair.close(1);
    }));
    // Request both stops before joining either. Every recoverable main assertion
    // takes this path, including an A transition while B still owns its endpoint.
    for worker in pair.workers.iter().flatten() {
        worker.progress.stop.store(true, Ordering::Release);
    }
    let mut cleanup_errors = Vec::new();
    for side in 0..2 {
        let cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pair.finish(side)));
        if let Err(error) = cleanup {
            cleanup_errors.push(error);
        }
    }
    for side in 0..2 {
        if pair.views[side].is_some() {
            let cleanup =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pair.close(side)));
            if let Err(error) = cleanup {
                cleanup_errors.push(error);
            }
        }
    }
    eprintln!(
        "two-instance operations (microseconds from host origin): {:?}",
        pair.operations
    );
    for (side, receipt) in &pair.receipts {
        eprintln!("side{side} returned receipt: {receipt:?}");
    }
    let worker_failure = pair
        .receipts
        .iter()
        .find(|(_, r)| r.failure.is_some())
        .map(|(side, r)| (*side, r.failure, r.mismatch));
    let ledgers = pair.ledgers.clone();
    drop(pair); // Sessions and workers are gone before their shared Entry.
    for ledger in ledgers {
        assert!(ledger.borrow().live.is_empty());
    }
    assert!(
        cleanup_errors.is_empty(),
        "recoverable cleanup failure count {}",
        cleanup_errors.len()
    );
    assert!(
        worker_failure.is_none(),
        "audio failure: {worker_failure:?}"
    );
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
    drop(entry);
}
