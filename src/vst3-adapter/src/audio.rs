//! Unique prebuilt audio endpoint. Audio never borrows the main-thread owner.
use dandrum_trigger_host::{
    TriggerHostConfiguration,
    runtime::{ControlEvent, MAX_BLOCK_FRAMES, MAX_EVENTS, RuntimeAudio, TimedEvent},
};
use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use vst3::{
    ComRef,
    Steinberg::{Vst::*, *},
};
const EMPTY: TimedEvent = TimedEvent {
    frame_offset: 0,
    event: ControlEvent::NoteOff { note: 0 },
};
pub(crate) struct Endpoint {
    pub runtime: RuntimeAudio,
    pub configuration: TriggerHostConfiguration,
    pub mode: i32,
    pub audio_enabled: bool,
    pub events_enabled: bool,
    events: Box<[TimedEvent; MAX_EVENTS]>,
}
impl Endpoint {
    pub fn new(runtime: RuntimeAudio, configuration: TriggerHostConfiguration) -> Self {
        Self {
            runtime,
            configuration,
            mode: 0,
            audio_enabled: true,
            events_enabled: true,
            events: Box::new([EMPTY; MAX_EVENTS]),
        }
    }
}
pub(crate) struct Slot {
    value: UnsafeCell<Option<Endpoint>>,
    borrowed: AtomicBool,
    pub live: AtomicBool,
    pub active: AtomicBool,
    pub processing: AtomicBool,
    pub configured: AtomicBool,
    pub max_frames: AtomicU32,
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            value: UnsafeCell::new(None),
            borrowed: AtomicBool::new(false),
            live: AtomicBool::new(false),
            active: AtomicBool::new(false),
            processing: AtomicBool::new(false),
            configured: AtomicBool::new(false),
            max_frames: AtomicU32::new(16),
        }
    }
}
struct Borrow<'a>(&'a AtomicBool);
impl Drop for Borrow<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Slot {
    fn acquire(&self) -> Option<Borrow<'_>> {
        self.borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Borrow(&self.borrowed))
    }
    pub fn with<R>(&self, f: impl FnOnce(&mut Endpoint) -> R) -> Option<R> {
        let _guard = self.acquire()?;
        unsafe { (&mut *self.value.get()).as_mut().map(f) }
    }
    pub fn install(&self, endpoint: Endpoint) {
        let _guard = self
            .acquire()
            .expect("initial endpoint has no callback owner");
        unsafe {
            *self.value.get() = Some(endpoint);
        }
    }
    pub fn clear_main(&self) {
        self.live.store(false, Ordering::Release);
        self.active.store(false, Ordering::Release);
        self.processing.store(false, Ordering::Release);
        loop {
            if let Some(guard) = self.acquire() {
                let old = unsafe { (&mut *self.value.get()).take() };
                drop(guard);
                drop(old);
                break;
            }
            std::thread::yield_now();
        }
    }
    pub fn set_processing(&self, state: TBool) -> tresult {
        if state > 1 {
            return kInvalidArgument;
        }
        if !self.live.load(Ordering::Acquire) {
            return kNotInitialized;
        }
        if state != 0
            && (!self.active.load(Ordering::Acquire) || !self.configured.load(Ordering::Acquire))
        {
            return kNotInitialized;
        }
        self.with(|e| {
            if state == 0 {
                e.runtime.reset();
            }
            self.processing.store(state != 0, Ordering::Release);
            kResultOk
        })
        .unwrap_or(kResultFalse)
    }
    pub unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        if data.is_null() {
            return kInvalidArgument;
        }
        let data = unsafe { &mut *data };
        if !self.live.load(Ordering::Acquire) {
            unsafe { clear(data) };
            return kNotInitialized;
        }
        let result = self
            .with(|endpoint| unsafe {
                endpoint.process(
                    data,
                    self.active.load(Ordering::Acquire),
                    self.processing.load(Ordering::Acquire),
                    self.configured.load(Ordering::Acquire),
                )
            })
            .unwrap_or(kResultFalse);
        if result != kResultOk {
            unsafe { clear(data) };
        }
        result
    }
}
impl Endpoint {
    unsafe fn process(
        &mut self,
        data: &mut ProcessData,
        active: bool,
        processing: bool,
        configured: bool,
    ) -> tresult {
        if data.symbolicSampleSize != 0
            || !(0..=2).contains(&data.processMode)
            || data.processMode != self.mode
            || data.numSamples < 0
            || data.numSamples as u32 > self.configuration.max_block_frames
            || data.numInputs != 0
            || data.numOutputs < 0
            || data.numOutputs > 1
        {
            return kInvalidArgument;
        }
        if let Some(parameters) = unsafe { ComRef::from_raw(data.inputParameterChanges) }
            && unsafe { parameters.getParameterCount() } != 0
        {
            return kNotImplemented;
        }
        let no_audio = data.numSamples == 0 || data.numOutputs == 0 || !self.audio_enabled;
        if !no_audio && (!active || !processing || !configured) {
            return kNotInitialized;
        }
        let frames = data.numSamples as usize;
        let mut count = 0;
        let mut previous = 0;
        if let Some(events) = unsafe { ComRef::from_raw(data.inputEvents) } {
            let amount = unsafe { events.getEventCount() };
            if amount < 0 || amount as usize > MAX_EVENTS {
                return kInvalidArgument;
            }
            for i in 0..amount {
                let mut event: Event = unsafe { std::mem::zeroed() };
                if unsafe { events.getEvent(i, &mut event) } != kResultOk {
                    return kInvalidArgument;
                }
                let value = match u32::from(event.r#type) {
                    Event_::EventTypes_::kNoteOnEvent => {
                        let n = unsafe { event.__field0.noteOn };
                        if !(0..16).contains(&n.channel)
                            || !(0..128).contains(&n.pitch)
                            || !(0.0..=1.0).contains(&n.velocity)
                            || n.tuning != 0.0
                        {
                            return kInvalidArgument;
                        }
                        let velocity = (n.velocity * 127.0).round() as u8;
                        if velocity == 0 {
                            ControlEvent::NoteOff {
                                note: n.pitch as u8,
                            }
                        } else {
                            ControlEvent::NoteOn {
                                note: n.pitch as u8,
                                velocity,
                            }
                        }
                    }
                    Event_::EventTypes_::kNoteOffEvent => {
                        let n = unsafe { event.__field0.noteOff };
                        if !(0..16).contains(&n.channel)
                            || !(0..128).contains(&n.pitch)
                            || !(0.0..=1.0).contains(&n.velocity)
                            || n.tuning != 0.0
                        {
                            return kInvalidArgument;
                        }
                        ControlEvent::NoteOff {
                            note: n.pitch as u8,
                        }
                    }
                    _ => continue, // Unsupported expression/data/pressure events have no MIDI1 effect.
                };
                if event.busIndex != 0
                    || event.sampleOffset < 0
                    || event.sampleOffset < previous
                    || if frames == 0 {
                        event.sampleOffset != 0
                    } else {
                        event.sampleOffset as usize >= frames
                    }
                {
                    return kInvalidArgument;
                }
                previous = event.sampleOffset;
                if !no_audio && self.events_enabled {
                    self.events[count] = TimedEvent {
                        frame_offset: event.sampleOffset as u32,
                        event: value,
                    };
                    count += 1;
                }
            }
        }
        if no_audio {
            unsafe { clear(data) };
            return if self.runtime.flush_parameters(&[]).is_ok() {
                kResultOk
            } else {
                kResultFalse
            };
        }
        if data.outputs.is_null() {
            return kInvalidArgument;
        }
        let bus = unsafe { &mut *data.outputs };
        if bus.numChannels != 2 {
            return kInvalidArgument;
        }
        let channels = unsafe { bus.__field0.channelBuffers32 };
        if channels.is_null() {
            return kInvalidArgument;
        }
        let left = unsafe { *channels };
        let right = unsafe { *channels.add(1) };
        let length = frames * std::mem::size_of::<f32>();
        if left.is_null() || right.is_null() || !left.is_aligned() || !right.is_aligned() {
            return kInvalidArgument;
        }
        let (Some(lend), Some(rend)) = (
            (left as usize).checked_add(length),
            (right as usize).checked_add(length),
        ) else {
            return kInvalidArgument;
        };
        if (left as usize) < rend && (right as usize) < lend {
            return kInvalidArgument;
        }
        // Distinct writable planar extents are required by the host contract;
        // overlap was rejected before constructing simultaneous Rust borrows.
        let left = unsafe { std::slice::from_raw_parts_mut(left, frames) };
        let right = unsafe { std::slice::from_raw_parts_mut(right, frames) };
        if self
            .runtime
            .process_events(left, right, &self.events[..count])
            .is_err()
        {
            return kResultFalse;
        }
        bus.silenceFlags = u64::from(left.iter().all(|x| *x == 0.0))
            | (u64::from(right.iter().all(|x| *x == 0.0)) << 1);
        kResultOk
    }
}
/// Touch only the declared plugin bus/channels and bounded frame extent. The
/// foreign ABI must supply valid storage for each non-null declared extent.
unsafe fn clear(data: &mut ProcessData) {
    if data.numOutputs <= 0 || data.outputs.is_null() {
        return;
    }
    let frames = (data.numSamples.max(0) as usize).min(MAX_BLOCK_FRAMES as usize);
    let bus = unsafe { &mut *data.outputs };
    let channels = bus.numChannels.clamp(0, 2) as usize;
    match data.symbolicSampleSize {
        0 => {
            let pointers = unsafe { bus.__field0.channelBuffers32 };
            if !pointers.is_null() {
                for i in 0..channels {
                    let p = unsafe { *pointers.add(i) };
                    if !p.is_null() && p.is_aligned() {
                        unsafe { std::slice::from_raw_parts_mut(p, frames) }.fill(0.0);
                    }
                }
            }
        }
        1 => {
            let pointers = unsafe { bus.__field0.channelBuffers64 };
            if !pointers.is_null() {
                for i in 0..channels {
                    let p = unsafe { *pointers.add(i) };
                    if !p.is_null() && p.is_aligned() {
                        unsafe { std::slice::from_raw_parts_mut(p, frames) }.fill(0.0);
                    }
                }
            }
        }
        _ => {}
    }
    bus.silenceFlags = (1u64 << channels) - 1;
}
