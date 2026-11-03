mod common;
#[path = "../../trigger-host/tests/support/portable_state.rs"]
mod portable_state;
#[path = "../../trigger-host/tests/support/portable_state_edit.rs"]
mod portable_state_edit;
use common::{audio::*, runloop::*, stream::*, *};
use std::rc::Rc;
use vst3::{
    ComWrapper,
    Steinberg::{Vst::*, *},
};
const LEGACY: &[u8] = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
#[test]
fn processor_only_partial_streams_restore_all_named_values_and_portable_signed_sound_fresh() {
    let (saved, original, old_root) = {
        let bundle = Bundle::new();
        let module = bundle.load();
        let clock = Rc::new(LoopState::default());
        let host = ComWrapper::new(RunLoop(clock.clone()))
            .to_com_ptr::<FUnknown>()
            .unwrap();
        let factory = module.factory();
        assert_eq!(
            unsafe {
                factory
                    .cast::<IPluginFactory3>()
                    .unwrap()
                    .setHostContext(host.as_ptr())
            },
            kResultOk
        );
        let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
        assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
        let (_, input) = stream(LEGACY.to_vec(), 7);
        assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
        clock.tick();
        let (output, sink) = stream(vec![], 13);
        assert_eq!(unsafe { p.getState(sink.as_ptr()) }, kResultOk);
        let saved = output.bytes.borrow().clone();
        let metadata = portable_state::metadata(&saved);
        let legacy: serde_json::Value = serde_json::from_slice(LEGACY).unwrap();
        assert_eq!(metadata["parameters"], legacy["parameters"]);
        assert_eq!(metadata["document"], legacy["document"]);
        let original = std::fs::read(
            bundle
                .resources
                .join("instrument/assets/advanced-drums.wav"),
        )
        .unwrap();
        assert!(saved.ends_with(&original));
        assert_eq!(unsafe { p.terminate() }, kResultOk);
        assert_eq!(clock.unregistered.get(), 1);
        (saved, original, bundle.resources.clone())
    };
    assert!(
        !old_root.exists(),
        "source instance and its complete package were destroyed"
    );
    let bundle = Bundle::new();
    let sample = bundle
        .resources
        .join("instrument/assets/advanced-drums.wav");
    let mut decoy = original.clone();
    for word in decoy[44..].chunks_exact_mut(2) {
        word.copy_from_slice(&8192i16.to_le_bytes());
    }
    std::fs::write(&sample, decoy).unwrap();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    std::fs::remove_file(sample).unwrap();
    let (input, source) = stream(saved.clone(), 11);
    let query = (*p).clone();
    *input.on_io.borrow_mut() = Some(Rc::new(move || {
        assert_eq!(unsafe { query.getBusCount(0, 1) }, 1)
    }));
    assert_eq!(unsafe { p.setState(source.as_ptr()) }, kResultOk);
    assert!(input.calls.get() > 100);
    start(&p, &a, 48000.0, 16);
    let old_note = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = old_note.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(block.left, [0.0; 16]);
    let note = events(vec![note_on(5, 60, 1.0)]);
    let mut data = block.data();
    data.inputEvents = note.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(&block.left[..5], &[0.0; 5]);
    assert_eq!((block.left[5], block.right[5]), (-0.14536858, -0.14536858));
    assert_eq!((block.left[6], block.right[6]), (-0.1444931, -0.1444931));
    let (output, sink) = stream(vec![], 17);
    let query = (*p).clone();
    *output.on_io.borrow_mut() = Some(Rc::new(move || {
        assert_eq!(unsafe { query.getBusCount(0, 1) }, 1)
    }));
    assert_eq!(unsafe { p.getState(sink.as_ptr()) }, kResultOk);
    assert_eq!(
        portable_state::metadata(&output.bytes.borrow()),
        portable_state::metadata(&saved)
    );
    assert!(output.bytes.borrow().ends_with(&original));
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

fn save(p: &vst3::ComPtr<IComponent>) -> Vec<u8> {
    let (state, stream) = stream(vec![], 4096);
    assert_eq!(unsafe { p.getState(stream.as_ptr()) }, kResultOk);
    state.bytes.borrow().clone()
}

#[test]
fn invalid_state_and_failed_streams_preserve_nonempty_undo_redo_values_and_signed_cursor() {
    use common::owner::Peer;
    use serde_json::json;
    let bundle = Bundle::new();
    let module = bundle.load();
    let clock = Rc::new(LoopState::default());
    let host = ComWrapper::new(RunLoop(clock.clone()))
        .to_com_ptr::<FUnknown>()
        .unwrap();
    let factory = module.factory();
    assert_eq!(
        unsafe {
            factory
                .cast::<IPluginFactory3>()
                .unwrap()
                .setHostContext(host.as_ptr())
        },
        kResultOk
    );
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let peer = Peer::attach(&pc, 91);
    let b = peer.bootstrap();
    let mut serial = 0;
    for (job, revision, key) in [("1", "0", 60), ("2", "1", 72)] {
        serial += 1;
        let reply = peer.send(
            &pc,
            serial,
            "setInstrumentZoneKeyRange",
            json!([
                b["instanceId"],
                b["attachmentId"],
                job,
                revision,
                "kit",
                "kick",
                key,
                key
            ]),
        );
        assert_eq!(reply["status"], "accepted");
        peer.complete(&pc, &clock, &mut serial, job);
    }
    serial += 1;
    let reply = peer.send(
        &pc,
        serial,
        "undoInstrumentSession",
        json!([b["instanceId"], b["attachmentId"], "3", "2"]),
    );
    assert_eq!(reply["status"], "accepted");
    peer.complete(&pc, &clock, &mut serial, "3");
    serial += 1;
    let before = peer.send(&pc, serial, "getSamplerSession", json!([]));
    assert_eq!(
        before["result"]["connection"]["history"],
        json!({"undoCount":1,"redoCount":1})
    );
    let saved = save(&p);
    let metadata = portable_state::metadata(&saved);
    let a = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &a, 48000.0, 16);
    let notes = events(vec![note_on(0, 60, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(block.left[0], -0.5);
    let mut invalid = vec![
        saved[..1].to_vec(),
        saved[..8].to_vec(),
        saved[..64].to_vec(),
        saved[..saved.len() - 1].to_vec(),
    ];
    let mut checksum = saved.clone();
    *checksum.last_mut().unwrap() ^= 1;
    invalid.push(checksum);
    let mut trailing = saved.clone();
    trailing.push(0);
    invalid.push(trailing);
    invalid.push(portable_state_edit::edit(&saved, |m| {
        m["product"] = "foreign".into()
    }));
    invalid.push(portable_state_edit::edit(&saved, |m| {
        m["parameters"][0]["value"] = 9999.into()
    }));
    invalid.push(portable_state_edit::edit(&saved, |m| {
        m["parameters"][1] = m["parameters"][0].clone();
    }));
    for (case, bytes) in invalid.into_iter().enumerate() {
        let (_, input) = stream(bytes, 11);
        assert_eq!(
            unsafe { p.setState(input.as_ptr()) },
            kResultFalse,
            "invalid state{case}"
        );
    }
    for count in [ReplyCount::Negative, ReplyCount::Excess] {
        let (bad, input) = stream(saved.clone(), 11);
        bad.reply_count.set(count);
        assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultFalse);
        let (bad, output) = stream(vec![], 11);
        bad.reply_count.set(count);
        assert_eq!(unsafe { p.getState(output.as_ptr()) }, kResultFalse);
    }
    let (bad, input) = stream(saved.clone(), 0);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultFalse);
    assert!(bad.calls.get() > 0);
    let (bad, input) = stream(saved.clone(), 11);
    bad.fail.set(true);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultFalse);
    let (_, output) = stream(vec![], 0);
    assert_eq!(unsafe { p.getState(output.as_ptr()) }, kResultFalse);
    let (bad, output) = stream(vec![], 11);
    let weak = Rc::downgrade(&bad);
    *bad.on_io.borrow_mut() = Some(Rc::new(move || {
        let state = weak.upgrade().unwrap();
        if state.calls.get() == 2 {
            state.fail.set(true);
        }
    }));
    assert_eq!(unsafe { p.getState(output.as_ptr()) }, kResultFalse);
    assert_eq!(
        bad.bytes.borrow().len(),
        11,
        "failed foreign output may retain its prefix"
    );
    assert_eq!(
        unsafe { p.setState(std::ptr::null_mut()) },
        kInvalidArgument
    );
    assert_eq!(
        unsafe { p.getState(std::ptr::null_mut()) },
        kInvalidArgument
    );
    serial += 1;
    let after = peer.send(&pc, serial, "getSamplerSession", json!([]));
    assert_eq!(after["result"], before["result"]);
    assert_eq!(portable_state::metadata(&save(&p)), metadata);
    let mut next = Block::new(16);
    assert_eq!(unsafe { a.process(&mut next.data()) }, kResultOk);
    assert_eq!(
        (next.left[0], next.right[0]),
        (11869.0 / 32768.0, 11869.0 / 32768.0)
    );
    serial += 1;
    let reply = peer.send(
        &pc,
        serial,
        "redoInstrumentSession",
        json!([b["instanceId"], b["attachmentId"], "4", "3"]),
    );
    assert_eq!(reply["status"], "accepted");
    let redone = peer.complete(&pc, &clock, &mut serial, "4");
    assert_eq!(redone["result"]["revision"], "4");
    assert_eq!(unsafe { a.process(&mut next.data()) }, kResultOk);
    assert_eq!(next.left, [0.0; 16]);
    let notes = events(vec![note_on(0, 72, 1.0)]);
    let mut data = next.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!((next.left[0], next.right[0]), (-0.5, -0.5));
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn selected_payload_reencodes_and_restores_after_source_instance_and_file_loss() {
    use serde_json::json;
    let (saved, old_root) = {
        let bundle = Bundle::new();
        let module = bundle.load();
        let p = module.class::<IComponent>(&PROCESSOR);
        assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
        let (_, input) = stream(LEGACY.to_vec(), 4096);
        assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
        let mut metadata = portable_state::metadata(&save(&p));
        let mut selected = std::fs::read(
            bundle
                .resources
                .join("instrument/assets/advanced-drums.wav"),
        )
        .unwrap();
        for word in selected[44..].chunks_exact_mut(2) {
            word.copy_from_slice(&(-8192i16).to_le_bytes());
        }
        // Independent SHA-256 of this exact authoritative constant-PCM fixture.
        let hash = "b2dd0a0ed077a99560cef9359f8def1520a15a960f26b72d4255e86e5bb8fb50";
        let path = format!("selected/{hash}.wav");
        metadata["document"] = metadata["document"]
            .as_str()
            .unwrap()
            .replace("path: assets/advanced-drums.wav", &format!("path: {path}"))
            .into();
        metadata["resources"] = json!([{"path":path,"sha256":hash,"encoded_bytes":selected.len()}]);
        let encoded = serde_json::to_vec(&metadata).unwrap();
        let mut framed = b"DTRGSTAT".to_vec();
        framed.extend(3u32.to_le_bytes());
        framed.extend((encoded.len() as u32).to_le_bytes());
        framed.extend((selected.len() as u64).to_le_bytes());
        framed.extend(encoded);
        framed.extend(&selected);
        let original_file = bundle.resources.join("picked.wav");
        std::fs::write(&original_file, &selected).unwrap();
        let (_, input) = stream(framed, 17);
        assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
        std::fs::remove_file(original_file).unwrap();
        let saved = save(&p);
        assert_eq!(portable_state::metadata(&saved), metadata);
        assert!(saved.ends_with(&selected));
        assert_eq!(unsafe { p.terminate() }, kResultOk);
        (saved, bundle.resources.clone())
    };
    assert!(!old_root.exists());
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    std::fs::remove_file(
        bundle
            .resources
            .join("instrument/assets/advanced-drums.wav"),
    )
    .unwrap();
    let (_, input) = stream(saved.clone(), 13);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
    start(&p, &a, 48000.0, 16);
    let notes = events(vec![note_on(3, 60, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(&block.left[..3], &[0.0; 3]);
    assert_eq!(&block.left[3..], &[-0.09375; 13]);
    assert_eq!(block.left, block.right);
    assert_eq!(
        portable_state::metadata(&save(&p)),
        portable_state::metadata(&saved)
    );
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn state_lifecycle_checks_reentrant_termination_and_missing_encoded_factory_input() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let (input, source) = stream(LEGACY.to_vec(), 4096);
    let (output, sink) = stream(vec![], 4096);
    assert_eq!(unsafe { p.setState(source.as_ptr()) }, kNotInitialized);
    assert_eq!(unsafe { p.getState(sink.as_ptr()) }, kNotInitialized);
    assert_eq!((input.calls.get(), output.calls.get()), (0, 0));
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    std::fs::remove_file(
        bundle
            .resources
            .join("instrument/assets/advanced-drums.wav"),
    )
    .unwrap();
    assert_eq!(unsafe { p.getState(sink.as_ptr()) }, kResultFalse);
    assert_eq!(
        output.calls.get(),
        0,
        "snapshot failure cannot call foreign output"
    );
    let a = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &a, 48000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(block.left[0], -0.5);
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert_eq!(unsafe { p.setState(source.as_ptr()) }, kNotInitialized);
    assert_eq!(unsafe { p.getState(sink.as_ptr()) }, kNotInitialized);
    for during_payload in [false, true] {
        let bundle = Bundle::new();
        let module = bundle.load();
        let p = module.class::<IComponent>(&PROCESSOR);
        assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
        let bytes = if during_payload {
            save(&p)
        } else {
            LEGACY.to_vec()
        };
        let threshold = if during_payload {
            24 + u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize
        } else {
            0
        };
        let (input, source) = stream(bytes, 4096);
        let weak = Rc::downgrade(&input);
        let target = (*p).clone();
        let terminated = Rc::new(std::cell::Cell::new(false));
        let mark = terminated.clone();
        *input.on_io.borrow_mut() = Some(Rc::new(move || {
            let input = weak.upgrade().unwrap();
            if input.position.get() >= threshold && !mark.replace(true) {
                assert_eq!(unsafe { target.terminate() }, kResultOk);
            }
        }));
        assert_eq!(
            unsafe { p.setState(source.as_ptr()) },
            kNotInitialized,
            "liveness rechecked after foreign metadata/payload reads"
        );
        assert!(terminated.get());
    }
}
