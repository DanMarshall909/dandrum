use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

pub(crate) fn resource_root() -> Option<PathBuf> {
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
    // SAFETY: this private non-interposable function belongs to the executing
    // module, and dladdr receives one writable output. Loader strings remain
    // valid while the host retains the module containing the current call.
    if unsafe {
        libc::dladdr(
            resource_root as *const () as *const libc::c_void,
            info.as_mut_ptr(),
        )
    } == 0
    {
        return None;
    }
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(info.dli_fname) };
    let declared = PathBuf::from(OsStr::from_bytes(name.to_bytes()));
    if !declared.is_absolute() {
        return None;
    }
    let module = declared.canonicalize().ok()?;
    let architecture = module.parent()?;
    let contents = architecture.parent()?;
    if architecture.file_name()? != OsStr::new("x86_64-linux")
        || contents.file_name()? != OsStr::new("Contents")
        || contents.parent()?.extension() != Some(OsStr::new("vst3"))
    {
        return None;
    }
    Some(contents.join("Resources"))
}

/// Rust/GTK thread-local destructors may outlive a host's last dlclose. Promote
/// only this exact loaded object; objects, service references and workers still
/// have ordinary finite lifetimes. No application reference is leaked.
pub(crate) unsafe fn retain_code(handle: *mut libc::c_void) -> bool {
    if handle.is_null() {
        return false;
    }
    let mut declared: *mut libc::c_void = std::ptr::null_mut();
    let mut executing: *mut libc::c_void = std::ptr::null_mut();
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
    // SAFETY: ModuleEntry receives a live dlopen handle. Outputs designate local
    // storage and the private function address belongs to the executing module.
    if unsafe {
        libc::dlinfo(
            handle,
            libc::RTLD_DI_LINKMAP,
            (&mut declared as *mut *mut libc::c_void).cast(),
        )
    } != 0
        || unsafe {
            libc::dladdr1(
                retain_code as *const () as *const libc::c_void,
                info.as_mut_ptr(),
                &mut executing,
                2,
            )
        } == 0
        || declared != executing
        || executing.is_null()
    {
        return false;
    }
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return false;
    }
    // SAFETY: loader-owned NUL-terminated name remains live during ModuleEntry.
    // NOLOAD forbids loading another artifact, and the identity above forbids
    // using a foreign module's handle to promote our code.
    let promoted = unsafe {
        libc::dlopen(
            info.dli_fname,
            libc::RTLD_NOLOAD | libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NODELETE,
        )
    };
    !promoted.is_null() && unsafe { libc::dlclose(promoted) } == 0
}
