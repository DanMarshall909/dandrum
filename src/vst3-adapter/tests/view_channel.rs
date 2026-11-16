#![cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "trigger-editor"
))]
mod common;
use common::{message, runloop, *};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};

struct Host {
    processor: Rc<ComPtr<IConnectionPoint>>,
    frames: Rc<RefCell<Vec<Value>>>,
    faults: Rc<Faults>,
}
#[derive(Default)]
struct Faults {
    defer_confirm: Cell<bool>,
    refuse_reply: Cell<bool>,
    defer_retired_ack: Cell<bool>,
    reenter_offer: Cell<Option<u64>>,
    defer_automation_ack: Cell<bool>,
    refuse_projection: Cell<bool>,
}
impl Class for Host {
    type Interfaces = (IConnectionPoint,);
}
impl IConnectionPointTrait for Host {
    unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn notify(&self, pointer: *mut IMessage) -> tresult {
        // SAFETY: the adapter retains this standard message for the call.
        let packet = unsafe { message::decode(pointer) };
        assert!(
            self.frames.borrow().len() < 1024,
            "bounded host observation"
        );
        self.frames.borrow_mut().push(packet.clone());
        let route = packet.get("view").filter(|v| !v.is_null());
        if route.is_some()
            && packet["body"]["type"] == "challenge"
            && let Some(id) = self.faults.reenter_offer.take()
        {
            let control = observed(&self.frames, None, "projection");
            assert_eq!(
                send(&self.processor, &offer(&control, id)),
                kResultFalse,
                "a challenge callback cannot replace its active handshake"
            );
        }
        if route.is_some() && packet["body"]["type"] == "reply" && self.faults.refuse_reply.get() {
            return kResultFalse;
        }
        if route.is_some()
            && packet["body"]["type"] == "projection"
            && self.faults.refuse_projection.get()
        {
            return kResultFalse;
        }
        let response = match packet["body"]["type"].as_str() {
            Some("offer") => Some("challenge"),
            Some("challenge") if route.is_some() && !self.faults.defer_confirm.get() => {
                Some("confirm")
            }
            Some("goodbye" | "disconnect") => Some("closed"),
            Some("retired") if !self.faults.defer_retired_ack.get() => Some("retiredAck"),
            Some("automation") if !self.faults.defer_automation_ack.get() => Some("automationAck"),
            _ => None,
        };
        let Some(kind) = response else {
            return kResultOk;
        };
        let mut response = json!({"version":1,"source":host_scope(route.map(|v| v["id"].as_u64().unwrap())),"target":packet["source"],"sequence":0,"body":{"type":kind}});
        if kind == "automationAck" {
            response["body"]["delivery"] = packet["body"]["delivery"].clone();
        }
        if let Some(route) = route {
            response["view"] = json!({"id":route["id"],"parentSource":route["parentTarget"],"parentTarget":route["parentSource"]});
        }
        // SAFETY: the fixture retains the processor and the complete reply.
        unsafe { self.processor.notify(message::packet(&response).as_ptr()) }
    }
}

