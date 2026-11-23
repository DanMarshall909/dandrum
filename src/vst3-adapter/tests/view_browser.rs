#![cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "trigger-editor"
))]
mod common;
use common::{audio::*, parameters::*, runloop, *};
use std::time::{Duration, Instant};
use vst3::Steinberg::{Vst::*, *};
use vst3::{Class, ComPtr, ComWrapper};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, ImageFormat, WindowClass},
};

struct RecordingProxy {
    target: ComPtr<IConnectionPoint>,
    frames: std::rc::Rc<std::cell::RefCell<Vec<serde_json::Value>>>,
    defer_projection: std::rc::Rc<std::cell::Cell<bool>>,
    held: std::rc::Rc<std::cell::RefCell<Vec<ComPtr<IMessage>>>>,
}
impl Class for RecordingProxy {
    type Interfaces = (IConnectionPoint,);
}
impl IConnectionPointTrait for RecordingProxy {
    unsafe fn connect(&self, _: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn disconnect(&self, _: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        let packet = unsafe { common::message::decode(message) };
        if packet["body"]["type"] != "confirm" {
            let mut frames = self.frames.borrow_mut();
            if !frames.contains(&packet) {
                assert!(frames.len() < 128, "bounded distinct diagnostic frames");
                frames.push(packet.clone());
            }
        }
        if packet.get("view").is_some()
            && packet["body"]["type"] == "projection"
            && self.defer_projection.get()
        {
            if self.held.borrow().is_empty() {
                let message = unsafe { vst3::ComRef::from_raw(message) }
                    .unwrap()
                    .to_com_ptr();
                self.held.borrow_mut().push(message);
            }
            return kResultOk;
        }
        unsafe { self.target.notify(message) }
    }
}

#[test]
#[ignore = "real X11/browser protocol boundary lane; explicit display, separate process"]
fn browser_projection_reads_the_same_processor_after_native_recreation() {
    let bundle = Bundle::new();
    common::native::paint_package(&bundle);
    std::fs::write(
        bundle.resources.join("editor/app.css"),
        b"html,body{margin:0;width:100%;height:100%;background:#ff0000}",
    )
    .unwrap();
    std::fs::write(bundle.resources.join("editor/app.js"), br#"
        const ns=window.__DANDRUM_NATIVE__;
        window.__DANDRUM__={receiveNativeMessage(text){
            const reply=JSON.parse(text);
            if(reply.attachmentId!==ns.attachmentId || reply.instanceId!==ns.instanceId) return;
            const pan=reply.result?.parameters?.find(p=>p.id==='drums.pan');
            if(pan?.value===0.625) document.body.style.background='#123456';
        }};
        if(ns) window.ipc.postMessage(JSON.stringify({version:1,attachmentId:ns.attachmentId,requestId:ns.nextRequestId,method:'getParameterState',args:[]}));
    "#).unwrap();
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
    }
    let audio = processor.cast::<IAudioProcessor>().unwrap();
    assert_eq!(
        unsafe { controller.normalizedParamToPlain(0x1004, 0.625) },
        0.25,
        "normalized browser pan and physical DSP pan have distinct units"
    );
    unsafe {
        assert_eq!(audio.setupProcessing(&mut setup(48000.0, 16)), kResultOk);
        assert_eq!(processor.setActive(1), kResultOk);
    }
    let controls = changes(vec![queue(0x1004, vec![(0, 0.625)])]);
    let mut zero = Block::new(0);
    let mut flush = zero.data();
    flush.inputParameterChanges = controls.as_ptr();
    assert_eq!(unsafe { audio.process(&mut flush) }, kResultOk);
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let c = controller.cast::<IConnectionPoint>().unwrap();
    let frames = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let defer_projection = std::rc::Rc::new(std::cell::Cell::new(false));
    let held = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let to_controller = ComWrapper::new(RecordingProxy {
        target: c.clone(),
        frames: frames.clone(),
        defer_projection: defer_projection.clone(),
        held: held.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    let to_processor = ComWrapper::new(RecordingProxy {
        target: p.clone(),
        frames: frames.clone(),
        defer_projection: defer_projection.clone(),
        held: held.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    unsafe {
        assert_eq!(p.connect(to_controller.as_ptr()), kResultOk);
        assert_eq!(c.connect(to_processor.as_ptr()), kResultOk);
    }
    let (x, screen) = x11rb::connect(None).unwrap();
    let root = &x.setup().roots[screen];
    assert_eq!(root.root_depth, 24);
    let parent = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        parent,
        root.root,
        0,
        0,
        1200,
        800,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new().background_pixel(0xffffff),
    )
    .unwrap()
    .check()
    .unwrap();
    x.map_window(parent).unwrap().check().unwrap();
    for cycle in 0..2 {
        defer_projection.set(cycle == 0);
        let view =
            unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
        assert_eq!(
            unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultOk
        );
        if cycle == 0 {
            assert_eq!(
                x.query_tree(parent)
                    .unwrap()
                    .reply()
                    .unwrap()
                    .children
                    .len(),
                1,
                "native parenting completes before the asynchronous namespace reply"
            );
            clock.tick();
            assert!(
                !frames
                    .borrow()
                    .iter()
                    .any(|packet| packet.get("view").is_some()
                        && packet["body"]["type"] == "request"),
                "application requests wait for the authenticated browser namespace"
            );
            defer_projection.set(false);
            let delayed = std::mem::take(&mut *held.borrow_mut());
            assert_eq!(delayed.len(), 1, "one retained namespace reply");
            for message in delayed {
                assert_eq!(unsafe { c.notify(message.as_ptr()) }, kResultOk);
            }
        }
        let deadline = Instant::now() + Duration::from_secs(12);
        let mut ready = false;
        while Instant::now() < deadline {
            clock.tick();
            let pixel = x
                .get_image(ImageFormat::Z_PIXMAP, parent, 12, 12, 1, 1, u32::MAX)
                .unwrap()
                .reply()
                .unwrap()
                .data;
            ready = pixel.get(..3) == Some(&[0x56, 0x34, 0x12]);
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            ready,
            "cycle{cycle}: authenticated browser must read normalized pan0.625 from the one processor; public wire: {:?}",
            frames.borrow()
        );
        assert_eq!(unsafe { view.removed() }, kResultOk);
        assert!(
            x.query_tree(parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
                .is_empty()
        );
        clock.tick();
    }
    unsafe {
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
    }
    assert!(clock.handlers.borrow().is_empty());
    x.destroy_window(parent).unwrap().check().unwrap();
}
