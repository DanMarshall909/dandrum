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
    faults: std::rc::Rc<Faults>,
}
#[derive(Default)]
struct Faults {
    defer_projection: std::cell::Cell<bool>,
    held: std::cell::RefCell<Vec<ComPtr<IMessage>>>,
    refuse_retired_ack: std::cell::Cell<bool>,
    drop_retired_ack: std::cell::Cell<bool>,
    drop_automation_ack: std::cell::Cell<bool>,
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
            && self.faults.defer_projection.get()
        {
            if self.faults.held.borrow().is_empty() {
                let message = unsafe { vst3::ComRef::from_raw(message) }
                    .unwrap()
                    .to_com_ptr();
                self.faults.held.borrow_mut().push(message);
            }
            return kResultOk;
        }
        if packet.get("view").is_some() && packet["body"]["type"] == "retiredAck" {
            if self.faults.refuse_retired_ack.get() {
                return kResultFalse;
            }
            if self.faults.drop_retired_ack.get() {
                return kResultOk;
            }
        }
        if packet.get("view").is_some()
            && packet["body"]["type"] == "automationAck"
            && self.faults.drop_automation_ack.get()
        {
            return kResultOk;
        }
        unsafe { self.target.notify(message) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum HostEdit {
    Begin(u32),
    Value(u32, f64),
    End(u32),
}
#[derive(Default)]
struct HandlerState {
    accepted: std::cell::RefCell<Vec<HostEdit>>,
    attempts: std::cell::RefCell<Vec<HostEdit>>,
    refuse_once: std::cell::RefCell<Option<HostEdit>>,
    on_call: std::cell::RefCell<Option<Box<dyn FnOnce()>>>,
}
struct Handler(std::rc::Rc<HandlerState>);
impl Handler {
    fn record(&self, event: HostEdit) -> tresult {
        self.0.attempts.borrow_mut().push(event);
        assert!(
            self.0.attempts.borrow().len() <= 32,
            "bounded host delivery attempts"
        );
        if self.0.refuse_once.borrow().as_ref() == Some(&event) {
            self.0.refuse_once.borrow_mut().take();
            return kResultFalse;
        }
        self.0.accepted.borrow_mut().push(event);
        let callback = self.0.on_call.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        kResultOk
    }
}
impl Class for Handler {
    type Interfaces = (IComponentHandler,);
}
impl IComponentHandlerTrait for Handler {
    unsafe fn beginEdit(&self, id: u32) -> tresult {
        self.record(HostEdit::Begin(id))
    }
    unsafe fn performEdit(&self, id: u32, value: f64) -> tresult {
        self.record(HostEdit::Value(id, value))
    }
    unsafe fn endEdit(&self, id: u32) -> tresult {
        self.record(HostEdit::End(id))
    }
    unsafe fn restartComponent(&self, _: i32) -> tresult {
        kResultOk
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
    let browser_script = br#"
        const ns=window.__DANDRUM_NATIVE__;
        const hold=false;
        function send(id,method,args) {window.ipc.postMessage(JSON.stringify({version:1,attachmentId:ns.attachmentId,requestId:String(id),method,args}));}
        window.__DANDRUM__={receiveNativeMessage(text){
            const reply=JSON.parse(text);
            if(reply.attachmentId!==ns.attachmentId || reply.instanceId!==ns.instanceId) return;
            const pan=reply.result?.parameters?.find(p=>p.id==='drums.pan');
            if(pan?.value===0.625) send(2,'beginGesture',['drums.level',1]);
            if(reply.status==='accepted' && reply.requestId==='2') send(3,'setParameter',['drums.level',0.625,1]);
            if(reply.status==='accepted' && reply.requestId==='3') {if(hold) document.body.style.background='#123456';else send(4,'endGesture',['drums.level',1]);}
            if(reply.status==='accepted' && reply.requestId==='4') document.body.style.background='#123456';
        }};
        if(ns) window.ipc.postMessage(JSON.stringify({version:1,attachmentId:ns.attachmentId,requestId:ns.nextRequestId,method:'getParameterState',args:[]}));
    "#;
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
    }
    let edits = std::rc::Rc::new(HandlerState::default());
    let handler = ComWrapper::new(Handler(edits.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { controller.setComponentHandler(handler.as_ptr()) },
        kResultOk
    );
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
    let faults = std::rc::Rc::new(Faults::default());
    let to_controller = ComWrapper::new(RecordingProxy {
        target: c.clone(),
        frames: frames.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    let to_processor = ComWrapper::new(RecordingProxy {
        target: p.clone(),
        frames: frames.clone(),
        faults: faults.clone(),
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
    for cycle in 0..4 {
        let script = String::from_utf8_lossy(browser_script).replace(
            "const hold=false",
            if cycle == 3 {
                "const hold=true"
            } else {
                "const hold=false"
            },
        );
        std::fs::write(bundle.resources.join("editor/app.js"), script).unwrap();
        faults.drop_automation_ack.set(cycle == 0);
        if cycle == 1 {
            *edits.refuse_once.borrow_mut() = Some(HostEdit::Value(0x1003, 0.625));
        }
        faults.defer_projection.set(cycle == 0);
        let view =
            unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
        assert_eq!(
            unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultOk
        );
        let retained_view = view.clone();
        let retained_controller = (*controller).clone();
        let retained_processor = (*processor).clone();
        *edits.on_call.borrow_mut() = Some(Box::new(move || {
            assert_eq!(
                unsafe { retained_view.removed() },
                kResultFalse,
                "view teardown waits for host callback"
            );
            assert_eq!(unsafe { retained_controller.terminate() }, kResultFalse);
            assert_eq!(unsafe { retained_processor.terminate() }, kResultFalse);
        }));
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
            faults.defer_projection.set(false);
            let delayed = std::mem::take(&mut *faults.held.borrow_mut());
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
        for _ in 0..4 {
            clock.tick();
        }
        if cycle == 0 {
            assert_eq!(
                *edits.accepted.borrow(),
                [HostEdit::Begin(0x1003)],
                "lost application acknowledgements cannot duplicate a host begin"
            );
            faults.drop_automation_ack.set(false);
            for _ in 0..4 {
                clock.tick();
            }
        }
        let mut expected = [
            HostEdit::Begin(0x1003),
            HostEdit::Value(0x1003, 0.625),
            HostEdit::End(0x1003),
        ]
        .repeat(cycle + 1);
        if cycle == 3 {
            expected.pop();
        }
        assert_eq!(
            *edits.accepted.borrow(),
            expected,
            "real native browser gestures reach the host exactly once in each view"
        );
        if cycle == 3 {
            assert_eq!(
                unsafe { controller.setComponentHandler(std::ptr::null_mut()) },
                kResultFalse,
                "a held native gesture retains its accepting handler"
            );
            *edits.refuse_once.borrow_mut() = Some(HostEdit::End(0x1003));
        }
        faults.refuse_retired_ack.set(cycle == 0);
        faults.drop_retired_ack.set(cycle == 1);
        assert_eq!(
            unsafe { view.removed() },
            if cycle == 0 || cycle == 3 {
                kResultFalse
            } else {
                kResultOk
            },
            "refused teardown delivery must retain a retry token"
        );
        assert!(
            x.query_tree(parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
                .is_empty()
        );
        if cycle == 0 || cycle == 3 {
            faults.refuse_retired_ack.set(false);
            clock.tick();
            clock.tick();
            assert_eq!(
                clock.handlers.borrow().len(),
                1,
                "editor-absent host service settles refused teardown and removes its timer"
            );
            drop(view);
        } else {
            drop(view);
            faults.drop_retired_ack.set(false);
            clock.tick();
            clock.tick();
        }
        assert_eq!(
            clock.handlers.borrow().len(),
            1,
            "only persistent processor service remains between views"
        );
        assert_eq!(
            *edits.accepted.borrow(),
            [
                HostEdit::Begin(0x1003),
                HostEdit::Value(0x1003, 0.625),
                HostEdit::End(0x1003)
            ]
            .repeat(cycle + 1),
            "view removal balances held native gestures even after a refused end"
        );
    }
    unsafe {
        assert_eq!(audio.setProcessing(1), kResultOk);
        let notes = events(vec![note_on(5, 36, 1.0)]);
        let mut rendered = Block::new(16);
        let mut data = rendered.data();
        data.inputEvents = notes.as_ptr();
        assert_eq!(audio.process(&mut data), kResultOk);
        assert_eq!(&rendered.left[..5], &[0.0; 5]);
        assert_eq!(&rendered.right[..5], &[0.0; 5]);
        // Signed source -0.5, physical level2.5 and pan0.25, after every view is gone.
        assert_eq!((rendered.left[5], rendered.right[5]), (-0.9375, -1.25));
        assert_eq!(audio.setProcessing(0), kResultOk);
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
    }
    assert!(clock.handlers.borrow().is_empty());
    x.destroy_window(parent).unwrap().check().unwrap();
}
