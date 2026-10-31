use crate::EditorAssets;
use gtk::prelude::*;
use std::{
    borrow::Cow,
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
use webkit2gtk::{UserContentManagerExt, WebViewExt};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix, WebViewExtUnix};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass},
    rust_connection::RustConnection,
};

const ENTRY_URI: &str = "dandrum://localhost/index.html";

#[derive(Debug)]
pub enum NativeError {
    WrongThread,
    UnsupportedDisplay,
    UnsupportedSize,
    InvalidParent,
    X11(String),
    WebView(wry::Error),
    IpcRegistration,
}

/// UI-thread-only child. The host retains ownership of its X11 parent and loop.
pub struct NativeEditor {
    // Rust drops fields in declaration order: browser before its GTK container.
    view: WebView,
    _window: OwnedWindow,
    ipc: Option<(webkit2gtk::UserContentManager, gtk::glib::SignalHandlerId)>,
    protocol: Option<OwnedProtocol>,
}

type SelectionCallback = Box<dyn FnOnce(Option<Vec<PathBuf>>)>;

/// A disposable main-thread dialog; dropping it revokes its callback first.
pub struct NativeFileSelection {
    callback: Rc<RefCell<Option<SelectionCallback>>>,
    dialog: gtk::FileChooserDialog,
    handler: Option<gtk::glib::SignalHandlerId>,
    // Dispose with the same callback-first, synchronized window owner as Wry.
    _window: OwnedWindow,
}
impl NativeFileSelection {
    pub fn is_pending(&self) -> bool {
        self.callback.borrow().is_some()
    }
}
impl Drop for NativeFileSelection {
    fn drop(&mut self) {
        self.callback.borrow_mut().take();
        if let Some(handler) = self.handler.take() {
            self.dialog.disconnect(handler);
        }
        self.dialog.hide();
    }
}

type ResourceHandler = dyn Fn(wry::WebViewId<'_>, wry::http::Request<Vec<u8>>) -> wry::http::Response<Cow<'static, [u8]>>
    + Send
    + Sync;

// WebKit may retain the registered wrapper after the child is destroyed. This
// guard releases its application handler on both owner drop and build failure.
struct OwnedProtocol(Arc<Mutex<Option<Box<ResourceHandler>>>>);
impl Drop for OwnedProtocol {
    fn drop(&mut self) {
        let handler = self.0.lock().unwrap().take();
        drop(handler);
    }
}

// The editor owns its foreign X11 child; a file chooser owns only its GTK window.
struct OwnedWindow(gtk::Window, Option<(RustConnection, u32)>);
impl Drop for OwnedWindow {
    fn drop(&mut self) {
        let display = self.0.display();
        // SAFETY: disposal runs on the GTK UI thread after IPC and the browser
        // are released. This owner exports no native handles, but GTK can enumerate
        // the container globally; retained references do not extend its live-widget
        // lifetime and must not query it after destruction. This follows GTK's
        // destruction lifecycle, not a guarantee that no aliases exist. Our local
        // wrappers only unref afterward; only the display captured above is used
        // after this call.
        #[expect(
            unsafe_code,
            reason = "GTK exposes synchronous disposal of the exclusively owned container as unsafe"
        )]
        unsafe {
            gtk::prelude::WidgetExtManual::destroy(&self.0);
        }
        // GTK must finish unmapping before the connection that created the
        // foreign child destroys it, or a late GDK UnmapWindow gets BadWindow.
        display.sync();
        if let Some((connection, child)) = self.1.take() {
            if let Ok(request) = connection.destroy_window(child) {
                let _ = request.check();
            }
            let _ = connection.flush();
        }
        // Order native destruction before the host queries or destroys its own
        // parent through another connection, without dispatching GTK work here.
        display.sync();
    }
}

impl NativeEditor {
    pub const WIDTH: u16 = 1200;
    pub const HEIGHT: u16 = 800;

