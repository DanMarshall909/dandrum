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
fn native_view_framing_uses_only_its_live_negotiated_profile_without_admitting_work() {
    use dandrum_trigger_host::protocol::{MESSAGE_BYTES_LIMIT, PATCH_MESSAGE_BYTES_LIMIT};
    use serde_json::json;

    let mut fixture = Fixture::with_worker(false);
    let before = fixture.state();
    let attachment = fixture.connection.attachment_id.clone();
    let frame = |version, owner: &str, method: &str, bytes| {
        let mut text = json!({"version":version,"attachmentId":owner,
            "requestId":"2","method":method,"args":[]})
        .to_string();
        text.extend(std::iter::repeat_n(' ', bytes - text.len()));
        text
    };
    let large = frame(2, &attachment, "getSamplerSession", MESSAGE_BYTES_LIMIT + 1);
    assert!(
        fixture
            .controller
            .editor_message_fits_limits(&attachment, &" ".repeat(MESSAGE_BYTES_LIMIT))
    );
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&attachment, &large)
    );
    let negotiation = fixture.request("negotiateEditorProtocol", json!([2]));
    // Receiving the request alone does not activate a larger framing profile.
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&attachment, &large)
    );
    fixture
        .controller
        .negotiate_editor_protocol(&negotiation)
        .unwrap();
    let other = fixture.controller.attach_editor().unwrap();
    for method in ["getSamplerSession", "prepareInstrumentSessionPatch"] {
        let maximum = frame(2, &attachment, method, PATCH_MESSAGE_BYTES_LIMIT);
        assert!(
            fixture
                .controller
                .editor_message_fits_limits(&attachment, &maximum)
        );
        assert!(
            !fixture
                .controller
                .editor_message_fits_limits(&attachment, &(maximum + " "))
        );
    }
    for (version, owner, method) in [
        (1, attachment.as_str(), "getSamplerSession"),
        (3, attachment.as_str(), "getSamplerSession"),
        (2, attachment.as_str(), "getInstrumentSessionJobStatus"),
        (2, other.attachment_id.as_str(), "getSamplerSession"),
        (2, "999", "getSamplerSession"),
    ] {
        assert!(!fixture.controller.editor_message_fits_limits(
            &attachment,
            &frame(version, owner, method, MESSAGE_BYTES_LIMIT + 1)
        ));
    }
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&other.attachment_id, &large)
    );
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&attachment, &" ".repeat(MESSAGE_BYTES_LIMIT + 1))
    );
    // The framing check neither retains a transport request nor advances a
    // native edit cursor. Full admission still accepts this same request ID.
    assert!(matches!(
        fixture.controller.admit(&large),
        Ok(Admission::New(_))
    ));
    assert_eq!(fixture.controller.session().next_request_id().unwrap().0, 1);
    assert_eq!(fixture.state(), before);
    fixture.controller.detach_editor(&attachment).unwrap();
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&attachment, &large)
    );
    let fresh = fixture.controller.attach_editor().unwrap();
    let unnegotiated = frame(
        2,
        &fresh.attachment_id,
        "getSamplerSession",
        MESSAGE_BYTES_LIMIT + 1,
    );
    assert!(
        !fixture
            .controller
            .editor_message_fits_limits(&fresh.attachment_id, &unnegotiated)
    );
    assert_eq!(fixture.state(), before);
}

