//! Bounded main-thread packets over the public VST3 message ABI only.
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ffi::{CStr, c_void},
    rc::Rc,
};
use vst3::{
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Vst::*, *},
};

pub(crate) const PENDING: usize = 8;
const MAX_WIRE: usize = 2 * dandrum_trigger_host::protocol::PATCH_OUTBOUND_BYTES_LIMIT + 4096;
pub(crate) type Scope = [u8; 16];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Body {
    Offer,
    Challenge,
    Confirm,
    Projection { frame: String },
    Request { frame: String },
    Reply { frame: String },
    Event { frame: String },
    Goodbye,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Packet {
    version: u32,
    pub source: Scope,
    pub target: Option<Scope>,
    pub sequence: u64,
    pub body: Body,
}
struct Outbound {
    key: u64,
    packet: Packet,
    bytes: Vec<u8>,
}
struct State {
    peer: Option<Rc<ComPtr<IConnectionPoint>>>,
    local: Option<Scope>,
    remote: Option<Scope>,
    next: u64,
    outbox: VecDeque<Rc<Outbound>>,
}
pub(crate) struct Transport {
    state: RefCell<State>,
    sending: Cell<bool>,
}
impl Default for Transport {
    fn default() -> Self {
        Self {
            state: RefCell::new(State {
                peer: None,
                local: None,
                remote: None,
                next: 0,
                outbox: VecDeque::with_capacity(PENDING),
            }),
            sending: Cell::new(false),
        }
    }
}
impl Transport {
    pub fn bound(&self) -> Option<(Scope, Scope)> {
        let state = self.state.borrow();
        Some((state.local?, state.remote?))
    }
    pub fn queued(&self, sequence: u64, body: &Body) -> bool {
        self.state
            .borrow()
            .outbox
            .iter()
            .any(|item| item.packet.sequence == sequence && item.packet.body == *body)
    }
    pub fn has_space(&self) -> bool {
        self.state.borrow().outbox.len() < PENDING
    }
    pub unsafe fn connect(&self, peer: *mut IConnectionPoint) -> tresult {
        if peer.is_null() {
            return kInvalidArgument;
        }
        if self.state.borrow().peer.is_some() {
            return kResultFalse;
        }
        let Some(local) = crate::platform::scope() else {
            return kResultFalse;
        };
        // Acquire the foreign reference before borrowing local state: even addRef
        // is a foreign call, and may reenter an ill-behaved host proxy.
        let peer = Rc::new(
            unsafe { ComRef::from_raw(peer) }
                .expect("checked pointer")
                .to_com_ptr(),
        );
        {
            let mut state = self.state.borrow_mut();
            if state.peer.is_some() {
                return kResultFalse;
            }
            state.peer = Some(peer);
            state.local = Some(local);
            state.remote = None;
        }
        self.queue(0, Body::Offer, None)
    }
    pub fn disconnect(&self, peer: *mut IConnectionPoint) -> tresult {
        let old = {
            let mut state = self.state.borrow_mut();
            if state.peer.as_ref().is_none_or(|p| p.as_ptr() != peer) {
                return kInvalidArgument;
            }
            state.local = None;
            state.remote = None;
            state.outbox.clear();
            state.peer.take()
        };
        drop(old);
        kResultOk
    }
    pub fn terminate_peer(&self) {
        let target = {
            let state = self.state.borrow();
            state
                .peer
                .as_ref()
                .zip(state.local.zip(state.remote))
                .map(|(peer, (source, target))| (peer.clone(), source, target))
        };
        if let Some((peer, source, target)) = target {
            // Shutdown supersedes queued replies. This one bounded terminal
            // notification carries no musical admission and is never retried.
            let packet = Packet {
                version: 1,
                source,
                target: Some(target),
                sequence: 0,
                body: Body::Goodbye,
            };
            let bytes = serde_json::to_vec(&packet).expect("fixed terminal packet");
            let message = message(&bytes);
            let _ = unsafe { peer.notify(message.as_ptr()) };
        }
    }
    pub fn close(&self) {
        let old = {
            let mut state = self.state.borrow_mut();
            state.local = None;
            state.remote = None;
            state.outbox.clear();
            state.peer.take()
        };
        drop(old);
    }
    fn queue(&self, sequence: u64, body: Body, target: Option<Scope>) -> tresult {
        let mut state = self.state.borrow_mut();
        let Some(local) = state.local else {
            return kNotInitialized;
        };
        if state.outbox.len() == PENDING {
            return kResultFalse;
        }
        let Some(next) = state.next.checked_add(1) else {
            return kResultFalse;
        };
        let packet = Packet {
            version: 1,
            source: local,
            target,
            sequence,
            body,
        };
        let Ok(bytes) = serde_json::to_vec(&packet) else {
            return kResultFalse;
        };
        if bytes.len() > MAX_WIRE {
            return kInvalidArgument;
        }
        state.next = next;
        state.outbox.push_back(Rc::new(Outbound {
            key: next,
            packet,
            bytes,
        }));
        kResultOk
    }
    pub fn send(&self, sequence: u64, body: Body) -> tresult {
        let Some((_, remote)) = self.bound() else {
            return kNotInitialized;
        };
        self.queue(sequence, body, Some(remote))
    }
    /// Copy the complete foreign payload before touching affine state. The proxy
    /// may run us again from getMessageID/getAttributes/getBinary.
    pub unsafe fn receive(&self, message: *mut IMessage) -> Result<Option<Packet>, tresult> {
        let Some(message) = (unsafe { ComRef::from_raw(message) }) else {
            return Err(kInvalidArgument);
        };
        let id = unsafe { message.getMessageID() };
        if id.is_null() || unsafe { CStr::from_ptr(id) } != c"dandrum.trigger.v1" {
            return Err(kResultFalse);
        }
        let Some(attributes) = (unsafe { ComRef::from_raw(message.getAttributes()) }) else {
            return Err(kInvalidArgument);
        };
        let (mut bytes, mut length) = (std::ptr::null(), 0);
        if unsafe { attributes.getBinary(c"payload".as_ptr(), &mut bytes, &mut length) }
            != kResultOk
            || bytes.is_null()
            || length as usize > MAX_WIRE
        {
            return Err(kInvalidArgument);
        }
        // SAFETY: the standard attribute contract retains this readable extent
        // for the message's lifetime. Deserialization owns every incoming string.
        let packet: Packet = serde_json::from_slice(unsafe {
            std::slice::from_raw_parts(bytes.cast::<u8>(), length as usize)
        })
        .map_err(|_| kInvalidArgument)?;
        if packet.version != 1 {
            return Err(kInvalidArgument);
        }
        let (local, remote) = {
            let s = self.state.borrow();
            (s.local.ok_or(kNotInitialized)?, s.remote)
        };
        match packet.body {
            Body::Offer => {
                if packet.target.is_some()
                    || packet.sequence != 0
                    || remote.is_some_and(|r| r != packet.source)
                {
                    return Err(kResultFalse);
                }
                let result = self.queue(0, Body::Challenge, Some(packet.source));
                if result != kResultOk {
                    return Err(result);
                }
                Ok(None)
            }
            Body::Challenge | Body::Confirm => {
                if packet.target != Some(local)
                    || packet.sequence != 0
                    || remote.is_some_and(|r| r != packet.source)
                {
                    return Err(kResultFalse);
                }
                if matches!(packet.body, Body::Challenge) {
                    let result = self.queue(0, Body::Confirm, Some(packet.source));
                    if result != kResultOk {
                        return Err(result);
                    }
                }
                self.state.borrow_mut().remote = Some(packet.source);
                Ok(None)
            }
            _ => {
                if packet.target != Some(local) || remote != Some(packet.source) {
                    return Err(kResultFalse);
                }
                Ok(Some(packet))
            }
        }
    }
    /// Each attempt is finite, including concurrent refill. Failed sends retain
    /// their head. A reentrant disconnect cannot acknowledge another connection.
    pub fn pump(&self) {
        if self.sending.replace(true) {
            return;
        }
        struct Sending<'a>(&'a Cell<bool>);
        impl Drop for Sending<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _sending = Sending(&self.sending);
        for _ in 0..PENDING {
            let next = {
                let state = self.state.borrow();
                state
                    .peer
                    .as_ref()
                    .zip(state.outbox.front())
                    .map(|(peer, head)| (peer.clone(), head.clone()))
            };
            let Some((peer, head)) = next else {
                break;
            };
            let message = message(&head.bytes);
            // No local borrow, session owner or owned COM release crosses notify.
            let accepted = unsafe { peer.notify(message.as_ptr()) } == kResultOk;
            if !accepted {
                break;
            }
            let mut state = self.state.borrow_mut();
            if state.outbox.front().is_some_and(|current| {
                current.key == head.key && current.packet.source == head.packet.source
            }) {
                state.outbox.pop_front();
            }
        }
    }
}

