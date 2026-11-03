//! Direct Linux VST3 boundary for Trigger (ADR0001).
//!
//! This opt-in proof registers distinct processor/controller identities. Musical
//! ownership remains in the format-neutral Trigger host, never in this factory.
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
compile_error!(
    "The VST3 feasibility proof currently targets native GNU Linux x86_64; native Windows/macOS milestones remain required."
);
mod audio;
mod components;
mod parameters;
mod platform;
mod stream;
mod transport;
use std::{
    cell::RefCell,
    ffi::{c_char, c_void},
    rc::Rc,
};
use vst3::{
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Linux::IRunLoop, *},
};

pub const PROCESSOR_CLASS_ID: TUID = vst3::uid(0x2AA22A2B, 0xCCBA463A, 0x9029A89C, 0xCCF306A4);
pub const CONTROLLER_CLASS_ID: TUID = vst3::uid(0x440ABDC9, 0xBABB41E5, 0xB87AE823, 0x6F36C2CC);
const VENDOR: &str = "Dandrum";
const URL: &str = "https://github.com/DanMarshall909/dandrum";
const SDK_VERSION: &str = "VST 3.8.0";

fn ascii<const N: usize>(output: &mut [c_char; N], text: &str) {
    output.fill(0);
    for (destination, source) in output.iter_mut().take(N - 1).zip(text.bytes()) {
        *destination = source as c_char;
    }
}
fn utf16<const N: usize>(output: &mut [char16; N], text: &str) {
    output.fill(0);
    for (destination, source) in output.iter_mut().take(N - 1).zip(text.encode_utf16()) {
        *destination = source;
    }
}
fn class(index: i32) -> Option<(TUID, &'static str, &'static str, &'static str)> {
    match index {
        0 => Some((
            PROCESSOR_CLASS_ID,
            "Audio Module Class",
            "Dandrum Trigger",
            "Instrument|Sampler",
        )),
        1 => Some((
            CONTROLLER_CLASS_ID,
            "Component Controller Class",
            "Dandrum Trigger Controller",
            "",
        )),
        _ => None,
    }
}
struct Factory {
    thread: libc::pid_t,
    runloop: RefCell<Option<Rc<ComPtr<IRunLoop>>>>,
}
impl Class for Factory {
    type Interfaces = (IPluginFactory3,);
}
impl IPluginFactoryTrait for Factory {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: the ABI requires one exclusive writable PFactoryInfo. As with
        // every foreign buffer below, inaccessible memory is a host violation.
        let info = unsafe { &mut *info };
        ascii(&mut info.vendor, VENDOR);
        ascii(&mut info.url, URL);
        ascii(&mut info.email, "");
        info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as i32;
        kResultOk
    }
    unsafe fn countClasses(&self) -> i32 {
        2
    }
    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        let Some((cid, category, name, _)) = class(index) else {
            return kInvalidArgument;
        };
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one exclusive writable SDK output, checked non-null above.
        let info = unsafe { &mut *info };
        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as i32;
        ascii(&mut info.category, category);
        ascii(&mut info.name, name);
        kResultOk
    }
    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        object: *mut *mut c_void,
    ) -> tresult {
        if object.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: the host supplies one writable interface output. Always clear
        // it before rejection so an old pointer cannot impersonate an instance.
        unsafe {
            *object = std::ptr::null_mut();
        }
        if cid.is_null() || iid.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: non-null IDs each designate16 readable bytes for this call.
        let cid = unsafe { &*cid.cast::<TUID>() };
        let runloop = { self.runloop.borrow().clone() };
        let instance = match components::create(cid, runloop) {
            Ok(instance) => instance,
            Err(error) => return error,
        };
        let ptr = instance.as_ptr();
        // SAFETY: this live COM reference supports standard queryInterface;
        // it retains the returned interface while our temporary owner drops.
        unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid.cast::<TUID>(), object) }
    }
}
impl IPluginFactory2Trait for Factory {
    unsafe fn getClassInfo2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
        let Some((cid, category, name, subcategories)) = class(index) else {
            return kInvalidArgument;
        };
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one exclusive writable SDK output, checked non-null above.
        let info = unsafe { &mut *info };
        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as i32;
        ascii(&mut info.category, category);
        ascii(&mut info.name, name);
        info.classFlags = 0; // No distributable/remote execution contract.
        ascii(&mut info.subCategories, subcategories);
        ascii(&mut info.vendor, VENDOR);
        ascii(&mut info.version, env!("CARGO_PKG_VERSION"));
        ascii(&mut info.sdkVersion, SDK_VERSION);
        kResultOk
    }
}
impl IPluginFactory3Trait for Factory {
    unsafe fn getClassInfoUnicode(&self, index: i32, info: *mut PClassInfoW) -> tresult {
        let Some((cid, category, name, subcategories)) = class(index) else {
            return kInvalidArgument;
        };
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one exclusive writable SDK output, checked non-null above.
        let info = unsafe { &mut *info };
        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as i32;
        ascii(&mut info.category, category);
        utf16(&mut info.name, name);
        info.classFlags = 0;
        ascii(&mut info.subCategories, subcategories);
        utf16(&mut info.vendor, VENDOR);
        utf16(&mut info.version, env!("CARGO_PKG_VERSION"));
        utf16(&mut info.sdkVersion, SDK_VERSION);
        kResultOk
    }
    unsafe fn setHostContext(&self, context: *mut FUnknown) -> tresult {
        if self.thread != platform::thread_id() {
            return kResultFalse;
        }
        // No factory borrow crosses queryInterface/addRef/release.
        let runloop = unsafe { ComRef::from_raw(context) }
            .and_then(|c| c.cast::<IRunLoop>())
            .map(Rc::new);
        let old = self.runloop.replace(runloop);
        drop(old);
        kResultOk
    }
}

// No process/session globals are initialized or destroyed by these loader hooks.
// Factories and components own their individual references and service lifetimes.
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
unsafe extern "system" fn ModuleEntry(handle: *mut c_void) -> bool {
    // SAFETY: the VST3 loader supplies its live native module handle.
    unsafe { platform::retain_code(handle) }
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "system" fn ModuleExit() -> bool {
    true
}
#[unsafe(no_mangle)]
extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    ComWrapper::new(Factory {
        thread: platform::thread_id(),
        runloop: RefCell::new(None),
    })
    .to_com_ptr::<IPluginFactory>()
    .expect("factory includes the standard base interface")
    .into_raw()
}
