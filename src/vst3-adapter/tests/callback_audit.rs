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
        assert_eq!(a.setupProcessing(&mut setup(48_000.0, 1024)), kResultOk);
        assert_eq!(p.setActive(1), kResultOk);
    }
    let audio = a.clone();
    let (ready, observed) = std::sync::mpsc::channel();
    let (resume, continuation) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        use common::parameters::{changes, queue};
        macro_rules! clean {
            ($call:expr,$expected:expr) => {
                assert_eq!(audit.measure(|| unsafe { $call }), ($expected, (0, 0)));
            };
        }
        let notes = events(vec![note_on(5, 36, 1.0)]);
        let invalid = events(vec![note_off(0, 36), note_on(16, 36, 1.0)]);
        let off = events(vec![note_off(0, 36)]);
        let start = events(vec![note_on(0, 36, 1.0)]);
        let curves = changes(vec![queue(0x1003, (0..1023).map(|i| (i, 0.25)).collect())]);
        let bad = changes(vec![
            queue(0x1003, vec![(0, 0.0)]),
            queue(0x1004, vec![(0, f64::NAN)]),
        ]);
        let scalar = changes(vec![queue(0x1003, vec![(0, 0.125)])]);
        let mut block = Block::new(16);
        let mut large = Block::new(1024);
        let mut zero = Block::new(0);
        clean!(audio.setProcessing(1), kResultOk);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        assert_eq!((block.left[5], block.right[5]), (-0.5, -0.5));
        let mut data = block.data();
        data.inputEvents = invalid.as_ptr();
        clean!(audio.process(&mut data), kInvalidArgument);
        assert_eq!(block.left, [0.0; 16]);
        clean!(audio.process(&mut zero.data()), kResultOk);
        let mut data = block.data();
        data.inputEvents = off.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        clean!(audio.setProcessing(0), kResultOk);
        clean!(audio.setProcessing(1), kResultOk);
        clean!(audio.process(&mut block.data()), kResultOk);
        assert_eq!(block.left, [0.0; 16]);
        let mut data = large.data();
        data.inputEvents = start.as_ptr();
        data.inputParameterChanges = curves.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        assert_eq!((large.left[0], large.right[0]), (-0.5, -0.5));
        let mut data = block.data();
        data.inputParameterChanges = bad.as_ptr();
        clean!(audio.process(&mut data), kInvalidArgument);
        assert_eq!(block.left, [0.0; 16]);
        let mut data = zero.data();
        data.inputParameterChanges = scalar.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        clean!(audio.process(std::ptr::null_mut()), kInvalidArgument);

        // Main prepares each replacement while processing remains enabled.
        // Coordination and test-fixture allocation stay outside audit brackets.
        ready.send(()).unwrap();
        continuation
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let restored = events(vec![note_on(5, 60, 1.0)]);
        let mut data = block.data();
        data.inputEvents = invalid.as_ptr();
        clean!(audio.process(&mut data), kInvalidArgument);
        let mut data = block.data();
        data.inputEvents = restored.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        assert_eq!(&block.left[..5], &[0.0; 5]);
        assert_eq!((block.left[5], block.right[5]), (-0.14536858, -0.14536858));
        assert_eq!((block.left[6], block.right[6]), (-0.1444931, -0.1444931));

        ready.send(()).unwrap();
        continuation
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        clean!(audio.setProcessing(0), kResultOk);
        clean!(audio.setProcessing(1), kResultOk);
        clean!(audio.process(&mut block.data()), kResultOk);
        assert_eq!(block.left, [0.0; 16]);
        assert_eq!(block.right, [0.0; 16]);
        let restored = events(vec![note_on(3, 72, 1.0)]);
        let mut data = block.data();
        data.inputEvents = restored.as_ptr();
        clean!(audio.process(&mut data), kResultOk);
        assert_eq!(&block.left[..3], &[0.0; 3]);
        assert_eq!((block.left[3], block.right[3]), (-1.0, -1.0));
        assert_eq!(
            (block.left[4], block.right[4]),
            (-716.0 / 32768.0, -716.0 / 32768.0)
        );
        clean!(audio.setProcessing(0), kResultOk);
        println!(
            "calibrated real DSO malloc/free sentinel: complete valid/error/zero/note-off/reset,1024-input curves,pending replacement adoption/retirement and queued-work cancellation each0 allocations and0 deallocations"
        );
    });
    observed
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
    resume.send(()).unwrap();

    observed
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let mut replacement: serde_json::Value = serde_json::from_slice(legacy).unwrap();
    let document =
        std::fs::read_to_string(bundle.resources.join("instrument/advanced-drum-kit.yaml"))
            .unwrap();
    assert!(document.contains("key_range: [36, 36]"));
    replacement["document"] = document
        .replacen("key_range: [36, 36]", "key_range: [72, 72]", 1)
        .into();
    for parameter in replacement["parameters"].as_array_mut().unwrap() {
        let name = parameter["id"].as_str().unwrap();
        parameter["value"] = if name == "drums.level" {
            2.0
        } else if name.ends_with("level") || name.ends_with("pitch_ratio") {
            1.0
        } else {
            0.0
        }
        .into();
    }
    let (_, input) = common::stream::stream(serde_json::to_vec(&replacement).unwrap(), 4096);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let peer = common::owner::Peer::attach(&pc, 53);
    let state = peer.send(&pc, 1, "getParameterState", serde_json::json!([]));
    let changed = peer.send(
        &pc,
        2,
        "setParameter",
        serde_json::json!(["drums.level", 0.875, state["result"]["generation"]]),
    );
    assert_eq!(changed["status"], "accepted");
    resume.send(()).unwrap();
    worker.join().unwrap();
    let state = peer.send(&pc, 3, "getParameterState", serde_json::json!([]));
    assert_eq!(state["result"]["reset_cancelled"], true);
    let level = state["result"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "drums.level")
        .unwrap();
    assert_eq!(
        level["value"], 0.5,
        "reset cancels queued 3.5 but retains restored physical level2"
    );
    let session = peer.send(&pc, 4, "getSamplerSession", serde_json::json!([]));
    assert_eq!(
        session["result"]["connection"]["history"],
        serde_json::json!({"undoCount":0,"redoCount":0})
    );
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
    }
}
