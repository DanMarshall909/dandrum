//! Separate COM components: one processor sound owner and a metadata projection.
use crate::{
    CONTROLLER_CLASS_ID, PROCESSOR_CLASS_ID, parameters, platform,
    transport::{Body, HostAutomation, Transport},
    utf16,
};
use dandrum_trigger_host::{
    SampleRate, TriggerHostConfiguration,
    parameters::{
        AutomationEvent, ParameterController, ParameterFeedback, PublicParameter,
        SessionServiceWake, public_parameters_from_resources,
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
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Linux::*, Vst::*, *},
};

#[cfg(feature = "trigger-editor")]
mod view;

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
            automation: RefCell::new(None),
            next_automation: Cell::new(1),
            closing: Cell::new(false),
            shutdown_close: Cell::new(false),
            connection_busy: Cell::new(false),
        })))
        .to_com_ptr::<FUnknown>()
        .expect("component includes FUnknown"))
    } else {
        Ok(ComWrapper::new(Controller(Rc::new(ControllerCore {
            root,
            thread: platform::thread_id(),
            terminated: Cell::new(false),
            projection: RefCell::new(None),
            transport: Transport::default(),
            bootstrap: RefCell::new(None),
            next_sequence: Cell::new(1),
            awaiting: Cell::new(None),
            last_reply: RefCell::new(None),
            feedback_cutoff: Cell::new(None),
            feedback_refresh: Cell::new(false),
            handler: RefCell::new(None),
            host_busy: Cell::new(false),
            host_gestures: RefCell::new(std::array::from_fn(|_| None)),
            host_receipt: RefCell::new(None),
            closing: Cell::new(false),
            release_peer_on_retirement: Cell::new(false),
            retired_proof: Cell::new(false),
        })))
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
    automation: RefCell<Option<AutomationFlight>>,
    next_automation: Cell<u64>,
    closing: Cell<bool>,
    shutdown_close: Cell<bool>,
    connection_busy: Cell<bool>,
}
#[derive(Clone)]
struct AutomationFlight {
    delivery: u64,
    shared: AutomationEvent,
    event: HostAutomation,
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
        // Reject foreign threads before reading any main-thread Cell/RefCell.
        if self.thread != platform::thread_id() {
            return kResultFalse;
        }
        // A stopped/zero-sample callback may be inside a foreign event/point
        // getter. Reject reentrant teardown before acquiring/destroying audio.
        if self.audio.processing.load(Ordering::Acquire)
            || self.audio.in_use()
            || self.connection_busy.get()
            || (self.transport.normal_active() && self.automation.borrow().is_some())
        {
            return kResultFalse;
        }
        if !self.request_projection_close(true) {
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
    feedback: [Option<FeedbackValue>; 23],
}
#[derive(Clone, Copy)]
struct FeedbackValue {
    sequence: u64,
    publication: u64,
    snapshot_cutoff: Option<u64>,
    value: f64,
}
#[derive(Clone, Copy)]
struct ParameterRequest {
    sequence: u64,
    apply_values: bool,
}
struct Controller(Rc<ControllerCore>);
impl Deref for Controller {
    type Target = ControllerCore;
    fn deref(&self) -> &ControllerCore {
        &self.0
    }
}
struct ControllerCore {
    root: PathBuf,
    thread: libc::pid_t,
    terminated: Cell<bool>,
    projection: RefCell<Option<Projection>>,
    transport: Transport,
    bootstrap: RefCell<Option<String>>,
    next_sequence: Cell<u64>,
    awaiting: Cell<Option<ParameterRequest>>,
    last_reply: RefCell<Option<(u64, String, u64)>>,
    feedback_cutoff: Cell<Option<u64>>,
    feedback_refresh: Cell<bool>,
    handler: RefCell<Option<Rc<ComPtr<IComponentHandler>>>>,
    host_busy: Cell<bool>,
    host_gestures: RefCell<[Option<Rc<ComPtr<IComponentHandler>>>; 23]>,
    host_receipt: RefCell<Option<(u64, HostAutomation)>>,
    closing: Cell<bool>,
    release_peer_on_retirement: Cell<bool>,
    retired_proof: Cell<bool>,
}
struct HostCall<'a>(&'a Cell<bool>);
impl Drop for HostCall<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Class for Controller {
    type Interfaces = (IEditController, IConnectionPoint);
}
impl ControllerCore {
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
            feedback: [None; 23],
        });
        kResultOk
    }
    unsafe fn terminate(&self) -> tresult {
        if self.thread != platform::thread_id() || self.host_busy.get() {
            return kResultFalse;
        }
        self.closing.set(true);
        self.release_peer_on_retirement.set(true);
        self.transport.begin_closing();
        if !self.retired_proof.get() {
            let _ = self.transport.terminal(Body::Closing);
        }
        if !self.close_host_gestures() {
            return kResultFalse;
        }
        self.terminated.set(true);
        // Local safety is balanced/fenced callbacks. Keep the scoped terminal
        // responder alive for a peer whose certificate delivery was deferred.
        if self.retired_proof.get() || self.transport.bound().is_none() {
            self.transport.release_peer();
        } else {
            let _ = self.transport.terminal(Body::Closed);
        }
        self.clear_correlation();
        self.projection.borrow_mut().take();
        let handler = self.handler.borrow_mut().take();
        drop(handler);
        kResultOk
    }
}
impl IEditControllerTrait for Controller {
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        let parameters = {
            self.projection
                .borrow()
                .as_ref()
                .map(|p| p.parameters.clone())
        };
        let Some(parameters) = parameters else {
            return kNotInitialized;
        };
        let Some(mut input) = (unsafe { crate::stream::Stream::from_raw(state) }) else {
            return kInvalidArgument;
        };
        let Ok(header) = dandrum_trigger_host::state::read_header(&mut input) else {
            return kResultFalse;
        };
        if header.parameter_values(&parameters).is_err() {
            return kResultFalse;
        }
        let Ok(buffered) = header.buffer_payload(&mut input) else {
            return kResultFalse;
        };
        let Ok(values) = buffered.parameter_values(&parameters) else {
            return kResultFalse;
        };
        let normalized = std::array::from_fn(|i| {
            parameters[i]
                .physical_to_normalized(values[parameters[i].root_index])
                .expect("validated projection")
        });
        if !self.main() {
            return kNotInitialized;
        }
        let mut slot = self.projection.borrow_mut();
        let Some(projection) = slot.as_mut() else {
            return kNotInitialized;
        };
        projection.values = normalized;
        projection.feedback.fill(None);
        drop(slot);
        self.feedback_cutoff.set(None);
        // Capture a fresh processor barrier immediately, even if an older
        // snapshot reply is delayed. Issued obsolete queries are acknowledged
        // below without applying their values or moving this cutoff.
        self.awaiting.set(None);
        self.feedback_refresh.set(self.bootstrap.borrow().is_some());
        self.retry_parameter_request(false);
        self.transport.pump();
        kResultOk
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
    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        if !self.main() || self.host_busy.replace(true) {
            return kResultFalse;
        }
        let _call = HostCall(&self.host_busy);
        if self.host_gestures.borrow().iter().any(Option::is_some) {
            return kResultFalse;
        }
        // AddRef and Release are foreign calls too. Rc snapshots permit calls
        // outside affine borrows without another foreign reference operation.
        let handler = unsafe { ComRef::from_raw(handler) }.map(|h| Rc::new(h.to_com_ptr()));
        let old = self.handler.replace(handler);
        drop(old);
        kResultOk
    }
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        #[cfg(feature = "trigger-editor")]
        {
            if self.main()
                && !self.closing.get()
                && self.projection.try_borrow().is_ok_and(|p| p.is_some())
                // SAFETY: the host supplies a live NUL-terminated SDK name;
                // comparison stops at the first mismatch or known terminator.
                && unsafe { view::matches(_name, b"editor\0") }
            {
                return view::create(&self.0);
            }
        }
        std::ptr::null_mut()
    }
}

