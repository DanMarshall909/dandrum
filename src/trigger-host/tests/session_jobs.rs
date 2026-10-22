use dandrum_trigger_host::parameters::{
    AutomationEvent, ParameterController, ParameterError, ParameterReply, SessionJobProgress,
};
use dandrum_trigger_host::protocol::{Admission, AdmittedRequest, ProtocolError};
use dandrum_trigger_host::runtime::{ControlEvent, RuntimeAudio, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf, sync::mpsc, time::Duration};

fn package() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Trigger.resources");
    std::fs::create_dir_all(root.join("instrument/assets")).unwrap();
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
        std::fs::copy(repository.join(source), root.join(destination)).unwrap();
    }
    (temporary, root)
}
fn open(root: &std::path::Path) -> (ParameterController, RuntimeAudio) {
    ParameterController::open(
        root,
        TriggerHostConfiguration {
            sample_rate_hz: 48000,
            max_block_frames: 16,
        },
        NonZeroU64::new(9007199254740993).unwrap(),
        4,
        8,
    )
    .unwrap()
}
fn request(
    host: &mut ParameterController,
    attachment: &str,
    id: u64,
    method: &str,
    args: Value,
) -> AdmittedRequest {
    let message=json!({"version":1,"attachmentId":attachment,"requestId":id.to_string(),"method":method,"args":args}).to_string();
    match host.admit(&message).unwrap() {
        Admission::New(request) => request,
        Admission::Duplicate { .. } => panic!("expected new transport request"),
    }
}
fn result(reply: String) -> Value {
    let value: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(value["status"], "accepted");
    value["result"].clone()
}
fn status(host: &mut ParameterController, attachment: &str, transport: u64, job: &str) -> Value {
    let read = request(
        host,
        attachment,
        transport,
        "getInstrumentSessionJobStatus",
        json!([job]),
    );
    result(host.session_job_status(&read).unwrap())
}
fn snapshot(host: &mut ParameterController, attachment: &str, transport: u64) -> Value {
    let read = request(host, attachment, transport, "getSamplerSession", json!([]));
    result(host.sampler_session_reply(&read).unwrap())
}
fn mapping(
    host: &mut ParameterController,
    attachment: &str,
    transport: u64,
    job: &str,
    revision: &str,
    note: u8,
) -> AdmittedRequest {
    request(
        host,
        attachment,
        transport,
        "setInstrumentZoneKeyRange",
        json!([
            "9007199254740993",
            attachment,
            job,
            revision,
            "kit",
            "kick",
            note,
            note
        ]),
    )
}
fn render(audio: &mut RuntimeAudio, note: u8) -> ([f32; 16], [f32; 16]) {
    audio.reset();
    let (mut left, mut right) = ([0.; 16], [0.; 16]);
    let allocation = allocation_counter::measure(|| {
        audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note,
                        velocity: 100,
                    },
                }],
            )
            .unwrap();
    });
    assert_eq!(allocation, allocation_counter::AllocationInfo::default());
    (left, right)
}

#[test]
fn exhausted_next_request_projection_does_not_cancel_the_last_admitted_native_job() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let accepted = host.session().owner_snapshot();
    let parameters = host.parameter_state();
    let last_id = u64::MAX.to_string();
    let command = mapping(&mut host, &editor.attachment_id, 1, &last_id, "0", 60);
    assert_eq!(
        result(host.admit_session_command(&command).unwrap()),
        json!({"status":"accepted","job_id":last_id})
    );
    let read = request(
        &mut host,
        &editor.attachment_id,
        2,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(
        host.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::Session(
            dandrum_instrument_session::SessionError::RevisionExhausted
        )))
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
    assert_eq!(host.parameter_state(), parameters);
    assert!(host.peek_automation().is_none());
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));

    let completion = host.take_session_preparation().unwrap().prepare();
    host.submit_session_completion(completion).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let changed = host.session().owner_snapshot();
    assert_eq!(
        (changed.revision, changed.undo_len, changed.redo_len),
        (1, 1, 0)
    );
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 36), ([0.; 16], [0.; 16]));
    assert_eq!(
        status(&mut host, &editor.attachment_id, 3, &last_id),
        json!({"request_id":last_id,"state":"completed","revision":"1","undoCount":1,"redoCount":0,"error":""})
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert_eq!(
        host.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::Session(
            dandrum_instrument_session::SessionError::RevisionExhausted
        )))
    );
    assert_eq!(host.session().owner_snapshot(), changed);
}

