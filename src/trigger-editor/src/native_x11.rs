use crate::EditorAssets;
use gtk::prelude::*;
use std::borrow::Cow;
use webkit2gtk::{UserContentManagerExt, WebViewExt};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix, WebViewExtUnix};
use x11rb::{
    connection::Connection,
    protocol::xproto::{ConnectionExt, WindowClass},
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
}

struct OwnedWindow(gtk::Window);
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
        // Order native destruction before the host queries or destroys its own
        // parent through another connection, without dispatching GTK work here.
        display.sync();
    }
}

impl NativeEditor {
    pub const WIDTH: u16 = 1200;
    pub const HEIGHT: u16 = 800;

    /// Create an initially hidden fixed-size child after the caller initializes
    /// GTK on its UI thread. The caller supplies subsequent host-driven ticks.
    pub fn create(
        parent: u32,
        assets: EditorAssets,
        on_message: impl Fn(String) + 'static,
    ) -> Result<Self, NativeError> {
        let connection = Self::parent_connection(parent)?;
        let builder =
            WebViewBuilder::new().with_custom_protocol("dandrum".into(), move |_, request| {
                let (status, mime, bytes) = match assets.serve(&request.uri().to_string()) {
                    Ok(asset) => (200, asset.mime, asset.bytes.to_vec()),
                    Err(_) => (404, "text/plain", Vec::new()),
                };
                wry::http::Response::builder()
                    .status(status)
                    .header("Content-Type", mime)
                    .header("Content-Security-Policy", "connect-src 'self'")
                    .body(Cow::Owned(bytes))
                    .unwrap()
            });
        let builder = builder.with_navigation_handler(|uri| uri == ENTRY_URI);
        let mut owner = Self::build_child(parent, &connection, builder)?;
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

    fn build_child(
        parent: u32,
        connection: &RustConnection,
        builder: WebViewBuilder<'_>,
    ) -> Result<Self, NativeError> {
        let window = OwnedWindow(gtk::Window::new(gtk::WindowType::Popup));
        window
            .0
            .set_default_size(i32::from(Self::WIDTH), i32::from(Self::HEIGHT));
        window.0.set_resizable(false);
        let fixed = gtk::Fixed::new();
        window.0.add(&fixed);
        window.0.realize();
        let native = window
            .0
            .window()
            .and_then(|window| window.downcast::<gdkx11::X11Window>().ok())
            .ok_or(NativeError::UnsupportedDisplay)?;
        native.upcast_ref::<gtk::gdk::Window>().display().sync();
        let view = builder
            .with_bounds(wry::Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(Self::WIDTH, Self::HEIGHT).into(),
            })
            .build_gtk(&fixed)
            .map_err(NativeError::WebView)?;
        let owner = Self {
            view,
            _window: window,
            ipc: None,
        };
        connection
            .reparent_window(native.xid() as u32, parent, 0, 0)
            .map_err(|error| NativeError::X11(error.to_string()))?
            .check()
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
        // Fields then drop the WebView before OwnedWindow disposes the container.
    }
}

#[cfg(test)]
#[path = "../tests/internal/native_x11.rs"]
#[forbid(unsafe_code)]
mod tests;