impl ProcessorCore {
    fn request_projection_close(&self, shutdown: bool) -> bool {
        if self.transport.retired_pending() {
            let _ = self.transport.terminal_retired(Body::Retired);
            return !self.closing.get();
        }
        if self.transport.bound().is_none() {
            return true;
        }
        self.closing.set(true);
        self.shutdown_close
            .set(self.shutdown_close.get() || shutdown);
        self.transport.begin_closing();
        let _ = self.transport.terminal(if self.shutdown_close.get() {
            Body::Goodbye
        } else {
            Body::Disconnect
        });
        !self.closing.get()
    }
    fn service_automation(&self) {
        if self.closing.get() {
            return;
        }
        let Some(scope) = self.transport.bound() else {
            return;
        };
        for _ in 0..64 {
            if self.transport.bound() != Some(scope) || self.attachment.borrow().is_none() {
                return;
            }
            if self.automation.borrow().is_none() {
                let delivery = self.next_automation.get();
                let Some(next) = delivery.checked_add(1) else {
                    return;
                };
                let event = {
                    let owner = self.owner.borrow();
                    let Some(owner) = owner.as_ref() else {
                        return;
                    };
                    let Some(shared) = owner.peek_automation() else {
                        return;
                    };
                    let index = match shared {
                        AutomationEvent::Begin { parameter_index }
                        | AutomationEvent::Value {
                            parameter_index, ..
                        }
                        | AutomationEvent::End { parameter_index } => parameter_index,
                    };
                    let descriptor = &owner.parameters()[index];
                    let Some((parameter, _)) = parameters::IDS
                        .iter()
                        .find(|(_, name)| *name == descriptor.id)
                    else {
                        return;
                    };
                    let event = match shared {
                        AutomationEvent::Begin { .. } => HostAutomation::Begin {
                            parameter: *parameter,
                        },
                        AutomationEvent::Value { value, .. } => {
                            let Some(value) = descriptor.physical_to_normalized(value) else {
                                return;
                            };
                            HostAutomation::Value {
                                parameter: *parameter,
                                value,
                            }
                        }
                        AutomationEvent::End { .. } => HostAutomation::End {
                            parameter: *parameter,
                        },
                    };
                    AutomationFlight {
                        delivery,
                        shared,
                        event,
                    }
                };
                *self.automation.borrow_mut() = Some(event);
                self.next_automation.set(next);
            }
            let flight = self
                .automation
                .borrow()
                .as_ref()
                .expect("staged event")
                .clone();
            let body = Body::Automation {
                delivery: flight.delivery,
                event: flight.event,
            };
            if !self.transport.queued(0, &body) && self.transport.send(0, body) != kResultOk {
                return;
            }
            // The receiver's notify result only releases transport storage.
            // Only a correlated application acknowledgement retires the ledger.
            self.transport.pump();
            if self.automation.borrow().is_some() {
                return;
            }
        }
    }
    fn acknowledge_automation(&self, delivery: u64) -> tresult {
        let flight = self.automation.borrow().clone();
        let Some(flight) = flight.filter(|flight| flight.delivery == delivery) else {
            return if delivery != 0 && delivery < self.next_automation.get() {
                kResultOk
            } else {
                kResultFalse
            };
        };
        let Ok(mut owner) = self.owner.try_borrow_mut() else {
            return kResultFalse;
        };
        let Some(owner) = owner.as_mut() else {
            return kNotInitialized;
        };
        if owner.peek_automation() != Some(flight.shared) {
            return kResultFalse;
        }
        assert!(
            owner.acknowledge_automation(),
            "correlated host success owns one ledger head"
        );
        self.automation.borrow_mut().take();
        kResultOk
    }
    fn service_parameter_feedback(&self) {
        for _ in 0..64 {
            if !self.transport.has_space() {
                break;
            }
            let attachment = self.attachment.borrow().clone();
            let event = {
                let Ok(mut slot) = self.owner.try_borrow_mut() else {
                    return;
                };
                let Some(owner) = slot.as_mut() else {
                    return;
                };
                let (feedback, publication, origins) = match owner.take_feedback_with_publication()
                {
                    Some((feedback, publication)) => (feedback, publication, None),
                    None => {
                        // The audio ring deliberately drops instead of blocking.
                        // After its retained prefix drains, coalesce any loss into
                        // one authoritative snapshot. This signal is separate
                        // from endpoint ownership, so service cannot interrupt a
                        // live callback merely to observe feedback loss.
                        if !self.audio.feedback_lost.swap(false, Ordering::AcqRel) {
                            return;
                        }
                        let (state, publication, origins) = owner.parameter_feedback_snapshot();
                        (
                            ParameterFeedback {
                                generation: state.generation,
                                sequence: state.sequence,
                                reset_cancelled: state.reset_cancelled,
                                frame_offset: 0,
                                parameters: state.parameters,
                            },
                            publication,
                            Some(
                                origins
                                    .into_iter()
                                    .map(|origin| origin.to_string())
                                    .collect(),
                            ),
                        )
                    }
                };
                attachment.as_ref().and_then(|attachment| {
                    owner
                        .encode_parameter_feedback(attachment, feedback)
                        .ok()
                        .map(|frame| Body::Event {
                            frame,
                            publication: publication.to_string(),
                            origins,
                        })
                })
            };
            if let Some(event) = event {
                let snapshot = matches!(
                    &event,
                    Body::Event {
                        origins: Some(_),
                        ..
                    }
                );
                let result = self.transport.send(0, event);
                if result != kResultOk {
                    if snapshot {
                        self.audio.feedback_lost.store(true, Ordering::Release);
                    }
                    return;
                }
            }
        }
    }
    fn disconnect_projection(&self, cancel_automation: bool) {
        self.automation.borrow_mut().take();
        self.next_automation.set(1);
        self.processed.borrow_mut().clear();
        self.last_request.set(0);
        let attachment = self.attachment.borrow_mut().take();
        if let Some(attachment) = attachment
            && let Some(owner) = self.owner.borrow_mut().as_mut()
        {
            if cancel_automation {
                let _ = owner.detach_editor_and_cancel_automation(&attachment);
            } else {
                let _ = owner.detach_editor(&attachment);
            }
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
        if self.connection_busy.get() || self.closing.get() || self.transport.terminal_active() {
            return kResultFalse;
        }
        let result = {
            self.connection_busy.set(true);
            let _call = HostCall(&self.connection_busy);
            unsafe { self.transport.connect(other) }
        };
        if result == kResultOk {
            self.transport.pump();
            self.projection();
            self.transport.pump();
        }
        result
    }
    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        if !self.main()
            || self.connection_busy.get()
            || (self.transport.normal_active() && self.automation.borrow().is_some())
        {
            return kResultFalse;
        }
        self.connection_busy.set(true);
        let _call = HostCall(&self.connection_busy);
        if self.transport.retired_peer_matches(other) {
            // Explicit local disconnection ends the final-Ack retention window.
            self.transport.release_retired();
            self.closing.set(false);
            self.shutdown_close.set(false);
            return kResultOk;
        }
        if !self.transport.peer_matches(other) {
            return kInvalidArgument;
        }
        if self.attachment.borrow().is_some() && self.transport.bound().is_some() {
            return if self.request_projection_close(false) {
                kResultOk
            } else {
                kResultFalse
            };
        }
        let result = self.transport.disconnect(other);
        if result == kResultOk {
            self.disconnect_projection(false);
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
                Body::Closing | Body::Goodbye if packet.sequence == 0 => {
                    self.closing.set(true);
                    self.transport.begin_closing();
                    return kResultOk;
                }
                Body::Closed if packet.sequence == 0 => {
                    if self.transport.is_current(&packet) {
                        self.closing.set(true);
                        self.disconnect_projection(true);
                        self.transport.retire();
                    }
                    let _ = self.transport.terminal_retired(Body::Retired);
                    return kResultOk;
                }
                Body::RetiredAck if packet.sequence == 0 => {
                    if !self.transport.is_retired(&packet) {
                        return kResultFalse;
                    }
                    if !self.transport.retired_pending() {
                        return kResultOk;
                    }
                    self.transport.release_retired();
                    self.closing.set(false);
                    self.shutdown_close.set(false);
                    return kResultOk;
                }
                Body::Request { frame } => {
                    let result = self.request(packet.sequence, &frame);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                Body::AutomationAck { delivery } if packet.sequence == 0 => {
                    let result = self.acknowledge_automation(delivery);
                    if result != kResultOk {
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
        if !self.main()
            || self.host_busy.get()
            || self.transport.terminal_active()
            || self.projection.borrow().is_none()
        {
            return kNotInitialized;
        }
        if self.retired_proof.get() && self.transport.bound().is_none() {
            self.closing.set(false);
            self.retired_proof.set(false);
            self.release_peer_on_retirement.set(false);
        }
        let result = {
            self.host_busy.set(true);
            let _call = HostCall(&self.host_busy);
            unsafe { self.transport.connect(other) }
        };
        if result == kResultOk {
            self.transport.pump();
        }
        result
    }
    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        if self.thread != platform::thread_id() || self.host_busy.get() {
            return kResultFalse;
        }
        if !self.transport.peer_matches(other) {
            return kInvalidArgument;
        }
        if self.retired_proof.get() || self.transport.bound().is_none() {
            self.host_busy.set(true);
            let _call = HostCall(&self.host_busy);
            let result = self.transport.disconnect(other);
            if result == kResultOk {
                self.clear_correlation();
                self.closing.set(false);
                self.retired_proof.set(false);
                self.release_peer_on_retirement.set(false);
            }
            return result;
        }
        self.closing.set(true);
        self.release_peer_on_retirement.set(true);
        self.transport.begin_closing();
        let _ = self.transport.terminal(Body::Closing);
        if !self.close_host_gestures() {
            return kResultFalse;
        }
        let _ = self.transport.terminal(Body::Closed);
        if self.retired_proof.get() {
            self.clear_correlation();
            self.closing.set(false);
            self.retired_proof.set(false);
            self.release_peer_on_retirement.set(false);
            return kResultOk;
        }
        kResultFalse
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        if self.thread == platform::thread_id() && !self.host_busy.get() {
            self.transport.pump_terminal();
        }
        if self.thread == platform::thread_id()
            && !self.host_busy.get()
            && (self.closing.get() || self.terminated.get())
        {
            // Retained terminal scope still authenticates retries after logical
            // termination; ordinary callbacks remain fenced.
            return match unsafe { self.transport.receive(message) } {
                Ok(Some(packet))
                    if matches!(packet.body, Body::Retired) && packet.sequence == 0 =>
                {
                    self.receive_retirement()
                }
                Ok(Some(packet)) => {
                    let close_request = packet.sequence == 0
                        && matches!(packet.body, Body::Goodbye | Body::Disconnect);
                    if close_request && matches!(packet.body, Body::Goodbye) {
                        self.release_peer_on_retirement.set(true);
                    }
                    let closed = self.close_host_gestures();
                    if closed {
                        let _ = self.transport.terminal(Body::Closed);
                    }
                    if close_request {
                        if closed { kResultOk } else { kResultFalse }
                    } else {
                        kNotInitialized
                    }
                }
                Err(error) => error,
                Ok(None) => kResultFalse,
            };
        }
        if !self.main() || self.host_busy.get() || self.projection.borrow().is_none() {
            return kNotInitialized;
        }
        let mut retry_awaiting = false;
        match unsafe { self.transport.receive(message) } {
            Err(error) => {
                self.transport.pump();
                return error;
            }
            Ok(None) => retry_awaiting = true,
            Ok(Some(packet)) => match packet.body {
                Body::Projection { frame } if packet.sequence == 0 => {
                    let result = self.receive_projection(frame);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                Body::Goodbye | Body::Disconnect if packet.sequence == 0 => {
                    if matches!(packet.body, Body::Goodbye) {
                        self.release_peer_on_retirement.set(true);
                    }
                    self.closing.set(true);
                    self.transport.begin_closing();
                    if !self.close_host_gestures() {
                        return kResultFalse;
                    }
                    self.clear_correlation();
                    let _ = self.transport.terminal(Body::Closed);
                    return kResultOk;
                }
                Body::Retired if packet.sequence == 0 => return self.receive_retirement(),
                Body::Reply { frame, publication } => {
                    let result = self.receive_parameter_reply(packet.sequence, frame, publication);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                Body::Event {
                    frame,
                    publication,
                    origins,
                } if packet.sequence == 0 => {
                    let result = self.receive_parameter_event(frame, publication, origins);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                Body::Automation { delivery, event } if packet.sequence == 0 => {
                    let result = self.receive_automation(delivery, event);
                    if result != kResultOk {
                        self.transport.pump();
                        return result;
                    }
                }
                _ => return kNotImplemented,
            },
        }
        self.retry_parameter_request(retry_awaiting);
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
        self.transport.pump_terminal();
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
                    if let Ok(report) = audio.runtime.flush_parameters(&[])
                        && report.dropped_feedback != 0
                    {
                        self.audio.feedback_lost.store(true, Ordering::Release);
                    }
                });
            }
        }
        if self.closing.get() {
            if self.transport.retired_pending() {
                let _ = self.transport.terminal_retired(Body::Retired);
            } else {
                let _ = self.request_projection_close(self.shutdown_close.get());
            }
            return;
        }
        self.projection();
        self.service_parameter_feedback();
        self.transport.pump();
        self.service_automation();
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
        self.disconnect_projection(false);
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
    reply: Body,
}
impl ProcessorCore {
    fn request(&self, sequence: u64, frame: &str) -> tresult {
        if self.closing.get() {
            return kResultFalse;
        }
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
            let body = reply;
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
        let (reply, publication) = {
            let Ok(mut owner) = self.owner.try_borrow_mut() else {
                return kResultFalse;
            };
            let Some(owner) = owner.as_mut() else {
                return kNotInitialized;
            };
            match dandrum_trigger_host::editor_commands::dispatch_with_publication(
                owner, attachment, request_id, frame,
            ) {
                Ok(reply) => reply,
                Err(_) => return kInvalidArgument,
            }
        };
        let reply = Body::Reply {
            frame: reply,
            publication: publication.map(|stamp| stamp.to_string()),
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
        self.transport.send(sequence, reply)
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
impl ControllerCore {
    fn receive_retirement(&self) -> tresult {
        if !self.closing.get()
            || self.host_busy.get()
            || self.host_gestures.borrow().iter().any(Option::is_some)
        {
            return kResultFalse;
        }
        self.host_busy.set(true);
        let _call = HostCall(&self.host_busy);
        self.retired_proof.set(true);
        // The pending terminal slot owns this exact return channel even when
        // the current terminal send is still on-stack or the peer is released.
        let _ = self.transport.terminal(Body::RetiredAck);
        if self.release_peer_on_retirement.get() {
            self.transport.release_peer();
        }
        self.clear_correlation();
        kResultOk
    }
    fn close_host_gestures(&self) -> bool {
        if self.host_busy.replace(true) {
            return false;
        }
        let _call = HostCall(&self.host_busy);
        let mut closed = true;
        for index in 0..23 {
            let handler = { self.host_gestures.borrow()[index].clone() };
            let Some(handler) = handler else {
                continue;
            };
            if unsafe { handler.endEdit(parameters::IDS[index].0) } == kResultOk {
                let old = { self.host_gestures.borrow_mut()[index].take() };
                drop(old);
            } else {
                closed = false;
            }
        }
        closed
    }
    fn acknowledge_host_delivery(&self, delivery: u64) -> tresult {
        let body = Body::AutomationAck { delivery };
        if self.transport.queued(0, &body) {
            kResultOk
        } else {
            self.transport.send(0, body)
        }
    }
    fn receive_automation(&self, delivery: u64, event: HostAutomation) -> tresult {
        if delivery == 0 {
            return kInvalidArgument;
        }
        let parameter = match &event {
            HostAutomation::Begin { parameter }
            | HostAutomation::Value { parameter, .. }
            | HostAutomation::End { parameter } => *parameter,
        };
        let Some(index) = parameters::index(parameter) else {
            return kInvalidArgument;
        };
        if let HostAutomation::Value { value, .. } = &event
            && !(0.0..=1.0).contains(value)
        {
            return kInvalidArgument;
        }
        let receipt = self.host_receipt.borrow().clone();
        if let Some((last, previous)) = &receipt {
            if delivery < *last || (delivery == *last && event == *previous) {
                return self.acknowledge_host_delivery(delivery);
            }
            if delivery == *last {
                return kResultFalse;
            }
        }
        if delivery != receipt.map_or(1, |(last, _)| last.saturating_add(1)) {
            return kResultFalse;
        }
        if self.host_busy.replace(true) {
            return kResultFalse;
        }
        let _call = HostCall(&self.host_busy);
        let handler = match &event {
            HostAutomation::Begin { .. } => {
                if self.host_gestures.borrow()[index].is_some() {
                    return kResultFalse;
                }
                self.handler.borrow().clone()
            }
            _ => self.host_gestures.borrow()[index].clone(),
        };
        let Some(handler) = handler else {
            return kResultFalse;
        };
        let result = unsafe {
            match &event {
                HostAutomation::Begin { .. } => handler.beginEdit(parameter),
                HostAutomation::Value { value, .. } => handler.performEdit(parameter, *value),
                HostAutomation::End { .. } => handler.endEdit(parameter),
            }
        };
        if result != kResultOk {
            return result;
        }
        match &event {
            HostAutomation::Begin { .. } => self.host_gestures.borrow_mut()[index] = Some(handler),
            HostAutomation::End { .. } => {
                let old = self.host_gestures.borrow_mut()[index].take();
                drop(old);
            }
            _ => {}
        }
        *self.host_receipt.borrow_mut() = Some((delivery, event));
        self.acknowledge_host_delivery(delivery)
    }
    fn clear_correlation(&self) {
        self.host_receipt.borrow_mut().take();
        self.bootstrap.borrow_mut().take();
        self.awaiting.set(None);
        self.last_reply.borrow_mut().take();
        self.next_sequence.set(1);
        self.feedback_cutoff.set(None);
        self.feedback_refresh.set(false);
        if let Some(projection) = self.projection.borrow_mut().as_mut() {
            projection.feedback.fill(None);
        }
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
            self.awaiting.set(Some(ParameterRequest {
                sequence,
                apply_values: true,
            }));
            self.last_reply.borrow_mut().take();
            *self.bootstrap.borrow_mut() = Some(frame);
            self.feedback_cutoff.set(None);
            self.feedback_refresh.set(false);
            if let Some(projection) = self.projection.borrow_mut().as_mut() {
                projection.feedback.fill(None);
            }
        }
        result
    }
    fn retry_parameter_request(&self, retry_awaiting: bool) {
        if !self.main() {
            return;
        }
        if self.awaiting.get().is_some() && !retry_awaiting {
            return;
        }
        let Some(bootstrap) = self
            .bootstrap
            .borrow()
            .as_deref()
            .and_then(Bootstrap::parse)
        else {
            return;
        };
        let request = self.awaiting.get().or_else(|| {
            self.feedback_refresh.get().then_some(ParameterRequest {
                sequence: self.next_sequence.get(),
                apply_values: false,
            })
        });
        let Some(request) = request else {
            return;
        };
        let Some(next_sequence) = request.sequence.checked_add(1) else {
            return;
        };
        let body=Body::Request {frame:serde_json::json!({"version":1,"attachmentId":bootstrap.attachment_id,"requestId":request.sequence.to_string(),"method":"getParameterState","args":[]}).to_string()};
        if self.transport.queued(request.sequence, &body) {
            return;
        }
        if self.transport.send(request.sequence, body) == kResultOk && self.awaiting.get().is_none()
        {
            self.next_sequence.set(next_sequence);
            self.awaiting.set(Some(request));
            self.feedback_refresh.set(false);
        }
    }
    fn receive_parameter_reply(
        &self,
        sequence: u64,
        frame: String,
        publication: Option<String>,
    ) -> tresult {
        let Some(publication) = publication.as_deref().and_then(publication_stamp) else {
            return kInvalidArgument;
        };
        if let Some((last, previous, stamp)) = self.last_reply.borrow().as_ref()
            && *last == sequence
        {
            return if *previous == frame && *stamp == publication {
                kResultOk
            } else {
                kResultFalse
            };
        }
        // Queries use one contiguous connection-local identity for both the
        // transport and canonical request ID. A watermark permits bounded
        // acknowledgement of obsolete issued replies without retaining frames.
        if sequence == 0 || sequence >= self.next_sequence.get() {
            return kResultFalse;
        }
        let request = self.awaiting.get().filter(|r| r.sequence == sequence);
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
            || reply["requestId"].as_str().and_then(canonical_u64) != Some(sequence)
        {
            return kInvalidArgument;
        }
        let Some(parameters) = reply["result"]["parameters"]
            .as_array()
            .filter(|p| p.len() == 23)
        else {
            return kInvalidArgument;
        };
        let Some(values) = normalized_parameter_values(parameters) else {
            return kInvalidArgument;
        };
        let Some(request) = request else {
            return kResultOk;
        };
        if request.apply_values {
            let mut projection = self.projection.borrow_mut();
            let Some(projection) = projection.as_mut() else {
                return kNotInitialized;
            };
            projection.values = values.map(|value| value.expect("23 distinct named values"));
        }
        {
            self.feedback_cutoff.set(Some(publication));
            if let Some(projection) = self.projection.borrow_mut().as_mut() {
                for (index, pending) in projection.feedback.iter().enumerate() {
                    if let Some(pending) = pending
                        && publication_after(pending.publication, publication)
                    {
                        projection.values[index] = pending.value;
                    }
                }
            }
        }
        self.awaiting.set(None);
        *self.last_reply.borrow_mut() = Some((sequence, frame, publication));
        kResultOk
    }
    fn receive_parameter_event(
        &self,
        frame: String,
        publication: String,
        origins: Option<Vec<String>>,
    ) -> tresult {
        let Some(publication) = publication_stamp(&publication) else {
            return kInvalidArgument;
        };
        let Some(bootstrap) = self
            .bootstrap
            .borrow()
            .as_deref()
            .and_then(Bootstrap::parse)
        else {
            return kNotInitialized;
        };
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&frame) else {
            return kInvalidArgument;
        };
        let sequence = event["sequence"]
            .as_str()
            .and_then(canonical_u64)
            .filter(|n| *n != 0);
        let Some(sequence) = sequence else {
            return kInvalidArgument;
        };
        let payload = &event["payload"];
        if event["version"] != 1
            || event["instanceId"] != bootstrap.instance_id
            || event["attachmentId"] != bootstrap.attachment_id
            || event["event"] != "parameterStateChanged"
            || !payload["generation"].as_u64().is_some_and(|g| {
                (1..=dandrum_trigger_host::protocol::MAX_SAFE_GENERATION).contains(&g)
            })
            || payload["sequence"]
                .as_u64()
                .is_none_or(|s| s > dandrum_trigger_host::protocol::MAX_SAFE_GENERATION)
            || payload["reset_cancelled"].as_bool().is_none()
            || payload["frame_offset"]
                .as_u64()
                .is_none_or(|f| f >= u64::from(dandrum_trigger_host::runtime::MAX_BLOCK_FRAMES))
        {
            return kInvalidArgument;
        }
        let Some(parameters) = payload["parameters"]
            .as_array()
            .filter(|p| !p.is_empty() && p.len() <= 23)
        else {
            return kInvalidArgument;
        };
        let Some(values) = normalized_parameter_values(parameters) else {
            return kInvalidArgument;
        };
        let snapshot = origins.is_some();
        let mut write_origins = [publication; 23];
        if let Some(origins) = origins {
            if origins.len() != 23 || parameters.len() != 23 {
                return kInvalidArgument;
            }
            for (parameter, origin) in parameters.iter().zip(origins) {
                let Some(origin) = publication_stamp(&origin)
                    .filter(|origin| !publication_after(*origin, publication))
                else {
                    return kInvalidArgument;
                };
                let index = parameters::IDS
                    .iter()
                    .position(|(_, id)| parameter["id"] == *id)
                    .expect("validated named values");
                write_origins[index] = origin;
            }
        }
        let mut slot = self.projection.borrow_mut();
        let Some(projection) = slot.as_mut() else {
            return kNotInitialized;
        };
        for (index, value) in values.iter().enumerate() {
            if let Some(value) = value
                && let Some(previous) = projection.feedback[index]
                && previous.sequence == sequence
                && (previous.publication != write_origins[index]
                    || previous.value != *value
                    || (snapshot && previous.snapshot_cutoff != Some(publication)))
            {
                return kResultFalse;
            }
        }
        for (index, value) in values.into_iter().enumerate() {
            let origin = write_origins[index];
            if let Some(value) = value
                && (!snapshot
                    || self
                        .feedback_cutoff
                        .get()
                        .is_none_or(|cutoff| publication_after(origin, cutoff)))
                && projection.feedback[index].is_none_or(|old| {
                    sequence > old.sequence
                        && (origin == old.publication || publication_after(origin, old.publication))
                        && (snapshot
                            || old
                                .snapshot_cutoff
                                .is_none_or(|cutoff| publication_after(origin, cutoff)))
                })
            {
                let previous_cutoff =
                    projection.feedback[index].and_then(|old| old.snapshot_cutoff);
                projection.feedback[index] = Some(FeedbackValue {
                    sequence,
                    publication: origin,
                    snapshot_cutoff: if snapshot {
                        Some(publication)
                    } else {
                        previous_cutoff
                    },
                    value,
                });
                if self
                    .feedback_cutoff
                    .get()
                    .is_some_and(|cutoff| publication_after(origin, cutoff))
                {
                    projection.values[index] = value;
                }
            }
        }
        kResultOk
    }
}

// Replies and feedback share the same named, normalized public-parameter
// surface. Validate the whole batch before the controller publishes any value.
fn normalized_parameter_values(parameters: &[serde_json::Value]) -> Option<[Option<f64>; 23]> {
    let mut values = [None; 23];
    for parameter in parameters {
        let index = parameters::IDS
            .iter()
            .position(|(_, id)| parameter["id"] == *id)?;
        let value = parameter["value"]
            .as_f64()
            .filter(|v| (0.0..=1.0).contains(v))?;
        if values[index].replace(value).is_some() {
            return None;
        }
    }
    Some(values)
}

fn canonical_u64(text: &str) -> Option<u64> {
    text.parse::<u64>()
        .ok()
        .filter(|value| value.to_string() == text)
}
fn publication_stamp(text: &str) -> Option<u64> {
    canonical_u64(text).filter(|stamp| stamp & 1 == 0)
}
fn publication_after(candidate: u64, cutoff: u64) -> bool {
    // Same bounded-lifetime assumption as the shared scalar stamp, including wrap.
    (candidate.wrapping_sub(cutoff) as i64) > 0
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