#[test]
fn admitted_mapping_survives_editor_loss_and_recovers_one_native_outcome() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let command = mapping(&mut host, &editor.attachment_id, 90, "1", "0", 60);
    assert_eq!(
        result(host.admit_session_command(&command).unwrap()),
        json!({"status":"accepted","job_id":"1"})
    );
    assert_eq!(
        status(&mut host, &editor.attachment_id, 91, "1")["state"],
        "running"
    );
    assert_eq!(
        snapshot(&mut host, &editor.attachment_id, 92)["connection"]["nextRequestId"],
        "2"
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Pending
    );
    let preparation = host.take_session_preparation().unwrap();
    assert!(host.take_session_preparation().is_none());
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        ready_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        preparation.prepare()
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Pending
    );
    host.detach_editor(&editor.attachment_id).unwrap();
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
    let fresh = host.attach_editor().unwrap();
    assert_eq!(
        status(&mut host, &fresh.attachment_id, 1, "1")["state"],
        "running"
    );
    release_tx.send(()).unwrap();
    host.submit_session_completion(worker.join().unwrap())
        .unwrap();
    assert!(host.take_session_preparation().is_none());
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 36), ([0.; 16], [0.; 16]));
    let accepted = snapshot(&mut host, &fresh.attachment_id, 2);
    assert_eq!(accepted["connection"]["revision"], "1");
    assert_eq!(accepted["connection"]["nextRequestId"], "2");
    assert_eq!(
        accepted["connection"]["history"],
        json!({"undoCount":1,"redoCount":0})
    );
    assert_eq!(accepted["document"]["generation"], 2);
    assert_eq!(host.parameter_state().parameters[2].value, 0.25);
    for id in 3..=135 {
        let retained = status(&mut host, &fresh.attachment_id, id, "1");
        assert_eq!(retained["state"], "completed");
        assert_eq!(retained["revision"], "1");
        assert_eq!(retained["undoCount"], 1);
    }
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert!(host.take_session_preparation().is_none());
    host.detach_editor(&fresh.attachment_id).unwrap();
    let reopened = host.attach_editor().unwrap();
    assert_eq!(
        status(&mut host, &reopened.attachment_id, 1, "1")["revision"],
        "1"
    );
    assert_eq!(
        snapshot(&mut host, &reopened.attachment_id, 2)["connection"]["history"]["undoCount"],
        1
    );
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn prepared_mapping_waits_for_runtime_consumption_and_control_reclamation() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let first = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
    host.admit_session_command(&first).unwrap();
    let prepared = std::thread::spawn({
        let work = host.take_session_preparation().unwrap();
        move || work.prepare()
    })
    .join()
    .unwrap();
    host.submit_session_completion(prepared).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let second = mapping(&mut host, &editor.attachment_id, 2, "2", "1", 62);
    host.admit_session_command(&second).unwrap();
    let prepared = std::thread::spawn({
        let work = host.take_session_preparation().unwrap();
        move || work.prepare()
    })
    .join()
    .unwrap();
    host.submit_session_completion(prepared).unwrap();
    for transport in 3..=5 {
        let progress = host.service_session_jobs().unwrap();
        let current = status(&mut host, &editor.attachment_id, transport, "2");
        assert_eq!(
            current["state"], "running",
            "backpressure cannot consume a valid prepared job: {current}"
        );
        assert_eq!(progress, SessionJobProgress::Pending);
    }
    let before = snapshot(&mut host, &editor.attachment_id, 6);
    assert_eq!(before["connection"]["revision"], "1");
    assert_eq!(before["document"]["generation"], 2);
    assert_eq!(before["connection"]["history"]["undoCount"], 1);
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    // Audio consumed the first candidate; its retired runtime is still owned
    // by reclamation until the next bounded main-thread service call.
    assert_eq!(
        status(&mut host, &editor.attachment_id, 7, "2")["state"],
        "running"
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let after = snapshot(&mut host, &editor.attachment_id, 8);
    assert_eq!(after["connection"]["revision"], "2");
    assert_eq!(after["document"]["generation"], 3);
    assert_eq!(after["connection"]["history"]["undoCount"], 2);
    let (left, right) = render(&mut audio, 62);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert_eq!(
        status(&mut host, &editor.attachment_id, 9, "1")["revision"],
        "1"
    );
}

