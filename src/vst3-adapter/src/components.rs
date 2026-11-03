//! Separate COM components: one processor sound owner and a metadata projection.
use crate::{
    CONTROLLER_CLASS_ID, PROCESSOR_CLASS_ID, parameters, platform,
    transport::{Body, Transport},
    utf16,
};
use dandrum_trigger_host::{
    SampleRate, TriggerHostConfiguration,
    parameters::{
        ParameterController, PublicParameter, SessionServiceWake, public_parameters_from_resources,
    },
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ffi::c_char,
    num::NonZeroU64,
    ops::Deref,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Linux::*, Vst::*, *},
};

pub(crate) fn create(
    cid: &TUID,
    runloop: Option<Rc<ComPtr<IRunLoop>>>,
) -> Result<ComPtr<FUnknown>, tresult> {
    if *cid != PROCESSOR_CLASS_ID && *cid != CONTROLLER_CLASS_ID {
        return Err(kNoInterface);
    }
    let root = platform::resource_root().ok_or(kResultFalse)?;
    if *cid == PROCESSOR_CLASS_ID {
        Ok(ComWrapper::new(Processor(Rc::new(ProcessorCore {
            root,
            thread: platform::thread_id(),
            terminated: Cell::new(false),
            owner: RefCell::new(None),
            audio: crate::audio::Slot::default(),
            initialized: Cell::new(false),
            transport: Transport::default(),
            attachment: RefCell::new(None),
            runloop: RefCell::new(runloop),
            registration: RefCell::new(None),
            wake: Arc::new(WorkerWake::default()),
            servicing: Cell::new(false),
            last_request: Cell::new(0),
            processed: RefCell::new(VecDeque::with_capacity(crate::transport::PENDING)),
        })))
        .to_com_ptr::<FUnknown>()
        .expect("component includes FUnknown"))
    } else {
        Ok(ComWrapper::new(Controller {
            root,
            thread: platform::thread_id(),
            terminated: Cell::new(false),
            projection: RefCell::new(None),
            transport: Transport::default(),
            bootstrap: RefCell::new(None),
            next_sequence: Cell::new(1),
            awaiting: Cell::new(None),
            last_reply: RefCell::new(None),
        })
        .to_com_ptr::<FUnknown>()
        .expect("controller includes FUnknown"))
    }
}
struct Processor(Rc<ProcessorCore>);
impl Deref for Processor {
    type Target = ProcessorCore;
    fn deref(&self) -> &ProcessorCore {
        &self.0
    }
}
struct ProcessorCore {
    initialized: Cell<bool>,
    root: PathBuf,
    thread: libc::pid_t,
    terminated: Cell<bool>,
    // Main-thread session/history authority stays separate from the unique
    // prebuilt audio endpoint; callbacks never borrow the session controller.
    owner: RefCell<Option<ParameterController>>,
    audio: crate::audio::Slot,
    transport: Transport,
    attachment: RefCell<Option<String>>,
    runloop: RefCell<Option<Rc<ComPtr<IRunLoop>>>>,
    registration: RefCell<Option<Registration>>,
    wake: Arc<WorkerWake>,
    servicing: Cell<bool>,
    last_request: Cell<u64>,
    processed: RefCell<VecDeque<Processed>>,
}
impl Class for Processor {
    type Interfaces = (IComponent, IAudioProcessor, IConnectionPoint);
}
impl ProcessorCore {
    fn main(&self) -> bool {
        self.thread == platform::thread_id() && !self.terminated.get()
    }
    fn live(&self) -> bool {
        self.main() && self.initialized.get() && self.owner.borrow().is_some()
    }
}
impl IPluginBaseTrait for Processor {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        if !self.main() || self.terminated.get() || self.owner.borrow().is_some() {
            return kResultFalse;
        }
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let Ok(id) = NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        else {
            return kResultFalse;
        };
        let configuration = TriggerHostConfiguration {
            sample_rate_hz: SampleRate::new(48_000.0).expect("fixed supported clock"),
            max_block_frames: 16,
        };
        let Ok((mut owner, audio)) = ParameterController::open_with_reserved_events(
            &self.root,
            configuration,
            NonZeroU64::new(id).expect("counter starts nonzero"),
            1024,
            1024,
        ) else {
            return kResultFalse;
        };
        let Ok(parameters) = parameters::ordered(owner.parameters()) else {
            return kResultFalse;
        };
        let runloop = { self.runloop.borrow().clone() };
        if runloop.is_some() && owner.start_session_worker(self.wake.clone()).is_err() {
            return kResultFalse;
        }
        self.audio.install(crate::audio::Endpoint::new(
            audio,
            configuration,
            std::array::from_fn(|i| parameters[i].root_index),
        ));
        *self.owner.borrow_mut() = Some(owner);
        if let Some(runloop) = runloop {
            let timer = ComWrapper::new(Timer(self.thread, Rc::downgrade(&self.0)))
                .to_com_ptr::<ITimerHandler>()
                .expect("standard timer");
            let result = unsafe { runloop.registerTimer(timer.as_ptr(), 10) };
            if result != kResultOk {
                let owner = self.owner.borrow_mut().take();
                drop(owner);
                self.audio.clear_main();
                return kResultFalse;
            }
            let registration = Registration { runloop, timer };
            if self.terminated.get() {
                if let Err(registration) = registration.unregister() {
                    *self.registration.borrow_mut() = Some(registration);
                }
                return kResultFalse;
            }
            *self.registration.borrow_mut() = Some(registration);
        }
        self.initialized.set(true);
        self.audio.live.store(true, Ordering::Release);
        kResultOk
    }
    unsafe fn terminate(&self) -> tresult {
        // A stopped/zero-sample callback may be inside a foreign event/point
        // getter. Reject reentrant teardown before acquiring/destroying audio.
        if self.audio.processing.load(Ordering::Acquire) || self.audio.in_use() {
            return kResultFalse;
        }
        if self.thread != platform::thread_id() {
            return kResultFalse;
        }
        if self.shutdown() {
            kResultOk
        } else {
            kResultFalse
        }
    }
}
impl IComponentTrait for Processor {
    unsafe fn getControllerClassId(&self, id: *mut TUID) -> tresult {
        if id.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one writable16-byte SDK identity output.
        unsafe {
            *id = CONTROLLER_CLASS_ID;
        }
        kResultOk
    }
    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        if self.main() && (0..=2).contains(&mode) {
            kResultOk
        } else {
            kInvalidArgument
        }
    }
    unsafe fn getBusCount(&self, media: MediaType, direction: BusDirection) -> i32 {
        if self.live()
            && ((media == MediaTypes_::kAudio as i32
                && direction == BusDirections_::kOutput as i32)
                || (media == MediaTypes_::kEvent as i32
                    && direction == BusDirections_::kInput as i32))
        {
            1
        } else {
            0
        }
    }
    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        direction: BusDirection,
        index: i32,
        info: *mut BusInfo,
    ) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if info.is_null() || index != 0 {
            return kInvalidArgument;
        }
        let (name, channels) = if media == MediaTypes_::kAudio as i32
            && direction == BusDirections_::kOutput as i32
        {
            ("Stereo Output", 2)
        } else if media == MediaTypes_::kEvent as i32 && direction == BusDirections_::kInput as i32
        {
            ("MIDI Input", 16)
        } else {
            return kInvalidArgument;
        };
        // SAFETY: one exclusive writable BusInfo supplied by the host.
        let info = unsafe { &mut *info };
        info.mediaType = media;
        info.direction = direction;
        info.channelCount = channels;
        utf16(&mut info.name, name);
        info.busType = BusTypes_::kMain as i32;
        info.flags = BusInfo_::BusFlags_::kDefaultActive;
        kResultOk
    }
    unsafe fn getRoutingInfo(
        &self,
        _input: *mut RoutingInfo,
        _output: *mut RoutingInfo,
    ) -> tresult {
        kNotImplemented
    }
    unsafe fn activateBus(
        &self,
        media: MediaType,
        direction: BusDirection,
        index: i32,
        active: TBool,
    ) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if index != 0 || active > 1 {
            return kInvalidArgument;
        }
        self.audio
            .with(|audio| {
                if media == 0 && direction == 1 {
                    audio.audio_enabled = active != 0;
                } else if media == 1 && direction == 0 {
                    audio.events_enabled = active != 0;
                } else {
                    return kInvalidArgument;
                }
                kResultOk
            })
            .unwrap_or(kResultFalse)
    }
    unsafe fn setActive(&self, active: TBool) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if active > 1 {
            return kInvalidArgument;
        }
        if self.audio.processing.load(Ordering::Acquire) {
            return kResultFalse;
        }
        self.audio.active.store(active != 0, Ordering::Release);
        kResultOk
    }
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        let Some(mut input) = (unsafe { crate::stream::Stream::from_raw(state) }) else {
            return kInvalidArgument;
        };
        let Ok(header) = dandrum_trigger_host::state::read_header(&mut input) else {
            return kResultFalse;
        };
        if !self.live() {
            return kNotInitialized;
        }
        {
            let owner = self.owner.borrow();
            if owner
                .as_ref()
                .expect("live owner")
                .preflight_state(&header)
                .is_err()
            {
                return kResultFalse;
            }
        }
        // No owner/endpoint borrow crosses any foreign read. The shared owner
        // revalidates admission after reentrant host code has finished.
        let Ok(buffered) = header.buffer_payload(&mut input) else {
            return kResultFalse;
        };
        if !self.live() {
            return kNotInitialized;
        }
        {
            let mut owner = self.owner.borrow_mut();
            if owner
                .as_mut()
                .expect("live owner")
                .load_buffered_state(buffered)
                .is_err()
            {
                return kResultFalse;
            }
        }
        self.service();
        kResultOk
    }
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        use std::io::Write;
        if !self.live() {
            return kNotInitialized;
        }
        let Some(mut output) = (unsafe { crate::stream::Stream::from_raw(state) }) else {
            return kInvalidArgument;
        };
        let mut bytes = Vec::new();
        {
            let mut owner = self.owner.borrow_mut();
            if owner
                .as_mut()
                .expect("live owner")
                .save_state(&mut bytes)
                .is_err()
            {
                return kResultFalse;
            }
        }
        // All foreign writes observe an owned coherent snapshot; musical state
        // stays accepted even if the stream fails after a written prefix.
        if output
            .write_all(&bytes)
            .and_then(|_| output.flush())
            .is_err()
        {
            return kResultFalse;
        }
        kResultOk
    }
}
struct Projection {
    parameters: Vec<PublicParameter>,
    values: [f64; 23],
}
struct Controller {
    root: PathBuf,
    thread: libc::pid_t,
    terminated: Cell<bool>,
    projection: RefCell<Option<Projection>>,
    transport: Transport,
    bootstrap: RefCell<Option<String>>,
    next_sequence: Cell<u64>,
    awaiting: Cell<Option<u64>>,
    last_reply: RefCell<Option<(u64, String)>>,
}
impl Class for Controller {
    type Interfaces = (IEditController, IConnectionPoint);
}
impl Controller {
    fn main(&self) -> bool {
        self.thread == platform::thread_id() && !self.terminated.get()
    }
    fn plain(&self, id: u32, value: f64) -> Option<f64> {
        if !self.main() {
            return None;
        }
        let p = self.projection.borrow();
        p.as_ref()?.parameters[parameters::index(id)?].normalized_to_physical(value)
    }
    fn normalized(&self, id: u32, value: f64) -> Option<f64> {
        if !self.main() {
            return None;
        }
        let p = self.projection.borrow();
        p.as_ref()?.parameters[parameters::index(id)?].physical_to_normalized(value)
    }
}
impl IPluginBaseTrait for Controller {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        if !self.main() || self.terminated.get() || self.projection.borrow().is_some() {
            return kResultFalse;
        }
        let Ok(descriptors) =
            public_parameters_from_resources(&self.root).and_then(|p| parameters::ordered(&p))
        else {
            return kResultFalse;
        };
        let values = std::array::from_fn(|i| {
            descriptors[i]
                .physical_to_normalized(descriptors[i].physical_default)
                .expect("validated descriptor default")
        });
        *self.projection.borrow_mut() = Some(Projection {
            parameters: descriptors,
            values,
        });
        kResultOk
    }
    unsafe fn terminate(&self) -> tresult {
        if self.thread != platform::thread_id() {
            return kResultFalse;
        }
        self.terminated.set(true);
        self.transport.terminate_peer();
        self.transport.close();
        self.clear_correlation();
        self.projection.borrow_mut().take();
        kResultOk
    }
}
impl IEditControllerTrait for Controller {
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
    unsafe fn getParameterCount(&self) -> i32 {
        if self.main() && self.projection.borrow().is_some() {
            23
        } else {
            0
        }
    }
    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        let p = self.projection.borrow();
        let Some(p) = p.as_ref() else {
            return kNotInitialized;
        };
        let Some(descriptor) = usize::try_from(index)
            .ok()
            .and_then(|i| p.parameters.get(i))
        else {
            return kInvalidArgument;
        };
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one exclusive writable ParameterInfo. The index was checked before
        // converting it back to usize; descriptor values come from the shared policy.
        let info = unsafe { &mut *info };
        info.id = parameters::IDS[index as usize].0;
        utf16(&mut info.title, &descriptor.id);
        utf16(&mut info.shortTitle, &descriptor.id);
        utf16(&mut info.units, "");
        info.stepCount = 0;
        info.defaultNormalizedValue = descriptor
            .physical_to_normalized(descriptor.physical_default)
            .expect("validated default");
        info.unitId = 0;
        info.flags = ParameterInfo_::ParameterFlags_::kCanAutomate;
        kResultOk
    }
    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        normalized: f64,
        text: *mut String128,
    ) -> tresult {
        if text.is_null() {
            return kInvalidArgument;
        }
        let Some(value) = self.plain(id, normalized) else {
            return kInvalidArgument;
        };
        // SAFETY: a complete writable128-code-unit SDK string is supplied by host.
        utf16(unsafe { &mut *text }, &value.to_string());
        kResultOk
    }
    unsafe fn getParamValueByString(
        &self,
        id: u32,
        text: *mut TChar,
        normalized: *mut f64,
    ) -> tresult {
        if text.is_null() || normalized.is_null() {
            return kInvalidArgument;
        }
        let mut units = [0; 128];
        let mut length = 0;
        while length < units.len() {
            // SAFETY: host supplies a NUL-terminated readable UTF-16 string. Read only
            // through its terminator, rejecting text beyond the supported SDK extent.
            let unit = unsafe { *text.add(length) };
            if unit == 0 {
                break;
            }
            units[length] = unit;
            length += 1;
        }
        if length == units.len() {
            return kInvalidArgument;
        }
        let Ok(text) = String::from_utf16(&units[..length]) else {
            return kInvalidArgument;
        };
        let Some(value) = text
            .parse::<f64>()
            .ok()
            .and_then(|v| self.normalized(id, v))
        else {
            return kInvalidArgument;
        };
        // SAFETY: non-null writable scalar output is supplied by host.
        unsafe {
            *normalized = value;
        }
        kResultOk
    }
    unsafe fn normalizedParamToPlain(&self, id: u32, value: f64) -> f64 {
        self.plain(id, value).unwrap_or(f64::NAN)
    }
    unsafe fn plainParamToNormalized(&self, id: u32, value: f64) -> f64 {
        self.normalized(id, value).unwrap_or(f64::NAN)
    }
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if !self.main() {
            return 0.0;
        }
        self.projection
            .borrow()
            .as_ref()
            .and_then(|p| parameters::index(id).map(|i| p.values[i]))
            .unwrap_or(0.0)
    }
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        if !(0.0..=1.0).contains(&value) {
            return kInvalidArgument;
        }
        let Some(index) = parameters::index(id) else {
            return kInvalidArgument;
        };
        let mut p = self.projection.borrow_mut();
        let Some(p) = p.as_mut() else {
            return kNotInitialized;
        };
        p.values[index] = value;
        kResultOk
    }
    unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
        if self.main() { kResultOk } else { kResultFalse }
    }
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        std::ptr::null_mut()
    }
}

