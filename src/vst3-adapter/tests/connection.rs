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
    defer_type: RefCell<Option<String>>,
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
        if self
            .0
            .defer_type
            .borrow()
            .as_ref()
            .is_some_and(|kind| frame["body"]["type"].as_str() == Some(kind.as_str()))
        {
            return kResultOk;
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

// ComWrapper owns its generated AddRef implementation. This host fixture uses
// the public ABI directly to exercise reentry from that earlier foreign call.
// Allocation lifetime follows the ABI reference count, including assertion
// unwinding before the processor releases a retained reference.
#[repr(C)]
struct RetainProxy {
    vtbl: *const IConnectionPointVtbl,
    references: Cell<u32>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
    on_release: RefCell<Option<Box<dyn FnOnce()>>>,
    target: ComPtr<IConnectionPoint>,
}
impl RetainProxy {
    fn retained(target: ComPtr<IConnectionPoint>) -> RetainPeer {
        let proxy = Box::new(Self {
            vtbl: &RETAIN_PROXY_VTBL,
            references: Cell::new(1),
            callback: RefCell::new(None),
            on_release: RefCell::new(None),
            target,
        });
        // SAFETY: the fresh allocation owns exactly one initial ABI reference;
        // RetainPeer transfers it to ComPtr, and release destroys the final one.
        RetainPeer(unsafe { ComPtr::from_raw(Box::into_raw(proxy).cast()).unwrap() })
    }
    fn as_ptr(&self) -> *mut IConnectionPoint {
        std::ptr::from_ref(self).cast_mut().cast()
    }
}
struct RetainPeer(ComPtr<IConnectionPoint>);
impl std::ops::Deref for RetainPeer {
    type Target = RetainProxy;
    fn deref(&self) -> &RetainProxy {
        // SAFETY: this retained reference owns the live repr(C) allocation.
        unsafe { &*self.0.as_ptr().cast::<RetainProxy>() }
    }
}
unsafe extern "system" fn retain_proxy_query(
    this: *mut FUnknown,
    iid: *const TUID,
    out: *mut *mut std::ffi::c_void,
) -> tresult {
    if iid.is_null() || out.is_null() {
        return kInvalidArgument;
    }
    // SAFETY: the host supplies one readable IID and writable result; this is
    // the leading interface pointer of the Box-owned repr(C) fixture.
    unsafe {
        *out = std::ptr::null_mut();
        if *iid != FUnknown_iid && *iid != IConnectionPoint_iid {
            return kNoInterface;
        }
        *out = this.cast();
        retain_proxy_add_ref(this);
    }
    kResultOk
}
unsafe extern "system" fn retain_proxy_add_ref(this: *mut FUnknown) -> u32 {
    // SAFETY: every ABI reference points to the first field of live RetainProxy.
    let proxy = unsafe { &*this.cast::<RetainProxy>() };
    let references = proxy.references.get() + 1;
    proxy.references.set(references);
    let callback = proxy.callback.borrow_mut().take();
    if let Some(callback) = callback {
        callback();
    }
    references
}
unsafe extern "system" fn retain_proxy_release(this: *mut FUnknown) -> u32 {
    // SAFETY: this call consumes one live ABI reference. Keep the allocation
    // alive through foreign reentry, then destroy it only at the final release.
    let proxy = unsafe { &*this.cast::<RetainProxy>() };
    let references = proxy.references.get() - 1;
    proxy.references.set(references);
    let callback = proxy.on_release.borrow_mut().take();
    if let Some(callback) = callback {
        callback();
    }
    let remaining = proxy.references.get();
    if remaining == 0 {
        // SAFETY: every reference, including the host's RetainPeer, is gone.
        drop(unsafe { Box::from_raw(this.cast::<RetainProxy>()) });
    }
    remaining
}
unsafe extern "system" fn retain_proxy_connect(
    _this: *mut IConnectionPoint,
    _other: *mut IConnectionPoint,
) -> tresult {
    kNotImplemented
}
unsafe extern "system" fn retain_proxy_notify(
    this: *mut IConnectionPoint,
    message: *mut IMessage,
) -> tresult {
    // SAFETY: the fixture and its retained target are live during this ABI call.
    unsafe { (&*this.cast::<RetainProxy>()).target.notify(message) }
}
static RETAIN_PROXY_VTBL: IConnectionPointVtbl = IConnectionPointVtbl {
    base: FUnknownVtbl {
        queryInterface: retain_proxy_query,
        addRef: retain_proxy_add_ref,
        release: retain_proxy_release,
    },
    connect: retain_proxy_connect,
    disconnect: retain_proxy_connect,
    notify: retain_proxy_notify,
};

#[test]
fn processor_connection_publication_rejects_lifecycle_reentry_from_foreign_add_ref() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let peer = RetainProxy::retained(cc.clone());
    let processor = (*p).clone();
    let connection = pc.clone();
    let other = cc.clone();
    let results = Rc::new(RefCell::new(None));
    let during_retain = results.clone();
    *peer.callback.borrow_mut() = Some(Box::new(move || {
        *during_retain.borrow_mut() = Some(unsafe {
            (
                processor.terminate(),
                connection.connect(other.as_ptr()),
                connection.disconnect(other.as_ptr()),
            )
        });
    }));
    let connected = unsafe { pc.connect(peer.as_ptr()) };
    let buses = unsafe { p.getBusCount(0, 1) };
    let retained = peer.references.get();
    // Cleanup precedes assertions so a failing lifecycle oracle cannot unwind
    // the Box while the plugin still owns the fixture's ABI reference.
    let disconnected = unsafe { pc.disconnect(peer.as_ptr()) };
    let controller_closed = unsafe { c.terminate() };
    let processor_closed = unsafe { p.terminate() };
    drop(pc);
    drop(p);
    assert_eq!(
        *results.borrow(),
        Some((kResultFalse, kResultFalse, kResultFalse))
    );
    assert_eq!(connected, kResultOk);
    assert_eq!(disconnected, kResultOk);
    assert_eq!(
        peer.references.get(),
        1,
        "the plugin releases its retained ABI reference"
    );
    assert_eq!(controller_closed, kResultOk);
    assert_eq!(processor_closed, kResultOk);
    assert_eq!(buses, 1);
    assert_eq!(retained, 2);
}