#[test]
fn repeated_yaml_content_is_rejected_with_bounded_admission_memory_and_preserved_audio() {
    use dandrum_trigger_host::parameters::{ParameterError, SessionJobProgress};
    use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
    use serde_json::json;

    let mut fixture = Fixture::new();
    let original = fixture.controller.session().owner_snapshot().yaml;
    let before = fixture.state();
    let negotiation = fixture.request("negotiateEditorProtocol", json!([2]));
    fixture
        .controller
        .negotiate_editor_protocol(&negotiation)
        .unwrap();
    let request = |fixture: &mut Fixture, document: &str| {
        let frame = json!({"version":2,"attachmentId":fixture.connection.attachment_id,
            "requestId":fixture.next_request.to_string(),"method":"prepareInstrumentSessionPatch",
            "args":["113",fixture.connection.attachment_id,"41","0",document]})
        .to_string();
        fixture.next_request += 1;
        let Admission::New(request) = fixture.controller.admit(&frame).unwrap() else {
            panic!("fresh patch transport")
        };
        request
    };
    let warm = request(&mut fixture, "metadata: { name: true }");
    assert!(fixture.controller.admit_session_command(&warm).is_err());
    // A small source with repeated ordinary YAML content must be rejected
    // before constructing all the expanded owned strings and schema copies.
    let repeated = format!(
        "value: &text '{}'\ncopies: [{}]\n",
        "x".repeat(16_384),
        std::iter::repeat_n("*text", 128)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(repeated.len() < 18_000);
    let candidate = request(&mut fixture, &repeated);
    let mut result = None;
    let allocation = allocation_counter::measure(|| {
        result = Some(fixture.controller.admit_session_command(&candidate));
    });
    assert!(matches!(
        result.unwrap(),
        Err(ParameterError::Session(
            dandrum_instrument_session::SessionError::InvalidDocument(_)
        ))
    ));
    eprintln!(
        "REPEATED_YAML admission peak={} bytes",
        allocation.bytes_max
    );
    assert!(
        allocation.bytes_max < 1_048_576,
        "bounded rejected admission must not allocate the complete repeated tree: {allocation:?}"
    );
    assert_eq!(
        fixture.controller.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert_eq!(fixture.controller.session().next_request_id().unwrap().0, 1);
    assert_eq!(fixture.state(), before);

    // Normal aliases and literal punctuation remain usable in a valid patch.
    let normal = original
        .replace("Advanced Drum Kit", "'Kit & *'")
        .replace("channels: 2", "channels: *stereo")
        .replacen("channels: *stereo", "channels: &stereo 2", 1);
    assert!(normal.contains("channels: *stereo"));
    let candidate = request(&mut fixture, &normal);
    fixture
        .controller
        .admit_session_command(&candidate)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let progress = fixture.controller.service_session_jobs().unwrap();
        if fixture.controller.session().owner_snapshot().revision == 1 {
            break;
        }
        assert_ne!(progress, SessionJobProgress::Idle);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(fixture.controller.session().owner_snapshot().yaml, normal);
    let (mut left, mut right) = ([123.0; 16], [123.0; 16]);
    fixture
        ._audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 127,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
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

#[test]
fn failed_patch_retry_recovers_after_its_selected_content_is_released() {
    use serde_json::{Value, json};
    let mut fixture = Fixture::new();
    let before = fixture.state();
    let choose = fixture.request("chooseSamplerFiles", json!([]));
    let (permit, _) = fixture.controller.begin_sample_selection(&choose).unwrap();
    fixture
        .controller
        .admit_selected_files(&permit, vec![file()])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let selected = loop {
        fixture.controller.service_session_jobs().unwrap();
        let status = fixture.status("1");
        if status["phase"] == "ready" {
            break status;
        }
        assert_eq!(status["phase"], "preparing");
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    };
    let resource = selected["files"][0]["resourceId"].as_str().unwrap();
    // This real 2-frame source cannot satisfy the kit's authored 12,000-frame
    // kick region. Preparation must fail without changing accepted state.
    let document = fixture
        .controller
        .session()
        .owner_snapshot()
        .yaml
        .replace("assets/advanced-drums.wav", resource);
    let negotiation = fixture.request("negotiateEditorProtocol", json!([2]));
    fixture
        .controller
        .negotiate_editor_protocol(&negotiation)
        .unwrap();
    let patch_request = |fixture: &mut Fixture, job: &str, document: &str| {
        let frame = json!({"version":2,"attachmentId":fixture.connection.attachment_id,
            "requestId":fixture.next_request.to_string(),"method":"prepareInstrumentSessionPatch",
            "args":["113",fixture.connection.attachment_id,job,"0",document]})
        .to_string();
        fixture.next_request += 1;
        let Admission::New(request) = fixture.controller.admit(&frame).unwrap() else {
            panic!("fresh transport");
        };
        request
    };
    let request = patch_request(&mut fixture, "41", &document);
    fixture.controller.admit_session_command(&request).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let failed = loop {
        fixture.controller.service_session_jobs().unwrap();
        let query = fixture.request("getInstrumentSessionJobStatus", json!(["41"]));
        let status: Value =
            serde_json::from_str(&fixture.controller.session_job_status(&query).unwrap()).unwrap();
        if status["result"]["state"] == "failed" {
            break status["result"].clone();
        }
        assert_eq!(status["result"]["state"], "running");
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(fixture.state(), before);
    let release = fixture.request("releaseSamplerSelection", json!(["1"]));
    fixture.controller.sample_selection_reply(&release).unwrap();
    drop(selected);
    // A new chooser owns unadmitted selection work, not the session's retained
    // terminal outcomes. Exact retries must remain recoverable while it is open.
    let choose = fixture.request("chooseSamplerFiles", json!([]));
    let (chooser, reply) = fixture.controller.begin_sample_selection(&choose).unwrap();
    let reply: Value = serde_json::from_str(&reply).unwrap();
    let choosing = reply["result"]["selectionId"].as_str().unwrap();
    assert_eq!(fixture.status(choosing)["phase"], "choosing");
    let retry = patch_request(&mut fixture, "41", &document);
    let recovered = fixture.controller.admit_session_command(&retry);
    assert!(
        recovered.is_ok(),
        "exact failed request retry must recover during another chooser without re-acquiring released content: {recovered:?}"
    );
    assert_eq!(
        fixture.controller.service_session_jobs().unwrap(),
        dandrum_trigger_host::parameters::SessionJobProgress::Idle
    );
    let query = fixture.request("getInstrumentSessionJobStatus", json!(["41"]));
    let status: Value =
        serde_json::from_str(&fixture.controller.session_job_status(&query).unwrap()).unwrap();
    assert_eq!(status["result"], failed);
    let conflict = patch_request(&mut fixture, "41", &(document.clone() + "\n"));
    assert!(matches!(
        fixture.controller.admit_session_command(&conflict),
        Err(dandrum_trigger_host::parameters::ParameterError::Session(
            dandrum_instrument_session::SessionError::ConflictingRequest
        ))
    ));
    let busy = patch_request(&mut fixture, "42", &document);
    assert!(matches!(
        fixture.controller.admit_session_command(&busy),
        Err(dandrum_trigger_host::parameters::ParameterError::SessionWorkerBusy)
    ));
    assert_eq!(fixture.status(choosing)["phase"], "choosing");
    assert_eq!(
        fixture.controller.session().next_request_id().unwrap().0,
        42
    );
    assert_eq!(fixture.state(), before);
    fixture
        .controller
        .cancel_sample_selection(&chooser)
        .unwrap();
    let missing = patch_request(&mut fixture, "42", &document);
    assert!(
        fixture.controller.admit_session_command(&missing).is_err(),
        "new work still requires live selected authority"
    );
    assert_eq!(
        fixture.controller.session().next_request_id().unwrap().0,
        42
    );
    assert_eq!(fixture.state(), before);
}

#[test]
fn silent_empty_document_keeps_product_parameter_scopes_values_and_callback_ownership() {
    use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
    use serde_json::{Value, json};
    let mut fixture = Fixture::new();
    let set = fixture.request("setParameter", json!(["drums.level", 0.5, 1]));
    fixture.controller.apply(&set).unwrap();
    let (mut left, mut right) = ([123.; 16], [123.; 16]);
    fixture
        ._audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 127,
                },
            }],
        )
        .unwrap();
    assert_eq!(
        (left[0], right[0]),
        (-1.0, -1.0),
        "known signed nondefault control before Empty"
    );
    let read = fixture.request("getSamplerSession", json!([]));
    let before: Value =
        serde_json::from_str(&fixture.controller.sampler_session_reply(&read).unwrap()).unwrap();
    assert_eq!(
        before["result"]["document"]["parameters"]
            .as_array()
            .unwrap()
            .len(),
        23
    );
    let mut graph: serde_yaml::Value =
        serde_yaml::from_str(&fixture.controller.session().owner_snapshot().yaml).unwrap();
    let mut modules =
        vec![json!({"id":"empty_audio","type":"audio_mixer","static":{"channels":2}})];
    for (index, port) in graph["ports"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        if port["direction"].as_str() == Some("input") {
            let id = format!("empty_control_{index}");
            modules.push(json!({"id":id,"type":"control_mixer","static":{"channels":1}}));
            port["maps_to"] = format!("{id}.inputs").into();
        } else {
            port["maps_from"] = "empty_audio.mix".into();
        }
    }
    graph["modules"] = serde_yaml::to_value(modules).unwrap();
    graph["connections"] = serde_yaml::to_value(Vec::<Value>::new()).unwrap();
    graph
        .as_mapping_mut()
        .unwrap()
        .remove(serde_yaml::Value::String("assets".into()));
    graph["metadata"]["name"] = "Empty".into();
    let document = serde_yaml::to_string(&graph).unwrap();
    let negotiate = fixture.request("negotiateEditorProtocol", json!([2]));
    fixture
        .controller
        .negotiate_editor_protocol(&negotiate)
        .unwrap();
    let frame=json!({"version":2,"attachmentId":fixture.connection.attachment_id,"requestId":fixture.next_request.to_string(),
        "method":"prepareInstrumentSessionPatch","args":["113",fixture.connection.attachment_id,"1","0",document]}).to_string();
    fixture.next_request += 1;
    let Admission::New(request) = fixture.controller.admit(&frame).unwrap() else {
        panic!("new native request")
    };
    fixture.controller.admit_session_command(&request).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture.controller.session().owner_snapshot().revision != 1 {
        fixture.controller.service_session_jobs().unwrap();
        assert!(Instant::now() < deadline, "Empty worker publishes");
        std::thread::sleep(Duration::from_millis(2));
    }
    let allocations = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                allocation_counter::measure(|| {
                    let report = fixture
                        ._audio
                        .process_events(
                            &mut left,
                            &mut right,
                            &[TimedEvent {
                                frame_offset: 0,
                                event: ControlEvent::NoteOn {
                                    note: 36,
                                    velocity: 127,
                                },
                            }],
                        )
                        .unwrap();
                    assert!(report.swapped);
                })
            })
            .join()
            .unwrap()
    });
    assert_eq!(allocations, allocation_counter::AllocationInfo::default());
    assert!(
        left.into_iter().chain(right).all(|sample| sample == 0.0),
        "accepted Empty is literal zero after the real callback"
    );
    let read = fixture.request("getSamplerSession", json!([]));
    let after: Value =
        serde_json::from_str(&fixture.controller.sampler_session_reply(&read).unwrap()).unwrap();
    assert_eq!(
        after["result"]["document"]["parameters"], before["result"]["document"]["parameters"],
        "public descriptors and applied values remain stable in Empty"
    );
    assert!(
        after["result"]["document"]["maps"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        after["result"]["document"]["sources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
