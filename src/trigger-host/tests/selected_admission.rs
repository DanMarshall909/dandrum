use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, SampleRate, TriggerHostConfiguration,
    parameters::{ParameterController, SessionServiceWake},
    protocol::{Admission, AdmittedRequest, EditorConnection},
    runtime::RuntimeAudio,
    samples::{NativeSampleFile, SampleError},
};
use std::{
    num::NonZeroU64,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct Wake(AtomicBool);
impl SessionServiceWake for Wake {
    fn request_service(&self) {
        self.0.store(true, Ordering::Release);
    }
}
struct Fixture {
    controller: ParameterController,
    _audio: RuntimeAudio,
    connection: EditorConnection,
    next_request: u64,
    wake: Arc<Wake>,
    _root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self::with_worker(true)
    }
    fn with_worker(start_worker: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("instrument/assets")).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (source, destination) in [
            (
                "examples/patches/advanced-drum-kit.yaml",
                ADVANCED_DRUM_KIT_DOCUMENT,
            ),
            (
                "examples/patches/assets/advanced-drums.wav",
                ADVANCED_DRUM_KIT_SAMPLE,
            ),
        ] {
            std::fs::copy(repository.join(source), root.path().join(destination)).unwrap();
        }
        let (mut controller, audio) = ParameterController::open(
            root.path(),
            TriggerHostConfiguration {
                sample_rate_hz: SampleRate::new(48_000.0).unwrap(),
                max_block_frames: 16,
            },
            NonZeroU64::new(113).unwrap(),
            64,
            64,
        )
        .unwrap();
        let wake = Arc::new(Wake::default());
        if start_worker {
            controller.start_session_worker(wake.clone()).unwrap();
        }
        let connection = controller.attach_editor().unwrap();
        Self {
            controller,
            _audio: audio,
            connection,
            next_request: 1,
            wake,
            _root: root,
        }
    }
    fn request(&mut self, method: &str, args: serde_json::Value) -> AdmittedRequest {
        let frame = serde_json::json!({"version":1, "attachmentId":self.connection.attachment_id,
            "requestId":self.next_request.to_string(), "method":method, "args":args})
        .to_string();
        self.next_request += 1;
        let Admission::New(request) = self.controller.admit(&frame).unwrap() else {
            panic!("fresh request")
        };
        request
    }
    fn status(&mut self, selection: &str) -> serde_json::Value {
        let request = self.request("getSamplerSelectionStatus", serde_json::json!([selection]));
        serde_json::from_str::<serde_json::Value>(
            &self.controller.sample_selection_reply(&request).unwrap(),
        )
        .unwrap()["result"]
            .clone()
    }
    fn state(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.controller.save_state(&mut bytes).unwrap();
        bytes
    }
}
fn file() -> NativeSampleFile {
    let mut fd = tempfile::tempfile().unwrap();
    dandrum_engine::wav::write_wav_stereo_i16(&mut fd, 32_000, &[-0.75, -0.5], &[-0.25, -0.5])
        .unwrap();
    NativeSampleFile::new("distinct.wav".into(), fd).unwrap()
}

#[test]
fn rejected_file_count_and_decode_preserve_state_and_allow_later_selection() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    assert!(
        fixture
            .controller
            .admit_selected_files(&permit, vec![])
            .is_err()
    );
    assert_eq!(fixture.status("1")["error"], "fileCount");
    assert_eq!(fixture.status("1")["phase"], "failed");
    assert_eq!(fixture.state(), before);
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    let invalid =
        NativeSampleFile::new("malformed.wav".into(), tempfile::tempfile().unwrap()).unwrap();
    fixture
        .controller
        .admit_selected_files(&permit, vec![invalid])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.controller.service_session_jobs().unwrap();
        let status = fixture.status("2");
        if status["phase"] == "failed" {
            assert_eq!(status["error"], "invalidSample");
            break;
        }
        assert_eq!(status["phase"], "preparing");
        assert!(Instant::now() < deadline, "invalid decode must terminate");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(fixture.state(), before);
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    fixture
        .controller
        .admit_selected_files(&permit, vec![file()])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.controller.service_session_jobs().unwrap();
        let status = fixture.status("3");
        if status["phase"] == "ready" {
            break;
        }
        assert_eq!(status["phase"], "preparing");
        assert!(Instant::now() < deadline, "valid retry must terminate");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(fixture.state(), before);
}