    /// Open a local native chooser without entering a private or nested loop.
    /// This callback conveys paths only to the trusted native adapter, never IPC.
    pub fn choose_sample_files(
        &self,
        callback: impl FnOnce(Option<Vec<PathBuf>>) + 'static,
    ) -> Result<NativeFileSelection, NativeError> {
        Self::check_thread()?;
        let dialog = gtk::FileChooserDialog::with_buttons(
            Some("Choose Trigger samples"),
            Some(&self._window.0),
            gtk::FileChooserAction::Open,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Open", gtk::ResponseType::Accept),
            ],
        );
        dialog.set_modal(true);
        dialog.set_select_multiple(true);
        dialog.set_local_only(true);
        let callback: Rc<RefCell<Option<SelectionCallback>>> =
            Rc::new(RefCell::new(Some(Box::new(callback))));
        let response_callback = Rc::clone(&callback);
        let handler = dialog.connect_response(move |dialog, response| {
            let callback = response_callback.borrow_mut().take();
            if let Some(callback) = callback {
                let files = (response == gtk::ResponseType::Accept).then(|| dialog.filenames());
                callback(files);
            }
        });
        let selection = NativeFileSelection {
            callback,
            _window: OwnedWindow(dialog.clone().upcast(), None),
            dialog,
            handler: Some(handler),
        };
        selection.dialog.show_all();
        Ok(selection)
    }

    /// Create an initially hidden fixed-size child after the caller initializes
    /// GTK on its UI thread. The caller supplies subsequent host-driven ticks.
    pub fn create(
        parent: u32,
        assets: EditorAssets,
        on_message: impl Fn(String) + 'static,
    ) -> Result<Self, NativeError> {
        Self::create_initialized(parent, assets, "", on_message)
    }

    /// Install trusted host JavaScript before the packaged application loads.
    /// Attachment validation and bounded wire serialization belong to the caller.
    pub fn create_initialized(
        parent: u32,
        assets: EditorAssets,
        initialization_script: &str,
        on_message: impl Fn(String) + 'static,
    ) -> Result<Self, NativeError> {
        let connection = Self::parent_connection(parent)?;
        let builder = WebViewBuilder::new()
            .with_initialization_script(initialization_script)
            .with_navigation_handler(|uri| uri == ENTRY_URI);
        let mut owner = Self::build_child_with_owned_protocol(
            parent,
            &connection,
            builder,
            Box::new(move |_, request| {
                let (status, mime, bytes) = match assets.serve(&request.uri().to_string()) {
                    Ok(asset) => (200, asset.mime, asset.bytes.to_vec()),
                    Err(_) => (404, "text/plain", Vec::new()),
                };
                wry::http::Response::builder()
                    .status(status)
                    .header("Content-Type", mime)
                    .header("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; connect-src 'self'")
                    .body(Cow::Owned(bytes))
                    .unwrap()
            }),
        )?;
        let manager = owner
            .view
            .webview()
            .user_content_manager()
            .expect("Wry constructs every GTK WebView with a UserContentManager");
        // Wry's IPC signal captures its WebView strongly. Keep our signal ID and
        // capture only the callback so destruction can release it synchronously.
        let handler = manager.connect_script_message_received(None, move |_, message| {
            if let Some(value) = message.js_value() {
                on_message(value.to_string());
            }
        });
        if !manager.register_script_message_handler("ipc") {
            manager.disconnect(handler);
            return Err(NativeError::IpcRegistration);
        }
        owner.ipc = Some((manager, handler));
        owner.view.webview().load_uri(ENTRY_URI);
        Ok(owner)
    }

    /// Queue trusted host JavaScript on this child's UI thread. Success means
    /// queueing only; the protocol consumer must acknowledge actual delivery.
    pub fn evaluate_script(&self, script: &str) -> Result<(), NativeError> {
        Self::check_thread()?;
        self.view
            .evaluate_script(script)
            .map_err(NativeError::WebView)
    }

    /// At most 64 nonblocking iterations per external host tick; no private loop.
    pub fn service() -> Result<usize, NativeError> {
        Self::check_thread()?;
        let mut iterations = 0;
        while iterations < 64 && gtk::events_pending() {
            gtk::main_iteration_do(false);
            iterations += 1;
        }
        Ok(iterations)
    }

    /// Map or unmap the owned child, retaining its browser and document.
    pub fn set_visible(&self, visible: bool) -> Result<(), NativeError> {
        Self::check_thread()?;
        let display = self._window.0.display();
        if visible {
            self._window.0.show_all();
        } else {
            self._window.0.hide();
        }
        display.sync();
        Ok(())
    }

    /// Accept the declared fixed size without changing the native child.
    pub fn set_size(&self, width: u32, height: u32) -> Result<(), NativeError> {
        Self::check_thread()?;
        if width == u32::from(Self::WIDTH) && height == u32::from(Self::HEIGHT) {
            Ok(())
        } else {
            Err(NativeError::UnsupportedSize)
        }
    }

    fn check_thread() -> Result<(), NativeError> {
        if gtk::is_initialized_main_thread() {
            Ok(())
        } else {
            Err(NativeError::WrongThread)
        }
    }

    fn parent_connection(parent: u32) -> Result<RustConnection, NativeError> {
        Self::check_thread()?;
        let display = gtk::gdk::Display::default().ok_or(NativeError::UnsupportedDisplay)?;
        if !display.is::<gdkx11::X11Display>() {
            return Err(NativeError::UnsupportedDisplay);
        }
        let (connection, screen) = RustConnection::connect(Some(display.name().as_str()))
            .map_err(|e| NativeError::X11(e.to_string()))?;
        let attributes = connection
            .get_window_attributes(parent)
            .map_err(|_| NativeError::InvalidParent)?
            .reply()
            .map_err(|_| NativeError::InvalidParent)?;
        let geometry = connection
            .get_geometry(parent)
            .map_err(|_| NativeError::InvalidParent)?
            .reply()
            .map_err(|_| NativeError::InvalidParent)?;
        let root = &connection.setup().roots[screen];
        if attributes.class != WindowClass::INPUT_OUTPUT
            || geometry.root != root.root
            || geometry.depth != root.root_depth
        {
            return Err(NativeError::InvalidParent);
        }
        Ok(connection)
    }

    fn build_child_with_owned_protocol(
        parent: u32,
        connection: &RustConnection,
        builder: WebViewBuilder<'_>,
        handler: Box<ResourceHandler>,
    ) -> Result<Self, NativeError> {
        let protocol = OwnedProtocol(Arc::new(Mutex::new(Some(handler))));
        let shared = protocol.0.clone();
        let builder = builder.with_custom_protocol("dandrum".into(), move |id, request| {
            let handler = shared.lock().unwrap();
            match handler.as_ref() {
                Some(handler) => handler(id, request),
                None => wry::http::Response::builder()
                    .status(404)
                    .body(Cow::Borrowed(b"" as &[u8]))
                    .unwrap(),
            }
        });
        let mut owner = Self::build_child(parent, connection, builder)?;
        owner.protocol = Some(protocol);
        Ok(owner)
    }

    fn build_child(
        parent: u32,
        connection: &RustConnection,
        builder: WebViewBuilder<'_>,
    ) -> Result<Self, NativeError> {
        let display = gtk::gdk::Display::default().ok_or(NativeError::UnsupportedDisplay)?;
        let x11_display = display
            .clone()
            .downcast::<gdkx11::X11Display>()
            .map_err(|_| NativeError::UnsupportedDisplay)?;
        let (child_connection, _) = RustConnection::connect(Some(display.name().as_str()))
            .map_err(|error| NativeError::X11(error.to_string()))?;
        let child = child_connection
            .generate_id()
            .map_err(|error| NativeError::X11(error.to_string()))?;
        child_connection
            .create_window(
                COPY_DEPTH_FROM_PARENT,
                child,
                parent,
                0,
                0,
                Self::WIDTH,
                Self::HEIGHT,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new(),
            )
            .map_err(|error| NativeError::X11(error.to_string()))?
            .check()
            .map_err(|error| NativeError::X11(error.to_string()))?;
        // Reparenting a realized GTK toplevel leaves WebKit blank on real X11.
        // Register the already-parented child so GTK paints and receives events.
        let foreign = gdkx11::X11Window::foreign_new_for_display(&x11_display, child.into());
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.connect_realize(move |widget| {
            let original = widget.window();
            widget.set_window(foreign.clone().upcast());
            widget.register_window(foreign.upcast_ref::<gtk::gdk::Window>());
            // GTK creates a root toplevel during realize; replacing its GDK
            // pointer does not destroy that original XID.
            if let Some(original) = original {
                original.destroy();
            }
        });
        let window = OwnedWindow(window, Some((child_connection, child)));
        window.0.set_resizable(false);
        let fixed = gtk::Fixed::new();
        window.0.set_has_window(true);
        window.0.add(&fixed);
        window.0.realize();
        let scale = window.0.scale_factor().max(1);
        // Wry's bounds are physical; GTK's default size and WebKit's CSS
        // viewport are logical. Keep all three at the declared 1200x800 size.
        window.0.set_default_size(
            i32::from(Self::WIDTH) / scale,
            i32::from(Self::HEIGHT) / scale,
        );
        let view = builder
            .with_bounds(wry::Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(Self::WIDTH, Self::HEIGHT).into(),
            })
            .build_gtk(&fixed)
            .map_err(NativeError::WebView)?;
        view.webview().set_size_request(
            i32::from(Self::WIDTH) / scale,
            i32::from(Self::HEIGHT) / scale,
        );
        let dpi = gtk::prelude::WidgetExt::screen(&window.0)
            .map(|screen| screen.resolution())
            .filter(|dpi| dpi.is_finite() && *dpi > 0.0)
            .unwrap_or(96.0);
        // The same 192-DPI desktop reports either GTK scale 1 / 192 DPI or
        // scale 2 / 96 DPI. Both factors affect WebKit's CSS viewport.
        view.webview()
            .set_zoom_level(96.0 / (f64::from(scale) * dpi));
        let owner = Self {
            view,
            _window: window,
            ipc: None,
            protocol: None,
        };
        connection
            .flush()
            .map_err(|error| NativeError::X11(error.to_string()))?;
        Ok(owner)
    }
}

impl Drop for NativeEditor {
    fn drop(&mut self) {
        if let Some((manager, handler)) = self.ipc.take() {
            manager.unregister_script_message_handler("ipc");
            manager.disconnect(handler);
        }
        drop(self.protocol.take());
        // Fields then drop the WebView before OwnedWindow disposes the container.
    }
}

#[cfg(test)]
#[path = "../tests/internal/native_x11.rs"]
#[forbid(unsafe_code)]
mod tests;