struct Message {
    attributes: ComPtr<IAttributeList>,
}
impl Class for Message {
    type Interfaces = (IMessage,);
}
impl IMessageTrait for Message {
    unsafe fn getMessageID(&self) -> FIDString {
        c"dandrum.trigger.v1".as_ptr()
    }
    unsafe fn setMessageID(&self, _id: FIDString) { /* Immutable borrowed outbound message. */
    }
    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes.as_ptr()
    }
}
struct Attributes(Vec<u8>);
impl Class for Attributes {
    type Interfaces = (IAttributeList,);
}
impl IAttributeListTrait for Attributes {
    unsafe fn setInt(&self, _id: *const i8, _v: i64) -> tresult {
        kNotImplemented
    }
    unsafe fn getInt(&self, _id: *const i8, _v: *mut i64) -> tresult {
        kResultFalse
    }
    unsafe fn setFloat(&self, _id: *const i8, _v: f64) -> tresult {
        kNotImplemented
    }
    unsafe fn getFloat(&self, _id: *const i8, _v: *mut f64) -> tresult {
        kResultFalse
    }
    unsafe fn setString(&self, _id: *const i8, _v: *const u16) -> tresult {
        kNotImplemented
    }
    unsafe fn getString(&self, _id: *const i8, _v: *mut u16, _n: u32) -> tresult {
        kResultFalse
    }
    unsafe fn setBinary(&self, _id: *const i8, _v: *const c_void, _n: u32) -> tresult {
        kNotImplemented
    }
    unsafe fn getBinary(
        &self,
        id: *const i8,
        data: *mut *const c_void,
        length: *mut u32,
    ) -> tresult {
        if id.is_null() || data.is_null() || length.is_null() {
            return kInvalidArgument;
        }
        if unsafe { CStr::from_ptr(id) } != c"payload" {
            return kResultFalse;
        }
        unsafe {
            *data = self.0.as_ptr().cast();
            *length = self.0.len() as u32;
        }
        kResultOk
    }
}
pub(crate) fn message(bytes: &[u8]) -> ComPtr<IMessage> {
    let attributes = ComWrapper::new(Attributes(bytes.to_vec()))
        .to_com_ptr::<IAttributeList>()
        .expect("standard attributes");
    ComWrapper::new(Message { attributes })
        .to_com_ptr::<IMessage>()
        .expect("standard message")
}