#[test]
fn abandoned_unconfirmed_view_can_be_replaced_after_its_foreign_callback_returns() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _context) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(
        unsafe { processor.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    let p = processor.cast::<IConnectionPoint>().unwrap();
    assert_eq!(
        send(&p, &offer(&json!({"source":vec![3;16]}), 1)),
        kNotInitialized,
        "a view requires a bound control connection"
    );
    let frames = Rc::new(RefCell::new(Vec::new()));
    let faults = Rc::new(Faults::default());
    faults.defer_confirm.set(true);
    faults.reenter_offer.set(Some(2));
    let host = ComWrapper::new(Host {
        processor: Rc::new(p.clone()),
        frames: frames.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    assert_eq!(unsafe { p.connect(host.as_ptr()) }, kResultOk);
    let control = observed(&frames, None, "projection");
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(
        unsafe { controller.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    assert_eq!(
        send(
            &controller.cast::<IConnectionPoint>().unwrap(),
            &offer(&control, 1)
        ),
        kResultFalse,
        "view packets cannot enter the controller metadata transport"
    );
    assert_eq!(unsafe { controller.terminate() }, kResultOk);
    assert_eq!(send(&p, &offer(&control, 1)), kResultOk);
    assert!(
        !frames
            .borrow()
            .iter()
            .any(|p| p["body"]["type"] == "projection" && p.get("view").is_some())
    );
    let abandoned = observed(&frames, Some(1), "challenge");
    faults.defer_confirm.set(false);
    assert_eq!(
        send(&p, &offer(&control, 2)),
        kResultOk,
        "an abandoned unconfirmed view must not prevent a later attach"
    );
    let current = observed(&frames, Some(2), "projection");
    assert_eq!(
        send(
            &p,
            &packet(
                &control,
                Some((1, &abandoned)),
                0,
                json!({"type":"confirm"})
            )
        ),
        kResultFalse,
        "a late confirmation cannot restore the abandoned view"
    );
    assert_eq!(
        send(&p, &request(&control, Some((2, &current)), 1, 1)),
        kResultOk
    );
    close(&p, &control, 2, &current);
    assert_eq!(send(&p, &request(&control, None, 1, 1)), kResultOk);
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}
fn host_scope(view: Option<u64>) -> [u8; 16] {
    [view.map_or(61, |id| 70 + u8::try_from(id).unwrap()); 16]
}
fn observed(frames: &RefCell<Vec<Value>>, view: Option<u64>, kind: &str) -> Value {
    frames
        .borrow()
        .iter()
        .rev()
        .find(|p| p["body"]["type"] == kind && p.get("view").and_then(|v| v["id"].as_u64()) == view)
        .unwrap_or_else(|| panic!("missing {kind} in projection {view:?}"))
        .clone()
}
fn packet(control: &Value, view: Option<(u64, &Value)>, sequence: u64, body: Value) -> Value {
    let mut value = json!({"version":1,"source":host_scope(view.map(|v| v.0)),"target":view.map_or(&control["source"], |v| &v.1["source"]),"sequence":sequence,"body":body});
    if let Some((id, _)) = view {
        value["view"] =
            json!({"id":id,"parentSource":host_scope(None),"parentTarget":control["source"]});
    }
    value
}
fn request(control: &Value, view: Option<(u64, &Value)>, sequence: u64, id: u64) -> Value {
    command(control, view, sequence, id, "getParameterState", json!([]))
}
fn command(
    control: &Value,
    view: Option<(u64, &Value)>,
    sequence: u64,
    id: u64,
    method: &str,
    args: Value,
) -> Value {
    let projection = view.map_or(control, |v| v.1);
    let bootstrap: Value =
        serde_json::from_str(projection["body"]["frame"].as_str().unwrap()).unwrap();
    packet(
        control,
        view,
        sequence,
        json!({"type":"request","frame":json!({"version":1,"attachmentId":bootstrap["attachmentId"],"requestId":id.to_string(),"method":method,"args":args}).to_string()}),
    )
}

#[test]
fn snapshot_view_rejects_editing_and_closes_without_cancelling_pending_control_gestures() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _context) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(
        unsafe { processor.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let frames = Rc::new(RefCell::new(Vec::new()));
    let faults = Rc::new(Faults::default());
    let host = ComWrapper::new(Host {
        processor: Rc::new(p.clone()),
        frames: frames.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    assert_eq!(unsafe { p.connect(host.as_ptr()) }, kResultOk);
    let control = observed(&frames, None, "projection");
    assert_eq!(send(&p, &offer(&control, 1)), kResultOk);
    let first = observed(&frames, Some(1), "projection");
    assert_eq!(
        send(
            &p,
            &command(
                &control,
                Some((1, &first)),
                1,
                1,
                "beginGesture",
                json!(["drums.level", 1])
            )
        ),
        kNotImplemented,
        "the snapshot-only lane must not admit editing before view-owned automation is integrated"
    );
    assert_eq!(
        send(&p, &request(&control, Some((1, &first)), 1, 1)),
        kResultOk,
        "capability rejection preserves request identity"
    );
    faults.defer_automation_ack.set(true);
    assert_eq!(
        send(
            &p,
            &command(
                &control,
                None,
                1,
                1,
                "beginGesture",
                json!(["drums.level", 1])
            )
        ),
        kResultOk
    );
    clock.tick();
    assert_eq!(
        observed(&frames, None, "automation")["body"]["event"],
        json!({"phase":"begin","parameter":0x1003})
    );
    close(&p, &control, 1, &first);
    assert_eq!(send(&p, &offer(&control, 2)), kResultOk);
    let second = observed(&frames, Some(2), "projection");
    assert_eq!(
        send(&p, &request(&control, Some((2, &second)), 1, 1)),
        kResultOk
    );
    for (sequence, method, args) in [
        (2, "setParameter", json!(["drums.level", 0.625, 1])),
        (3, "endGesture", json!(["drums.level", 1])),
    ] {
        assert_eq!(
            send(
                &p,
                &command(&control, None, sequence, sequence, method, args)
            ),
            kResultOk
        );
    }
    faults.defer_automation_ack.set(false);
    clock.tick();
    for (sequence, method, args) in [
        (4, "beginGesture", json!(["drums.level", 1])),
        (5, "setParameter", json!(["drums.level", 0.25, 1])),
        (6, "endGesture", json!(["drums.level", 1])),
    ] {
        assert_eq!(
            send(
                &p,
                &command(&control, None, sequence, sequence, method, args)
            ),
            kResultOk
        );
    }
    clock.tick();
    let mut events = std::collections::BTreeMap::new();
    for frame in frames
        .borrow()
        .iter()
        .filter(|p| p["body"]["type"] == "automation")
    {
        assert!(frame.get("view").is_none());
        events.insert(
            frame["body"]["delivery"].as_u64().unwrap(),
            frame["body"]["event"].clone(),
        );
    }
    assert_eq!(
        events.into_iter().collect::<Vec<_>>(),
        vec![
            (1, json!({"phase":"begin","parameter":0x1003})),
            (2, json!({"phase":"value","parameter":0x1003,"value":0.625})),
            (3, json!({"phase":"end","parameter":0x1003})),
            (4, json!({"phase":"begin","parameter":0x1003})),
            (5, json!({"phase":"value","parameter":0x1003,"value":0.25})),
            (6, json!({"phase":"end","parameter":0x1003})),
        ]
    );
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}
fn send(p: &ComPtr<IConnectionPoint>, value: &Value) -> tresult {
    // SAFETY: all SDK objects and encoded buffers live through this call.
    unsafe { p.notify(message::packet(value).as_ptr()) }
}
fn offer(control: &Value, id: u64) -> Value {
    let mut value = packet(control, Some((id, control)), 0, json!({"type":"offer"}));
    value["target"] = Value::Null;
    value
}
fn close(p: &ComPtr<IConnectionPoint>, control: &Value, id: u64, projection: &Value) {
    for kind in ["closing", "closed"] {
        assert_eq!(
            send(
                p,
                &packet(control, Some((id, projection)), 0, json!({"type":kind}))
            ),
            kResultOk,
            "view {id} must acknowledge {kind} independently of control automation"
        );
    }
}

#[test]
fn disposable_view_connection_has_independent_shared_identity_and_preserves_control_service() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _context) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(
        unsafe { processor.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let frames = Rc::new(RefCell::new(Vec::new()));
    let faults = Rc::new(Faults::default());
    let host = ComWrapper::new(Host {
        processor: Rc::new(p.clone()),
        frames: frames.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    assert!(
        host.cast::<IEditController>().is_none(),
        "only standard host messages connect the projections"
    );
    assert_eq!(unsafe { p.connect(host.as_ptr()) }, kResultOk);
    let control = observed(&frames, None, "projection");
    let control_bootstrap: Value =
        serde_json::from_str(control["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(send(&p, &request(&control, None, 1, 1)), kResultOk);
    let initial = observed(&frames, None, "reply");
    let initial: Value = serde_json::from_str(initial["body"]["frame"].as_str().unwrap()).unwrap();

    for field in ["id", "parentSource", "parentTarget"] {
        let mut bad = offer(&control, 1);
        bad["view"][field] = if field == "id" {
            json!(0)
        } else {
            json!(vec![9; 16])
        };
        assert_eq!(send(&p, &bad), kResultFalse, "reject forged {field}");
    }
    assert_eq!(
        send(&p, &request(&control, Some((1, &control)), 1, 1)),
        kResultFalse
    );
    faults.refuse_projection.set(true);
    assert_eq!(
        send(&p, &offer(&control, 1)),
        kResultOk,
        "a control-bound disposable view offer must bind over standard interfaces"
    );
    let first = observed(&frames, Some(1), "projection");
    let first_bootstrap: Value =
        serde_json::from_str(first["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(
        first_bootstrap["instanceId"],
        control_bootstrap["instanceId"]
    );
    assert_ne!(
        first_bootstrap["attachmentId"],
        control_bootstrap["attachmentId"]
    );
    assert_eq!(first_bootstrap["nextRequestId"], "1");
    assert_ne!(first["source"], control["source"]);
    clock.tick();
    assert_eq!(
        observed(&frames, Some(1), "projection")["body"],
        first["body"],
        "retrying a refused bootstrap preserves its shared attachment"
    );
    faults.refuse_projection.set(false);
    clock.tick();
    for frame in [
        "{".to_string(),
        json!({"args":[]}).to_string(),
        " ".repeat(512 * 1024 + 1),
    ] {
        let mut invalid = request(&control, Some((1, &first)), 1, 1);
        invalid["body"]["frame"] = json!(frame);
        assert_eq!(
            send(&p, &invalid),
            kInvalidArgument,
            "malformed/oversized snapshot preserves the next request"
        );
    }
    assert_eq!(
        send(&p, &offer(&control, 2)),
        kResultFalse,
        "a live view cannot be replaced"
    );
    for field in ["source", "target"] {
        let mut bad = request(&control, Some((1, &first)), 1, 1);
        bad[field] = json!(vec![9; 16]);
        assert_eq!(send(&p, &bad), kResultFalse, "reject forged {field}");
    }
    let mut wrong_owner = request(&control, Some((1, &first)), 1, 1);
    let mut frame: Value =
        serde_json::from_str(wrong_owner["body"]["frame"].as_str().unwrap()).unwrap();
    frame["attachmentId"] = control_bootstrap["attachmentId"].clone();
    wrong_owner["body"]["frame"] = json!(frame.to_string());
    assert_eq!(send(&p, &wrong_owner), kResultFalse);
    assert_eq!(
        send(&p, &request(&control, Some((1, &first)), 0, 1)),
        kInvalidArgument
    );
    assert_eq!(
        send(
            &p,
            &packet(&control, Some((1, &first)), 0, json!({"type":"closed"}))
        ),
        kResultFalse
    );
    assert_eq!(
        send(
            &p,
            &packet(&control, Some((1, &first)), 0, json!({"type":"retiredAck"}))
        ),
        kResultFalse
    );
    assert_eq!(
        send(
            &p,
            &packet(
                &control,
                Some((1, &first)),
                0,
                json!({"type":"automationAck","delivery":1})
            )
        ),
        kNotImplemented
    );
    clock.tick();
    assert_eq!(
        send(&p, &request(&control, Some((1, &first)), 1, 1)),
        kResultOk
    );
    let reply = observed(&frames, Some(1), "reply");
    let reply: Value = serde_json::from_str(reply["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(reply["result"], initial["result"]);
    assert_eq!(send(&p, &request(&control, None, 2, 2)), kResultOk);

    faults.defer_retired_ack.set(true);
    assert_eq!(
        send(
            &p,
            &packet(&control, Some((1, &first)), 0, json!({"type":"closing"}))
        ),
        kResultOk
    );
    clock.tick();
    assert_eq!(
        send(
            &p,
            &packet(&control, Some((1, &first)), 0, json!({"type":"closed"}))
        ),
        kResultOk
    );
    clock.tick();
    assert_eq!(
        send(
            &p,
            &packet(&control, Some((1, &first)), 0, json!({"type":"closed"}))
        ),
        kResultOk,
        "a repeated close certificate is idempotent while its retirement reply is pending"
    );
    assert_eq!(
        send(&p, &offer(&control, 2)),
        kResultFalse,
        "retain a closing connection until host acknowledgement"
    );
    faults.defer_retired_ack.set(false);
    clock.tick();
    clock.tick();
    assert_eq!(
        send(&p, &request(&control, Some((1, &first)), 2, 2)),
        kResultFalse
    );
    assert_eq!(send(&p, &request(&control, None, 3, 3)), kResultOk);
    assert_eq!(
        clock.handlers.borrow().len(),
        1,
        "the instrument service outlives the view connection"
    );

    assert_eq!(send(&p, &offer(&control, 2)), kResultOk);
    let second = observed(&frames, Some(2), "projection");
    let second_bootstrap: Value =
        serde_json::from_str(second["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(
        second_bootstrap["instanceId"],
        control_bootstrap["instanceId"]
    );
    assert_ne!(
        second_bootstrap["attachmentId"],
        first_bootstrap["attachmentId"]
    );
    assert_eq!(
        send(&p, &request(&control, Some((2, &second)), 1, 1)),
        kResultOk
    );
    assert_eq!(
        send(&p, &request(&control, Some((1, &first)), 2, 2)),
        kResultFalse
    );
    assert_eq!(send(&p, &request(&control, None, 4, 4)), kResultOk);
    faults.refuse_reply.set(true);
    for id in 2..=9 {
        assert_eq!(
            send(&p, &request(&control, Some((2, &second)), id, id)),
            kResultOk
        );
    }
    assert_eq!(
        send(&p, &request(&control, Some((2, &second)), 10, 10)),
        kResultFalse,
        "reply room is reserved before command admission"
    );
    faults.refuse_reply.set(false);
    clock.tick();
    assert_eq!(
        send(&p, &request(&control, Some((2, &second)), 10, 10)),
        kResultOk
    );
    close(&p, &control, 2, &second);
    for id in 3..=22 {
        assert_eq!(send(&p, &offer(&control, id)), kResultOk);
        let projection = observed(&frames, Some(id), "projection");
        assert_eq!(
            send(&p, &request(&control, Some((id, &projection)), 1, 1)),
            kResultOk,
            "closed views release the bounded shared attachment lease"
        );
        close(&p, &control, id, &projection);
    }
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}