#[test]
fn worker_loss_before_file_admission_finishes_the_view_owned_selection() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    fixture.controller.shutdown_session_worker().unwrap();
    assert!(
        fixture
            .controller
            .admit_selected_files(&permit, vec![file()])
            .is_err()
    );
    assert_eq!(
        fixture.status("1")["phase"],
        "failed",
        "closing a chooser after worker loss cannot leave an eternal choosing status"
    );
    assert_eq!(fixture.status("1")["error"], "workerUnavailable");
    assert_eq!(fixture.state(), before);
}

#[test]
fn shutdown_finishes_admitted_selection_without_publishing_a_patch() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    fixture
        .controller
        .admit_selected_files(&permit, vec![file()])
        .unwrap();
    fixture.controller.shutdown_session_worker().unwrap();
    assert_eq!(
        fixture.status("1")["phase"],
        "failed",
        "joined, discarded work must not remain preparing forever"
    );
    assert_eq!(fixture.status("1")["error"], "workerUnavailable");
    assert_eq!(fixture.state(), before);
}

#[test]
fn admitted_selection_survives_view_loss_and_releases_only_its_outcome() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, reply) = fixture.controller.begin_sample_selection(&request).unwrap();
    let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(reply["result"]["phase"], "choosing");
    let id = reply["result"]["selectionId"].as_str().unwrap().to_owned();
    assert!(
        fixture.controller.begin_sample_selection(&request).is_err(),
        "same admitted intent cannot reopen a chooser"
    );
    fixture
        .controller
        .admit_selected_files(&permit, vec![file()])
        .unwrap();
    fixture
        .controller
        .detach_editor(&fixture.connection.attachment_id)
        .unwrap();
    fixture.connection = fixture.controller.attach_editor().unwrap();
    fixture.next_request = 1;
    let deadline = Instant::now() + Duration::from_secs(5);
    let ready = loop {
        fixture.controller.service_session_jobs().unwrap();
        let status = fixture.status(&id);
        if status["phase"] == "ready" {
            break status;
        }
        assert_eq!(status["phase"], "preparing");
        assert!(
            Instant::now() < deadline,
            "actual worker must complete one selection"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(fixture.wake.0.load(Ordering::Acquire));
    assert_eq!(ready["files"][0]["name"], "distinct.wav");
    assert_eq!(ready["files"][0]["sampleRateHz"], 32_000);
    assert_eq!(ready["files"][0]["channelCount"], 2);
    assert_eq!(ready["files"][0]["frameCount"], 2);
    assert_eq!(
        fixture.state(),
        before,
        "selection does not replace the instrument or state"
    );
    fixture.controller.shutdown_session_worker().unwrap();
    assert_eq!(
        fixture.status(&id),
        ready,
        "stopping preparation retains already-ready content"
    );
    let request = fixture.request("releaseSamplerSelection", serde_json::json!([id]));
    let released: serde_json::Value =
        serde_json::from_str(&fixture.controller.sample_selection_reply(&request).unwrap())
            .unwrap();
    assert_eq!(released["result"]["phase"], "released");
    let request = fixture.request("releaseSamplerSelection", serde_json::json!([id]));
    assert!(fixture.controller.sample_selection_reply(&request).is_ok());
    assert_eq!(fixture.status(&id)["phase"], "released");
}

#[test]
fn choosing_is_view_owned_and_stale_or_foreign_callbacks_cannot_admit_files() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    let mut other = Fixture::new();
    let other_request = other.request("chooseSamplerFiles", serde_json::json!([]));
    let (other_permit, _) = other
        .controller
        .begin_sample_selection(&other_request)
        .unwrap();
    assert!(
        other
            .controller
            .admit_selected_files(&permit, vec![file()])
            .is_err()
    );
    assert_eq!(other.status("1")["phase"], "choosing");
    other
        .controller
        .cancel_sample_selection(&other_permit)
        .unwrap();
    fixture
        .controller
        .detach_editor(&fixture.connection.attachment_id)
        .unwrap();
    assert!(
        fixture
            .controller
            .admit_selected_files(&permit, vec![file()])
            .is_err()
    );
    fixture.connection = fixture.controller.attach_editor().unwrap();
    fixture.next_request = 1;
    assert_eq!(fixture.status("1")["phase"], "cancelled");
    assert_eq!(fixture.state(), before);
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (new_permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    fixture
        .controller
        .cancel_sample_selection(&new_permit)
        .unwrap();
    assert_eq!(fixture.status("2")["phase"], "cancelled");
    assert!(
        fixture
            .controller
            .admit_selected_files(&new_permit, vec![file()])
            .is_err()
    );
}

#[test]
fn selection_outcomes_are_bounded_recoverable_and_retired_explicitly() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    for id in 1..=33 {
        let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
        let (permit, reply) = fixture.controller.begin_sample_selection(&request).unwrap();
        let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply["result"]["selectionId"], id.to_string());
        if id == 1 {
            assert_eq!(fixture.status("1")["phase"], "choosing");
            let release = fixture.request("releaseSamplerSelection", serde_json::json!(["1"]));
            let response: serde_json::Value =
                serde_json::from_str(&fixture.controller.sample_selection_reply(&release).unwrap())
                    .unwrap();
            assert_eq!(response["error"]["code"], "busy");
        }
        fixture.controller.cancel_sample_selection(&permit).unwrap();
    }
    let request = fixture.request("getSamplerSession", serde_json::json!([]));
    let response: serde_json::Value =
        serde_json::from_str(&fixture.controller.sampler_session_reply(&request).unwrap()).unwrap();
    let retained = response["result"]["sampleSelections"].as_array().unwrap();
    assert_eq!(retained.len(), 32);
    assert_eq!(retained[0]["selectionId"], "2");
    assert_eq!(retained[31]["selectionId"], "33");
    assert_eq!(fixture.status("33")["phase"], "cancelled");
    let request = fixture.request("getSamplerSelectionStatus", serde_json::json!(["1"]));
    let response: serde_json::Value =
        serde_json::from_str(&fixture.controller.sample_selection_reply(&request).unwrap())
            .unwrap();
    assert_eq!(response["error"]["code"], "unknownSelection");
    assert_eq!(fixture.state(), before);
}

