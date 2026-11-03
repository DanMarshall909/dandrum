//! Separate COM components: one processor sound owner and a metadata projection.
use crate::{CONTROLLER_CLASS_ID, PROCESSOR_CLASS_ID, parameters, platform, utf16};
use dandrum_trigger_host::{
    SampleRate, TriggerHostConfiguration,
    parameters::{ParameterController, PublicParameter, public_parameters_from_resources},
    runtime::RuntimeAudio,
};
use std::{
    cell::{Cell, RefCell},
    ffi::c_char,
    num::NonZeroU64,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread::ThreadId,
};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};

pub(crate) fn create(cid: &TUID) -> Result<ComPtr<FUnknown>, tresult> {
    if *cid != PROCESSOR_CLASS_ID && *cid != CONTROLLER_CLASS_ID {
        return Err(kNoInterface);
    }
    let root = platform::resource_root().ok_or(kResultFalse)?;
    if *cid == PROCESSOR_CLASS_ID {
        Ok(ComWrapper::new(Processor {
            root,
            thread: std::thread::current().id(),
            terminated: Cell::new(false),
            owner: RefCell::new(None),
        })
        .to_com_ptr::<FUnknown>()
        .expect("component includes FUnknown"))
    } else {
        Ok(ComWrapper::new(Controller {
            root,
            thread: std::thread::current().id(),
            terminated: Cell::new(false),
            projection: RefCell::new(None),
        })
        .to_com_ptr::<FUnknown>()
        .expect("controller includes FUnknown"))
    }
}
struct Processor {
    root: PathBuf,
    thread: ThreadId,
    terminated: Cell<bool>,
    // One tuple owns the session/control side and its unique audio endpoint. It
    // stays off audio until the later setup/process boundary transfers the endpoint.
    owner: RefCell<Option<(ParameterController, RuntimeAudio)>>,
}
impl Class for Processor {
    type Interfaces = (IComponent,);
}
impl Processor {
    fn main(&self) -> bool {
        self.thread == std::thread::current().id()
    }
    fn live(&self) -> bool {
        self.main() && self.owner.borrow().is_some()
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
        let Ok((owner, audio)) = ParameterController::open(
            &self.root,
            configuration,
            NonZeroU64::new(id).expect("counter starts nonzero"),
            64,
            1024,
        ) else {
            return kResultFalse;
        };
        if parameters::ordered(owner.parameters()).is_err() {
            return kResultFalse;
        }
        *self.owner.borrow_mut() = Some((owner, audio));
        kResultOk
    }
    unsafe fn terminate(&self) -> tresult {
        if !self.main() {
            return kResultFalse;
        }
        self.owner.borrow_mut().take();
        self.terminated.set(true);
        kResultOk
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
        _media: MediaType,
        _direction: BusDirection,
        _index: i32,
        _active: TBool,
    ) -> tresult {
        kNotImplemented
    }
    unsafe fn setActive(&self, _active: TBool) -> tresult {
        kNotImplemented
    }
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
}
struct Projection {
    parameters: Vec<PublicParameter>,
    values: [f64; 23],
}
struct Controller {
    root: PathBuf,
    thread: ThreadId,
    terminated: Cell<bool>,
    projection: RefCell<Option<Projection>>,
}
impl Class for Controller {
    type Interfaces = (IEditController,);
}
impl Controller {
    fn main(&self) -> bool {
        self.thread == std::thread::current().id()
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
        if !self.main() {
            return kResultFalse;
        }
        self.projection.borrow_mut().take();
        self.terminated.set(true);
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
