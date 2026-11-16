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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewRoute {
    pub id: u64,
    pub parent_source: Scope,
    pub parent_target: Scope,
}
impl ViewRoute {
    pub fn reversed(&self) -> Self {
        Self {
            id: self.id,
            parent_source: self.parent_target,
            parent_target: self.parent_source,
        }
    }
}
fn terminal_phase(body: &Body) -> u8 {
    match body {
        Body::Closing => 0,
        Body::Disconnect => 1,
        Body::Goodbye => 2,
        Body::Closed => 3,
        Body::Retired => 4,
        Body::RetiredAck => 5,
        _ => unreachable!("terminal slot carries only close phases"),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "phase", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum HostAutomation {
    Begin { parameter: u32 },
    Value { parameter: u32, value: f64 },
    End { parameter: u32 },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Body {
    Offer,
    Challenge,
    Confirm,
    Projection {
        frame: String,
    },
    Request {
        frame: String,
    },
    Reply {
        frame: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        publication: Option<String>,
    },
    Event {
        frame: String,
        publication: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origins: Option<Vec<String>>,
    },
    Automation {
        delivery: u64,
        event: HostAutomation,
    },
    AutomationAck {
        delivery: u64,
    },
    Closed,
    Closing,
    Disconnect,
    Retired,
    RetiredAck,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewRoute>,
}
struct Outbound {
    key: u64,
    packet: Packet,
    bytes: Vec<u8>,
}
struct RetiredPeer {
    local: Scope,
    remote: Scope,
    peer: Option<Rc<ComPtr<IConnectionPoint>>>,
}
struct Terminal {
    peer: Rc<ComPtr<IConnectionPoint>>,
    packet: Packet,
    bytes: Vec<u8>,
}
struct State {
    peer: Option<Rc<ComPtr<IConnectionPoint>>>,
    local: Option<Scope>,
    remote: Option<Scope>,
    next: u64,
    outbox: VecDeque<Rc<Outbound>>,
    closing: bool,
    retired: Option<RetiredPeer>,
}
pub(crate) struct Transport {
    route: Option<ViewRoute>,
    state: RefCell<State>,
    sending: Cell<bool>,
    automation_sending: Cell<bool>,
    terminal_sending: Cell<bool>,
    terminal_pending: RefCell<Option<Rc<Terminal>>>,
}
impl Default for Transport {
    fn default() -> Self {
        Self {
            route: None,
            state: RefCell::new(State {
                peer: None,
                local: None,
                remote: None,
                next: 0,
                outbox: VecDeque::with_capacity(PENDING),
                closing: false,
                retired: None,
            }),
            sending: Cell::new(false),
            automation_sending: Cell::new(false),
            terminal_sending: Cell::new(false),
            terminal_pending: RefCell::new(None),
        }
    }
}
impl Transport {
    #[cfg(feature = "trigger-editor")]
    pub fn for_view(route: ViewRoute, peer: Rc<ComPtr<IConnectionPoint>>) -> Option<Self> {
        let local = crate::platform::scope()?;
        let mut transport = Self {
            route: Some(route),
            ..Self::default()
        };
        let state = transport.state.get_mut();
        state.peer = Some(peer);
        state.local = Some(local);
        Some(transport)
    }
    /// Carry the validated parent scopes and already retained standard peer
    /// together. Rc cloning makes no foreign AddRef under the state borrow.
    #[cfg(feature = "trigger-editor")]
    pub fn bound_peer(&self) -> Option<(Scope, Scope, Rc<ComPtr<IConnectionPoint>>)> {
        let state = self.state.borrow();
        Some((state.local?, state.remote?, state.peer.clone()?))
    }
    pub fn peer_matches(&self, peer: *mut IConnectionPoint) -> bool {
        self.state
            .borrow()
            .peer
            .as_ref()
            .is_some_and(|p| p.as_ptr() == peer)
    }
    pub fn retired_peer_matches(&self, peer: *mut IConnectionPoint) -> bool {
        self.state
            .borrow()
            .retired
            .as_ref()
            .and_then(|r| r.peer.as_ref())
            .is_some_and(|p| p.as_ptr() == peer)
    }
    pub fn retired_pending(&self) -> bool {
        self.state
            .borrow()
            .retired
            .as_ref()
            .is_some_and(|r| r.peer.is_some())
    }
    pub fn terminal_active(&self) -> bool {
        self.terminal_sending.get()
    }
    #[cfg(feature = "trigger-editor")]
    pub fn normal_active(&self) -> bool {
        self.sending.get()
    }
    pub fn automation_active(&self) -> bool {
        self.automation_sending.get()
    }
    pub fn is_current(&self, packet: &Packet) -> bool {
        self.bound()
            .is_some_and(|(local, remote)| packet.target == Some(local) && packet.source == remote)
    }
    pub fn is_retired(&self, packet: &Packet) -> bool {
        self.state
            .borrow()
            .retired
            .as_ref()
            .is_some_and(|r| packet.target == Some(r.local) && packet.source == r.remote)
    }
    pub fn retire(&self) {
        let old = {
            let mut state = self.state.borrow_mut();
            let Some((local, remote)) = state.local.zip(state.remote) else {
                return;
            };
            let peer = state.peer.take();
            state.local = None;
            state.remote = None;
            state.outbox.clear();
            state.retired.replace(RetiredPeer {
                local,
                remote,
                peer,
            })
        };
        drop(old);
    }
    pub fn release_retired(&self) {
        let (old, scope) = {
            let mut state = self.state.borrow_mut();
            let Some(retired) = state.retired.as_mut() else {
                return;
            };
            (retired.peer.take(), (retired.local, retired.remote))
        };
        let pending = {
            let mut pending = self.terminal_pending.borrow_mut();
            if pending
                .as_ref()
                .is_some_and(|p| p.packet.source == scope.0 && p.packet.target == Some(scope.1))
            {
                pending.take()
            } else {
                None
            }
        };
        drop(old);
        drop(pending);
    }
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
        self.pump_terminal();
        if self.terminal_pending.borrow().is_some() {
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
            state.closing = false;
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
                view: self.route.clone(),
            };
            let bytes = serde_json::to_vec(&packet).expect("fixed terminal packet");
            let message = message(&bytes);
            let _ = unsafe { peer.notify(message.as_ptr()) };
        }
    }
    /// Closing supersedes ordinary traffic, but retains the scoped peer for
    /// terminal retries until explicit disconnect/final COM lifetime.
    pub fn begin_closing(&self) {
        let mut state = self.state.borrow_mut();
        state.closing = true;
        state.outbox.clear();
    }
    pub fn terminal(&self, body: Body) -> tresult {
        self.stage_terminal(body, false)
    }
    pub fn terminal_retired(&self, body: Body) -> tresult {
        self.stage_terminal(body, true)
    }
    fn stage_terminal(&self, body: Body, retired: bool) -> tresult {
        let target = {
            let state = self.state.borrow();
            if retired {
                state
                    .retired
                    .as_ref()
                    .and_then(|r| r.peer.as_ref().map(|p| (p.clone(), r.local, r.remote)))
            } else {
                state
                    .peer
                    .as_ref()
                    .zip(state.local.zip(state.remote))
                    .map(|(peer, (source, target))| (peer.clone(), source, target))
            }
        };
        let Some((peer, source, target)) = target else {
            return self.pump_terminal();
        };
        let packet = Packet {
            version: 1,
            source,
            target: Some(target),
            sequence: 0,
            body,
            view: self.route.clone(),
        };
        let bytes = serde_json::to_vec(&packet).expect("bounded terminal packet");
        let item = Rc::new(Terminal {
            peer,
            packet,
            bytes,
        });
        let old = {
            let mut pending = self.terminal_pending.borrow_mut();
            if let Some(previous) = pending.as_ref()
                && previous.packet.source == source
                && previous.packet.target == Some(target)
                && terminal_phase(&previous.packet.body) > terminal_phase(&item.packet.body)
            {
                None
            } else {
                pending.replace(item)
            }
        };
        drop(old);
        self.pump_terminal()
    }
    pub fn pump_terminal(&self) -> tresult {
        if self.terminal_sending.replace(true) {
            return kResultOk;
        }
        struct Sending<'a>(&'a Cell<bool>);
        impl Drop for Sending<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _sending = Sending(&self.terminal_sending);
        for _ in 0..8 {
            let item = self.terminal_pending.borrow_mut().take();
            let Some(item) = item else {
                return kResultOk;
            };
            let message = message(&item.bytes);
            let result = unsafe { item.peer.notify(message.as_ptr()) };
            if result != kResultOk {
                let retry = matches!(item.packet.body, Body::RetiredAck) || {
                    let state = self.state.borrow();
                    state.peer.is_some()
                        && state.local == Some(item.packet.source)
                        && state.remote == item.packet.target
                        || state.retired.as_ref().is_some_and(|r| {
                            r.peer.is_some()
                                && r.local == item.packet.source
                                && Some(r.remote) == item.packet.target
                        })
                };
                let mut pending = self.terminal_pending.borrow_mut();
                if retry && pending.is_none() {
                    *pending = Some(item);
                }
                return result;
            }
        }
        kResultOk
    }
    /// A prepared terminal Ack owns its return channel independently of the
    /// ordinary peer, and must survive this synchronous peer release.
    pub fn release_peer(&self) {
        let old = {
            let mut state = self.state.borrow_mut();
            state.local = None;
            state.remote = None;
            state.outbox.clear();
            state.peer.take()
        };
        drop(old);
    }
    pub fn close(&self) {
        let old = {
            let mut state = self.state.borrow_mut();
            state.local = None;
            state.remote = None;
            state.outbox.clear();
            (state.peer.take(), state.retired.take())
        };
        drop(old);
        let pending = self.terminal_pending.borrow_mut().take();
        drop(pending);
    }
    fn queue(&self, sequence: u64, body: Body, target: Option<Scope>) -> tresult {
        let mut state = self.state.borrow_mut();
        let Some(local) = state.local else {
            return kNotInitialized;
        };
        if state.closing {
            return kResultFalse;
        }
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
            view: self.route.clone(),
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
        let packet = unsafe { Self::decode(message) }?;
        self.receive_packet(packet)
    }
    pub unsafe fn decode(message: *mut IMessage) -> Result<Packet, tresult> {
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
        Ok(packet)
    }
    pub fn receive_packet(&self, packet: Packet) -> Result<Option<Packet>, tresult> {
        if packet.view != self.route.as_ref().map(ViewRoute::reversed) {
            return Err(kResultFalse);
        }
        if matches!(packet.body, Body::Closed | Body::RetiredAck) && packet.sequence == 0 {
            let state = self.state.borrow();
            if state
                .retired
                .as_ref()
                .is_some_and(|r| packet.target == Some(r.local) && packet.source == r.remote)
            {
                return Ok(Some(packet));
            }
        }
        let (local, remote) = {
            let s = self.state.borrow();
            if s.closing && matches!(packet.body, Body::Offer | Body::Challenge | Body::Confirm) {
                return Err(kResultFalse);
            }
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
            // Every pump may retry an Automation, including reply/feedback
            // paths. Keep this exact foreign call fenced after an inline Ack.
            self.automation_sending
                .set(matches!(&head.packet.body, Body::Automation { .. }));
            struct AutomationSending<'a>(&'a Cell<bool>);
            impl Drop for AutomationSending<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }
            let _automation = AutomationSending(&self.automation_sending);
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
