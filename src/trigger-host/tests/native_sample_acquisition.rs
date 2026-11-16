#![cfg(unix)]
use dandrum_trigger_host::samples::open_native_sample_files;
use std::path::PathBuf;

#[test]
fn native_acquisition_rejects_nonregular_paths_and_reads_exact_os_bytes() {
    use dandrum_trigger_host::samples::{SampleCatalog, SampleError};
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let directory = tempfile::tempdir().unwrap();
    let ordinary = directory.path().join("ordinary.wav");
    let mut encoded = Vec::new();
    dandrum_engine::wav::write_wav_stereo_i16(&mut encoded, 32_000, &[-0.75, 0.25], &[-0.25, 0.5])
        .unwrap();
    std::fs::write(&ordinary, &encoded).unwrap();
    let exact = directory
        .path()
        .join(std::ffi::OsStr::from_bytes(b"selected-\xff.wav"));
    std::fs::write(&exact, &encoded).unwrap();
    let mut catalog = SampleCatalog::new();
    for (path, expected_name) in [
        (&ordinary, "ordinary.wav"),
        (&exact, "selected-\u{fffd}.wav"),
    ] {
        let files =
            open_native_sample_files(vec![path.clone()]).expect("open the exact regular OS path");
        let job = catalog.begin(files).unwrap();
        let selected = catalog.accept(job.prepare()).unwrap().unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, expected_name);
        assert_eq!(selected[0].sample.encoded_bytes(), encoded);
        assert_eq!(selected[0].sample.sample().frame_count(), 2);
        assert_eq!(selected[0].sample.sample().sample_rate_hz(), 32_000);
    }
    let link = directory.path().join("final-symlink.wav");
    symlink(&ordinary, &link).unwrap();
    for path in [link, directory.path().to_owned()] {
        assert!(matches!(
            open_native_sample_files(vec![path]),
            Err(SampleError::InvalidFile)
        ));
    }
    assert!(
        matches!(
            open_native_sample_files(vec![ordinary.clone(), directory.path().to_owned()]),
            Err(SampleError::InvalidFile)
        ),
        "one nonregular member rejects the whole opened batch"
    );
    assert!(matches!(
        open_native_sample_files(vec![directory.path().join("absent"); 17]),
        Err(SampleError::FileCount)
    ));
    assert!(
        open_native_sample_files(vec![ordinary]).is_ok(),
        "rejection does not poison a later valid open"
    );
    assert!(matches!(
        open_native_sample_files(Vec::new()),
        Err(SampleError::FileCount)
    ));
    assert!(matches!(
        open_native_sample_files(vec![PathBuf::from("/")]),
        Err(SampleError::InvalidName)
    ));
    assert!(matches!(
        open_native_sample_files(vec![directory.path().join("absent.wav")]),
        Err(SampleError::InvalidFile)
    ));
    let maximal = vec![directory.path().join("ordinary.wav"); 16];
    assert_eq!(open_native_sample_files(maximal).unwrap().len(), 16);
    // Preserve exact OS bytes for opening, while the existing display-name
    // limit applies after lossy conversion of an otherwise valid filename.
    let long_display = directory
        .path()
        .join(std::ffi::OsStr::from_bytes(&[0xff; 128]));
    std::fs::write(&long_display, encoded).unwrap();
    assert!(matches!(
        open_native_sample_files(vec![long_display]),
        Err(SampleError::InvalidName)
    ));
}

#[test]
fn native_acquisition_rejects_fifo_without_waiting_for_a_writer() {
    use std::{
        os::unix::ffi::OsStrExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    let fifo = directory.path().join("not-a-sample.wav");
    let marker = directory.path().join("fifo-rejected");
    let fifo_c = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: the NUL-terminated owned pathname stays live and names only a
    // fresh FIFO in this test's exclusive temporary directory.
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "native_file_fifo_child",
            "--exact",
            "--ignored",
            "--test-threads=1",
        ])
        .env("DANDRUM_TEST_NATIVE_FIFO", &fifo)
        .env("DANDRUM_TEST_NATIVE_FIFO_RESULT", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            other => {
                // This exact owned child has no descendants. Reap it before
                // reporting timeout/error, even if opening a FIFO blocked.
                let _ = child.kill();
                child.wait().expect("reap the owned FIFO boundary process");
                break Err(format!(
                    "FIFO acquisition failed to finish without a writer: {other:?}"
                ));
            }
        }
    };
    assert!(result.expect("bounded native FIFO rejection").success());
    assert_eq!(
        std::fs::read(&marker).unwrap(),
        b"rejected without writer",
        "a zero-test child exit cannot count as acquisition evidence"
    );
}

#[test]
#[ignore = "only the owned FIFO boundary parent may launch this exact child"]
fn native_file_fifo_child() {
    use dandrum_trigger_host::samples::SampleError;
    use std::os::unix::fs::FileTypeExt;
    let fifo =
        PathBuf::from(std::env::var_os("DANDRUM_TEST_NATIVE_FIFO").expect("owned FIFO parent"));
    let marker = PathBuf::from(
        std::env::var_os("DANDRUM_TEST_NATIVE_FIFO_RESULT").expect("owned FIFO result marker"),
    );
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert!(matches!(
        open_native_sample_files(vec![fifo]),
        Err(SampleError::InvalidFile)
    ));
    std::fs::write(marker, b"rejected without writer").unwrap();
}
