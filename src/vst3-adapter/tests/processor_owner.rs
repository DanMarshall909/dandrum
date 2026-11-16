#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::{message, owner::Peer, runloop, *};
use serde_json::{Value, json};
use vst3::Steinberg::{Vst::*, *};
#[test]
fn processor_only_service_retains_admitted_edit_and_undo_across_projection_loss() {
    let bundle = Bundle::new();
    let original =
        std::fs::read_to_string(bundle.resources.join("instrument/advanced-drum-kit.yaml"))
            .unwrap();
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let peer = Peer::attach(&pc, 73);
    let bootstrap = peer.bootstrap();
    peer.refuse.set(true);
    let command = peer.wire(
        1,
        1,
        "setInstrumentZoneKeyRange",
        json!([
            bootstrap["instanceId"],
            bootstrap["attachmentId"],
            "41",
            "0",
            "kit",
            "kick",
            60,
            60
        ]),
    );
    let packet = message::packet(&command);
    unsafe {
        assert_eq!(pc.notify(packet.as_ptr()), kResultOk);
    }
    let admitted = peer.reply(1);
    assert_eq!(admitted["status"], "accepted");
    assert_eq!(admitted["result"]["job_id"], "41");
    unsafe {
        assert_eq!(pc.notify(packet.as_ptr()), kResultOk);
    }
    assert_eq!(
        peer.reply(1),
        admitted,
        "lost reply must not be re-admitted as a duplicate command"
    );
    let mut conflict = command.clone();
    let mut frame: Value =
        serde_json::from_str(conflict["body"]["frame"].as_str().unwrap()).unwrap();
    frame["args"][6] = 61.into();
    conflict["body"]["frame"] = frame.to_string().into();
    unsafe {
        assert_eq!(pc.notify(message::packet(&conflict).as_ptr()), kResultFalse);
        assert_eq!(pc.disconnect(peer.interface.as_ptr()), kResultOk);
    }
    // No VST3 controller or native editor has ever been instantiated. The same
    // processor-owned worker and main-thread service retain the admitted job.
    drop(peer);
    clock.tick();
    let fresh = Peer::attach(&pc, 74);
    assert_eq!(fresh.bootstrap()["instanceId"], bootstrap["instanceId"]);
    assert_ne!(fresh.bootstrap()["attachmentId"], bootstrap["attachmentId"]);
    let mut serial = 0;
    let complete = fresh.complete(&pc, &clock, &mut serial, "41");
    assert_eq!(complete["result"]["revision"], "1");
    assert_eq!(complete["result"]["undoCount"], 1);
    serial += 1;
    let session = fresh.send(&pc, serial, "getSamplerSession", json!([]));
    assert_eq!(session["status"], "accepted");
    assert_eq!(session["result"]["connection"]["history"]["undoCount"], 1);
    let kick = &session["result"]["document"]["maps"][0]["zones"][0];
    assert_eq!(kick["keyLow"], 60);
    assert_eq!(kick["keyHigh"], 60);
    assert_ne!(session["result"]["yaml"], original);
    let b = fresh.bootstrap();
    serial += 1;
    let undo = fresh.send(
        &pc,
        serial,
        "undoInstrumentSession",
        json!([b["instanceId"], b["attachmentId"], "42", "1"]),
    );
    assert_eq!(undo["status"], "accepted");
    let restored = fresh.complete(&pc, &clock, &mut serial, "42");
    assert_eq!(restored["result"]["revision"], "2");
    assert_eq!(restored["result"]["undoCount"], 0);
    assert_eq!(restored["result"]["redoCount"], 1);
    serial += 1;
    let session = fresh.send(&pc, serial, "getSamplerSession", json!([]));
    let restored_kick = &session["result"]["document"]["maps"][0]["zones"][0];
    assert_eq!(restored_kick["keyLow"], 36);
    assert_eq!(restored_kick["keyHigh"], 36);
    assert_eq!(
        session["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":1})
    );
    unsafe {
        assert_eq!(pc.disconnect(fresh.interface.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
    assert_eq!(clock.registered.get(), 1);
    assert_eq!(clock.unregistered.get(), 1);
}

#[test]
fn full_reply_storage_rejects_new_commands_before_shared_owner_admission() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let peer = Peer::attach(&pc, 75);
    peer.refuse.set(true);
    for sequence in 1..=8 {
        let packet =
            message::packet(&peer.wire(sequence, sequence, "getParameterState", json!([])));
        unsafe {
            assert_eq!(pc.notify(packet.as_ptr()), kResultOk);
        }
    }
    let blocked = message::packet(&peer.wire(9, 9, "beginGesture", json!(["drums.level", 1])));
    unsafe {
        assert_eq!(pc.notify(blocked.as_ptr()), kResultFalse);
    }
    peer.refuse.set(false);
    let duplicate = message::packet(&peer.wire(8, 8, "getParameterState", json!([])));
    unsafe {
        assert_eq!(pc.notify(duplicate.as_ptr()), kResultOk);
    }
    // End without a corresponding Begin must still reject. If the full-storage
    // call admitted the gesture, this observable result would be accepted.
    let result = peer.send(&pc, 9, "endGesture", json!(["drums.level", 1]));
    assert_eq!(result["status"], "rejected", "{result}");
    let retired = message::packet(&peer.wire(1, 1, "getParameterState", json!([])));
    unsafe {
        assert_eq!(
            pc.notify(retired.as_ptr()),
            kResultFalse,
            "evicted transport identities cannot re-run a request"
        );
        assert_eq!(pc.disconnect(peer.interface.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn invalid_request_batches_do_not_consume_transport_or_musical_identity() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let peer = Peer::attach(&pc, 76);
    let valid = peer.wire(1, 1, "getParameterState", json!([]));
    let mut cases = Vec::new();
    for (sequence, expected) in [(0, kInvalidArgument), (2, kResultFalse)] {
        let mut packet = valid.clone();
        packet["sequence"] = sequence.into();
        cases.push((packet, expected));
    }
    for (frame, expected) in [
        ("broken JSON".to_string(), kInvalidArgument),
        ("x".repeat(524_289), kInvalidArgument),
        ("{}".to_string(), kInvalidArgument),
    ] {
        let mut packet = valid.clone();
        packet["body"]["frame"] = frame.into();
        cases.push((packet, expected));
    }
    for (field, value, expected) in [
        ("attachmentId", json!("999999"), kResultFalse),
        ("requestId", json!("bad"), kInvalidArgument),
    ] {
        let mut packet = valid.clone();
        let mut frame: Value =
            serde_json::from_str(packet["body"]["frame"].as_str().unwrap()).unwrap();
        frame[field] = value;
        packet["body"]["frame"] = frame.to_string().into();
        cases.push((packet, expected));
    }
    for (packet, expected) in cases {
        let packet = message::packet(&packet);
        unsafe {
            assert_eq!(pc.notify(packet.as_ptr()), expected);
        }
    }
    let state = peer.send(&pc, 1, "getParameterState", json!([]));
    assert_eq!(state["status"], "accepted");
    assert_eq!(state["result"]["sequence"], 0);
    let level = state["result"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "drums.level")
        .unwrap();
    assert_eq!(level["value"], 0.25);
    let session = peer.send(&pc, 2, "getSamplerSession", json!([]));
    assert_eq!(session["result"]["connection"]["revision"], "0");
    assert_eq!(
        session["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
    unsafe {
        assert_eq!(pc.disconnect(peer.interface.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn separate_processors_keep_histories_service_and_message_authority_independent() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let first = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let second = module.class_from::<IComponent>(&factory, &PROCESSOR);
    unsafe {
        assert_eq!(first.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(second.initialize(std::ptr::null_mut()), kResultOk);
    }
    let p1 = first.cast::<IConnectionPoint>().unwrap();
    let p2 = second.cast::<IConnectionPoint>().unwrap();
    let a = Peer::attach(&p1, 77);
    let b = Peer::attach(&p2, 78);
    let identity = a.bootstrap();
    assert_ne!(identity["instanceId"], b.bootstrap()["instanceId"]);
    let command = a.wire(
        1,
        1,
        "setInstrumentZoneKeyRange",
        json!([
            identity["instanceId"],
            identity["attachmentId"],
            "41",
            "0",
            "kit",
            "kick",
            60,
            60
        ]),
    );
    let packet = message::packet(&command);
    unsafe {
        assert_eq!(
            p2.notify(packet.as_ptr()),
            kResultFalse,
            "another processor cannot accept a foreign channel"
        );
        assert_eq!(p1.notify(packet.as_ptr()), kResultOk);
    }
    let mut serial = 1;
    let completed = a.complete(&p1, &clock, &mut serial, "41");
    assert_eq!(completed["result"]["revision"], "1");
    let untouched = b.send(&p2, 1, "getSamplerSession", json!([]));
    assert_eq!(untouched["result"]["connection"]["revision"], "0");
    assert_eq!(
        untouched["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
    assert_eq!(
        untouched["result"]["document"]["maps"][0]["zones"][0]["keyLow"],
        36
    );
    unsafe {
        assert_eq!(first.terminate(), kResultOk);
        assert_eq!(second.getBusCount(0, 1), 1);
    }
    assert_eq!(clock.handlers.borrow().len(), 1);
    let still_live = b.send(&p2, 2, "getParameterState", json!([]));
    assert_eq!(still_live["status"], "accepted");
    unsafe {
        assert_eq!(p2.disconnect(b.interface.as_ptr()), kResultOk);
        assert_eq!(second.terminate(), kResultOk);
    }
    assert_eq!(clock.registered.get(), 2);
    assert_eq!(clock.unregistered.get(), 2);
}