#[test]
fn processor_local_disconnect_defers_reconnect_from_foreign_release_until_cleanup_finishes() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let peer = RetainProxy::retained(cc.clone());
    assert_eq!(unsafe { pc.connect(peer.as_ptr()) }, kResultOk);
    // Only the processor is connected: no musical attachment exists yet, so
    // disconnection takes the local unbound-peer cleanup path.
    let connection = pc.clone();
    let other = cc.clone();
    let result = Rc::new(Cell::new(None));
    let during_release = result.clone();
    *peer.on_release.borrow_mut() = Some(Box::new(move || {
        during_release.set(Some(unsafe { connection.connect(other.as_ptr()) }));
    }));
    let disconnected = unsafe { pc.disconnect(peer.as_ptr()) };
    // Remove an incorrectly published replacement before any RED assertion.
    if result.get() == Some(kResultOk) {
        assert_eq!(unsafe { pc.disconnect(cc.as_ptr()) }, kResultOk);
    }
    assert_eq!(disconnected, kResultOk);
    assert_eq!(peer.references.get(), 1);
    assert_eq!(unsafe { pc.connect(cc.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(pc.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParameterCount() }, 23);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert_eq!(result.get(), Some(kResultFalse));
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
    for publication in [
        serde_json::Value::Null,
        "3".into(),
        "02".into(),
        "invalid".into(),
    ] {
        let mut malformed = packet.clone();
        malformed["body"]["publication"] = publication;
        assert_eq!(
            unsafe { cc.notify(message::packet(&malformed).as_ptr()) },
            kInvalidArgument
        );
        assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
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

#[test]
fn stopped_processor_feedback_updates_its_projection_and_retries_refused_delivery() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (_to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let audio = p.cast::<IAudioProcessor>().unwrap();
    let flush = |value| {
        let changes =
            common::parameters::changes(vec![common::parameters::queue(0x1003, vec![(0, value)])]);
        let mut block = common::audio::Block::new(0);
        let mut data = block.data();
        data.inputParameterChanges = changes.as_ptr();
        assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    };
    flush(0.125);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.25);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    let events = || {
        to_c.frames
            .borrow()
            .iter()
            .filter(|f| f["body"]["type"] == "event")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(events().len(), 1);
    clock.tick();
    assert_eq!(events().len(), 1, "accepted feedback is consumed once");
    *to_c.refuse_type.borrow_mut() = Some("event".into());
    flush(0.375);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    let first = events().last().unwrap().clone();
    clock.tick();
    assert_eq!(
        events().last(),
        Some(&first),
        "retry retains exact event identity"
    );
    to_c.refuse_type.borrow_mut().take();
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
    let accepted = events().len();
    clock.tick();
    assert_eq!(events().len(), accepted);
    // A controller-only restore must keep its imported values even when the
    // processor differs. Neither already emitted nor still queued old feedback
    // may overwrite them after the fresh snapshot barrier.
    flush(0.625);
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    let old = common::message::packet(&first);
    assert_eq!(unsafe { cc.notify(old.as_ptr()) }, kResultOk);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    flush(0.5);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.5);

    *to_c.defer_type.borrow_mut() = Some("event".into());
    let count = events().len();
    let changes = common::parameters::changes(vec![
        common::parameters::queue(0x1003, vec![(0, 0.375)]),
        common::parameters::queue(0x1004, vec![(0, 0.625)]),
    ]);
    let mut block = common::audio::Block::new(0);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    clock.tick();
    let delayed = events()[count..].to_vec();
    assert_eq!(delayed.len(), 2);
    to_c.defer_type.borrow_mut().take();
    for frame in delayed.iter().rev() {
        let event = common::message::packet(frame);
        assert_eq!(unsafe { cc.notify(event.as_ptr()) }, kResultOk);
    }
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
    assert_eq!(
        unsafe { c.getParamNormalized(0x1004) },
        0.625,
        "reordering distinct parameters preserves both updates"
    );
    *to_c.refuse_type.borrow_mut() = Some("event".into());
    for value in [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875] {
        flush(value);
        clock.tick();
    }
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
    to_c.refuse_type.borrow_mut().take();
    for _ in 0..4 {
        clock.tick();
    }
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.875,
        "bounded transport saturation retains the later native feedback"
    );
    assert_eq!(unsafe { pc.disconnect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.disconnect(proxy_p.as_ptr()) }, kResultOk);
    flush(0.75);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.875);
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.75,
        "reattachment obtains the latest value after editor-absent servicing"
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn feedback_converges_after_transport_and_native_ring_overflow() {
    for active in [false, true] {
        let bundle = Bundle::new();
        let module = bundle.load();
        let (clock, factory, _host) = common::runloop::host(&module);
        let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
        let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
        assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
        assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
        let pc = p.cast::<IConnectionPoint>().unwrap();
        let cc = c.cast::<IConnectionPoint>().unwrap();
        let (to_c, proxy_c) = proxy(cc.clone());
        let (_to_p, proxy_p) = proxy(pc.clone());
        assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
        assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
        let audio = p.cast::<IAudioProcessor>().unwrap();
        if active {
            common::audio::start(&p, &audio, 48_000.0, 16);
        }
        let flush = |value| {
            let changes = common::parameters::changes(vec![common::parameters::queue(
                0x1003,
                vec![(0, value)],
            )]);
            let mut block = common::audio::Block::new(usize::from(active));
            let mut data = block.data();
            data.inputParameterChanges = changes.as_ptr();
            assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
            if active {
                assert_eq!(
                    (&block.left[..], &block.right[..]),
                    (&[0.0][..], &[0.0][..])
                );
            }
        };
        let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
        let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
        assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultOk);
        assert_eq!(unsafe { c.getParamNormalized(0x1004) }, 0.375);
        *to_c.refuse_type.borrow_mut() = Some("event".into());
        // Eight staged packets plus the 1024-entry native ring are both bounded.
        // The final accepted write must remain discoverable after newer feedback
        // is dropped, even with no subsequent automation to repair the display.
        for _ in 0..1100 {
            flush(0.375);
            clock.tick();
        }
        if active {
            // Both pan writes are dropped from the already-full native ring.
            // Its final processor value equals its pre-import default, so a
            // value-diff heuristic would miss this genuine later automation.
            for value in [0.625, 0.5] {
                let changes = common::parameters::changes(vec![common::parameters::queue(
                    0x1004,
                    vec![(0, value)],
                )]);
                let mut block = common::audio::Block::new(1);
                let mut data = block.data();
                data.inputParameterChanges = changes.as_ptr();
                assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
                assert_eq!(
                    (&block.left[..], &block.right[..]),
                    (&[0.0][..], &[0.0][..])
                );
            }
        }
        let expected_pan = if active { 0.5 } else { 0.375 };
        flush(0.875);
        clock.tick();
        assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
        to_c.refuse_type.borrow_mut().take();
        for _ in 0..160 {
            clock.tick();
        }
        assert_eq!(
            unsafe { c.getParamNormalized(0x1003) },
            0.875,
            "projection converges to the authoritative last value after overflow"
        );
        assert_eq!(
            unsafe { c.getParamNormalized(0x1004) },
            expected_pan,
            "overflow recovery preserves an imported control with no later write"
        );
        let snapshot = to_c
            .frames
            .borrow()
            .iter()
            .find(|f| f["body"]["origins"].is_array())
            .unwrap()
            .clone();
        let canonical: serde_json::Value =
            serde_json::from_str(snapshot["body"]["frame"].as_str().unwrap()).unwrap();
        let stamp = snapshot["body"]["publication"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        for origins in [
            serde_json::json!([]),
            serde_json::json!(vec!["3"; 23]),
            serde_json::json!(vec!["02"; 23]),
            serde_json::json!(vec![(stamp + 2).to_string(); 23]),
        ] {
            let mut malformed = snapshot.clone();
            malformed["body"]["origins"] = origins;
            let message = common::message::packet(&malformed);
            assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kInvalidArgument);
            assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.875);
            assert_eq!(unsafe { c.getParamNormalized(0x1004) }, expected_pan);
        }
        let duplicate = common::message::packet(&snapshot);
        assert_eq!(unsafe { cc.notify(duplicate.as_ptr()) }, kResultOk);
        let mut conflicting = snapshot.clone();
        conflicting["body"]["publication"] = serde_json::json!((stamp + 2).to_string());
        let conflicting = common::message::packet(&conflicting);
        assert_eq!(
            unsafe { cc.notify(conflicting.as_ptr()) },
            kResultFalse,
            "a repeated snapshot identity cannot change its supersession cutoff"
        );
        assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.875);
        // A recovery snapshot represents the final value of its entire interval.
        // A delayed ordinary point in that same interval is obsolete even if it is
        // encoded with a newer event sequence after the snapshot was produced.
        let mut delayed = canonical;
        delayed["sequence"] = serde_json::json!(
            (delayed["sequence"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                + 1)
            .to_string()
        );
        delayed["payload"]["parameters"] = serde_json::json!([{"id":"drums.level","value":0.125}]);
        let mut envelope = snapshot;
        envelope["body"].as_object_mut().unwrap().remove("origins");
        envelope["body"]["frame"] = serde_json::json!(delayed.to_string());
        let message = common::message::packet(&envelope);
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
        assert_eq!(
            unsafe { c.getParamNormalized(0x1003) },
            0.875,
            "snapshot supersedes equal-publication intermediate points"
        );
        if active {
            assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
            assert_eq!(unsafe { p.setActive(0) }, kResultOk);
        }
        assert_eq!(unsafe { c.terminate() }, kResultOk);
        assert_eq!(unsafe { p.terminate() }, kResultOk);
    }
}