fn finish_preparation(host: &mut ParameterController) {
    let preparation = host.take_session_preparation().unwrap();
    host.submit_session_completion(
        std::thread::spawn(move || preparation.prepare())
            .join()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
}
#[test]
fn native_undo_survives_reconnect_and_known_outcome_stays_recoverable_after_new_history() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let first = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
    host.admit_session_command(&first).unwrap();
    finish_preparation(&mut host);
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    host.detach_editor(&editor.attachment_id).unwrap();
    let fresh = host.attach_editor().unwrap();
    let undo = request(
        &mut host,
        &fresh.attachment_id,
        1,
        "undoInstrumentSession",
        json!(["9007199254740993", fresh.attachment_id, "2", "1"]),
    );
    assert_eq!(
        result(host.admit_session_command(&undo).unwrap()),
        json!({"status":"accepted","job_id":"2"})
    );
    finish_preparation(&mut host);
    let undone = snapshot(&mut host, &fresh.attachment_id, 2);
    assert_eq!(undone["document"]["generation"], 3);
    assert_eq!(undone["connection"]["revision"], "2");
    assert_eq!(
        undone["connection"]["history"],
        json!({"undoCount":0,"redoCount":1})
    );
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
    let next = mapping(&mut host, &fresh.attachment_id, 3, "3", "2", 61);
    host.admit_session_command(&next).unwrap();
    finish_preparation(&mut host);
    let (left, right) = render(&mut audio, 61);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    let retained = status(&mut host, &fresh.attachment_id, 4, "2");
    assert_eq!(retained["state"], "completed");
    assert_eq!(retained["revision"], "2");
    assert_eq!(retained["undoCount"], 0);
    assert_eq!(retained["redoCount"], 1);
    let current = snapshot(&mut host, &fresh.attachment_id, 5);
    assert_eq!(current["connection"]["revision"], "3");
    assert_eq!(
        current["connection"]["history"],
        json!({"undoCount":1,"redoCount":0})
    );
}