impl ProcessorCore {
    fn disconnect_projection(&self) {
        self.processed.borrow_mut().clear();
        self.last_request.set(0);
        let attachment = self.attachment.borrow_mut().take();
        if let Some(attachment) = attachment
            && let Some(owner) = self.owner.borrow_mut().as_mut()
        {
            let _ = owner.detach_editor(&attachment);
        }
    }
    fn projection(&self) {
        if self.transport.bound().is_none()
            || self.attachment.borrow().is_some()
            || !self.transport.has_space()
        {
            return;
        }
        let frame = {
            let mut owner = self.owner.borrow_mut();
            let Some(owner) = owner.as_mut() else {
                return;
            };
            let Ok(connection) = owner.attach_editor() else {
                return;
            };
            *self.attachment.borrow_mut() = Some(connection.attachment_id.clone());
            serde_json::json!({"protocolVersion":1,"instanceId":connection.instance_id,"attachmentId":connection.attachment_id,"nextRequestId":"1"}).to_string()
        };
        let _ = self.transport.send(0, Body::Projection { frame });
    }
}
impl IConnectionPointTrait for Processor {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        let result = unsafe { self.transport.connect(other) };
        if result == kResultOk {
            self.transport.pump();
            self.projection();
            self.transport.pump();
        }
        result
    }
    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        let result = self.transport.disconnect(other);
        if result == kResultOk {
            self.disconnect_projection();
        }
        result
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        match unsafe { self.transport.receive(message) } {
            Err(error) => {
                self.transport.pump();
                return error;
            }
            Ok(None) => {}
            Ok(Some(packet)) => match packet.body {
                Body::Goodbye if packet.sequence == 0 => {
                    self.disconnect_projection();
                    self.transport.close();
                    return kResultOk;
                }
                Body::Request { frame } => {
                    let result = self.request(packet.sequence, &frame);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                _ => return kNotImplemented,
            },
        }
        self.projection();
        self.transport.pump();
        kResultOk
    }
}
impl IConnectionPointTrait for Controller {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        if !self.main() || self.projection.borrow().is_none() {
            return kNotInitialized;
        }
        let result = unsafe { self.transport.connect(other) };
        if result == kResultOk {
            self.transport.pump();
        }
        result
    }
    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        let result = self.transport.disconnect(other);
        if result == kResultOk {
            self.clear_correlation();
        }
        result
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        if !self.main() || self.projection.borrow().is_none() {
            return kNotInitialized;
        }
        match unsafe { self.transport.receive(message) } {
            Err(error) => {
                self.transport.pump();
                return error;
            }
            Ok(None) => {}
            Ok(Some(packet)) => match packet.body {
                Body::Projection { frame } if packet.sequence == 0 => {
                    let result = self.receive_projection(frame);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                Body::Goodbye if packet.sequence == 0 => {
                    self.clear_correlation();
                    self.transport.close();
                    return kResultOk;
                }
                Body::Reply { frame } => {
                    let result = self.receive_parameter_reply(packet.sequence, frame);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                _ => return kNotImplemented,
            },
        }
        self.transport.pump();
        kResultOk
    }
}

