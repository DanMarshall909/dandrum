//! Affine VST3 view projection. The instrument authority remains in Processor.
use super::{Bootstrap, ControllerCore, HostCall, HostDelivery, Registration, platform};
use crate::transport::{Body, NativeFiles, Packet, Transport, ViewRoute};
use dandrum_trigger_editor::{EditorAssets, NativeEditor, NativeFileSelection};
use dandrum_trigger_host::editor_bridge::{
    browser_request, delivery_script, initialization_script,
};
use std::{
    cell::{Cell, RefCell},
    ffi::c_char,
    rc::{Rc, Weak},
};
use vst3::{
    Class, ComWrapper,
    Steinberg::{Linux::*, *},
};

const WIDTH: i32 = NativeEditor::WIDTH as i32;
const HEIGHT: i32 = NativeEditor::HEIGHT as i32;

pub(super) fn create(controller: &Rc<ControllerCore>) -> *mut IPlugView {
    let core = Rc::new(ViewCore {
        thread: controller.thread,
        controller: Rc::downgrade(controller),
        child: RefCell::new(None),
        registration: RefCell::new(None),
        connection: RefCell::new(None),
        busy: Cell::new(false),
        selection_dialog: RefCell::new(None),
        frame: RefCell::new(None),
        abi: Cell::new(std::ptr::null_mut()),
        middle_c_octave: controller.middle_c_octave.get(),
        content_scale: Cell::new(1.0),
        size: Cell::new((WIDTH, HEIGHT)),
        pending_scale: Cell::new(None),
    });
    let view = ComWrapper::new(View(core.clone()))
        .to_com_ptr::<IPlugView>()
        .expect("view includes the standard interface")
        .into_raw();
    core.abi.set(view);
    view
}

/// Compare only the known SDK name, including its NUL. Never scan an unknown
/// host string without a bound, and stop before reading beyond a shorter name.
pub(super) unsafe fn matches(value: *const c_char, expected: &[u8]) -> bool {
    !value.is_null()
        && expected.iter().enumerate().all(|(index, byte)| {
            // SAFETY: the caller supplies a NUL-terminated SDK string. Matching
            // stops at its first differing byte, including a shorter NUL.
            unsafe { *value.add(index) as u8 == *byte }
        })
}