#[test]
fn selections_and_structural_work_share_one_worker_admission() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    let args = serde_json::json!([
        fixture.connection.instance_id,
        fixture.connection.attachment_id,
        "1",
        "0",
        "kit",
        "kick",
        60,
        60
    ]);
    let mapping = fixture.request("setInstrumentZoneKeyRange", args.clone());
    assert!(matches!(
        fixture.controller.admit_session_command(&mapping),
        Err(dandrum_trigger_host::parameters::ParameterError::SessionWorkerBusy)
    ));
    assert_eq!(fixture.state(), before);
    fixture.controller.cancel_sample_selection(&permit).unwrap();
    let mapping = fixture.request("setInstrumentZoneKeyRange", args);
    fixture.controller.admit_session_command(&mapping).unwrap();
    let selection = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    assert!(matches!(
        fixture.controller.begin_sample_selection(&selection),
        Err(dandrum_trigger_host::parameters::ParameterError::SessionWorkerBusy)
    ));
}

#[test]
fn wrong_methods_retired_admission_and_missing_workers_do_not_create_selections() {
    let mut fixture = Fixture::new();
    let request = fixture.request("getParameterState", serde_json::json!([]));
    assert!(fixture.controller.begin_sample_selection(&request).is_err());
    assert!(fixture.controller.sample_selection_reply(&request).is_err());
    let stale = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    fixture
        .controller
        .detach_editor(&fixture.connection.attachment_id)
        .unwrap();
    assert!(fixture.controller.begin_sample_selection(&stale).is_err());
    assert!(fixture.controller.sample_selection_reply(&stale).is_err());
    let mut manual = Fixture::with_worker(false);
    let request = manual.request("chooseSamplerFiles", serde_json::json!([]));
    assert!(matches!(
        manual.controller.begin_sample_selection(&request),
        Err(dandrum_trigger_host::parameters::ParameterError::SessionWorkerUnavailable)
    ));
}

#[test]
fn rejected_native_acquisition_is_bounded_and_a_later_selection_remains_possible() {
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    let busy = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    assert!(fixture.controller.begin_sample_selection(&busy).is_err());
    fixture
        .controller
        .fail_sample_selection(&permit, SampleError::InvalidFile)
        .unwrap();
    assert_eq!(fixture.status("1")["phase"], "failed");
    assert_eq!(fixture.status("1")["error"], "invalidFile");
    assert!(
        fixture.controller.begin_sample_selection(&busy).is_err(),
        "a semantically rejected request cannot execute later"
    );
    assert_eq!(fixture.state(), before);
    let request = fixture.request("chooseSamplerFiles", serde_json::json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&request).unwrap();
    fixture.controller.cancel_sample_selection(&permit).unwrap();
    assert_eq!(fixture.status("2")["phase"], "cancelled");
}