#[test]
fn feedback_before_a_delayed_initial_snapshot_is_retained_after_its_cutoff() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (_to_p, proxy_p) = proxy(pc.clone());
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let snapshot = {
        let frames = to_c.frames.borrow();
        frames
            .iter()
            .find(|f| f["body"]["type"] == "reply")
            .unwrap()
            .clone()
    };
    let audio = p.cast::<IAudioProcessor>().unwrap();
    let changes =
        common::parameters::changes(vec![common::parameters::queue(0x1003, vec![(0, 0.125)])]);
    let mut block = common::audio::Block::new(0);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.25);
    to_c.defer_type.borrow_mut().take();
    let snapshot = common::message::packet(&snapshot);
    assert_eq!(unsafe { cc.notify(snapshot.as_ptr()) }, kResultOk);
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.125,
        "newer feedback received before the older snapshot must survive it"
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn post_restore_feedback_survives_a_delayed_initial_reply() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    let audio = p.cast::<IAudioProcessor>().unwrap();
    let changes =
        common::parameters::changes(vec![common::parameters::queue(0x1003, vec![(0, 0.625)])]);
    let mut block = common::audio::Block::new(0);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.125);
    to_c.defer_type.borrow_mut().take();
    let mut replies: Vec<_> = to_c
        .frames
        .borrow()
        .iter()
        .filter(|f| f["body"]["type"] == "reply")
        .cloned()
        .collect();
    replies.dedup();
    for reply in replies {
        let reply = common::message::packet(&reply);
        assert_eq!(unsafe { cc.notify(reply.as_ptr()) }, kResultOk);
    }
    clock.tick();
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.625,
        "valid post-restore feedback survives the delayed initial reply"
    );
    let old_request = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request")
        .unwrap()
        .clone();
    let replay = common::message::packet(&old_request);
    assert_eq!(unsafe { pc.notify(replay.as_ptr()) }, kResultOk);
    clock.tick();
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.625,
        "obsolete query retries neither roll back values nor block feedback"
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn controller_import_freshness_begins_at_the_correlated_processor_barrier() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (_to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    *to_p.defer_type.borrow_mut() = Some("request".into());
    let legacy = include_bytes!("../../trigger-host/tests/fixtures/trigger-v2-state.json");
    let (_, input) = common::stream::stream(legacy.to_vec(), 4096);
    assert_eq!(unsafe { c.setComponentState(input.as_ptr()) }, kResultOk);
    let audio = p.cast::<IAudioProcessor>().unwrap();
    let flush = |value| {
        let changes =
            common::parameters::changes(vec![common::parameters::queue(0x1003, vec![(0, value)])]);
        let mut block = common::audio::Block::new(0);
        let mut data = block.data();
        data.inputParameterChanges = changes.as_ptr();
        assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    };
    flush(0.625);
    clock.tick();
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.125,
        "import remains authoritative while the host has not delivered the barrier request"
    );
    let barrier = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request" && f["sequence"] == 2)
        .unwrap()
        .clone();
    to_p.defer_type.borrow_mut().take();
    let barrier = common::message::packet(&barrier);
    assert_eq!(unsafe { pc.notify(barrier.as_ptr()) }, kResultOk);
    assert_eq!(
        unsafe { c.getParamNormalized(0x1003) },
        0.125,
        "barrier captures a cutoff without copying differing processor values"
    );
    flush(0.875);
    clock.tick();
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.875);
    assert_eq!(unsafe { c.getParamNormalized(0x1004) }, 0.375);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn invalid_feedback_batches_preserve_all_cached_values_before_any_publication() {
    use serde_json::{Value, json};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (_to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.setParamNormalized(0x1003, 0.875) }, kResultOk);
    assert_eq!(unsafe { c.setParamNormalized(0x1004, 0.375) }, kResultOk);
    *to_c.defer_type.borrow_mut() = Some("event".into());
    let audio = p.cast::<IAudioProcessor>().unwrap();
    let changes =
        common::parameters::changes(vec![common::parameters::queue(0x1003, vec![(0, 0.375)])]);
    let mut block = common::audio::Block::new(0);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    clock.tick();
    let valid = to_c
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "event")
        .unwrap()
        .clone();
    let event: Value = serde_json::from_str(valid["body"]["frame"].as_str().unwrap()).unwrap();
    let mut malformed = Vec::new();
    for (key, value) in [
        ("version", json!(2)),
        ("instanceId", json!("999")),
        ("attachmentId", json!("999")),
        ("event", json!("unknown")),
        ("sequence", json!("01")),
        ("sequence", json!("0")),
    ] {
        let mut changed = event.clone();
        changed[key] = value;
        malformed.push(changed);
    }
    for (key, value) in [
        ("generation", json!(0)),
        ("generation", json!(9007199254740992u64)),
        ("sequence", json!(-1)),
        ("reset_cancelled", json!(0)),
        ("frame_offset", json!(16384)),
    ] {
        let mut changed = event.clone();
        changed["payload"][key] = value;
        malformed.push(changed);
    }
    for parameters in [
        json!([]),
        json!([{"id":"unknown","value":0.5}]),
        json!([{"id":"drums.level","value":1.1}]),
        json!([{"id":"drums.level","value":0.625},{"id":"drums.pan","value":"NaN"}]),
        json!([{"id":"drums.level","value":0.625},{"id":"drums.level","value":0.5}]),
        json!(vec![json!({"id":"drums.level","value":0.5}); 24]),
    ] {
        let mut changed = event.clone();
        changed["payload"]["parameters"] = parameters;
        malformed.push(changed);
    }
    let mut packets = malformed
        .into_iter()
        .map(|changed| {
            let mut packet = valid.clone();
            packet["body"]["frame"] = changed.to_string().into();
            packet
        })
        .collect::<Vec<_>>();
    let mut invalid_json = valid.clone();
    invalid_json["body"]["frame"] = "not JSON".into();
    packets.push(invalid_json);
    for publication in ["3", "02", "invalid"] {
        let mut packet = valid.clone();
        packet["body"]["publication"] = publication.into();
        packets.push(packet);
    }
    for (index, packet) in packets.iter().enumerate() {
        let message = common::message::packet(packet);
        assert_eq!(
            unsafe { cc.notify(message.as_ptr()) },
            kInvalidArgument,
            "malformed event{index}"
        );
        assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.875);
        assert_eq!(unsafe { c.getParamNormalized(0x1004) }, 0.375);
    }
    let message = common::message::packet(&valid);
    assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
    assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
    let mut conflict = event;
    conflict["payload"]["parameters"][0]["value"] = 0.625.into();
    let mut packet = valid;
    packet["body"]["frame"] = conflict.to_string().into();
    let message = common::message::packet(&packet);
    assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultFalse);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.375);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[derive(Clone, Debug, PartialEq)]