#[derive(Default)]
struct WorkerWake(AtomicBool);
impl SessionServiceWake for WorkerWake {
    fn request_service(&self) {
        self.0.store(true, Ordering::Release);
    }
}
struct Registration {
    runloop: Rc<ComPtr<IRunLoop>>,
    timer: ComPtr<ITimerHandler>,
}
impl Registration {
    fn unregister(self) -> Result<(), Self> {
        // Ownership comes back on refusal: no promised cleanup token is lost.
        if unsafe { self.runloop.unregisterTimer(self.timer.as_ptr()) } == kResultOk {
            Ok(())
        } else {
            Err(self)
        }
    }
}
struct Timer(libc::pid_t, Weak<ProcessorCore>);
impl Class for Timer {
    type Interfaces = (ITimerHandler,);
}
impl ITimerHandlerTrait for Timer {
    unsafe fn onTimer(&self) {
        if self.0 != platform::thread_id() {
            return;
        }
        if let Some(core) = self.1.upgrade() {
            core.service();
        }
    }
}
impl ProcessorCore {
    fn service(&self) {
        if !self.live() || self.servicing.replace(true) {
            return;
        }
        struct Servicing<'a>(&'a Cell<bool>);
        impl Drop for Servicing<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _servicing = Servicing(&self.servicing);
        self.wake.0.store(false, Ordering::Release);
        {
            let Ok(mut owner) = self.owner.try_borrow_mut() else {
                return;
            };
            let Some(owner) = owner.as_mut() else {
                return;
            };
            let _ = owner.service_editor_notes(std::time::Instant::now());
            let _ = owner.service_session_jobs();
            if !self.audio.processing.load(Ordering::Acquire) {
                let _ = self.audio.with(|audio| {
                    let _ = owner.service_quiescent(&mut audio.runtime);
                    let _ = audio.runtime.flush_parameters(&[]);
                });
            }
        }
        self.projection();
        self.transport.pump();
        if self.transport.bound().is_some()
            && self.transport.has_space()
            && !self.transport.queued(0, &Body::Confirm)
        {
            let _ = self.transport.send(0, Body::Confirm);
            self.transport.pump();
        }
    }
    fn shutdown(&self) -> bool {
        self.terminated.set(true);
        self.initialized.set(false);
        self.audio.live.store(false, Ordering::Release);
        let registration = self.registration.borrow_mut().take();
        if let Some(registration) = registration
            && let Err(registration) = registration.unregister()
        {
            // Disabled weak token is retained for an explicit cleanup retry.
            *self.registration.borrow_mut() = Some(registration);
        }
        self.disconnect_projection();
        self.transport.terminate_peer();
        self.transport.close();
        let owner = self.owner.borrow_mut().take();
        drop(owner);
        self.audio.clear_main();
        let runloop = self.runloop.borrow_mut().take();
        drop(runloop);
        self.registration.borrow().is_none()
    }
}
impl Drop for ProcessorCore {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

struct Processed {
    sequence: u64,
    request: String,
    reply: String,
}
impl ProcessorCore {
    fn request(&self, sequence: u64, frame: &str) -> tresult {
        if sequence == 0 || frame.len() > dandrum_trigger_host::protocol::PATCH_MESSAGE_BYTES_LIMIT
        {
            return kInvalidArgument;
        }
        let cached = self
            .processed
            .borrow()
            .iter()
            .find(|p| p.sequence == sequence)
            .map(|p| (p.request == frame, p.reply.clone()));
        if let Some((same, reply)) = cached {
            if !same {
                return kResultFalse;
            }
            let body = Body::Reply { frame: reply };
            return if self.transport.queued(sequence, &body) {
                kResultOk
            } else {
                self.transport.send(sequence, body)
            };
        }
        if self.last_request.get().checked_add(1) != Some(sequence) {
            return kResultFalse;
        }
        // Reserve bounded reply room before admitting any musical command.
        if !self.transport.has_space() {
            return kResultFalse;
        }
        let Ok(request) = serde_json::from_str::<serde_json::Value>(frame) else {
            return kInvalidArgument;
        };
        let (Some(attachment), Some(request_id)) = (
            request["attachmentId"].as_str(),
            request["requestId"].as_str(),
        ) else {
            return kInvalidArgument;
        };
        if self.attachment.borrow().as_deref() != Some(attachment) {
            return kResultFalse;
        }
        let reply = {
            let Ok(mut owner) = self.owner.try_borrow_mut() else {
                return kResultFalse;
            };
            let Some(owner) = owner.as_mut() else {
                return kNotInitialized;
            };
            match dandrum_trigger_host::editor_commands::dispatch(
                owner, attachment, request_id, frame,
            ) {
                Ok(reply) => reply,
                Err(_) => return kInvalidArgument,
            }
        };
        // Retain the outcome before attempting transport delivery. A retry can
        // never run the accepted command a second time, even if notify refuses.
        {
            let mut processed = self.processed.borrow_mut();
            if processed.len() == crate::transport::PENDING {
                processed.pop_front();
            }
            processed.push_back(Processed {
                sequence,
                request: frame.into(),
                reply: reply.clone(),
            });
        }
        self.last_request.set(sequence);
        self.transport.send(sequence, Body::Reply { frame: reply })
    }
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bootstrap {
    protocol_version: u32,
    instance_id: String,
    attachment_id: String,
    next_request_id: String,
}
impl Bootstrap {
    fn parse(frame: &str) -> Option<Self> {
        let bootstrap: Self = serde_json::from_str(frame).ok()?;
        if bootstrap.protocol_version != 1 || bootstrap.next_request_id != "1" {
            return None;
        }
        let instance = bootstrap.instance_id.parse::<u64>().ok()?;
        let attachment = bootstrap.attachment_id.parse::<u64>().ok()?;
        (instance != 0
            && instance.to_string() == bootstrap.instance_id
            && attachment.to_string() == bootstrap.attachment_id)
            .then_some(bootstrap)
    }
}
impl Controller {
    fn clear_correlation(&self) {
        self.bootstrap.borrow_mut().take();
        self.awaiting.set(None);
        self.last_reply.borrow_mut().take();
        self.next_sequence.set(1);
    }
    fn receive_projection(&self, frame: String) -> tresult {
        if self.bootstrap.borrow().as_deref() == Some(&frame) {
            return kResultOk;
        }
        let Some(bootstrap) = Bootstrap::parse(&frame) else {
            return kInvalidArgument;
        };
        if !self.transport.has_space() {
            return kResultFalse;
        }
        let sequence = self.next_sequence.get();
        let Some(next) = sequence.checked_add(1) else {
            return kResultFalse;
        };
        let request = serde_json::json!({"version":1,"attachmentId":bootstrap.attachment_id,"requestId":"1","method":"getParameterState","args":[]}).to_string();
        let result = self
            .transport
            .send(sequence, Body::Request { frame: request });
        if result == kResultOk {
            self.next_sequence.set(next);
            self.awaiting.set(Some(sequence));
            self.last_reply.borrow_mut().take();
            *self.bootstrap.borrow_mut() = Some(frame);
        }
        result
    }
    fn receive_parameter_reply(&self, sequence: u64, frame: String) -> tresult {
        if let Some((last, previous)) = self.last_reply.borrow().as_ref()
            && *last == sequence
        {
            return if *previous == frame {
                kResultOk
            } else {
                kResultFalse
            };
        }
        if self.awaiting.get() != Some(sequence) {
            return kResultFalse;
        }
        let Some(bootstrap) = self
            .bootstrap
            .borrow()
            .as_deref()
            .and_then(Bootstrap::parse)
        else {
            return kNotInitialized;
        };
        let Ok(reply) = serde_json::from_str::<serde_json::Value>(&frame) else {
            return kInvalidArgument;
        };
        if reply["status"] != "accepted"
            || reply["version"] != 1
            || reply["instanceId"] != bootstrap.instance_id
            || reply["attachmentId"] != bootstrap.attachment_id
            || reply["requestId"] != "1"
        {
            return kInvalidArgument;
        }
        let Some(parameters) = reply["result"]["parameters"]
            .as_array()
            .filter(|p| p.len() == 23)
        else {
            return kInvalidArgument;
        };
        let mut values = [None; 23];
        for parameter in parameters {
            let Some(index) = parameters::IDS
                .iter()
                .position(|(_, id)| parameter["id"] == *id)
            else {
                return kInvalidArgument;
            };
            let Some(value) = parameter["value"]
                .as_f64()
                .filter(|v| (0.0..=1.0).contains(v))
            else {
                return kInvalidArgument;
            };
            if values[index].replace(value).is_some() {
                return kInvalidArgument;
            }
        }
        let mut projection = self.projection.borrow_mut();
        let Some(projection) = projection.as_mut() else {
            return kNotInitialized;
        };
        projection.values = values.map(|value| value.expect("23 distinct named values"));
        self.awaiting.set(None);
        *self.last_reply.borrow_mut() = Some((sequence, frame));
        kResultOk
    }
}

impl IAudioProcessorTrait for Processor {
    unsafe fn setBusArrangements(
        &self,
        _inputs: *mut SpeakerArrangement,
        num_inputs: i32,
        outputs: *mut SpeakerArrangement,
        num_outputs: i32,
    ) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if num_inputs != 0 || num_outputs != 1 || outputs.is_null() {
            return kInvalidArgument;
        }
        if unsafe { *outputs } != 3 {
            return kResultFalse;
        }
        kResultOk
    }
    unsafe fn getBusArrangement(
        &self,
        direction: BusDirection,
        index: i32,
        output: *mut SpeakerArrangement,
    ) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if direction != 1 || index != 0 || output.is_null() {
            return kInvalidArgument;
        }
        unsafe {
            *output = 3;
        }
        kResultOk
    }
    unsafe fn canProcessSampleSize(&self, size: i32) -> tresult {
        if size == 0 { kResultOk } else { kResultFalse }
    }
    unsafe fn getLatencySamples(&self) -> u32 {
        0
    }
    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        if !self.live() {
            return kNotInitialized;
        }
        if setup.is_null() {
            return kInvalidArgument;
        }
        let setup = unsafe { *setup };
        if setup.symbolicSampleSize != 0
            || !(0..=2).contains(&setup.processMode)
            || !(1..=16_384).contains(&setup.maxSamplesPerBlock)
        {
            return kInvalidArgument;
        }
        let Ok(rate) = SampleRate::new(setup.sampleRate) else {
            return kInvalidArgument;
        };
        if self.audio.processing.load(Ordering::Acquire)
            || self.audio.active.load(Ordering::Acquire)
        {
            return kResultFalse;
        }
        let configuration = TriggerHostConfiguration {
            sample_rate_hz: rate,
            max_block_frames: setup.maxSamplesPerBlock as u32,
        };
        self.audio
            .with(|audio| {
                let mut owner = self.owner.borrow_mut();
                let Some(owner) = owner.as_mut() else {
                    return kNotInitialized;
                };
                if owner
                    .reconfigure_quiescent(&mut audio.runtime, configuration)
                    .is_err()
                {
                    return kResultFalse;
                }
                audio.configuration = configuration;
                audio.mode = setup.processMode;
                self.audio
                    .max_frames
                    .store(configuration.max_block_frames, Ordering::Release);
                self.audio.configured.store(true, Ordering::Release);
                kResultOk
            })
            .unwrap_or(kResultFalse)
    }
    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        self.audio.set_processing(state)
    }
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        unsafe { self.audio.process(data) }
    }
    unsafe fn getTailSamples(&self) -> u32 {
        u32::MAX
    }
}
