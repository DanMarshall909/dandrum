//! Installed CLAP acceptance through the real JACK/PipeWire graph and XTest input.
//! This is neither a synthetic processing loop nor named-DAW/DAC acceptance.
use super::*;
use std::ffi::{CStr, CString, c_char, c_int, c_ulong, c_void};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

const MAX_FRAMES: usize = 8192;
const NOTE_OFFSET: u32 = 3;
type Ptr = *mut c_void;
type Process = unsafe extern "C" fn(u32, Ptr) -> c_int;

// JACK's stable public C ABI: https://jackaudio.org/api/group__ClientFunctions.html
// and group__MIDIAPI.html. These bindings belong only to the acceptance host.
#[derive(Clone, Copy)]
struct Jack {
    open: unsafe extern "C" fn(*const c_char, u32, *mut u32, ...) -> Ptr,
    close: unsafe extern "C" fn(Ptr) -> c_int,
    activate: unsafe extern "C" fn(Ptr) -> c_int,
    deactivate: unsafe extern "C" fn(Ptr) -> c_int,
    callback: unsafe extern "C" fn(Ptr, Process, Ptr) -> c_int,
    rate: unsafe extern "C" fn(Ptr) -> u32,
    quantum: unsafe extern "C" fn(Ptr) -> u32,
    port: unsafe extern "C" fn(Ptr, *const c_char, *const c_char, c_ulong, c_ulong) -> Ptr,
    name: unsafe extern "C" fn(Ptr) -> *const c_char,
    connect: unsafe extern "C" fn(Ptr, *const c_char, *const c_char) -> c_int,
    buffer: unsafe extern "C" fn(Ptr, u32) -> Ptr,
    clear: unsafe extern "C" fn(Ptr),
    write: unsafe extern "C" fn(Ptr, u32, *const u8, usize) -> c_int,
    count: unsafe extern "C" fn(Ptr) -> u32,
    event: unsafe extern "C" fn(*mut JackMidi, Ptr, u32) -> c_int,
}
#[repr(C)]
struct JackMidi {
    time: u32,
    size: usize,
    buffer: *mut u8,
}
impl Jack {
    fn load(library: &libloading::Library) -> Self {
        macro_rules! symbol {
            ($name:literal) => {
                unsafe { *library.get(concat!($name, "\0").as_bytes()).unwrap() }
            };
        }
        Self {
            open: symbol!("jack_client_open"),
            close: symbol!("jack_client_close"),
            activate: symbol!("jack_activate"),
            deactivate: symbol!("jack_deactivate"),
            callback: symbol!("jack_set_process_callback"),
            rate: symbol!("jack_get_sample_rate"),
            quantum: symbol!("jack_get_buffer_size"),
            port: symbol!("jack_port_register"),
            name: symbol!("jack_port_name"),
            connect: symbol!("jack_connect"),
            buffer: symbol!("jack_port_get_buffer"),
            clear: symbol!("jack_midi_clear_buffer"),
            write: symbol!("jack_midi_event_write"),
            count: symbol!("jack_midi_get_event_count"),
            event: symbol!("jack_midi_event_get"),
        }
    }
    fn client(self, name: &str) -> Client {
        let name = CString::new(name).unwrap();
        let mut status = 0;
        let raw = unsafe { (self.open)(name.as_ptr(), 1, &mut status) }; // JackNoStartServer
        assert!(
            !raw.is_null(),
            "existing JACK/PipeWire server unavailable: {status}"
        );
        Client {
            raw,
            api: self,
            active: false,
        }
    }
}
struct Client {
    raw: Ptr,
    api: Jack,
    active: bool,
}
impl Drop for Client {
    fn drop(&mut self) {
        self.quiesce();
        if unsafe { (self.api.close)(self.raw) } != 0 {
            eprintln!("failed to close owned JACK client");
            std::process::abort();
        }
    }
}
impl Client {
    fn activate(&mut self) {
        assert_eq!(
            unsafe { (self.api.activate)(self.raw) },
            0,
            "activate owned JACK client"
        );
        self.active = true;
    }
    fn quiesce(&mut self) {
        if self.active {
            if unsafe { (self.api.deactivate)(self.raw) } != 0 {
                // Never release callback data when server ownership is uncertain.
                eprintln!("cannot establish native callback quiescence");
                std::process::abort();
            }
            self.active = false;
        }
    }
    fn port(&self, name: &CStr, kind: &CStr, flags: c_ulong) -> Ptr {
        let port = unsafe { (self.api.port)(self.raw, name.as_ptr(), kind.as_ptr(), flags, 0) };
        assert!(!port.is_null(), "register owned JACK port");
        port
    }
}
#[derive(Default)]
struct Progress {
    reset: AtomicBool,
    resets: AtomicUsize,
    stop: AtomicBool,
    stopped: AtomicBool,
    failed: AtomicUsize,
    callbacks: AtomicUsize,
    received: AtomicUsize,
    phase: AtomicUsize,
}
#[derive(Clone, Copy, Debug)]
struct Observation {
    phase: usize,
    key: u8,
    start_us: u64,
    end_us: u64,
    // Each instance/channel: onset, next two frames and pre-onset silence.
    pcm: [[[f32; 4]; 2]; 2],
    events: [Option<Recorded>; 16],
    len: usize,
}
struct Source {
    api: Jack,
    port: Ptr,
    progress: Arc<Progress>,
    rate: u32,
    frames: u64,
    next: u64,
    key: u8,
}
unsafe extern "C" fn source_callback(frames: u32, data: Ptr) -> c_int {
    let source = unsafe { &mut *data.cast::<Source>() };
    let buffer = unsafe { (source.api.buffer)(source.port, frames) };
    unsafe {
        (source.api.clear)(buffer);
    }
    if frames <= 64 {
        source.progress.failed.store(1, Ordering::Release);
        return 0;
    }
    // The packaged kick lasts 0.25 seconds. Half-second spacing keeps each
    // independent onset oracle clear of prior one-shot voices without resets.
    if source.frames >= source.next {
        let bytes = [0x90, source.key, 127];
        if unsafe { (source.api.write)(buffer, NOTE_OFFSET, bytes.as_ptr(), bytes.len()) } != 0 {
            source.progress.failed.store(2, Ordering::Release);
        }
        source.key = if source.key == 36 { 48 } else { 36 };
        source.next = source.frames + u64::from(source.rate / 2);
    }
    source.frames += u64::from(frames);
    0
}
struct Processor {
    api: Jack,
    midi: Ptr,
    outputs: [Ptr; 2],
    progress: Arc<Progress>,
    stopped: [Option<clack_host::process::StoppedPluginAudioProcessor<GuiHost>>; 2],
    audio: [Option<StartedPluginAudioProcessor<GuiHost>>; 2],
    input: AudioPorts,
    output: AudioPorts,
    events: EventBuffer,
    pcm: Vec<f32>,
    records: rtrb::Producer<Observation>,
    origin: Instant,
    audit: (Audit, libloading::Library),
    calibrated: bool,
    allocate: unsafe extern "C" fn() -> *mut u8,
    free: unsafe extern "C" fn(*mut u8),
    frames: u64,
}
unsafe extern "C" fn process_callback(frames: u32, data: Ptr) -> c_int {
    let processor = unsafe { &mut *data.cast::<Processor>() };
    // Never unwind through the server's foreign callback, including fixture faults.
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| processor.process(frames))).is_err()
    {
        processor.progress.failed.store(3, Ordering::Release);
    }
    0
}
impl Processor {
    fn process(&mut self, frames: u32) {
        for port in self.outputs {
            let buffer = unsafe { (self.api.buffer)(port, frames).cast::<f32>() };
            if buffer.is_null() {
                self.progress.failed.store(13, Ordering::Release);
                return;
            }
            unsafe {
                std::slice::from_raw_parts_mut(buffer, frames as usize).fill(0.0);
            }
        }
        if self.progress.stop.load(Ordering::Acquire) {
            for i in 0..2 {
                if let Some(audio) = self.audio[i].take() {
                    self.stopped[i] = Some(audio.stop_processing());
                }
            }
            self.progress.stopped.store(true, Ordering::Release);
            return;
        }
        if frames as usize > MAX_FRAMES || frames <= 64 {
            self.progress.failed.store(4, Ordering::Release);
            return;
        }
        if self.progress.failed.load(Ordering::Acquire) != 0 {
            return;
        }
        for i in 0..2 {
            if let Some(stopped) = self.stopped[i].take() {
                match stopped.start_processing() {
                    Ok(audio) => self.audio[i] = Some(audio),
                    Err(error) => {
                        self.stopped[i] = Some(error.into_stopped_processor());
                        self.progress.failed.store(11, Ordering::Release);
                        return;
                    }
                }
            }
        }
        if !self.calibrated {
            // Symbols are resolved off audio; only bounded calibration runs here.
            let audit = self.audit.0;
            let old = unsafe { (self.allocate)() };
            let (fresh, allocated) = audit.measure(|| unsafe { (self.allocate)() });
            let (_, freed) = audit.measure(|| unsafe { (self.free)(old) });
            let empty = audit.measure(|| ()).1;
            unsafe {
                (self.free)(fresh);
            }
            if old.is_null()
                || fresh.is_null()
                || allocated.0 == 0
                || allocated.1 != 0
                || freed.0 != 0
                || freed.1 == 0
                || empty != (0, 0)
            {
                self.progress.failed.store(12, Ordering::Release);
                return;
            }
            self.calibrated = true;
        }
        if self.progress.reset.swap(false, Ordering::AcqRel) {
            for audio in &mut self.audio {
                let (_, counts) = self.audit.0.measure(|| audio.as_mut().unwrap().reset());
                if counts != (0, 0) {
                    self.progress.failed.store(14, Ordering::Release);
                    return;
                }
                self.progress.resets.fetch_add(1, Ordering::Release);
            }
        }
        let start = self.origin.elapsed().as_micros() as u64;
        let phase = self.progress.phase.load(Ordering::Acquire);
        self.events.clear();
        let buffer = unsafe { (self.api.buffer)(self.midi, frames) };
        let count = unsafe { (self.api.count)(buffer) };
        if count > 1 {
            self.progress.failed.store(5, Ordering::Release);
            return;
        }
        let mut key = 0;
        if count == 1 {
            let mut event = JackMidi {
                time: 0,
                size: 0,
                buffer: std::ptr::null_mut(),
            };
            let result = unsafe { (self.api.event)(&mut event, buffer, 0) };
            if result != 0 || event.time != NOTE_OFFSET || event.size != 3 || event.buffer.is_null()
            {
                self.progress.failed.store(6, Ordering::Release);
                return;
            }
            let bytes = unsafe { std::slice::from_raw_parts(event.buffer, 3) };
            if bytes[0] != 0x90 || ![36, 48].contains(&bytes[1]) || bytes[2] != 127 {
                self.progress.failed.store(6, Ordering::Release);
                return;
            }
            key = bytes[1];
            self.events.push(&MidiEvent::new(
                event.time,
                0,
                [bytes[0], bytes[1], bytes[2]],
            ));
        }
        self.progress
            .received
            .fetch_add(count as usize, Ordering::Release);
        let mut observation = Observation {
            phase,
            key,
            start_us: start,
            end_us: 0,
            pcm: [[[0.0; 4]; 2]; 2],
            events: [None; 16],
            len: 0,
        };
        for instance in 0..2 {
            let samples = &mut self.pcm[instance * MAX_FRAMES * 2..(instance + 1) * MAX_FRAMES * 2];
            samples.fill(123.0);
            let (left, right) = samples.split_at_mut(MAX_FRAMES);
            let input = self.input.with_input_buffers(std::iter::empty::<
                AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
            >());
            let mut output = self.output.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only([
                    &mut left[..frames as usize],
                    &mut right[..frames as usize],
                ]),
            }]);
            let mut recorded = Recorder::new(64);
            let audio = self.audio[instance].as_mut().unwrap();
            let call = || {
                audio.process(
                    &input,
                    &mut output,
                    &self.events.as_input(),
                    &mut OutputEvents::from_buffer(&mut recorded),
                    Some(self.frames),
                    None,
                )
            };
            let (status, counts) = self.audit.0.measure(call);
            if !matches!(status, Ok(ProcessStatus::Continue)) || counts != (0, 0) {
                self.progress.failed.store(7, Ordering::Release);
                return;
            }
            for (channel, samples) in [left, right].into_iter().enumerate() {
                observation.pcm[instance][channel] =
                    [samples[3], samples[4], samples[5], samples[0]];
                if instance == 0 {
                    let out =
                        unsafe { (self.api.buffer)(self.outputs[channel], frames).cast::<f32>() };
                    unsafe {
                        std::ptr::copy_nonoverlapping(samples.as_ptr(), out, frames as usize);
                    }
                }
            }
            if instance == 0 {
                if recorded.len > observation.events.len() {
                    self.progress.failed.store(8, Ordering::Release);
                    return;
                }
                observation.len = recorded.len;
                observation.events[..recorded.len]
                    .copy_from_slice(&recorded.accepted[..recorded.len]);
            } else if recorded.len != 0 {
                self.progress.failed.store(9, Ordering::Release);
                return;
            }
        }
        observation.end_us = self.origin.elapsed().as_micros() as u64;
        if self.records.push(observation).is_err() {
            self.progress.failed.store(10, Ordering::Release);
        }
        self.frames += frames as u64;
        self.progress.callbacks.fetch_add(1, Ordering::Release);
    }
}
struct NativeHost {
    first: PluginInstance<GuiHost>,
    second: PluginInstance<GuiHost>,
    receiver: Client,
    sender: Client,
    processor: Box<Processor>,
    source: Box<Source>,
    progress: Arc<Progress>,
    records: rtrb::Consumer<Observation>,
    observations: Vec<Observation>,
    rate: u32,
    origin: Instant,
    _library: libloading::Library,
}
impl NativeHost {
    fn new() -> Self {
        let library = unsafe { libloading::Library::new("libjack.so.0") }.unwrap();
        let api = Jack::load(&library);
        let receiver = api.client("dandrum-acceptance-host");
        let sender = api.client("dandrum-acceptance-midi");
        let rate = unsafe { (api.rate)(receiver.raw) };
        let quantum = unsafe { (api.quantum)(receiver.raw) };
        assert!(
            rate >= 32000 && quantum > 64 && quantum as usize <= MAX_FRAMES,
            "qualified server rate/quantum required: {rate}/{quantum}"
        );
        let midi = receiver.port(c"midi", c"8 bit raw midi", 1);
        let outputs = [
            receiver.port(c"left", c"32 bit float mono audio", 2),
            receiver.port(c"right", c"32 bit float mono audio", 2),
        ];
        let source_port = sender.port(c"notes", c"8 bit raw midi", 2);
        let progress = Arc::new(Progress::default());
        let (producer, records) = rtrb::RingBuffer::new(16384);
        let mut events = EventBuffer::new();
        events.push(&MidiEvent::new(NOTE_OFFSET, 0, [0x90, 36, 127]));
        events.clear(); // reserve outside the callbacks
        let audit = calibrated_callback_audit();
        let allocate = unsafe {
            *audit
                .1
                .get::<unsafe extern "C" fn() -> *mut u8>(b"probe_allocate\0")
                .unwrap()
        };
        let free = unsafe {
            *audit
                .1
                .get::<unsafe extern "C" fn(*mut u8)>(b"probe_deallocate\0")
                .unwrap()
        };
        let origin = Instant::now();
        let mut host = Self {
            first: copied_plugin(),
            second: copied_plugin(),
            receiver,
            sender,
            processor: Box::new(Processor {
                api,
                midi,
                outputs,
                progress: progress.clone(),
                stopped: [None, None],
                audio: [None, None],
                input: AudioPorts::with_capacity(0, 0),
                output: AudioPorts::with_capacity(2, 1),
                events,
                pcm: vec![0.0; MAX_FRAMES * 4],
                records: producer,
                origin,
                audit,
                allocate,
                free,
                calibrated: false,
                frames: 0,
            }),
            source: Box::new(Source {
                api,
                port: source_port,
                progress: progress.clone(),
                rate,
                frames: 0,
                next: 0,
                key: 36,
            }),
            progress,
            records,
            observations: Vec::new(),
            rate,
            origin,
            _library: library,
        };
        for (index, plugin) in [&mut host.first, &mut host.second].into_iter().enumerate() {
            host.processor.stopped[index] = Some(
                plugin
                    .activate(
                        |_, _| (),
                        PluginAudioConfiguration {
                            sample_rate: f64::from(rate),
                            min_frames_count: 1,
                            max_frames_count: MAX_FRAMES as u32,
                        },
                    )
                    .unwrap(),
            );
        }
        unsafe {
            assert_eq!(
                (api.callback)(
                    host.receiver.raw,
                    process_callback,
                    (&mut *host.processor as *mut Processor).cast()
                ),
                0
            );
            assert_eq!(
                (api.callback)(
                    host.sender.raw,
                    source_callback,
                    (&mut *host.source as *mut Source).cast()
                ),
                0
            );
            host.receiver.activate();
            host.sender.activate();
            assert_eq!(
                (api.connect)(host.sender.raw, (api.name)(source_port), (api.name)(midi)),
                0
            );
            eprintln!(
                "REAL_SERVER rate={rate} quantum={quantum} MIDI={} -> {} allocation_audited=true",
                CStr::from_ptr((api.name)(source_port)).to_string_lossy(),
                CStr::from_ptr((api.name)(midi)).to_string_lossy()
            );
        }
        host
    }
    fn collect(&mut self) {
        while let Ok(record) = self.records.pop() {
            self.observations.push(record);
        }
        assert_eq!(
            self.progress.failed.load(Ordering::Acquire),
            0,
            "native callback ownership/MIDI/process failure code"
        );
    }
    fn prove(&mut self, phase: usize, selected: bool, level: f32) {
        self.progress.phase.store(phase, Ordering::Release);
        let begin = self.progress.callbacks.load(Ordering::Acquire);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            dispatch_context(&mut self.first);
            dispatch_context(&mut self.second);
            self.collect();
            if [36, 48].iter().all(|key| {
                self.observations
                    .iter()
                    .any(|r| r.phase == phase && r.key == *key)
            }) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "actual server audio/MIDI progress stalled in phase {phase}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        for record in self
            .observations
            .iter()
            .filter(|r| r.phase == phase && r.key != 0)
        {
            self.assert_audio(record, selected, level);
        }
        eprintln!(
            "PHASE {phase}: selected={selected} level={level}, callbacks={}",
            self.progress.callbacks.load(Ordering::Acquire) - begin
        );
    }
    fn assert_continuity(&self) {
        // A bounded operational watchdog, not a hard-realtime deadline promise.
        for adjacent in self.observations.windows(2) {
            assert!(
                adjacent[1].end_us - adjacent[0].end_us < 500_000,
                "native callbacks stalled across phases {} -> {}",
                adjacent[0].phase,
                adjacent[1].phase
            );
        }
        for r in &self.observations {
            assert!(
                r.end_us - r.start_us < 500_000,
                "native callback exceeded watchdog"
            );
            assert!(
                r.pcm.iter().flatten().flatten().all(|v| v.is_finite()),
                "nonfinite native output"
            );
            if r.key == 0 {
                continue;
            }
            // Every onset is checked, including transitions, not just settled tail records.
            let (allow_selected, allow_empty, gains): (bool, bool, &[f32]) = match r.phase {
                0 => (false, true, &[1.0]),
                1 => (true, true, &[1.0]),       // import
                2 => (true, false, &[1.0, 0.5]), // public edit
                3 | 4 => (true, false, &[0.5]),  // destruction, absent editor
                5 | 6 => (true, true, &[0.5]),   // native Undo/Redo
                7 => (true, false, &[0.5]),
                _ => panic!("unknown acceptance phase"),
            };
            let selected = r.key == 48 && r.pcm[0][0][0] != 0.0;
            if r.key == 48 {
                assert!(
                    if selected {
                        allow_selected
                    } else {
                        allow_empty
                    },
                    "unexpected selected transition in phase {}",
                    r.phase
                );
            }
            let gain = gains
                .iter()
                .copied()
                .find(|gain| {
                    let expected = if r.key == 36 {
                        -0.5 * gain
                    } else if selected {
                        -0.125 * gain
                    } else {
                        0.0
                    };
                    (r.pcm[0][0][0] - expected).abs() < 0.000001
                })
                .expect("onset must match an allowed old/new accepted state");
            self.assert_audio(r, selected, gain);
        }
    }
    fn assert_audio(&self, r: &Observation, selected: bool, level: f32) {
        // Independently decoded literal PCM16 stereo means, linearly resampled.
        let source = [-0.125_f64, 0.0, -0.1875, -0.125];
        for channel in 0..2 {
            let expected = if r.key == 36 { -0.5 } else { 0.0 };
            assert_eq!(
                r.pcm[1][channel][0], expected,
                "independent instance at every MIDI onset"
            );
            assert_eq!(r.pcm[1][channel][3], 0.0, "independent leading silence");
            assert_eq!(r.pcm[0][channel][3], 0.0, "first instance leading silence");
            if r.key == 36 {
                assert_eq!(
                    r.pcm[0][channel][0],
                    -0.5 * level,
                    "phase {}: kit onset",
                    r.phase
                );
            } else {
                for frame in 0..3 {
                    let position = frame as f64 * 32000.0 / f64::from(self.rate);
                    let index = position.floor() as usize;
                    let expected = if selected {
                        (source[index] + (source[index + 1] - source[index]) * position.fract())
                            as f32
                            * level
                    } else {
                        0.0
                    };
                    assert!(
                        (r.pcm[0][channel][frame] - expected).abs() < 0.000001,
                        "phase {}: selected signed frame {frame}: {:?}, expected {expected}",
                        r.phase,
                        r.pcm[0][channel]
                    );
                }
            }
        }
    }
}
impl NativeHost {
    fn shutdown(&mut self) {
        if self.progress.stop.load(Ordering::Acquire)
            && !self.receiver.active
            && !self.sender.active
        {
            return;
        }
        // The host parent outlives this guard, including failed assertions.
        for plugin in [&mut self.first, &mut self.second] {
            let handle = plugin.plugin_handle();
            if let Some(gui) = handle.get_extension::<PluginGui>() {
                gui.destroy(&handle);
            }
        }
        self.progress.stop.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.receiver.active
            && !self.progress.stopped.load(Ordering::Acquire)
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        self.receiver.quiesce();
        self.sender.quiesce();
        // Server callbacks have stopped. Return each endpoint through CLAP's ownership API.
        for (i, plugin) in [&mut self.first, &mut self.second].into_iter().enumerate() {
            if let Some(audio) = self.processor.audio[i].take() {
                self.processor.stopped[i] = Some(
                    std::thread::spawn(move || audio.stop_processing())
                        .join()
                        .unwrap(),
                );
            }
            if let Some(stopped) = self.processor.stopped[i].take() {
                plugin.deactivate(stopped);
            }
        }
        eprintln!(
            "NATIVE_CLEANUP callbacks={} received_midi={} failure={}",
            self.progress.callbacks.load(Ordering::Acquire),
            self.progress.received.load(Ordering::Acquire),
            self.progress.failed.load(Ordering::Acquire)
        );
    }
}
impl Drop for NativeHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn xdo(args: &[&str]) {
    let mut child = std::process::Command::new("timeout")
        .args(["5", "xdotool"])
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // This host owns GTK's main loop: keep native key press/release and focus
    // delivery live while the external XTest sender runs.
    while child.try_wait().unwrap().is_none() {
        let context = gtk::glib::MainContext::default();
        for _ in 0..32 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "native input {args:?} exited {}: {}",
        result.status,
        String::from_utf8_lossy(&result.stderr)
    );
}
// Always release native input even if an observation/assertion fails mid-drag.
struct HeldPointer(bool);
impl HeldPointer {
    fn press() -> Self {
        xdo(&["mousedown", "1"]);
        Self(true)
    }
    fn release(mut self) {
        xdo(&["mouseup", "1"]);
        self.0 = false;
    }
}
impl Drop for HeldPointer {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        let _ = std::process::Command::new("timeout")
            .args(["5", "xdotool", "mouseup", "1"])
            .status();
    }
}
fn click(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
    parent: &Parent,
    expression: &str,
) {
    wait_dom(plugin, browser, expression);
    // DOM is observation only; input crosses XTest -> X11 -> GTK -> WebKit -> React.
    let coordinates = javascript(
        plugin,
        browser,
        &format!(
            "(()=>{{const r=({expression}).getBoundingClientRect();return JSON.stringify([r.x+r.width/2,r.y+r.height/2]);}})()"
        ),
    );
    let xy: [f64; 2] = serde_json::from_str(&coordinates).unwrap();
    let child = parent.children()[0];
    xdo(&[
        "windowactivate",
        &parent.id.to_string(),
        "windowfocus",
        &child.to_string(),
        "mousemove",
        "--window",
        &child.to_string(),
        &(xy[0] as i32).to_string(),
        &(xy[1] as i32).to_string(),
    ]);
    wait_dom(
        plugin,
        browser,
        &format!("({expression}).matches(':hover')"),
    );
    xdo(&["click", "1"]);

    dispatch_context(plugin);
}
fn snapshot_until(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = structural_native::snapshot(plugin, browser);
        if predicate(&state) {
            return state;
        }
        assert!(
            Instant::now() < deadline,
            "native input did not publish expected state: connection={}, selections={}",
            state["connection"],
            state["sampleSelections"]
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "installed native acceptance: existing JACK/PipeWire and X11/XTest, no DAW or physical device claim"]
fn installed_native_edits_overlap_server_audio_and_midi() {
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    assert!(
        !artifact().canonicalize().unwrap().starts_with(checkout),
        "acceptance requires an installed artifact outside the source checkout"
    );
    run_allocator_audit_child("installed_native::installed_native_overlap_audit_child");
}
#[test]
#[ignore = "only the calibrated installed-native audit parent launches this child"]
fn installed_native_overlap_audit_child() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut host = NativeHost::new();
        let mut browser = structural_native::open_view(&mut host.first, &parent);
        let independent = saved_state(&mut host.second);
        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("overlap-selected.wav");
        std::fs::write(&path, selection_native::SELECTED_WAV).unwrap();
        host.prove(1, false, 1.0);
        let import_start = host.origin.elapsed().as_micros() as u64;
        if javascript(&mut host.first, &browser, "String(Boolean(document.querySelector('button[aria-label=\"Add source…\"]')))") != "true" {
            click(&mut host.first, &browser, &parent, "document.querySelector('button[aria-label=\"Instrument tree\"]')");
        }
        click(&mut host.first, &browser, &parent, "document.querySelector('button[aria-label=\"Add source…\"]')");
        gtk::init().unwrap();
        let chooser = selection_native::native_dialog(&mut host.first);
        // Real chooser window; native typing selects the path instead of GTK's set_filename.
        use gtk::glib::translate::ToGlibPtr;
        unsafe extern "C" { fn gdk_x11_window_get_xid(window: *mut gtk::gdk::ffi::GdkWindow) -> c_ulong; }
        let window = chooser.window().unwrap();
        let xid = unsafe { gdk_x11_window_get_xid(window.to_glib_none().0) };
        let mapped_deadline = Instant::now() + Duration::from_secs(5);
        while parent.x.get_window_attributes(xid as u32).unwrap().reply().unwrap().map_state
            != x11rb::protocol::xproto::MapState::VIEWABLE {
            dispatch_context(&mut host.first);
            assert!(Instant::now() < mapped_deadline, "native chooser becomes viewable before input");
            std::thread::sleep(Duration::from_millis(5));
        }
        xdo(&["windowactivate", &xid.to_string()]);
        while !chooser.has_toplevel_focus() {
            dispatch_context(&mut host.first);
            assert!(Instant::now() < mapped_deadline, "native chooser receives window-manager focus");
            std::thread::sleep(Duration::from_millis(5));
        }
        xdo(&["key", "--clearmodifiers", "ctrl+l"]);
        let entry = loop {
            dispatch_context(&mut host.first);
            if let Some(entry) = chooser.focused_widget().and_then(|widget| widget.downcast::<gtk::Entry>().ok())
                && entry.is_realized() && entry.is_mapped() { break entry; }
            assert!(Instant::now() < mapped_deadline, "native Ctrl+L realizes the chooser location entry");
            std::thread::sleep(Duration::from_millis(5));
        };
        xdo(&["key", "--clearmodifiers", "ctrl+a"]);
        xdo(&["type", "--clearmodifiers", "--delay", "1", path.to_str().unwrap()]);
        while entry.text().as_str() != path.to_str().unwrap() {
            dispatch_context(&mut host.first);
            assert!(Instant::now() < mapped_deadline, "native filename typing: {:?}", entry.text());
            std::thread::sleep(Duration::from_millis(5));
        }
        let accept = chooser.widget_for_response(gtk::ResponseType::Accept).expect("native Open button");
        let (button_x, button_y) = accept.translate_coordinates(&chooser, 0, 0).unwrap();
        let bounds = accept.allocation();
        let open_deadline = Instant::now() + Duration::from_secs(5);
        while !accept.is_mapped() || !accept.is_sensitive() {
            dispatch_context(&mut host.first);
            assert!(Instant::now() < open_deadline,
                "native Open button: mapped={} sensitive={} entry_type={} text={:?} files={:?}",
                accept.is_mapped(), accept.is_sensitive(), entry.type_().name(), entry.text(), chooser.filenames());
            std::thread::sleep(Duration::from_millis(5));
        }
        xdo(&["mousemove", "--window", &xid.to_string(),
            &(button_x + bounds.width()/2).to_string(), &(button_y + bounds.height()/2).to_string(), "click", "1"]);
        drop(accept); drop(entry);

        drop(chooser);
        let imported = snapshot_until(&mut host.first, &browser,
            |s| s["yaml"].as_str().unwrap().contains(selection_native::CONTENT_SHA256));
        let import_end = host.origin.elapsed().as_micros() as u64;
        assert_eq!(imported["connection"]["history"], serde_json::json!({"undoCount":1,"redoCount":0}));
        selected.close().unwrap();
        host.prove(2, true, 1.0);
        let edit_start = host.origin.elapsed().as_micros() as u64;
        click(&mut host.first, &browser, &parent,
            "Array.from(document.querySelectorAll('[role=tab]')).find(t=>t.textContent.trim()==='Voice')");
        click(&mut host.first, &browser, &parent,
            "document.querySelector('[data-knob=\"drums.level\"] [role=slider]')");
        let held = HeldPointer::press();
        // Round-trip observation lets the browser consume native pointer-down.
        eprintln!("NATIVE_DRAG start={}", javascript(&mut host.first, &browser,
            "JSON.stringify({value:document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow'),activeRole:document.activeElement?.getAttribute('role')})"));
        xdo(&["mousemove_relative", "--", "0", "25"]);
        dispatch_context(&mut host.first);
        held.release();
        host.prove(3, true, 0.5);
        let edit_end = host.origin.elapsed().as_micros() as u64;
        let accepted = structural_native::snapshot(&mut host.first, &browser);
        let gui = host.first.plugin_handle().get_extension::<PluginGui>().unwrap();
        let weak = weak_browser_in_child(parent.children()[0]);
        let destroy_start = host.origin.elapsed().as_micros() as u64;
        gui.destroy(&host.first.plugin_handle()); drop(browser);
        assert!(parent.children().is_empty()); assert!(weak.upgrade().is_none());
        host.prove(4, true, 0.5);
        browser = structural_native::open_view(&mut host.first, &parent);
        let recreate_end = host.origin.elapsed().as_micros() as u64;
        let recreated = structural_native::snapshot(&mut host.first, &browser);
        assert_ne!(recreated["connection"]["attachmentId"], accepted["connection"]["attachmentId"]);
        assert_eq!(recreated["yaml"], accepted["yaml"]);
        host.progress.reset.store(true, Ordering::Release);
        host.prove(5, true, 0.5);
        assert_eq!(host.progress.resets.load(Ordering::Acquire), 2);
        // Structural history survives the view; host-owned public values stay at 0.5.
        for (phase, action, selected, level) in [(6,"Undo",false,0.5), (7,"Redo",true,0.5)] {
            let prior = structural_native::snapshot(&mut host.first, &browser)["connection"]["revision"].clone();
            click(&mut host.first, &browser, &parent,
                &format!("document.querySelector('header button[aria-label^=\"{action} \"]:not(:disabled)')"));
            snapshot_until(&mut host.first, &browser, |s| s["connection"]["revision"] != prior);
            host.prove(phase, selected, level);
        }
        assert_eq!(saved_state(&mut host.second), independent);
        host.collect();
        host.assert_continuity();
        for (name, start, end) in [("native chooser/import",import_start,import_end),
            ("native public edit",edit_start,edit_end),
            ("destroy/absent/recreate",destroy_start,recreate_end)] {
            let count = host.observations.iter().filter(|r| r.start_us >= start && r.end_us <= end).count();
            assert!(count > 0, "real audio/MIDI callbacks during {name}");
            let midi = host.observations.iter().filter(|r| r.start_us >= start && r.end_us <= end && r.key != 0).count();
            eprintln!("OVERLAP {name}: [{start},{end}]us callbacks={count} MIDI={midi}");
        }
        gui.destroy(&host.first.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        host.shutdown();
        host.collect();
        host.assert_continuity();
        let events: Vec<_> = host.observations.iter()
            .flat_map(|r| r.events[..r.len].iter().flatten().copied()).collect();
        let mut active = false;
        let mut begins = 0;
        let mut ends = 0;
        for event in &events {
            match event {
                Recorded::Begin(3) => { assert!(!active, "duplicate Begin: {events:?}"); active = true; begins += 1; }
                Recorded::Value(3, _) => assert!(active, "value outside gesture: {events:?}"),
                Recorded::End(3) => { assert!(active, "unpaired End: {events:?}"); active = false; ends += 1; }
                _ => panic!("unexpected native automation: {events:?}"),
            }
        }
        assert!(begins > 0);
        assert!(!active, "held gesture after native teardown");
        assert_eq!(begins, ends);
        assert!(events.contains(&Recorded::Value(3, 0.5)));
        eprintln!("NATIVE_ACCEPTANCE callbacks={} MIDI={} gestures={begins}/{ends}",
            host.progress.callbacks.load(Ordering::Acquire), host.progress.received.load(Ordering::Acquire));
    }).join().expect("installed native server/input acceptance completed");
}
