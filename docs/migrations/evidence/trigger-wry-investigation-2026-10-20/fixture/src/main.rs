use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle, XlibWindowHandle};
use std::{borrow::Cow, cell::RefCell, ptr, rc::Rc, time::{Duration, Instant}};
use wry::{Rect, WebViewBuilder, dpi::{PhysicalPosition, PhysicalSize}};
use x11_dl::xlib;

struct Parent(u64);
impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Xlib(XlibWindowHandle::new(self.0))) })
    }
}
fn pump() {
    // One bounded host tick; never enter gtk::main or wait on a nested GTK loop.
    for _ in 0..64 { if !gtk::events_pending() { break } gtk::main_iteration_do(false); }
}
unsafe fn children(x: &xlib::Xlib, display: *mut xlib::Display, parent: u64) -> Vec<u64> {
    let (mut root, mut ancestor, mut ids, mut count) = (0, 0, ptr::null_mut(), 0);
    assert_ne!(unsafe { (x.XQueryTree)(display, parent, &mut root, &mut ancestor, &mut ids, &mut count) }, 0);
    let result = if count == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(ids, count as usize) }.to_vec() };
    if !ids.is_null() { unsafe { (x.XFree)(ids.cast()) }; }
    result
}
fn main() {
    gtk::init().expect("GTK initialization");
    let x = xlib::Xlib::open().unwrap();
    unsafe {
        let display = (x.XOpenDisplay)(ptr::null());
        assert!(!display.is_null());
        let root = (x.XDefaultRootWindow)(display);
        let parent = Parent((x.XCreateSimpleWindow)(display, root, 0, 0, 640, 480, 0, 0, 0));
        (x.XMapWindow)(display, parent.0); (x.XSync)(display, 0);
        assert!(children(&x, display, parent.0).is_empty());
        for iteration in 0..2 {
            let messages = Rc::new(RefCell::new(Vec::new()));
            let sink = messages.clone();
            let view = WebViewBuilder::new()
                .with_bounds(Rect { position: PhysicalPosition::new(0,0).into(), size: PhysicalSize::new(640,480).into() })
                .with_custom_protocol("dandrum".into(), |_, request| {
                    assert_eq!(request.uri().path(), "/index.html");
                    wry::http::Response::builder().header("Content-Type", "text/html").body(Cow::Borrowed(b"<!doctype html><title>local-probe</title><script>window.ipc.postMessage('local-ready')</script>" as &[u8])).unwrap()
                })
                .with_ipc_handler(move |request| sink.borrow_mut().push(request.body().clone()))
                .with_url("dandrum://localhost/index.html")
                .build_as_child(&parent).expect("child creation");
            let deadline = Instant::now() + Duration::from_secs(15);
            while messages.borrow().is_empty() && Instant::now() < deadline { pump(); std::thread::sleep(Duration::from_millis(5)); }
            assert_eq!(&*messages.borrow(), &["local-ready"]);
            pump();
            let child = children(&x, display, parent.0);
            assert_eq!(child.len(), 1, "one direct X11 child");
            view.set_visible(false).unwrap(); pump();
            let mut attributes: xlib::XWindowAttributes = std::mem::zeroed();
            assert_ne!((x.XGetWindowAttributes)(display, child[0], &mut attributes), 0);
            assert_eq!(attributes.map_state, xlib::IsUnmapped);
            view.set_visible(true).unwrap(); pump();
            assert_ne!((x.XGetWindowAttributes)(display, child[0], &mut attributes), 0);
            assert_eq!(attributes.map_state, xlib::IsViewable);
            drop(view); pump();
            assert!(children(&x, display, parent.0).is_empty(), "view drop releases child while host parent survives");
            println!("cycle {iteration}: local IPC, parent, hide/show, destruction passed");
        }
        (x.XDestroyWindow)(display, parent.0); (x.XCloseDisplay)(display);
    }
}
