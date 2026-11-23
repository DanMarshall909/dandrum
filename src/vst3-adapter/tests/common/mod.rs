#![allow(dead_code)]
pub mod audio;
pub mod message;
#[cfg(feature = "trigger-editor")]
pub mod native;
pub mod owner;
pub mod parameters;
pub mod runloop;
pub mod stream;
use libloading::Library;
use std::{ffi::c_void, marker::PhantomData, mem::ManuallyDrop, ops::Deref, path::PathBuf};
use vst3::{ComPtr, Interface, Steinberg::*};

pub const PROCESSOR: TUID = vst3::uid(0x2AA22A2B, 0xCCBA463A, 0x9029A89C, 0xCCF306A4);
pub const CONTROLLER: TUID = vst3::uid(0x440ABDC9, 0xBABB41E5, 0xB87AE823, 0x6F36C2CC);

pub struct Bundle {
    directory: tempfile::TempDir,
    pub binary: PathBuf,
    pub resources: PathBuf,
}
impl Bundle {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory
            .path()
            .join("installed/DandrumTrigger.vst3/Contents");
        let binary = bundle.join("x86_64-linux/DandrumTrigger.so");
        let resources = bundle.join("Resources");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::create_dir_all(resources.join("instrument/assets")).unwrap();
        let source = std::env::var_os("DANDRUM_VST3_TEST_LIBRARY")
            .expect("set DANDRUM_VST3_TEST_LIBRARY to the freshly built private cdylib");
        // Debug DWARF is ~170MiB per copy and NODELETE keeps each fixture's
        // backing inode until process exit. Keep code, symbols and LLVM maps,
        // stripping only disposable debug sections to respect the2GiB scratch
        // envelope. Normal release candidates are copied byte-for-byte.
        let source = PathBuf::from(source);
        if source.metadata().unwrap().len() > 80 * 1024 * 1024 {
            assert!(
                std::process::Command::new("objcopy")
                    .arg("--strip-debug")
                    .arg(&source)
                    .arg(&binary)
                    .status()
                    .expect("GNU objcopy for bounded debug fixtures")
                    .success()
            );
        } else {
            std::fs::copy(&source, &binary).unwrap();
        }
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::copy(
            repository.join("examples/patches/advanced-drum-kit.yaml"),
            resources.join("instrument/advanced-drum-kit.yaml"),
        )
        .unwrap();
        std::fs::copy(
            repository.join("examples/patches/assets/advanced-drums.wav"),
            resources.join("instrument/assets/advanced-drums.wav"),
        )
        .unwrap();
        Self {
            directory,
            binary,
            resources,
        }
    }
    pub fn load(&self) -> Module<'_> {
        // SAFETY: the explicit task-built, trusted artifact is copied into the
        // owned fixture. Module owns the loader reference through every call.
        let library = unsafe { Library::new(&self.binary) }.unwrap();
        let platform: libloading::os::unix::Library = library.into();
        let handle = platform.into_raw();
        // SAFETY: ownership of this exact live handle was transferred above.
        let library = Library::from(unsafe { libloading::os::unix::Library::from_raw(handle) });
        let entry = unsafe {
            *library
                .get::<unsafe extern "system" fn(*mut c_void) -> bool>(b"ModuleEntry\0")
                .expect("installed VST3 must export ModuleEntry")
        };
        let exit = unsafe {
            *library
                .get::<unsafe extern "system" fn() -> bool>(b"ModuleExit\0")
                .expect("installed VST3 must export ModuleExit")
        };
        assert!(
            !unsafe { entry(std::ptr::null_mut()) },
            "missing loader identity is rejected"
        );
        assert!(
            unsafe { entry(handle) },
            "real module handle must initialize"
        );
        Module {
            library: ManuallyDrop::new(library),
            exit,
            _bundle: PhantomData,
        }
    }
}

pub struct Module<'a> {
    library: ManuallyDrop<Library>,
    exit: unsafe extern "system" fn() -> bool,
    _bundle: PhantomData<&'a Bundle>,
}
impl Module<'_> {
    pub fn factory(&self) -> Handle<'_, IPluginFactory> {
        // SAFETY: the exported signature is the pinned VST3 ABI. The returned
        // reference is owned; Handle borrows the module until it releases it.
        let factory = unsafe {
            *self
                .library
                .get::<unsafe extern "system" fn() -> *mut IPluginFactory>(b"GetPluginFactory\0")
                .expect("installed VST3 must export GetPluginFactory")
        };
        let inner = unsafe { ComPtr::from_raw(factory()) }.expect("factory must be available");
        Handle {
            inner,
            _module: PhantomData,
        }
    }
    pub fn class<T: Interface>(&self, cid: &TUID) -> Handle<'_, T> {
        let factory = self.factory();
        self.class_from(&factory, cid)
    }
    pub fn class_from<T: Interface>(
        &self,
        factory: &ComPtr<IPluginFactory>,
        cid: &TUID,
    ) -> Handle<'_, T> {
        let mut object = std::ptr::null_mut();
        let iid = T::IID.map(|b| b as i8);
        // SAFETY: IDs each contain16 readable bytes; out pointer is writable.
        assert_eq!(
            unsafe { factory.createInstance(cid.as_ptr(), iid.as_ptr(), &mut object) },
            kResultOk
        );
        let inner = unsafe { ComPtr::from_raw(object.cast::<T>()) }.expect("created interface");
        Handle {
            inner,
            _module: PhantomData,
        }
    }
}
impl Drop for Module<'_> {
    fn drop(&mut self) {
        assert!(unsafe { (self.exit)() }, "module must terminate cleanly");
        // SAFETY: every Handle borrow has ended before this unique reference.
        unsafe { ManuallyDrop::drop(&mut self.library) };
    }
}
pub struct Handle<'a, T: Interface> {
    inner: ComPtr<T>,
    _module: PhantomData<&'a Module<'a>>,
}
impl<T: Interface> Deref for Handle<'_, T> {
    type Target = ComPtr<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
pub fn ascii(value: &[std::ffi::c_char]) -> String {
    value
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8 as char)
        .collect()
}
pub fn utf16(value: &[char16]) -> String {
    String::from_utf16(
        &value
            .iter()
            .take_while(|c| **c != 0)
            .copied()
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

/// Bound an owned test child and retain its diagnostic output on every exit.
pub fn finish_child(
    mut child: std::process::Child,
    seconds: u64,
    label: &str,
) -> std::process::Output {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "{label}-return-deadline: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}