struct View(Rc<ViewCore>);
pub(super) struct ViewCore {
    thread: libc::pid_t,
    controller: Weak<ControllerCore>,
    child: RefCell<Option<Rc<NativeEditor>>>,
    registration: RefCell<Option<Registration>>,
    connection: RefCell<Option<Rc<BrowserConnection>>>,
    busy: Cell<bool>,
    selection_dialog: RefCell<Option<NativeFileSelection>>,
    frame: RefCell<Option<Rc<vst3::ComPtr<IPlugFrame>>>>,
    // Non-owning ABI identity, used only during a live scale-interface call.
    abi: Cell<*mut IPlugView>,
    middle_c_octave: u8,
    content_scale: Cell<f32>,
    size: Cell<(i32, i32)>,
    pending_scale: Cell<Option<f32>>,
}
pub(super) struct BrowserConnection {
    transport: Transport,
    bootstrap: RefCell<Option<String>>,
    next_request: Cell<u64>,
    started: Cell<bool>,
    closing: Cell<bool>,
    closing_confirmed: Cell<bool>,
    closed: Cell<bool>,
    host_delivery: HostDelivery,
    selection_request: Cell<Option<u64>>,
    selection_started: Cell<u64>,
    selection_result: RefCell<Option<Body>>,
}
impl BrowserConnection {
    pub(super) fn receive_retired(&self, packet: Packet) -> tresult {
        match self.transport.receive_packet(packet) {
            Ok(Some(packet))
                if self.closed.get()
                    && packet.sequence == 0
                    && matches!(packet.body, Body::Retired) =>
            {
                self.acknowledge_retirement()
            }
            Err(error) => error,
            _ => kResultFalse,
        }
    }
    fn acknowledge_retirement(&self) -> tresult {
        self.closed.set(true);
        self.transport.terminal(Body::RetiredAck)
    }
    pub(super) fn revoke(&self) {
        self.transport.close();
    }
}
impl std::ops::Deref for View {
    type Target = ViewCore;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Class for View {
    type Interfaces = (IPlugView, IPlugViewContentScaleSupport);
}
impl ViewCore {
    fn live_controller(&self) -> Option<Rc<ControllerCore>> {
        // Reject a foreign thread before upgrading or inspecting affine Rc data.
        if self.thread != platform::thread_id() {
            return None;
        }
        self.controller.upgrade().filter(|controller| {
            controller.main()
                && !controller.closing.get()
                && controller
                    .projection
                    .try_borrow()
                    .is_ok_and(|p| p.is_some())
        })
    }
    fn live(&self) -> bool {
        self.live_controller().is_some()
    }
    fn sync_native_scale(&self, factor: f32) -> bool {
        let child = self.child.borrow().clone();
        child.is_none_or(|child| child.sync_host_scale(f64::from(factor)).is_ok())
    }
    pub(super) fn host_gesture_active(&self) -> bool {
        self.connection
            .borrow()
            .as_ref()
            .is_some_and(|c| c.host_delivery.active())
    }
    pub(super) fn route_matches(&self, packet: &Packet) -> bool {
        self.connection
            .borrow()
            .as_ref()
            .is_some_and(|connection| connection.transport.route_matches(packet))
    }
    fn attach(self: &Rc<Self>, parent: u32) -> tresult {
        let Some(controller) = self.live_controller() else {
            return kResultFalse;
        };
        if self.busy.replace(true) {
            return kResultFalse;
        }
        let _call = HostCall(&self.busy);
        if self.child.borrow().is_some()
            || self.registration.borrow().is_some()
            || controller
                .native_view
                .borrow()
                .as_ref()
                .is_some_and(|view| {
                    view.child.borrow().is_some()
                        || view.registration.borrow().is_some()
                        || view.connection.borrow().is_some()
                })
            || controller.transport.bound().is_none()
            || controller.host_busy.replace(true)
        {
            return kResultFalse;
        }
        let _host_call = HostCall(&controller.host_busy);
        let Some(runloop) = controller.view_runloop.borrow().clone() else {
            return kResultFalse;
        };
        let Ok(assets) = EditorAssets::open(&controller.root) else {
            return kResultFalse;
        };
        if gtk::is_initialized() && !gtk::is_initialized_main_thread() {
            return kResultFalse;
        }
        if gtk::init().is_err() {
            return kResultFalse;
        }
        let weak = Rc::downgrade(self);
        let Ok(child) = NativeEditor::create_deferred(parent, assets, move |message| {
            if let Some(view) = weak.upgrade() {
                view.request(&message);
            }
        }) else {
            return kResultFalse;
        };
        if child
            .sync_host_scale(f64::from(self.content_scale.get()))
            .is_err()
            || child.set_visible(true).is_err()
        {
            return kResultFalse;
        }
        // Publish ownership before any foreign registration can reenter. The
        // WebView is blank until the separately authenticated namespace arrives.
        let old = controller.native_view.replace(Some(self.clone()));
        drop(old);
        let timer = ComWrapper::new(ViewTimer(self.thread, Rc::downgrade(self)))
            .to_com_ptr::<ITimerHandler>()
            .expect("view timer interface");
        if unsafe { runloop.registerTimer(timer.as_ptr(), 10) } != kResultOk {
            return kResultFalse;
        }
        *self.registration.borrow_mut() = Some(Registration { runloop, timer });
        let opened = (|| {
            let (local, remote, peer) = controller.transport.bound_peer()?;
            let id = controller.next_view.get();
            let next = id.checked_add(1)?;
            let transport = Transport::for_view(
                ViewRoute {
                    id,
                    parent_source: local,
                    parent_target: remote,
                },
                peer,
            )?;
            controller.next_view.set(next);
            Some(Rc::new(BrowserConnection {
                transport,
                bootstrap: RefCell::new(None),
                next_request: Cell::new(1),
                started: Cell::new(false),
                closing: Cell::new(false),
                closing_confirmed: Cell::new(false),
                closed: Cell::new(false),
                host_delivery: HostDelivery::default(),
                selection_request: Cell::new(None),
                selection_started: Cell::new(0),
                selection_result: RefCell::new(None),
            }))
        })();
        let Some(connection) = opened else {
            self.release_registration();
            return kResultFalse;
        };
        *self.child.borrow_mut() = Some(Rc::new(child));
        *self.connection.borrow_mut() = Some(connection.clone());
        let _ = connection.transport.offer();
        connection.transport.pump();
        kResultOk
    }
    pub(super) fn receive(self: &Rc<Self>, packet: Packet) -> tresult {
        if self.thread != platform::thread_id() {
            return kResultFalse;
        }
        // Inline attach/close callbacks already hold this fence. An external
        // delivery takes it too, so GTK/host reentry cannot destroy this view.
        let _call = (!self.busy.replace(true)).then(|| HostCall(&self.busy));
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection else {
            return kResultFalse;
        };
        let received = connection.transport.receive_packet(packet);
        match received {
            Err(error) => return error,
            Ok(None) => {}
            Ok(Some(packet)) => match packet.body {
                Body::Projection { frame } if packet.sequence == 0 && !connection.closing.get() => {
                    let Some(namespace) = Bootstrap::parse(&frame) else {
                        return kInvalidArgument;
                    };
                    if connection
                        .bootstrap
                        .borrow()
                        .as_ref()
                        .is_some_and(|previous| previous != &frame)
                    {
                        return kResultFalse;
                    }
                    *connection.bootstrap.borrow_mut() = Some(frame);
                    if let Some(controller) = self.controller.upgrade() {
                        let retired = controller.retired_browser.borrow_mut().take();
                        if let Some(retired) = retired {
                            retired.revoke();
                        }
                    }
                    if !connection.started.get() {
                        let child = self.child.borrow().clone();
                        let Some(child) = child else {
                            return kResultFalse;
                        };
                        let initializer = initialization_script(
                            &namespace.instance_id,
                            &namespace.attachment_id,
                            std::num::NonZeroU64::new(1).expect("first browser request"),
                            self.middle_c_octave,
                        );
                        if child.start_application(&initializer).is_err() {
                            return kResultFalse;
                        }
                        connection.started.set(true);
                    }
                }
                Body::Reply { frame, .. } | Body::Event { frame, .. }
                    if !connection.closing.get() =>
                {
                    let child = self.child.borrow().clone();
                    let Some(child) = child else {
                        return kResultFalse;
                    };
                    self.start_selection(&connection, packet.sequence, &frame, &child);
                    if child.evaluate_script(&delivery_script(&frame)).is_err() {
                        return kResultFalse;
                    }
                }
                Body::ClosingAck if packet.sequence == 0 && connection.closing.get() => {
                    connection.closing_confirmed.set(true);
                }
                Body::NativeSelectionAck { request }
                    if packet.sequence == 0 && !connection.closing.get() =>
                {
                    let matches=connection.selection_result.borrow().as_ref().is_some_and(|result|matches!(result,Body::NativeSelection{request:pending,..} if *pending==request));
                    if matches {
                        connection.selection_result.borrow_mut().take();
                    } else if request != connection.selection_started.get() {
                        return kResultFalse;
                    }
                }
                Body::Automation { delivery, event }
                    if packet.sequence == 0 && !connection.closing.get() =>
                {
                    let Some(controller) = self.live_controller() else {
                        return kResultFalse;
                    };
                    let result = controller.receive_automation_at(
                        &connection.host_delivery,
                        &connection.transport,
                        delivery,
                        event,
                    );
                    if result != kResultOk {
                        return result;
                    }
                }
                Body::Retired if packet.sequence == 0 && connection.closing.get() => {
                    let result = connection.acknowledge_retirement();
                    if result != kResultOk {
                        return result;
                    }
                }
                _ => return kNotImplemented,
            },
        }
        connection.transport.pump();
        kResultOk
    }
    fn request(&self, message: &str) {
        if self.live_controller().is_none() {
            return;
        }
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection else {
            return;
        };
        if connection.closing.get() {
            return;
        }
        let bootstrap = connection
            .bootstrap
            .borrow()
            .as_deref()
            .and_then(Bootstrap::parse);
        let Some(bootstrap) = bootstrap else {
            return;
        };
        let Some(request) = browser_request(&bootstrap.attachment_id, message) else {
            return;
        };
        let sequence = connection.next_request.get();
        let Some(next) = sequence.checked_add(1) else {
            return;
        };
        if connection.transport.send(
            sequence,
            Body::Request {
                frame: message.into(),
            },
        ) == kResultOk
        {
            if request.is_native_selection() {
                connection.selection_request.set(Some(sequence));
            }
            connection.next_request.set(next);
            connection.transport.pump();
        }
    }
    fn start_selection(
        self: &Rc<Self>,
        connection: &Rc<BrowserConnection>,
        sequence: u64,
        frame: &str,
        child: &NativeEditor,
    ) {
        if sequence == 0
            || connection.selection_request.get() != Some(sequence)
            || sequence <= connection.selection_started.get()
        {
            return;
        }
        let Ok(reply) = serde_json::from_str::<serde_json::Value>(frame) else {
            return;
        };
        if reply["status"] != "accepted" || reply["result"]["phase"] != "choosing" {
            return;
        }
        connection.selection_started.set(sequence);
        connection.selection_request.set(None);
        let weak = Rc::downgrade(self);
        let channel = Rc::downgrade(connection);
        let dialog = child.choose_sample_files(move |files| {
            use std::os::unix::ffi::OsStrExt;
            let (Some(view), Some(connection)) = (weak.upgrade(), channel.upgrade()) else {
                return;
            };
            if !view.live() || connection.closing.get() {
                return;
            }
            let result = match files {
                None => NativeFiles::Cancelled,
                Some(paths)
                    if paths.is_empty()
                        || paths.len() > dandrum_trigger_host::samples::MAX_SELECTION_FILES =>
                {
                    NativeFiles::InvalidFiles
                }
                Some(paths) => {
                    let paths: Vec<Vec<u8>> = paths
                        .into_iter()
                        .map(|path| path.as_os_str().as_bytes().to_vec())
                        .collect();
                    if paths.iter().any(|path| {
                        path.is_empty()
                            || path.len() >= libc::PATH_MAX as usize
                            || path.contains(&0)
                    }) {
                        NativeFiles::InvalidFiles
                    } else {
                        NativeFiles::Selected { paths }
                    }
                }
            };
            *connection.selection_result.borrow_mut() = Some(Body::NativeSelection {
                request: sequence,
                result,
            });
        });
        match dialog {
            Ok(dialog) => {
                *self.selection_dialog.borrow_mut() = Some(dialog);
            }
            Err(_) => {
                *connection.selection_result.borrow_mut() = Some(Body::NativeSelection {
                    request: sequence,
                    result: NativeFiles::Unavailable,
                });
            }
        }
    }
    fn service_selection(&self, connection: &BrowserConnection) {
        let finished = self
            .selection_dialog
            .borrow()
            .as_ref()
            .is_some_and(|dialog| !dialog.is_pending());
        if finished {
            let dialog = self.selection_dialog.borrow_mut().take();
            drop(dialog);
        }
        let result = connection.selection_result.borrow().clone();
        if let Some(result) = result
            && !connection.closing.get()
            && !connection.transport.queued(0, &result)
        {
            let _ = connection.transport.send(0, result);
        }
    }
    fn close_connection(&self) -> bool {
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection else {
            return true;
        };
        connection.closing.set(true);
        if connection.transport.bound().is_none() {
            connection.revoke();
            self.connection.borrow_mut().take();
            return true;
        }
        if connection.closed.get() {
            let _ = connection.transport.pump_terminal();
        } else {
            connection.transport.begin_closing();
            if !connection.closing_confirmed.get() {
                let _ = connection.transport.terminal(Body::Closing);
                if !connection.closing_confirmed.get() {
                    return false;
                }
            }
            let _ = connection.transport.terminal(Body::Closed);
        }
        if !connection.closed.get() || connection.transport.terminal_pending() {
            return false;
        }
        self.connection.borrow_mut().take();
        if let Some(controller) = self.controller.upgrade() {
            let old = controller.retired_browser.replace(Some(connection));
            if let Some(old) = old {
                old.revoke();
            }
        }
        true
    }
    fn release_registration(&self) -> bool {
        let registration = self.registration.borrow_mut().take();
        if let Some(registration) = registration
            && let Err(registration) = registration.unregister()
        {
            *self.registration.borrow_mut() = Some(registration);
            return false;
        }
        true
    }
    pub(super) fn detach(&self) -> tresult {
        self.detach_impl(false)
    }
    pub(super) fn revoke_parent(&self) -> tresult {
        self.detach_impl(true)
    }
    fn detach_impl(&self, parent_closing: bool) -> tresult {
        if self.thread != platform::thread_id() || self.busy.replace(true) {
            return kResultFalse;
        }
        let _call = HostCall(&self.busy);
        let dialog = self.selection_dialog.borrow_mut().take();
        drop(dialog);
        let child = self.child.borrow_mut().take();
        drop(child);
        let frame = self.frame.borrow_mut().take();
        drop(frame);
        let connection = self.connection.borrow().clone();
        if let Some(connection) = connection
            && let Some(controller) = self.controller.upgrade()
            && !controller.close_host_delivery(&connection.host_delivery)
        {
            return kResultFalse;
        }
        if parent_closing {
            // The parent connection's close certificate retires both roles.
            // Do not wait for a view handshake with an already-closing parent.
            let connection = self.connection.borrow_mut().take();
            if let Some(connection) = connection {
                connection.revoke();
            }
        } else if !self.close_connection() {
            return kResultFalse;
        }
        if self.release_registration() {
            if let Some(controller) = self.controller.upgrade() {
                let old = {
                    let mut active = controller.native_view.borrow_mut();
                    if active
                        .as_ref()
                        .is_some_and(|view| std::ptr::eq(view.as_ref(), self))
                    {
                        active.take()
                    } else {
                        None
                    }
                };
                drop(old);
            }
            kResultOk
        } else {
            kResultFalse
        }
    }
    fn service(&self) {
        if self.thread != platform::thread_id() || self.busy.get() {
            return;
        }
        if self.child.borrow().is_none() {
            let _ = self.detach();
            return;
        }
        self.busy.set(true);
        let _call = HostCall(&self.busy);
        let _ = NativeEditor::service();
        // Hosts can realize a scaled parent after accepting the size. A tick
        // nested inside resizeView may instead see a provisional parent.
        if self.pending_scale.get().is_none() {
            let _ = self.sync_native_scale(self.content_scale.get());
        }
        let connection = self.connection.borrow().clone();
        if let Some(connection) = connection {
            self.service_selection(&connection);
            connection.transport.pump();
        }
    }
}
impl Drop for ViewCore {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}
impl Drop for View {
    fn drop(&mut self) {
        let _ = self.0.detach();
    }
}
struct ViewTimer(libc::pid_t, Weak<ViewCore>);
impl Class for ViewTimer {
    type Interfaces = (ITimerHandler,);
}
impl ITimerHandlerTrait for ViewTimer {
    unsafe fn onTimer(&self) {
        if self.0 == platform::thread_id()
            && let Some(view) = self.1.upgrade()
        {
            view.service();
        }
    }
}
impl IPlugViewTrait for View {
    unsafe fn isPlatformTypeSupported(&self, kind: FIDString) -> tresult {
        if self.live()
            // SAFETY: the host provides the optional SDK platform name.
            && unsafe { matches(kind, b"X11EmbedWindowID\0") }
        {
            kResultOk
        } else {
            kResultFalse
        }
    }
    unsafe fn attached(&self, parent: *mut std::ffi::c_void, kind: FIDString) -> tresult {
        // SAFETY: same live host platform string contract as the query above.
        if unsafe { self.isPlatformTypeSupported(kind) } != kResultOk {
            return kResultFalse;
        }
        if parent.is_null() {
            return kInvalidArgument;
        }
        let Ok(parent) = u32::try_from(parent as usize) else {
            return kInvalidArgument;
        };
        self.0.attach(parent)
    }
    unsafe fn removed(&self) -> tresult {
        self.detach()
    }
    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyDown(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyUp(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }
    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        if size.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one writable SDK output, checked non-null above.
        unsafe {
            *size = ViewRect {
                left: 0,
                top: 0,
                right: self.size.get().0,
                bottom: self.size.get().1,
            };
        }
        kResultOk
    }
    unsafe fn onSize(&self, size: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        // SAFETY: the host supplies one readable SDK rectangle or null.
        let Some(size) = (unsafe { size.as_ref() }) else {
            return kInvalidArgument;
        };
        let pending = self.pending_scale.get();
        let factor = pending.unwrap_or(self.content_scale.get());
        if (
            size.right.checked_sub(size.left),
            size.bottom.checked_sub(size.top),
        ) == {
            let (w, h) = NativeEditor::scaled_size(f64::from(factor)).unwrap();
            (Some(i32::from(w)), Some(i32::from(h)))
        } {
            // During resizeView the host can observe its onSize acknowledgment.
            // The native child waits for the frame to accept the resize.
            if pending.is_none() && !self.sync_native_scale(factor) {
                return kResultFalse;
            }
            self.size
                .set((size.right - size.left, size.bottom - size.top));
            kResultOk
        } else {
            kInvalidArgument
        }
    }
    unsafe fn onFocus(&self, state: TBool) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        if state != 0
            && self
                .child
                .borrow()
                .as_ref()
                .is_some_and(|child| child.focus().is_err())
        {
            return kResultFalse;
        }
        kResultOk
    }
    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        // SAFETY: the host supplies a live frame or null; retain its COM reference.
        let frame =
            unsafe { vst3::ComRef::from_raw(frame) }.map(|frame| Rc::new(frame.to_com_ptr()));
        let old = self.frame.replace(frame);
        drop(old);
        kResultOk
    }
    unsafe fn canResize(&self) -> tresult {
        kResultFalse
    }
    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        // SAFETY: the host supplies one exclusive writable SDK rectangle or null.
        let Some(rect) = (unsafe { rect.as_mut() }) else {
            return kInvalidArgument;
        };
        let factor = self.pending_scale.get().unwrap_or(self.content_scale.get());
        let (width, height) = NativeEditor::scaled_size(f64::from(factor)).unwrap();
        let (Some(right), Some(bottom)) = (
            rect.left.checked_add(i32::from(width)),
            rect.top.checked_add(i32::from(height)),
        ) else {
            return kInvalidArgument;
        };
        rect.right = right;
        rect.bottom = bottom;
        kResultOk
    }
}

