#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vst3::{
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Vst::*, *},
};

#[derive(Default)]
struct Forward {
    target: RefCell<Option<ComPtr<IConnectionPoint>>>,
    frames: RefCell<Vec<serde_json::Value>>,
    refuse: Cell<bool>,
    refuse_type: RefCell<Option<String>>,
    messages: RefCell<Vec<ComPtr<IMessage>>>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
    on_drop: RefCell<Option<Box<dyn FnOnce()>>>,
}
struct Proxy(Rc<Forward>);
impl Drop for Proxy {
    fn drop(&mut self) {
        let callback = self.0.on_drop.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl Class for Proxy {
    type Interfaces = (IConnectionPoint,);
}
impl IConnectionPointTrait for Proxy {
    unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        let message_ref = unsafe { ComRef::from_raw(message) }.unwrap();
        let retained = message_ref.to_com_ptr();
        self.0.messages.borrow_mut().push(retained);
        let frame = unsafe { common::message::decode(message) };
        self.0.frames.borrow_mut().push(frame.clone());
        let callback = self.0.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        let refused_type = self.0.refuse_type.borrow().clone();
        let type_refused = refused_type
            .as_ref()
            .is_some_and(|kind| frame["body"]["type"].as_str() == Some(kind.as_str()));
        if self.0.refuse.get() || type_refused {
            return kResultFalse;
        }
        let target = self.0.target.borrow().clone().unwrap();
        unsafe { target.notify(message) }
    }
}
fn proxy(target: ComPtr<IConnectionPoint>) -> (Rc<Forward>, ComPtr<IConnectionPoint>) {
    let state = Rc::new(Forward::default());
    *state.target.borrow_mut() = Some(target);
    let ptr = ComWrapper::new(Proxy(state.clone()))
        .to_com_ptr::<IConnectionPoint>()
        .unwrap();
    assert!(ptr.cast::<IComponent>().is_none());
    assert!(ptr.cast::<IEditController>().is_none());
    (state, ptr)
}

#[test]
fn separate_components_exchange_projection_through_standard_interface_only_proxies() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p
        .cast::<IConnectionPoint>()
        .expect("processor has standard messaging");
    let cc = c
        .cast::<IConnectionPoint>()
        .expect("controller has standard messaging");
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultFalse);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultFalse);
    }
    let frames = to_c.frames.borrow();
    let projection = frames
        .iter()
        .find(|f| f["body"]["type"] == "projection")
        .expect("processor sends real projection inline");
    let bootstrap: serde_json::Value =
        serde_json::from_str(projection["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(bootstrap["protocolVersion"], 1);
    assert!(!bootstrap["instanceId"].as_str().unwrap().is_empty());
    assert!(!bootstrap["attachmentId"].as_str().unwrap().is_empty());
    assert_eq!(bootstrap["nextRequestId"], "1");
    drop(frames);
    assert!(
        to_p.frames
            .borrow()
            .iter()
            .any(|f| f["body"]["type"] == "confirm")
    );
    unsafe {
        assert_eq!(
            pc.disconnect(proxy_p.as_ptr()),
            kInvalidArgument,
            "foreign peer cannot revoke attachment"
        );
        assert_eq!(pc.disconnect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.disconnect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn reconnect_uses_fresh_scopes_and_rejects_stale_or_foreign_packets() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
    }
    let index = to_c
        .frames
        .borrow()
        .iter()
        .position(|v| v["body"]["type"] == "projection")
        .unwrap();
    let old_projection = to_c.messages.borrow()[index].clone();
    let old_frame = to_c.frames.borrow()[index].clone();
    unsafe {
        assert_eq!(pc.disconnect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.disconnect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(
            cc.connect(proxy_p.as_ptr()),
            kResultOk,
            "reverse connect order"
        );
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
    }
    let frames = to_c.frames.borrow();
    let new_frame = frames
        .iter()
        .rev()
        .find(|v| v["body"]["type"] == "projection")
        .unwrap();
    assert_ne!(old_frame["source"], new_frame["source"]);
    assert_ne!(old_frame["target"], new_frame["target"]);
    let old_bootstrap: serde_json::Value =
        serde_json::from_str(old_frame["body"]["frame"].as_str().unwrap()).unwrap();
    let new_bootstrap: serde_json::Value =
        serde_json::from_str(new_frame["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(old_bootstrap["instanceId"], new_bootstrap["instanceId"]);
    assert_ne!(old_bootstrap["attachmentId"], new_bootstrap["attachmentId"]);
    drop(frames);
    unsafe {
        assert_eq!(cc.notify(old_projection.as_ptr()), kResultFalse);
        let other = module.class::<IEditController>(&CONTROLLER);
        assert_eq!(other.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(
            other
                .cast::<IConnectionPoint>()
                .unwrap()
                .notify(old_projection.as_ptr()),
            kNotInitialized
        );
        assert_eq!(other.terminate(), kResultOk);
        assert_eq!(pc.disconnect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.disconnect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn inline_disconnect_and_reconnect_does_not_acknowledge_the_new_queue_head() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    let reentrant_pc = pc.clone();
    let reentrant_proxy = proxy_c.clone();
    *to_c.callback.borrow_mut() = Some(Box::new(move || unsafe {
        assert_eq!(reentrant_pc.disconnect(reentrant_proxy.as_ptr()), kResultOk);
        assert_eq!(reentrant_pc.connect(reentrant_proxy.as_ptr()), kResultOk);
    }));
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
    }
    assert!(
        to_c.frames
            .borrow()
            .iter()
            .any(|v| v["body"]["type"] == "projection")
    );
    unsafe {
        assert_eq!(pc.disconnect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.disconnect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn refused_projection_send_retains_the_same_attachment_and_retries_once_accepted() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    *to_c.refuse_type.borrow_mut() = Some("projection".into());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
    }
    let index = to_p
        .frames
        .borrow()
        .iter()
        .position(|v| v["body"]["type"] == "confirm")
        .unwrap();
    let confirm = to_p.messages.borrow()[index].clone();
    let original = to_c
        .frames
        .borrow()
        .iter()
        .find(|v| v["body"]["type"] == "projection")
        .unwrap()
        .clone();
    to_c.refuse_type.borrow_mut().take();
    unsafe {
        assert_eq!(pc.notify(confirm.as_ptr()), kResultOk);
    }
    let frames = to_c.frames.borrow();
    let projections: Vec<_> = frames
        .iter()
        .filter(|v| v["body"]["type"] == "projection")
        .collect();
    assert!(projections.len() >= 2);
    assert!(
        projections.iter().all(|packet| **packet == original),
        "failed delivery must retain the original owner attachment"
    );
    drop(frames);
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    drop(proxy_p);
    drop(proxy_c);
    assert_eq!(Rc::strong_count(&to_p), 1);
    assert_eq!(Rc::strong_count(&to_c), 1);
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn bounded_full_handshake_queue_rejects_input_and_recovers_after_delivery_resumes() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    to_c.refuse.set(true);
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
    }
    let offer = to_p.messages.borrow()[0].clone();
    // The retained offer + initial challenge occupy2 of the declared8 slots.
    for _ in 0..6 {
        unsafe {
            assert_eq!(pc.notify(offer.as_ptr()), kResultOk);
        }
    }
    unsafe {
        assert_eq!(pc.notify(offer.as_ptr()), kResultFalse);
    }
    assert!(
        !to_c
            .frames
            .borrow()
            .iter()
            .any(|v| v["body"]["type"] == "projection")
    );
    to_c.refuse.set(false);
    unsafe {
        // The rejecting call must still permit bounded progress of older work.
        assert_eq!(pc.notify(offer.as_ptr()), kResultFalse);
        let _ = pc.notify(offer.as_ptr());
    }
    assert!(
        to_c.frames
            .borrow()
            .iter()
            .any(|v| v["body"]["type"] == "projection"),
        "a full queue must not permanently prevent retries"
    );
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn termination_rejects_reentrant_connect_from_foreign_reference_release() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (old, old_peer) = proxy(cc.clone());
    let (new, new_peer) = proxy(cc.clone());
    unsafe {
        assert_eq!(pc.connect(old_peer.as_ptr()), kResultOk);
    }
    let result = Rc::new(Cell::new(i32::MIN));
    let observed = result.clone();
    let reentrant = pc.clone();
    *old.on_drop.borrow_mut() = Some(Box::new(move || {
        observed.set(unsafe { reentrant.connect(new_peer.as_ptr()) });
    }));
    drop(old_peer);
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
    }
    assert_eq!(
        result.get(),
        kNotInitialized,
        "termination must revoke admission before calling foreign release"
    );
    unsafe {
        assert_eq!(c.terminate(), kResultOk);
    }
    old.target.borrow_mut().take();
    new.target.borrow_mut().take();
}

#[test]
fn malformed_foreign_messages_preserve_the_connection_and_controller_projection() {
    use common::message::{self, Blob};
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    unsafe {
        assert_eq!(pc.connect(std::ptr::null_mut()), kInvalidArgument);
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(c.setParamNormalized(0x1003, 0.375), kResultOk);
        assert_eq!(pc.notify(std::ptr::null_mut()), kInvalidArgument);
    }
    let projection = to_c
        .frames
        .borrow()
        .iter()
        .find(|v| v["body"]["type"] == "projection")
        .unwrap()
        .clone();
    let valid = serde_json::json!({"version":1,"source":projection["target"],"target":projection["source"],"sequence":0,"body":{"type":"confirm"}});
    for (field, value, expected) in [
        ("version", serde_json::json!(2), kInvalidArgument),
        ("source", serde_json::json!(vec![1; 16]), kResultFalse),
        ("target", serde_json::json!(vec![2; 16]), kResultFalse),
        ("target", serde_json::Value::Null, kResultFalse),
        ("sequence", serde_json::json!(1), kResultFalse),
        ("body", serde_json::json!({"type":"offer"}), kResultFalse),
        (
            "body",
            serde_json::json!({"type":"unknown"}),
            kInvalidArgument,
        ),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        let bad = message::packet(&bad);
        unsafe {
            assert_eq!(pc.notify(bad.as_ptr()), expected, "{field}");
        }
    }
    for fault in [
        "json",
        "id",
        "nullId",
        "noAttributes",
        "failedBinary",
        "nullBinary",
        "oversizedBinary",
    ] {
        let mut input = message::raw(b"{}".to_vec());
        match fault {
            "json" => input = message::raw(b"not json".to_vec()),
            "id" => input.id = Some(std::ffi::CString::new("other.protocol").unwrap()),
            "nullId" => input.id = None,
            "noAttributes" => input.attributes = None,
            "failedBinary" | "nullBinary" | "oversizedBinary" => {
                input.attributes = Some(
                    ComWrapper::new(Blob {
                        bytes: vec![0],
                        reported: Some(if fault == "oversizedBinary" {
                            2 * 1_048_576 + 4097
                        } else {
                            1
                        }),
                        null_data: fault == "nullBinary",
                        fail: fault == "failedBinary",
                    })
                    .to_com_ptr::<IAttributeList>()
                    .unwrap(),
                )
            }
            _ => unreachable!(),
        }
        let input = message::input(input);
        let expected = if matches!(fault, "id" | "nullId") {
            kResultFalse
        } else {
            kInvalidArgument
        };
        unsafe {
            assert_eq!(pc.notify(input.as_ptr()), expected, "{fault}");
            assert_eq!(c.getParamNormalized(0x1003), 0.375);
        }
    }
    let valid = message::packet(&valid);
    unsafe {
        assert_eq!(pc.notify(valid.as_ptr()), kResultOk);
        assert_eq!(pc.disconnect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.disconnect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn controller_requests_the_processor_values_instead_of_keeping_its_own_defaults() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.setParamNormalized(0x1003, 0.875), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(
            c.getParamNormalized(0x1003),
            0.25,
            "projection must read the real processor's physical level1"
        );
    }
    let frames = to_p.frames.borrow();
    let request = frames
        .iter()
        .find(|v| v["body"]["type"] == "request")
        .expect("controller uses the standard-message command path");
    let request_frame: serde_json::Value =
        serde_json::from_str(request["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(request_frame["method"], "getParameterState");
    assert_eq!(request_frame["requestId"], "1");
    drop(frames);
    let frames = to_c.frames.borrow();
    let reply = frames
        .iter()
        .find(|v| v["body"]["type"] == "reply")
        .expect("shared owner replies inline");
    let reply: serde_json::Value =
        serde_json::from_str(reply["body"]["frame"].as_str().unwrap()).unwrap();
    assert_eq!(reply["status"], "accepted");
    assert_eq!(reply["result"]["parameters"].as_array().unwrap().len(), 23);
    assert_eq!(reply["result"]["generation"], 1);
    assert_eq!(reply["result"]["sequence"], 0);
    drop(frames);
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn invalid_projection_replies_preserve_all_cached_values_and_original_correlation() {
    use common::message;
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    *to_c.refuse_type.borrow_mut() = Some("reply".into());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(c.setParamNormalized(0x1003, 0.375), kResultOk);
    }
    let packet = to_c
        .frames
        .borrow()
        .iter()
        .find(|v| v["body"]["type"] == "reply")
        .unwrap()
        .clone();
    let valid: serde_json::Value =
        serde_json::from_str(packet["body"]["frame"].as_str().unwrap()).unwrap();
    for fault in [
        "json",
        "status",
        "version",
        "instance",
        "attachment",
        "request",
        "short",
        "unknown",
        "duplicate",
        "value",
    ] {
        let mut response = valid.clone();
        match fault {
            "status" => response["status"] = "rejected".into(),
            "version" => response["version"] = 2.into(),
            "instance" => response["instanceId"] = "999999".into(),
            "attachment" => response["attachmentId"] = "999999".into(),
            "request" => response["requestId"] = "2".into(),
            "short" => {
                response["result"]["parameters"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "unknown" => response["result"]["parameters"][0]["id"] = "unknown".into(),
            "duplicate" => {
                response["result"]["parameters"][1]["id"] =
                    response["result"]["parameters"][0]["id"].clone()
            }
            "value" => response["result"]["parameters"][0]["value"] = 2.into(),
            "json" => {}
            _ => unreachable!(),
        }
        let mut malformed = packet.clone();
        malformed["body"]["frame"] = if fault == "json" {
            "not JSON".into()
        } else {
            response.to_string().into()
        };
        let malformed = message::packet(&malformed);
        unsafe {
            assert_eq!(cc.notify(malformed.as_ptr()), kInvalidArgument, "{fault}");
            assert_eq!(c.getParamNormalized(0x1003), 0.375);
        }
    }
    let projection = to_c
        .frames
        .borrow()
        .iter()
        .find(|v| v["body"]["type"] == "projection")
        .unwrap()
        .clone();
    for raw in ["not JSON".to_string(),"{}".to_string(),serde_json::json!({"protocolVersion":2,"instanceId":"1","attachmentId":"0","nextRequestId":"1"}).to_string(),serde_json::json!({"protocolVersion":1,"instanceId":"0","attachmentId":"0","nextRequestId":"1"}).to_string(),serde_json::json!({"protocolVersion":1,"instanceId":"1","attachmentId":"bad","nextRequestId":"1"}).to_string()]{
        let mut malformed=projection.clone();malformed["body"]["frame"]=raw.into();let malformed=message::packet(&malformed);unsafe{assert_eq!(cc.notify(malformed.as_ptr()),kInvalidArgument);assert_eq!(c.getParamNormalized(0x1003),0.375);}
    }
    let before = to_p
        .frames
        .borrow()
        .iter()
        .filter(|v| v["body"]["type"] == "request")
        .count();
    unsafe {
        assert_eq!(cc.notify(message::packet(&projection).as_ptr()), kResultOk);
    }
    assert_eq!(
        to_p.frames
            .borrow()
            .iter()
            .filter(|v| v["body"]["type"] == "request")
            .count(),
        before,
        "replayed bootstrap cannot admit another request"
    );
    let valid_packet = message::packet(&packet);
    unsafe {
        assert_eq!(cc.notify(valid_packet.as_ptr()), kResultOk);
        assert_eq!(c.getParamNormalized(0x1003), 0.25);
        assert_eq!(c.setParamNormalized(0x1003, 0.625), kResultOk);
        assert_eq!(cc.notify(valid_packet.as_ptr()), kResultOk);
        assert_eq!(
            c.getParamNormalized(0x1003),
            0.625,
            "a duplicate reply cannot overwrite a later host value"
        );
    }
    let mut changed = packet.clone();
    changed["body"]["frame"] = format!("{} ", valid).into();
    unsafe {
        assert_eq!(cc.notify(message::packet(&changed).as_ptr()), kResultFalse);
        assert_eq!(c.getParamNormalized(0x1003), 0.625);
    }
    changed["sequence"] = 99.into();
    unsafe {
        assert_eq!(cc.notify(message::packet(&changed).as_ptr()), kResultFalse);
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn processor_service_retries_a_controller_request_without_an_editor_or_new_input() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.setParamNormalized(0x1003, 0.875), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    *to_p.refuse_type.borrow_mut() = Some("request".into());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
        assert_eq!(c.getParamNormalized(0x1003), 0.875);
    }
    to_p.refuse_type.borrow_mut().take();
    clock.tick();
    unsafe {
        assert_eq!(
            c.getParamNormalized(0x1003),
            0.25,
            "processor-owned service must drive a retained projection request"
        );
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(c.terminate(), kResultOk);
    }
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn processor_termination_revokes_a_surviving_projection_before_late_delivery() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(c.initialize(std::ptr::null_mut()), kResultOk);
    }
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    unsafe {
        assert_eq!(pc.connect(proxy_c.as_ptr()), kResultOk);
        assert_eq!(cc.connect(proxy_p.as_ptr()), kResultOk);
    }
    let index = to_c
        .frames
        .borrow()
        .iter()
        .position(|v| v["body"]["type"] == "reply")
        .unwrap();
    let late = to_c.messages.borrow()[index].clone();
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
        assert_eq!(
            cc.notify(late.as_ptr()),
            kNotInitialized,
            "closed processor cannot remain a live authority"
        );
        assert_eq!(
            cc.disconnect(proxy_p.as_ptr()),
            kInvalidArgument,
            "projection released its peer on shutdown"
        );
        assert_eq!(c.terminate(), kResultOk);
    }
    assert!(
        to_c.frames
            .borrow()
            .iter()
            .any(|v| v["body"]["type"] == "goodbye")
    );
    to_p.target.borrow_mut().take();
    to_c.target.borrow_mut().take();
}

#[test]
fn delayed_initial_parameter_reply_cannot_overwrite_component_state_restoration() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (_to_p, proxy_p) = proxy(pc.clone());
    *to_c.refuse_type.borrow_mut() = Some("reply".into());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let old_reply = {
        let frames = to_c.frames.borrow();
        let i = frames
            .iter()
            .position(|f| f["body"]["type"] == "reply")
            .expect("proxy retained initial default-value reply");
        to_c.messages.borrow()[i].clone()
    };
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
    let (_, input) = common::stream::stream(legacy.to_vec(), 7);
    assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    assert_eq!(
        unsafe { cc.notify(old_reply.as_ptr()) },
        kResultOk,
        "obsolete accepted reply must be acknowledged to release bounded peer storage"
    );
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.125,
        "old initial projection cannot roll back restored state"
    );
    assert_eq!(unsafe { cc.notify(old_reply.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    assert_eq!(unsafe { c.setParamNormalized(0x1003, 0.875) }, kResultOk);
    assert_eq!(unsafe { pc.disconnect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.disconnect(proxy_p.as_ptr()) }, kResultOk);
    to_c.refuse_type.borrow_mut().take();
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.125,
        "a new connection must still synchronize from the processor"
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn failed_component_state_restore_keeps_a_pending_parameter_reply_usable() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { p.setState(input.as_ptr()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (_to_p, proxy_p) = proxy(pc.clone());
    *to_c.refuse_type.borrow_mut() = Some("reply".into());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let reply = {
        let frames = to_c.frames.borrow();
        let i = frames
            .iter()
            .position(|f| f["body"]["type"] == "reply")
            .unwrap();
        to_c.messages.borrow()[i].clone()
    };
    let (_, input) = common::stream::stream(legacy[..legacy.len() / 2].to_vec(), 7);
    assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultFalse);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.25);
    assert_eq!(unsafe { cc.notify(reply.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}