enum HostEdit {
    Begin(u32),
    Value(u32, f64),
    End(u32),
    Restart(i32),
}
#[derive(Default)]
struct HostEditLog {
    accepted: RefCell<Vec<(std::thread::ThreadId, HostEdit)>>,
    attempts: RefCell<Vec<HostEdit>>,
    refuse_once: RefCell<Option<HostEdit>>,
    on_call: RefCell<Option<Box<dyn FnOnce()>>>,
}
struct ComponentHandler(Rc<HostEditLog>);
impl Class for ComponentHandler {
    type Interfaces = (IComponentHandler,);
}
impl ComponentHandler {
    fn record(&self, event: HostEdit) -> tresult {
        self.0.attempts.borrow_mut().push(event.clone());
        let refused = {
            let mut refusal = self.0.refuse_once.borrow_mut();
            if refusal.as_ref() == Some(&event) {
                refusal.take();
                true
            } else {
                false
            }
        };
        if refused {
            return kResultFalse;
        }
        self.0
            .accepted
            .borrow_mut()
            .push((std::thread::current().id(), event));
        let callback = self.0.on_call.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        kResultOk
    }
}
impl IComponentHandlerTrait for ComponentHandler {
    unsafe fn beginEdit(&self, id: u32) -> tresult {
        self.record(HostEdit::Begin(id))
    }
    unsafe fn performEdit(&self, id: u32, value: f64) -> tresult {
        self.record(HostEdit::Value(id, value))
    }
    unsafe fn endEdit(&self, id: u32) -> tresult {
        self.record(HostEdit::End(id))
    }
    unsafe fn restartComponent(&self, flags: i32) -> tresult {
        self.record(HostEdit::Restart(flags))
    }
}

