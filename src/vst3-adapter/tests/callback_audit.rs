#![cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
mod common;
use common::{audio::*, *};
use vst3::Steinberg::{Vst::*, *};

#[test]
fn complete_callbacks_use_the_real_allocator_sentinel() {
    let temporary = tempfile::tempdir().unwrap();
    let c = temporary.path().join("audit.c");
    let rust = temporary.path().join("probe.rs");
    let preload = temporary.path().join("audit.so");
    let probe = temporary.path().join("probe.so");
    std::fs::write(
        &c,
        include_str!("../../clap-adapter/tests/fixtures/allocator_interpose.c"),
    )
    .unwrap();
    std::fs::write(
        &rust,
        include_str!("../../clap-adapter/tests/fixtures/allocator_probe.rs"),
    )
    .unwrap();
    for (mut command, label) in [
        (std::process::Command::new("cc"), "C sentinel"),
        (
            std::process::Command::new("rustc"),
            "Rust calibration probe",
        ),
    ] {
        if label == "C sentinel" {
            command
                .args([
                    "-shared", "-fPIC", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
                ])
                .arg(&c)
                .arg("-o")
                .arg(&preload);
        } else {
            command
                .args([
                    "--edition=2024",
                    "--crate-type=cdylib",
                    "-Copt-level=1",
                    "-Ccodegen-units=1",
                ])
                .arg(&rust)
                .arg("-o")
                .arg(&probe);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "allocator_audit_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LD_PRELOAD", &preload)
        .env("DANDRUM_ALLOCATOR_AUDIT_PROBE", &probe)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let output = finish_child(output, 20, "callback");
    if let Some(root) = std::env::var_os("DANDRUM_VST3_AUDIT_EVIDENCE_DIR") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        for path in [&c, &rust, &preload, &probe] {
            std::fs::copy(path, root.join(path.file_name().unwrap())).unwrap();
        }
        std::fs::write(root.join("stdout.log"), &output.stdout).unwrap();
        std::fs::write(root.join("stderr.log"), &output.stderr).unwrap();
    }
    assert!(
        output.status.success(),
        "whole-callback allocator audit failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
#[ignore = "only the calibrated parent may launch this exact preload child"]
fn allocator_audit_child() {
    type Begin = unsafe extern "C" fn();
    type End = unsafe extern "C" fn(*mut u64, *mut u64);
    struct Audit {
        begin: Begin,
        end: End,
    }
    impl Audit {
        fn measure<T>(&self, f: impl FnOnce() -> T) -> (T, (u64, u64)) {
            unsafe {
                (self.begin)();
            }
            let result = f();
            let (mut a, mut d) = (0, 0);
            unsafe {
                (self.end)(&mut a, &mut d);
            }
            (result, (a, d))
        }
    }
    let global = libloading::os::unix::Library::this();
    let audit = unsafe {
        Audit {
            begin: *global.get::<Begin>(b"dandrum_audit_begin\0").unwrap(),
            end: *global.get::<End>(b"dandrum_audit_end\0").unwrap(),
        }
    };
    let probe = unsafe {
        libloading::Library::new(std::env::var_os("DANDRUM_ALLOCATOR_AUDIT_PROBE").unwrap())
    }
    .unwrap();
    let allocate = unsafe {
        *probe
            .get::<unsafe extern "C" fn() -> *mut u8>(b"probe_allocate\0")
            .unwrap()
    };
    let free = unsafe {
        *probe
            .get::<unsafe extern "C" fn(*mut u8)>(b"probe_deallocate\0")
            .unwrap()
    };
    let old = unsafe { allocate() };
    let (fresh, (a, d)) = audit.measure(|| unsafe { allocate() });
    assert!(
        a > 0 && d == 0,
        "calibration must catch an actual Rust DSO allocation"
    );
    let (_, (a, d)) = audit.measure(|| unsafe { free(old) });
    assert!(
        a == 0 && d > 0,
        "calibration must catch destruction of preexisting Rust DSO storage"
    );
    unsafe {
        free(fresh);
    }
    assert_eq!(audit.measure(|| ()).1, (0, 0));
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let a = p.cast::<IAudioProcessor>().unwrap();
    unsafe {
        assert_eq!(a.setupProcessing(&mut setup(48_000.0, 16)), kResultOk);
        assert_eq!(p.setActive(1), kResultOk);
    }
    let audio = a.clone();
    std::thread::spawn(move||{
    let notes = events(vec![note_on(5, 36, 1.0)]);
    let invalid = events(vec![note_off(0, 36), note_on(16, 36, 1.0)]);
    let off = events(vec![note_off(0, 36)]);
    let mut block = Block::new(16);
    let mut zero = Block::new(0);

        assert_eq!(audit.measure(||unsafe{audio.setProcessing(1)}),(kResultOk,(0,0)));
        let mut data=block.data();data.inputEvents=notes.as_ptr();assert_eq!(audit.measure(||unsafe{audio.process(&mut data)}),(kResultOk,(0,0)));assert_eq!((block.left[5],block.right[5]),(-0.5,-0.5));
        let mut data=block.data();data.inputEvents=invalid.as_ptr();assert_eq!(audit.measure(||unsafe{audio.process(&mut data)}),(kInvalidArgument,(0,0)));assert_eq!(block.left,[0.0;16]);
        let mut data=zero.data();assert_eq!(audit.measure(||unsafe{audio.process(&mut data)}),(kResultOk,(0,0)));
        let mut data=block.data();data.inputEvents=off.as_ptr();assert_eq!(audit.measure(||unsafe{audio.process(&mut data)}),(kResultOk,(0,0)));
        assert_eq!(audit.measure(||unsafe{audio.setProcessing(0)}),(kResultOk,(0,0)));assert_eq!(audit.measure(||unsafe{audio.setProcessing(1)}),(kResultOk,(0,0)));
        let mut data=block.data();assert_eq!(audit.measure(||unsafe{audio.process(&mut data)}),(kResultOk,(0,0)));assert_eq!(block.left,[0.0;16]);
        assert_eq!(audit.measure(||unsafe{audio.process(std::ptr::null_mut())}),(kInvalidArgument,(0,0)));assert_eq!(audit.measure(||unsafe{audio.setProcessing(0)}),(kResultOk,(0,0)));
        println!("calibrated real DSO malloc/free sentinel: full valid/invalid/zero/note-off/reset callbacks each0 allocations and0 deallocations");
    }).join().unwrap();
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
    }
}