impl IPlugViewContentScaleSupportTrait for View {
    unsafe fn setContentScaleFactor(&self, factor: f32) -> tresult {
        if !self.live() || self.pending_scale.get().is_some() {
            return kResultFalse;
        }
        let Some((width, height)) = NativeEditor::scaled_size(f64::from(factor)) else {
            return kInvalidArgument;
        };
        if self.content_scale.get() == factor {
            return kResultOk;
        }
        let requested = (i32::from(width), i32::from(height));
        // Retain the frame locally across reentrant host calls.
        let frame = self.frame.borrow().clone();
        if let Some(frame) = frame.as_ref() {
            let previous = self.size.get();
            let mut rect = ViewRect {
                left: 0,
                top: 0,
                right: requested.0,
                bottom: requested.1,
            };
            self.pending_scale.set(Some(factor));
            // SAFETY: this method is called through the live view interface;
            // the retained host frame may synchronously call onSize/getSize.
            let result = match unsafe { frame.resizeView(self.abi.get(), &mut rect) } {
                result if result == kResultOk && self.size.get() != requested => kResultFalse,
                result if result == kResultOk && !self.sync_native_scale(factor) => kResultFalse,
                result => result,
            };
            self.pending_scale.set(None);
            if result != kResultOk {
                self.size.set(previous);
                return result;
            }
        } else if self.child.borrow().is_none() {
            // Before attachment, the host obtains the requested size through
            // getSize. An attached view needs its frame to negotiate a resize.
            self.size.set(requested);
        } else {
            return kResultFalse;
        }
        self.content_scale.set(factor);
        kResultOk
    }
}