fn run_gesture_delivery(refuse: Option<HostEdit>, defer_acks: bool) {
    use serde_json::{Value, json};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let log = Rc::new(HostEditLog::default());
    *log.refuse_once.borrow_mut() = refuse.clone();
    let during_call = (*c).clone();
    let processor_during_call = (*p).clone();
    let processor_connection_during_call = pc.clone();
    let peer_during_call = proxy_c.clone();
    *log.on_call.borrow_mut() = Some(Box::new(move || {
        assert_eq!(unsafe { during_call.terminate() }, kResultFalse);
        assert_eq!(unsafe { processor_during_call.terminate() }, kResultFalse);
        assert_eq!(
            unsafe { processor_connection_during_call.disconnect(peer_during_call.as_ptr()) },
            kResultFalse
        );
        assert_eq!(
            unsafe { during_call.setComponentHandler(std::ptr::null_mut()) },
            kResultFalse
        );
    }));
    if defer_acks {
        *to_p.defer_type.borrow_mut() = Some("automationAck".into());
    }
    let handler = ComWrapper::new(ComponentHandler(log.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { c.setComponentHandler(handler.as_ptr()) },
        kResultOk
    );
    drop(handler); // The controller must retain the standard host interface.
    let template = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request")
        .unwrap()
        .clone();
    let initial: Value = serde_json::from_str(template["body"]["frame"].as_str().unwrap()).unwrap();
    // The actual view is task4. Here the existing canonical editor commands
    // enter through the same standard processor connection. Their replies are
    // retained by the proxy as they would be for a separate editor consumer.
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    let mut serial = 1u64;
    let mut send = |method: &str, args: Value| {
        serial += 1;
        let mut envelope = template.clone();
        envelope["sequence"] = json!(serial);
        envelope["body"]["frame"]=json!(json!({"version":1,"attachmentId":initial["attachmentId"],"requestId":serial.to_string(),"method":method,"args":args}).to_string());
        let message = common::message::packet(&envelope);
        assert_eq!(unsafe { pc.notify(message.as_ptr()) }, kResultOk);
        let reply = to_c
            .frames
            .borrow()
            .iter()
            .rev()
            .find(|f| f["body"]["type"] == "reply" && f["sequence"] == serial)
            .unwrap()
            .clone();
        let reply: Value = serde_json::from_str(reply["body"]["frame"].as_str().unwrap()).unwrap();
        assert_eq!(reply["status"], "accepted");
        reply
    };
    send("beginGesture", json!(["drums.level", 1]));
    send("setParameter", json!(["drums.level", 0.625, 1]));
    send("endGesture", json!(["drums.level", 1]));
    send("setParameter", json!(["drums.pan", 0.25, 1]));
    assert!(
        log.accepted.borrow().is_empty(),
        "host edits are serviced outside command admission"
    );
    let mut delayed_ack = None;
    if defer_acks {
        for _ in 0..3 {
            clock.tick();
        }
        assert_eq!(
            *log.accepted.borrow(),
            vec![(std::thread::current().id(), HostEdit::Begin(0x1003))],
            "transport acceptance alone cannot retire the shared ledger"
        );
        let ack = to_p
            .frames
            .borrow()
            .iter()
            .find(|f| f["body"]["type"] == "automationAck")
            .unwrap()
            .clone();
        to_p.defer_type.borrow_mut().take();
        let message = common::message::packet(&ack);
        assert_eq!(unsafe { pc.notify(message.as_ptr()) }, kResultOk);
        delayed_ack = Some(ack);
    }
    for _ in 0..8 {
        clock.tick();
    }
    let thread = std::thread::current().id();
    assert_eq!(
        *log.accepted.borrow(),
        vec![
            (thread, HostEdit::Begin(0x1003)),
            (thread, HostEdit::Value(0x1003, 0.625)),
            (thread, HostEdit::End(0x1003)),
            (thread, HostEdit::Begin(0x1004)),
            (thread, HostEdit::Value(0x1004, 0.25)),
            (thread, HostEdit::End(0x1004))
        ]
    );
    clock.tick();
    assert_eq!(
        log.accepted.borrow().len(),
        6,
        "accepted host events are not delivered twice"
    );
    assert_eq!(
        log.attempts.borrow().len(),
        if refuse.is_some() { 7 } else { 6 }
    );
    if let Some(refused) = refuse {
        assert_eq!(
            log.attempts
                .borrow()
                .iter()
                .filter(|event| **event == refused)
                .count(),
            2
        );
    }
    if let Some(ack) = delayed_ack {
        let message = common::message::packet(&ack);
        assert_eq!(unsafe { pc.notify(message.as_ptr()) }, kResultOk);
        let old = to_c
            .frames
            .borrow()
            .iter()
            .find(|f| f["body"]["type"] == "automation")
            .unwrap()
            .clone();
        let message = common::message::packet(&old);
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
        assert_eq!(
            log.accepted.borrow().len(),
            6,
            "old successes never call the host again"
        );
    }
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.625);
    assert_eq!(unsafe { c.getParamNormalized(0x1004) }, 0.25);
    let session = send("getSamplerSession", json!([]));
    assert_eq!(
        session["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn editor_gestures_deliver_normalized_host_edits_on_main_without_native_history() {
    run_gesture_delivery(None, false);
}
#[test]
fn host_gestures_retry_each_refused_phase_and_lost_ack_without_duplicate_acceptance() {
    for refusal in [
        HostEdit::Begin(0x1003),
        HostEdit::Value(0x1003, 0.625),
        HostEdit::End(0x1003),
    ] {
        run_gesture_delivery(Some(refusal), false);
    }
    run_gesture_delivery(None, true);
}

fn with_gesture_connection(
    test: impl FnOnce(
        &common::runloop::LoopState,
        &ComPtr<IComponent>,
        &ComPtr<IEditController>,
        &ComPtr<IConnectionPoint>,
        &ComPtr<IConnectionPoint>,
        &Forward,
        &Forward,
        &mut dyn FnMut(&str, serde_json::Value) -> serde_json::Value,
    ),
) {
    use serde_json::{Value, json};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let template = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request")
        .unwrap()
        .clone();
    let initial: Value = serde_json::from_str(template["body"]["frame"].as_str().unwrap()).unwrap();
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    let mut serial = 1u64;
    let mut send = |method: &str, args: Value| {
        serial += 1;
        let mut envelope = template.clone();
        envelope["sequence"] = json!(serial);
        envelope["body"]["frame"] = json!(json!({"version":1,"attachmentId":initial["attachmentId"],"requestId":serial.to_string(),"method":method,"args":args}).to_string());
        let message = common::message::packet(&envelope);
        assert_eq!(unsafe { pc.notify(message.as_ptr()) }, kResultOk);
        let reply = to_c
            .frames
            .borrow()
            .iter()
            .rev()
            .find(|f| f["body"]["type"] == "reply" && f["sequence"] == serial)
            .unwrap()
            .clone();
        serde_json::from_str(reply["body"]["frame"].as_str().unwrap()).unwrap()
    };
    test(&clock, &p, &c, &pc, &cc, &to_c, &to_p, &mut send);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn absent_handler_retains_edits_until_installed_and_cancels_unaccepted_begins_without_end() {
    use serde_json::json;
    for install in [false, true] {
        with_gesture_connection(|clock, _p, c, _pc, _cc, _to_c, _to_p, send| {
            let log = Rc::new(HostEditLog::default());
            assert_eq!(
                send("setParameter", json!(["drums.level", 0.625, 1]))["status"],
                "accepted"
            );
            for _ in 0..3 {
                clock.tick();
            }
            assert!(log.attempts.borrow().is_empty());
            if install {
                let handler = ComWrapper::new(ComponentHandler(log.clone()))
                    .to_com_ptr::<IComponentHandler>()
                    .unwrap();
                assert_eq!(
                    unsafe { c.setComponentHandler(handler.as_ptr()) },
                    kResultOk
                );
                drop(handler);
                for _ in 0..3 {
                    clock.tick();
                }
                let thread = std::thread::current().id();
                assert_eq!(
                    *log.accepted.borrow(),
                    vec![
                        (thread, HostEdit::Begin(0x1003)),
                        (thread, HostEdit::Value(0x1003, 0.625)),
                        (thread, HostEdit::End(0x1003)),
                    ]
                );
            }
            assert_eq!(unsafe { c.terminate() }, kResultOk);
            if !install {
                assert!(log.attempts.borrow().is_empty());
            }
        });
    }
}

#[test]
fn malformed_and_out_of_order_automation_cannot_call_the_host_or_consume_valid_delivery() {
    use serde_json::json;
    with_gesture_connection(|clock, _p, c, pc, cc, to_c, to_p, send| {
        let log = Rc::new(HostEditLog::default());
        let handler = ComWrapper::new(ComponentHandler(log.clone()))
            .to_com_ptr::<IComponentHandler>()
            .unwrap();
        assert_eq!(
            unsafe { c.setComponentHandler(handler.as_ptr()) },
            kResultOk
        );
        // Hold the first real delivery at the proxy, so malformed packets use
        // the authentic instance/attachment scope rather than a test back door.
        *to_c.refuse_type.borrow_mut() = Some("automation".into());
        assert_eq!(
            send("setParameter", json!(["drums.level", 0.625, 1]))["status"],
            "accepted"
        );
        clock.tick();
        let valid = to_c
            .frames
            .borrow()
            .iter()
            .find(|f| f["body"]["type"] == "automation")
            .unwrap()
            .clone();
        let mut malformed = Vec::new();
        for (field, value, expected) in [
            ("delivery", json!(0), kInvalidArgument),
            ("delivery", json!(2), kResultFalse),
            (
                "event",
                json!({"phase":"begin","parameter":0xffff}),
                kInvalidArgument,
            ),
            (
                "event",
                json!({"phase":"value","parameter":0x1003,"value":1.01}),
                kInvalidArgument,
            ),
            (
                "event",
                json!({"phase":"value","parameter":0x1003,"value":-0.01}),
                kInvalidArgument,
            ),
            (
                "event",
                json!({"phase":"value","parameter":0x1003,"value":0.625}),
                kResultFalse,
            ),
            (
                "event",
                json!({"phase":"end","parameter":0x1003}),
                kResultFalse,
            ),
            (
                "event",
                json!({"phase":"other","parameter":0x1003}),
                kInvalidArgument,
            ),
            (
                "event",
                json!({"phase":"begin","parameter":0x1003,"extra":true}),
                kInvalidArgument,
            ),
        ] {
            let mut packet = valid.clone();
            packet["body"][field] = value;
            malformed.push((packet, expected));
        }
        let mut wrong_sequence = valid.clone();
        wrong_sequence["sequence"] = json!(1);
        malformed.push((wrong_sequence, kNotImplemented));
        for (index, (packet, expected)) in malformed.iter().enumerate() {
            let message = common::message::packet(packet);
            assert_eq!(
                unsafe { cc.notify(message.as_ptr()) },
                *expected,
                "case {index}"
            );
            assert!(log.attempts.borrow().is_empty(), "case {index}");
        }
        let nested = common::message::packet(&valid);
        let during_call = cc.clone();
        let reentry = Rc::new(Cell::new(None));
        let nested_result = reentry.clone();
        *log.on_call.borrow_mut() = Some(Box::new(move || {
            nested_result.set(Some(unsafe { during_call.notify(nested.as_ptr()) }));
        }));
        let message = common::message::packet(&valid);
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
        assert_eq!(reentry.get(), Some(kNotInitialized));
        // An exact retry is acknowledged; a conflicting success receipt is refused.
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultOk);
        let mut conflict = valid.clone();
        conflict["body"]["event"] = json!({"phase":"begin","parameter":0x1004});
        let message = common::message::packet(&conflict);
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultFalse);
        let mut duplicate_begin = valid;
        duplicate_begin["body"]["delivery"] = json!(2);
        let message = common::message::packet(&duplicate_begin);
        assert_eq!(unsafe { cc.notify(message.as_ptr()) }, kResultFalse);
        let ack = to_p
            .frames
            .borrow()
            .iter()
            .find(|f| f["body"]["type"] == "automationAck")
            .unwrap()
            .clone();
        for delivery in [0, u64::MAX] {
            let mut invalid_ack = ack.clone();
            invalid_ack["body"]["delivery"] = json!(delivery);
            let message = common::message::packet(&invalid_ack);
            assert_eq!(unsafe { pc.notify(message.as_ptr()) }, kResultFalse);
        }
        assert_eq!(log.accepted.borrow().len(), 1);
        to_c.refuse_type.borrow_mut().take();
        for _ in 0..3 {
            clock.tick();
        }
        let thread = std::thread::current().id();
        assert_eq!(
            *log.accepted.borrow(),
            vec![
                (thread, HostEdit::Begin(0x1003)),
                (thread, HostEdit::Value(0x1003, 0.625)),
                (thread, HostEdit::End(0x1003)),
            ]
        );
        assert_eq!(
            *log.attempts.borrow(),
            vec![
                HostEdit::Begin(0x1003),
                HostEdit::Value(0x1003, 0.625),
                HostEdit::End(0x1003)
            ]
        );
    });
}

#[test]
fn a_full_host_automation_ledger_reserves_end_and_drains_in_exact_order_after_ack_resumes() {
    use serde_json::json;
    with_gesture_connection(|clock, _p, c, _pc, _cc, _to_c, to_p, send| {
        let log = Rc::new(HostEditLog::default());
        let handler = ComWrapper::new(ComponentHandler(log.clone()))
            .to_com_ptr::<IComponentHandler>()
            .unwrap();
        assert_eq!(
            unsafe { c.setComponentHandler(handler.as_ptr()) },
            kResultOk
        );
        *to_p.defer_type.borrow_mut() = Some("automationAck".into());
        assert_eq!(
            send("beginGesture", json!(["drums.level", 1]))["status"],
            "accepted"
        );
        clock.tick();
        let thread = std::thread::current().id();
        let mut expected = vec![(thread, HostEdit::Begin(0x1003))];
        // The production ledger has1024 slots: one pending Begin and one
        // reserved End leave1022 values. Service flushes DSP work each pass,
        // while the withheld application acknowledgement keeps this ledger full.
        for index in 0..1022 {
            let value = if index % 2 == 0 { 0.25 } else { 0.625 };
            let reply = send("setParameter", json!(["drums.level", value, 1]));
            assert_eq!(reply["status"], "accepted", "value {index}: {reply}");
            expected.push((thread, HostEdit::Value(0x1003, value)));
            clock.tick();
        }
        assert_eq!(log.accepted.borrow().len(), 1);
        let blocked = send("setParameter", json!(["drums.level", 0.99, 1]));
        assert_eq!(blocked["status"], "rejected");
        assert_eq!(
            send("endGesture", json!(["drums.level", 1]))["status"],
            "accepted"
        );
        expected.push((thread, HostEdit::End(0x1003)));
        assert_eq!(
            send("beginGesture", json!(["drums.pan", 1]))["status"],
            "rejected"
        );
        to_p.defer_type.borrow_mut().take();
        for _ in 0..32 {
            clock.tick();
        }
        assert_eq!(*log.accepted.borrow(), expected);
        assert_eq!(log.attempts.borrow().len(), 1024);
        assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.625);
        let session = send("getSamplerSession", json!([]));
        assert_eq!(
            session["result"]["connection"]["history"],
            json!({"undoCount":0,"redoCount":0})
        );
        assert_eq!(
            send("setParameter", json!(["drums.pan", 0.25, 1]))["status"],
            "accepted",
            "acknowledgement returns capacity"
        );
        clock.tick();
        assert_eq!(
            &log.accepted.borrow()[1024..],
            &[
                (thread, HostEdit::Begin(0x1004)),
                (thread, HostEdit::Value(0x1004, 0.25)),
                (thread, HostEdit::End(0x1004)),
            ]
        );
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CloseFault {
    None,
    RefusedEnd,
    LostRetired,
    LostFinalAck,
}

fn run_controller_close(fault: CloseFault) {
    use serde_json::{Value, json};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let log = Rc::new(HostEditLog::default());
    let handler = ComWrapper::new(ComponentHandler(log.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { c.setComponentHandler(handler.as_ptr()) },
        kResultOk
    );
    drop(handler);
    let template = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request")
        .unwrap()
        .clone();
    let initial: Value = serde_json::from_str(template["body"]["frame"].as_str().unwrap()).unwrap();
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    *to_p.defer_type.borrow_mut() = Some("automationAck".into());
    for (serial, method, args) in [
        (2, "beginGesture", json!(["drums.level", 1])),
        (3, "setParameter", json!(["drums.level", 0.625, 1])),
    ] {
        let mut envelope = template.clone();
        envelope["sequence"] = json!(serial);
        envelope["body"]["frame"] = json!(json!({"version":1,"attachmentId":initial["attachmentId"],"requestId":serial.to_string(),"method":method,"args":args}).to_string());
        assert_eq!(
            unsafe { pc.notify(common::message::packet(&envelope).as_ptr()) },
            kResultOk
        );
    }
    for _ in 0..3 {
        clock.tick();
    }
    let thread = std::thread::current().id();
    assert_eq!(
        *log.accepted.borrow(),
        vec![(thread, HostEdit::Begin(0x1003))]
    );
    match fault {
        CloseFault::RefusedEnd => {
            *log.refuse_once.borrow_mut() = Some(HostEdit::End(0x1003));
            assert_eq!(unsafe { c.terminate() }, kResultFalse);
            assert_eq!(
                *log.accepted.borrow(),
                vec![(thread, HostEdit::Begin(0x1003))]
            );
            assert_eq!(unsafe { c.getParameterCount() }, 23);
            assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.625);
            assert_eq!(
                unsafe { c.setComponentHandler(std::ptr::null_mut()) },
                kResultFalse
            );
            assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultFalse);
            let mut blocked = template.clone();
            blocked["sequence"] = json!(4);
            blocked["body"]["frame"] = json!(json!({"version":1,"attachmentId":initial["attachmentId"],"requestId":"4","method":"setParameter","args":["drums.pan",0.99,1]}).to_string());
            assert_eq!(
                unsafe { pc.notify(common::message::packet(&blocked).as_ptr()) },
                kResultFalse,
                "closing fences admission without consuming a new editor command"
            );
        }
        CloseFault::LostRetired => *to_c.defer_type.borrow_mut() = Some("retired".into()),
        CloseFault::LostFinalAck => *to_p.defer_type.borrow_mut() = Some("retiredAck".into()),
        CloseFault::None => {}
    }
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(
        *log.accepted.borrow(),
        vec![
            (thread, HostEdit::Begin(0x1003)),
            (thread, HostEdit::End(0x1003))
        ],
        "termination closes the actually accepted Begin without publishing the queued Value"
    );
    let mut old_retired_ack = None;
    match fault {
        CloseFault::LostRetired => {
            let first = to_c
                .frames
                .borrow()
                .iter()
                .find(|f| f["body"]["type"] == "retired")
                .unwrap()
                .clone();
            for _ in 0..3 {
                clock.tick();
            }
            assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultFalse);
            assert!(
                to_c.frames
                    .borrow()
                    .iter()
                    .filter(|f| f["body"]["type"] == "retired")
                    .all(|f| *f == first)
            );
            assert_eq!(
                log.accepted.borrow().len(),
                2,
                "retirement retry cannot repeat End"
            );
            to_c.defer_type.borrow_mut().take();
            assert_eq!(
                unsafe { cc.notify(common::message::packet(&first).as_ptr()) },
                kResultOk
            );
        }
        CloseFault::LostFinalAck => {
            old_retired_ack = Some(
                to_p.frames
                    .borrow()
                    .iter()
                    .find(|f| f["body"]["type"] == "retiredAck")
                    .unwrap()
                    .clone(),
            );
            clock.tick();
            assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultFalse);
            assert_eq!(log.accepted.borrow().len(), 2);
            // C has already released its only return channel. Explicit local
            // disconnect ends P's finite one-way final-Ack retention window.
            assert_eq!(unsafe { pc.disconnect(proxy_c.as_ptr()) }, kResultOk);
        }
        _ => {}
    }
    assert_eq!(
        log.attempts
            .borrow()
            .iter()
            .filter(|e| **e == HostEdit::End(0x1003))
            .count(),
        if fault == CloseFault::RefusedEnd {
            2
        } else {
            1
        }
    );
    assert_eq!(unsafe { c.terminate() }, kResultOk);
    assert_eq!(log.accepted.borrow().len(), 2);
    let late = to_c
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "automation")
        .unwrap()
        .clone();
    assert_eq!(
        unsafe { cc.notify(common::message::packet(&late).as_ptr()) },
        kNotInitialized
    );
    let fresh = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { fresh.initialize(std::ptr::null_mut()) }, kResultOk);
    let fresh_cc = fresh.cast::<IConnectionPoint>().unwrap();
    let (_fresh_to_c, fresh_proxy_c) = proxy(fresh_cc.clone());
    let (_fresh_to_p, fresh_proxy_p) = proxy(pc.clone());
    let fresh_log = Rc::new(HostEditLog::default());
    let fresh_handler = ComWrapper::new(ComponentHandler(fresh_log.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { fresh.setComponentHandler(fresh_handler.as_ptr()) },
        kResultOk
    );
    assert_eq!(unsafe { pc.connect(fresh_proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(
        unsafe { fresh_cc.connect(fresh_proxy_p.as_ptr()) },
        kResultOk
    );
    for _ in 0..3 {
        clock.tick();
    }
    assert!(
        fresh_log.accepted.borrow().is_empty(),
        "a fresh connection cannot replay cancelled automation from its predecessor"
    );
    assert_eq!(
        unsafe { fresh.getParamNormalized(0x1003) },
        0.625,
        "cancelling host delivery preserves the admitted runtime value"
    );
    assert_eq!(
        unsafe { fresh.getParamNormalized(0x1004) },
        0.5,
        "the request refused during close was never admitted"
    );
    if let Some(old_ack) = old_retired_ack {
        assert_eq!(
            unsafe { pc.notify(common::message::packet(&old_ack).as_ptr()) },
            kResultOk
        );
        assert!(fresh_log.accepted.borrow().is_empty());
    }
    assert_eq!(unsafe { fresh.terminate() }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn controller_termination_closes_a_host_begin_and_cancels_only_undelivered_automation() {
    run_controller_close(CloseFault::None);
}
#[test]
fn refused_close_end_and_lost_terminal_receipts_preserve_balancing_and_admitted_values() {
    for fault in [
        CloseFault::RefusedEnd,
        CloseFault::LostRetired,
        CloseFault::LostFinalAck,
    ] {
        run_controller_close(fault);
    }
}

#[test]
fn controller_disconnect_defers_reentrant_connect_until_terminal_cleanup_returns() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (_to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let (_replacement, replacement) = proxy(pc.clone());
    let during_release = cc.clone();
    let replacement_during_release = replacement.clone();
    let nested = Rc::new(Cell::new(None));
    let observed = nested.clone();
    *to_p.on_drop.borrow_mut() = Some(Box::new(move || {
        observed.set(Some(unsafe {
            during_release.connect(replacement_during_release.as_ptr())
        }));
    }));
    let old_address = proxy_p.as_ptr();
    drop(proxy_p);
    let result = unsafe { cc.disconnect(old_address) };
    assert_eq!(
        nested.get(),
        Some(kNotInitialized),
        "foreign Release cannot install a connection before the old terminal transaction finishes"
    );
    assert_eq!(result, kResultOk);
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(replacement.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.25);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
}

#[test]
fn reconnect_rejects_termination_from_the_old_return_channels_final_release() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let c = module.class::<IEditController>(&CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (_to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    *to_p.refuse_type.borrow_mut() = Some("retiredAck".into());
    assert_eq!(unsafe { cc.disconnect(proxy_p.as_ptr()) }, kResultOk);
    let nested = Rc::new(Cell::new(None));
    let observed = nested.clone();
    let during_release = (*c).clone();
    *to_p.on_drop.borrow_mut() = Some(Box::new(move || {
        observed.set(Some(unsafe { during_release.terminate() }));
    }));
    drop(proxy_p);
    assert_eq!(
        nested.get(),
        None,
        "refused terminal Ack retains its return channel"
    );
    to_p.refuse_type.borrow_mut().take();
    let (_replacement, replacement) = proxy(pc.clone());
    let connected = unsafe { cc.connect(replacement.as_ptr()) };
    assert_eq!(
        nested.get(),
        Some(kResultFalse),
        "reconnect must protect liveness while pumping/releasing its old return channel"
    );
    assert_eq!(connected, kResultOk);
    assert_eq!(unsafe { c.getParameterCount() }, 23);
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.getParamNormalized(0x1003) }, 0.25);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
}

#[test]
fn processor_close_bypasses_full_refused_traffic_and_preserves_signed_audio_until_end_acceptance() {
    use serde_json::{Value, json};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = common::runloop::host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let c = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    assert_eq!(unsafe { c.initialize(std::ptr::null_mut()) }, kResultOk);
    let pc = p.cast::<IConnectionPoint>().unwrap();
    let cc = c.cast::<IConnectionPoint>().unwrap();
    let (to_c, proxy_c) = proxy(cc.clone());
    let (to_p, proxy_p) = proxy(pc.clone());
    assert_eq!(unsafe { pc.connect(proxy_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { cc.connect(proxy_p.as_ptr()) }, kResultOk);
    let log = Rc::new(HostEditLog::default());
    let handler = ComWrapper::new(ComponentHandler(log.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { c.setComponentHandler(handler.as_ptr()) },
        kResultOk
    );
    let audio = p.cast::<IAudioProcessor>().unwrap();
    common::audio::start(&p, &audio, 48_000.0, 16);
    assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
    let template = to_p
        .frames
        .borrow()
        .iter()
        .find(|f| f["body"]["type"] == "request")
        .unwrap()
        .clone();
    let initial: Value = serde_json::from_str(template["body"]["frame"].as_str().unwrap()).unwrap();
    let wire = |serial: u64, method: &str, args: Value| {
        let mut envelope = template.clone();
        envelope["sequence"] = json!(serial);
        envelope["body"]["frame"] = json!(json!({"version":1,"attachmentId":initial["attachmentId"],"requestId":serial.to_string(),"method":method,"args":args}).to_string());
        common::message::packet(&envelope)
    };
    *to_c.defer_type.borrow_mut() = Some("reply".into());
    *to_p.refuse_type.borrow_mut() = Some("automationAck".into());
    for (serial, method, args) in [
        (2, "beginGesture", json!(["drums.level", 1])),
        (3, "setParameter", json!(["drums.level", 0.625, 1])),
    ] {
        assert_eq!(
            unsafe { pc.notify(wire(serial, method, args).as_ptr()) },
            kResultOk
        );
    }
    for _ in 0..3 {
        clock.tick();
    }
    let thread = std::thread::current().id();
    assert_eq!(
        *log.accepted.borrow(),
        vec![(thread, HostEdit::Begin(0x1003))]
    );
    *to_c.refuse_type.borrow_mut() = Some("automation".into());
    for _ in 0..3 {
        clock.tick();
    }
    let mut admitted = 0;
    let mut full = false;
    for serial in 4..=20 {
        let result = unsafe { pc.notify(wire(serial, "getParameterState", json!([])).as_ptr()) };
        if result == kResultFalse {
            full = true;
            break;
        }
        assert_eq!(result, kResultOk);
        admitted += 1;
    }
    assert!(
        admitted > 0 && full,
        "refused head fills bounded ordinary reply storage"
    );
    *log.refuse_once.borrow_mut() = Some(HostEdit::End(0x1003));
    assert_eq!(
        unsafe { p.terminate() },
        kResultFalse,
        "processor keeps ownership while the original handler refuses closure End"
    );
    assert_eq!(clock.unregistered.get(), 0);
    assert_eq!(unsafe { p.getBusCount(0, 1) }, 1);
    assert_eq!(unsafe { c.getParameterCount() }, 23);
    assert_eq!(
        *log.accepted.borrow(),
        vec![(thread, HostEdit::Begin(0x1003))]
    );
    // Failed teardown retains the already prepared endpoint and exact scalar
    // values. Exercise a real sample-map child, with an independent signed oracle.
    assert_eq!(unsafe { audio.setProcessing(1) }, kResultOk);
    let notes = common::audio::events(vec![common::audio::note_on(3, 36, 1.0)]);
    let mut block = common::audio::Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    assert_eq!(
        (&block.left[..3], &block.right[..3]),
        (&[0.0; 3][..], &[0.0; 3][..])
    );
    assert_eq!((block.left[3], block.right[3]), (-1.25, -1.25));
    assert_eq!(
        (block.left[4], block.right[4]),
        (-895.0 / 32768.0, -895.0 / 32768.0)
    );
    assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.setActive(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert_eq!(clock.unregistered.get(), 1);
    assert_eq!(
        *log.accepted.borrow(),
        vec![
            (thread, HostEdit::Begin(0x1003)),
            (thread, HostEdit::End(0x1003))
        ]
    );
    assert_eq!(
        log.attempts
            .borrow()
            .iter()
            .filter(|e| **e == HostEdit::End(0x1003))
            .count(),
        2
    );
    assert_eq!(unsafe { cc.disconnect(proxy_p.as_ptr()) }, kInvalidArgument);
    assert_eq!(unsafe { c.terminate() }, kResultOk);
}