#[test]
fn public_request_fields_cannot_take_another_editor_or_controllers_parameter_authority() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let (mut other, _other_audio) = open(&root);
    let first = host.attach_editor().unwrap();
    let owner = host.attach_editor().unwrap();
    let foreign_editor = other.attach_editor().unwrap();
    let begin = request(
        &mut host,
        &owner.attachment_id,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&begin).unwrap();
    let mut forged = request(
        &mut host,
        &first.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.5, 1]),
    );
    forged.attachment_id = owner.attachment_id.parse().unwrap();
    assert_eq!(
        host.apply(&forged),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    forged.attachment_id = first.attachment_id.parse().unwrap();
    assert_eq!(host.apply(&forged), Err(ParameterError::GestureConflict));
    let mut foreign = request(
        &mut other,
        &foreign_editor.attachment_id,
        1,
        "getParameterState",
        json!([]),
    );
    foreign.attachment = forged.attachment.clone();
    assert_eq!(
        host.apply(&foreign),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(
        host.sampler_session_reply(&foreign),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(
        host.admit_session_command(&foreign),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(
        host.session_job_status(&foreign),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(host.parameter_state().sequence, 0);
    let end = request(
        &mut host,
        &owner.attachment_id,
        2,
        "endGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&end).unwrap();
}

#[test]
fn failed_preparation_reports_only_its_real_error_and_preserves_accepted_state() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let missing = request(
        &mut host,
        &editor.attachment_id,
        1,
        "getInstrumentSessionJobStatus",
        json!(["999"]),
    );
    assert_eq!(
        host.session_job_status(&missing),
        Err(ParameterError::UnknownJob)
    );
    let command = request(
        &mut host,
        &editor.attachment_id,
        2,
        "setInstrumentZoneKeyRange",
        json!([
            "9007199254740993",
            editor.attachment_id,
            "1",
            "0",
            "kit",
            "missing",
            60,
            60
        ]),
    );
    host.admit_session_command(&command).unwrap();
    finish_preparation(&mut host);
    let failed = status(&mut host, &editor.attachment_id, 3, "1");
    assert_eq!(
        failed,
        json!({"request_id":"1","state":"failed","error":"Session(InvalidMapping)"}),
        "failed native outcomes contain no historical snapshot to fabricate"
    );
    let unchanged = snapshot(&mut host, &editor.attachment_id, 4);
    assert_eq!(unchanged["yaml"], editor.snapshot.yaml);
    assert_eq!(unchanged["document"]["generation"], 1);
    assert_eq!(unchanged["connection"]["revision"], "0");
    assert_eq!(unchanged["connection"]["nextRequestId"], "2");
    assert_eq!(
        unchanged["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
    host.detach_editor(&editor.attachment_id).unwrap();
    let fresh = host.attach_editor().unwrap();
    assert_eq!(status(&mut host, &fresh.attachment_id, 1, "1"), failed);
}

#[test]
fn foreign_completion_is_returned_to_its_owner_and_cannot_replace_real_pending_work() {
    let (_package, root) = package();
    let (mut first, mut first_audio) = open(&root);
    let (mut second, mut second_audio) = open(&root);
    let a = first.attach_editor().unwrap();
    let b = second.attach_editor().unwrap();
    let map_a = mapping(&mut first, &a.attachment_id, 1, "1", "0", 60);
    let map_b = mapping(&mut second, &b.attachment_id, 1, "1", "0", 61);
    first.admit_session_command(&map_a).unwrap();
    second.admit_session_command(&map_b).unwrap();
    assert_eq!(
        result(first.admit_session_command(&map_a).unwrap())["job_id"],
        "1"
    );
    let work_a = first.take_session_preparation().unwrap();
    assert!(first.take_session_preparation().is_none());
    let work_b = second.take_session_preparation().unwrap();
    let completion_a = std::thread::spawn(move || work_a.prepare()).join().unwrap();
    let rejected = second.submit_session_completion(completion_a).unwrap_err();
    assert!(format!("{rejected:?}").starts_with("RejectedSessionCompletion"));
    assert_eq!(
        status(&mut first, &a.attachment_id, 2, "1")["state"],
        "running"
    );
    assert_eq!(
        status(&mut second, &b.attachment_id, 2, "1")["state"],
        "running"
    );
    first
        .submit_session_completion(rejected.into_completion())
        .unwrap();
    assert_eq!(
        first.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let (left, right) = render(&mut first_audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    let (left, right) = render(&mut second_audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut second_audio, 61), ([0.; 16], [0.; 16]));
    second
        .submit_session_completion(std::thread::spawn(move || work_b.prepare()).join().unwrap())
        .unwrap();
    assert_eq!(
        second.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let (left, right) = render(&mut second_audio, 61);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut first_audio, 61), ([0.; 16], [0.; 16]));
    let read = request(
        &mut first,
        &a.attachment_id,
        3,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(
        first.admit_session_command(&read),
        Err(ParameterError::UnsupportedCommand)
    );
    assert_eq!(
        first.session_job_status(&read),
        Err(ParameterError::UnsupportedCommand)
    );
}

#[test]
fn native_outcome_window_is_128_and_missing_or_evicted_status_is_never_running() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    for id in 1..=129 {
        let command = request(
            &mut host,
            &editor.attachment_id,
            id,
            "setInstrumentZoneKeyRange",
            json!([
                "9007199254740993",
                editor.attachment_id,
                id.to_string(),
                "0",
                "kit",
                "missing",
                60,
                60
            ]),
        );
        host.admit_session_command(&command).unwrap();
        let work = host.take_session_preparation().unwrap();
        host.submit_session_completion(work.prepare()).unwrap();
        assert_eq!(
            host.service_session_jobs().unwrap(),
            SessionJobProgress::Completed
        );
        if id == 129 {
            assert_eq!(
                result(host.admit_session_command(&command).unwrap())["job_id"],
                "129"
            );
            assert!(host.take_session_preparation().is_none());
        }
    }
    for (transport, id) in [(130, "1"), (131, "999")] {
        let read = request(
            &mut host,
            &editor.attachment_id,
            transport,
            "getInstrumentSessionJobStatus",
            json!([id]),
        );
        assert_eq!(
            host.session_job_status(&read),
            Err(ParameterError::UnknownJob)
        );
    }
    for (transport, id) in [(132, "2"), (133, "129")] {
        assert_eq!(
            status(&mut host, &editor.attachment_id, transport, id)["state"],
            "failed"
        );
    }
    let unchanged = snapshot(&mut host, &editor.attachment_id, 134);
    assert_eq!(unchanged["document"]["generation"], 1);
    assert_eq!(unchanged["connection"]["revision"], "0");
    assert_eq!(unchanged["connection"]["nextRequestId"], "130");
    assert_eq!(unchanged["connection"]["history"]["undoCount"], 0);
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

fn automation(host: &mut ParameterController) -> Vec<AutomationEvent> {
    let mut events = Vec::new();
    while let Some(event) = host.peek_automation() {
        events.push(event);
        assert!(host.acknowledge_automation());
    }
    assert!(!host.acknowledge_automation());
    events
}

#[test]
fn successful_mapping_balances_all_held_gestures_and_retains_scalar_settlement() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let first = host.attach_editor().unwrap();
    let second = host.attach_editor().unwrap();
    let level = host
        .parameters()
        .iter()
        .position(|p| p.id == "drums.level")
        .unwrap();
    let pan = host
        .parameters()
        .iter()
        .position(|p| p.id == "drums.pan")
        .unwrap();
    for (editor, name) in [(&first, "drums.level"), (&second, "drums.pan")] {
        let begin = request(
            &mut host,
            &editor.attachment_id,
            1,
            "beginGesture",
            json!([name, 1]),
        );
        host.apply(&begin).unwrap();
    }
    let value = request(
        &mut host,
        &first.attachment_id,
        2,
        "setParameter",
        json!(["drums.level", 0.5, 1]),
    );
    assert_eq!(
        host.apply(&value),
        Ok(ParameterReply::Accepted {
            generation: 1,
            sequence: 1
        })
    );
    audio.process(&mut [0.; 16], &mut [0.; 16]).unwrap();
    assert_eq!(host.parameter_state().sequence, 1);
    let command = mapping(&mut host, &first.attachment_id, 3, "1", "0", 60);
    host.admit_session_command(&command).unwrap();
    finish_preparation(&mut host);
    let expected = vec![
        AutomationEvent::Begin {
            parameter_index: level,
        },
        AutomationEvent::Begin {
            parameter_index: pan,
        },
        AutomationEvent::Value {
            parameter_index: level,
            value: 2.,
        },
        AutomationEvent::End {
            parameter_index: level,
        },
        AutomationEvent::End {
            parameter_index: pan,
        },
    ];
    assert_eq!(automation(&mut host), expected);
    let state = host.parameter_state();
    assert_eq!(
        (
            state.generation,
            state.sequence,
            state.parameters[level].value
        ),
        (2, 1, 0.5)
    );
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-1., -1.));
    assert_eq!(render(&mut audio, 36), ([0.; 16], [0.; 16]));
    let stale = request(
        &mut host,
        &first.attachment_id,
        4,
        "endGesture",
        json!(["drums.level", 1]),
    );
    assert_eq!(host.apply(&stale), Err(ParameterError::StaleGeneration));
    let next = request(
        &mut host,
        &first.attachment_id,
        5,
        "setParameter",
        json!(["drums.level", 0.25, 2]),
    );
    assert_eq!(
        host.apply(&next),
        Ok(ParameterReply::Accepted {
            generation: 2,
            sequence: 2
        })
    );
    audio.process(&mut [0.; 16], &mut [0.; 16]).unwrap();
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(host.parameter_state().sequence, 2);
    assert_eq!(
        automation(&mut host),
        vec![
            AutomationEvent::Begin {
                parameter_index: level
            },
            AutomationEvent::Value {
                parameter_index: level,
                value: 1.
            },
            AutomationEvent::End {
                parameter_index: level
            }
        ]
    );
    host.detach_editor(&first.attachment_id).unwrap();
    host.detach_editor(&second.attachment_id).unwrap();
    assert!(automation(&mut host).is_empty());
}

#[test]
fn failure_and_defer_preserve_gestures_until_successful_publication() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let editor = host.attach_editor().unwrap();
    let level = host
        .parameters()
        .iter()
        .position(|p| p.id == "drums.level")
        .unwrap();
    let begin = request(
        &mut host,
        &editor.attachment_id,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&begin).unwrap();
    let invalid = request(
        &mut host,
        &editor.attachment_id,
        2,
        "setInstrumentZoneKeyRange",
        json!([
            "9007199254740993",
            editor.attachment_id,
            "1",
            "0",
            "kit",
            "missing",
            60,
            60
        ]),
    );
    host.admit_session_command(&invalid).unwrap();
    finish_preparation(&mut host);
    assert_eq!(
        automation(&mut host),
        vec![AutomationEvent::Begin {
            parameter_index: level
        }]
    );
    let again = request(
        &mut host,
        &editor.attachment_id,
        3,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    assert_eq!(
        host.apply(&again),
        Err(ParameterError::GestureAlreadyActive)
    );
    let end = request(
        &mut host,
        &editor.attachment_id,
        4,
        "endGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&end).unwrap();
    assert_eq!(
        automation(&mut host),
        vec![AutomationEvent::End {
            parameter_index: level
        }]
    );
    let first = mapping(&mut host, &editor.attachment_id, 5, "2", "0", 60);
    host.admit_session_command(&first).unwrap();
    finish_preparation(&mut host);
    let held = request(
        &mut host,
        &editor.attachment_id,
        6,
        "beginGesture",
        json!(["drums.level", 2]),
    );
    host.apply(&held).unwrap();
    let next = mapping(&mut host, &editor.attachment_id, 7, "3", "1", 61);
    host.admit_session_command(&next).unwrap();
    let prepared = host.take_session_preparation().unwrap().prepare();
    host.submit_session_completion(prepared).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Pending
    );
    assert_eq!(
        automation(&mut host),
        vec![AutomationEvent::Begin {
            parameter_index: level
        }]
    );
    let again = request(
        &mut host,
        &editor.attachment_id,
        8,
        "beginGesture",
        json!(["drums.level", 2]),
    );
    assert_eq!(
        host.apply(&again),
        Err(ParameterError::GestureAlreadyActive)
    );
    assert_eq!(host.parameter_state().generation, 2);
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    assert_eq!(
        automation(&mut host),
        vec![AutomationEvent::End {
            parameter_index: level
        }]
    );
    assert_eq!(host.parameter_state().generation, 3);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    host.detach_editor(&editor.attachment_id).unwrap();
    assert!(automation(&mut host).is_empty());
    let (left, right) = render(&mut audio, 61);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

struct ServiceScheduler(mpsc::SyncSender<std::thread::ThreadId>);
impl dandrum_trigger_host::parameters::SessionServiceWake for ServiceScheduler {
    fn request_service(&self) {
        self.0.try_send(std::thread::current().id()).unwrap();
    }
}

#[test]
fn instrument_worker_finishes_after_last_editor_detaches_and_only_main_publishes() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let (mut other, mut other_audio) = open(&root);
    let other_editor = other.attach_editor().unwrap();
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    host.start_session_worker(std::sync::Arc::new(ServiceScheduler(wake_tx)))
        .unwrap();
    let editor = host.attach_editor().unwrap();
    let command = mapping(&mut host, &editor.attachment_id, 90, "1", "0", 60);
    assert_eq!(
        result(host.admit_session_command(&command).unwrap()),
        json!({"status":"accepted","job_id":"1"})
    );
    assert!(
        host.take_session_preparation().is_none(),
        "installed worker exclusively owns admitted preparation"
    );
    host.detach_editor(&editor.attachment_id).unwrap();
    let worker_thread = wake_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_ne!(worker_thread, std::thread::current().id());
    assert_eq!(host.parameter_state().generation, 1);
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
    let fresh = host.attach_editor().unwrap();
    let before = snapshot(&mut host, &fresh.attachment_id, 1);
    assert_eq!(before["connection"]["revision"], "0");
    assert_eq!(before["connection"]["history"]["undoCount"], 0);
    assert_eq!(before["connection"]["nextRequestId"], "2");
    assert_eq!(
        status(&mut host, &fresh.attachment_id, 2, "1")["state"],
        "running"
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 36), ([0.; 16], [0.; 16]));
    let after = snapshot(&mut host, &fresh.attachment_id, 3);
    assert_eq!(after["connection"]["revision"], "1");
    assert_eq!(after["connection"]["history"]["undoCount"], 1);
    assert_eq!(after["connection"]["nextRequestId"], "2");
    assert_eq!(after["document"]["generation"], 2);
    let outcome = status(&mut host, &fresh.attachment_id, 4, "1");
    assert_eq!(outcome["state"], "completed");
    assert_eq!(outcome["revision"], "1");
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert!(matches!(wake_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    let independent = snapshot(&mut other, &other_editor.attachment_id, 1);
    assert_eq!(independent["connection"]["revision"], "0");
    assert_eq!(independent["connection"]["history"]["undoCount"], 0);
    let (left, right) = render(&mut other_audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut other_audio, 60), ([0.; 16], [0.; 16]));
}

#[test]
fn worker_dispatch_is_unique_and_real_audio_swap_reschedules_a_busy_completion() {
    use dandrum_instrument_session::SessionError;
    use dandrum_trigger_host::parameters::SessionServiceWake;
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    let scheduler = std::sync::Arc::new(ServiceScheduler(wake_tx));
    host.start_session_worker(scheduler.clone()).unwrap();
    assert_eq!(
        host.start_session_worker(scheduler.clone()),
        Err(ParameterError::SessionWorkerAlreadyStarted)
    );
    let editor = host.attach_editor().unwrap();
    let first = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
    host.admit_session_command(&first).unwrap();
    wake_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..3 {
        assert_eq!(
            result(host.admit_session_command(&first).unwrap())["job_id"],
            "1"
        );
    }
    let busy = mapping(&mut host, &editor.attachment_id, 2, "2", "0", 62);
    assert_eq!(
        host.admit_session_command(&busy),
        Err(ParameterError::Session(SessionError::Busy))
    );
    assert_eq!(
        snapshot(&mut host, &editor.attachment_id, 3)["connection"]["nextRequestId"],
        "2"
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let second = mapping(&mut host, &editor.attachment_id, 4, "2", "1", 62);
    host.admit_session_command(&second).unwrap();
    host.detach_editor(&editor.attachment_id).unwrap();
    wake_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Pending
    );
    assert_eq!(host.parameter_state().generation, 2);
    assert!(
        matches!(wake_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "Busy must not reschedule itself"
    );
    let (mut left, mut right) = ([0.; 16], [0.; 16]);
    let mut swapped = false;
    let allocations = allocation_counter::measure(|| {
        swapped = audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note: 60,
                        velocity: 100,
                    },
                }],
            )
            .unwrap()
            .swapped;
    });
    assert_eq!(allocations, allocation_counter::AllocationInfo::default());
    assert!(swapped);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    // This models a format facade forwarding the real callback report. It is
    // not calibration of CLAP request_callback/on_main_thread.
    scheduler.request_service();
    assert_eq!(
        wake_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        std::thread::current().id()
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    assert!(matches!(wake_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    let fresh = host.attach_editor().unwrap();
    let accepted = snapshot(&mut host, &fresh.attachment_id, 1);
    assert_eq!(accepted["connection"]["revision"], "2");
    assert_eq!(accepted["connection"]["nextRequestId"], "3");
    assert_eq!(accepted["connection"]["history"]["undoCount"], 2);
    assert_eq!(accepted["document"]["generation"], 3);
    assert_eq!(
        status(&mut host, &fresh.attachment_id, 2, "1")["revision"],
        "1"
    );
    assert_eq!(
        status(&mut host, &fresh.attachment_id, 3, "2")["revision"],
        "2"
    );
    let (left, right) = render(&mut audio, 62);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
}

#[test]
fn worker_start_cannot_steal_an_already_admitted_manual_preparation() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    host.shutdown_session_worker().unwrap();
    let editor = host.attach_editor().unwrap();
    let first = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
    host.admit_session_command(&first).unwrap();
    let (wake_tx, _wake_rx) = mpsc::sync_channel(1);
    assert_eq!(
        host.start_session_worker(std::sync::Arc::new(ServiceScheduler(wake_tx))),
        Err(ParameterError::SessionWorkerBusy)
    );
    assert_eq!(
        snapshot(&mut host, &editor.attachment_id, 2)["connection"]["nextRequestId"],
        "2"
    );
    finish_preparation(&mut host);
    let (left, right) = render(&mut audio, 60);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn idle_worker_shutdown_is_terminal_idempotent_and_preserves_native_cursor_and_audio() {
    let (_package, root) = package();
    let (mut host, mut audio) = open(&root);
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    host.start_session_worker(std::sync::Arc::new(ServiceScheduler(wake_tx)))
        .unwrap();
    let editor = host.attach_editor().unwrap();
    host.shutdown_session_worker().unwrap();
    host.shutdown_session_worker().unwrap();
    assert!(matches!(
        wake_rx.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
    let command = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
    assert_eq!(
        host.admit_session_command(&command),
        Err(ParameterError::SessionWorkerUnavailable)
    );
    assert_eq!(
        host.service_session_jobs(),
        Err(ParameterError::SessionWorkerUnavailable)
    );
    assert!(host.take_session_preparation().is_none());
    let unchanged = snapshot(&mut host, &editor.attachment_id, 2);
    assert_eq!(unchanged["connection"]["nextRequestId"], "1");
    assert_eq!(unchanged["connection"]["revision"], "0");
    assert_eq!(unchanged["connection"]["history"]["undoCount"], 0);
    let (left, right) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
}

// This test-only barrier occupies the public wake after a genuine completion
// was inserted; it does not inject/pause preparation. Production wakes must
// remain nonblocking. The bounded barrier lets the test observe join ordering.
struct OccupiedWake {
    entered: mpsc::SyncSender<std::thread::ThreadId>,
    release: std::sync::Mutex<mpsc::Receiver<()>>,
    destroyed: mpsc::SyncSender<std::thread::ThreadId>,
}
impl dandrum_trigger_host::parameters::SessionServiceWake for OccupiedWake {
    fn request_service(&self) {
        self.entered.try_send(std::thread::current().id()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
}
impl Drop for OccupiedWake {
    fn drop(&mut self) {
        self.destroyed
            .try_send(std::thread::current().id())
            .unwrap();
    }
}

#[test]
fn instrument_shutdown_joins_occupied_completion_wake_before_returning_or_dropping() {
    let (_package, root) = package();
    let (mut independent, mut independent_audio) = open(&root);
    let independent_editor = independent.attach_editor().unwrap();
    for explicit in [true, false] {
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (destroyed_tx, destroyed_rx) = mpsc::sync_channel(1);
        let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(1);
        let (joining_tx, joining_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let root = root.clone();
        let control = std::thread::spawn(move || {
            // This thread constructs the control owner; the !Send law forbids
            // moving an already constructed controller to another thread.
            let (mut host, mut audio) = open(&root);
            host.start_session_worker(std::sync::Arc::new(OccupiedWake {
                entered: entered_tx,
                release: std::sync::Mutex::new(release_rx),
                destroyed: destroyed_tx,
            }))
            .unwrap();
            let editor = host.attach_editor().unwrap();
            let command = mapping(&mut host, &editor.attachment_id, 1, "1", "0", 60);
            host.admit_session_command(&command).unwrap();
            host.detach_editor(&editor.attachment_id).unwrap();
            shutdown_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let (left, right) = render(&mut audio, 36);
            assert_eq!((left[0], right[0]), (-0.5, -0.5));
            assert_eq!(host.parameter_state().generation, 1);
            joining_tx.send(std::thread::current().id()).unwrap();
            if explicit {
                host.shutdown_session_worker().unwrap();
                assert_eq!(
                    host.service_session_jobs(),
                    Err(ParameterError::SessionWorkerUnavailable)
                );
                let fresh = host.attach_editor().unwrap();
                let read = request(
                    &mut host,
                    &fresh.attachment_id,
                    1,
                    "getInstrumentSessionJobStatus",
                    json!(["1"]),
                );
                assert_eq!(
                    host.session_job_status(&read),
                    Err(ParameterError::SessionWorkerUnavailable)
                );
                let unchanged = snapshot(&mut host, &fresh.attachment_id, 2);
                assert_eq!(unchanged["connection"]["revision"], "0");
                assert_eq!(unchanged["connection"]["nextRequestId"], "2");
                assert_eq!(unchanged["connection"]["history"]["undoCount"], 0);
                let (left, right) = render(&mut audio, 36);
                assert_eq!((left[0], right[0]), (-0.5, -0.5));
                assert_eq!(render(&mut audio, 60), ([0.; 16], [0.; 16]));
            }
            drop(host);
            done_tx.send(()).unwrap();
        });
        let worker_thread = entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        shutdown_tx.send(()).unwrap();
        let control_thread = joining_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_ne!(worker_thread, control_thread);
        assert!(
            matches!(
                done_rx.recv_timeout(Duration::from_millis(40)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "shutdown must not return while the owned worker is in its completion wake"
        );
        let (left, right) = render(&mut independent_audio, 36);
        assert_eq!((left[0], right[0]), (-0.5, -0.5));
        release_tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            destroyed_rx.try_recv().unwrap(),
            worker_thread,
            "worker resources are released before control teardown returns"
        );
        control.join().unwrap();
    }
    let unchanged = snapshot(&mut independent, &independent_editor.attachment_id, 1);
    assert_eq!(unchanged["connection"]["revision"], "0");
    assert_eq!(unchanged["connection"]["history"]["undoCount"], 0);
}
