use super::{message, runloop};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};
struct Probe {
    processor: Rc<ComPtr<IConnectionPoint>>,
    pub scope: [u8; 16],
    pub frames: Rc<RefCell<Vec<Value>>>,
    refuse_reply: Rc<Cell<bool>>,
}
impl Class for Probe {
    type Interfaces = (IConnectionPoint,);
}
impl IConnectionPointTrait for Probe {
    unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        let frame = unsafe { message::decode(message) };
        assert!(
            self.frames.borrow().len() < 4096,
            "bounded host observation budget"
        );
        self.frames.borrow_mut().push(frame.clone());
        if frame["body"]["type"] == "offer" {
            let response = message::packet(
                &json!({"version":1,"source":self.scope,"target":frame["source"],"sequence":0,"body":{"type":"challenge"}}),
            );
            return unsafe { self.processor.notify(response.as_ptr()) };
        }
        // This processor-only host has no edit controller or accepted host
        // Begins. The real-controller contract tests calibrate these terminal
        // certificates; it can close its empty gesture set immediately.
        let terminal = match frame["body"]["type"].as_str() {
            Some("goodbye" | "disconnect") => Some("closed"),
            Some("retired") => Some("retiredAck"),
            _ => None,
        };
        if let Some(kind) = terminal {
            let response = message::packet(&json!({"version":1,"source":self.scope,
                "target":frame["source"],"sequence":0,"body":{"type":kind}}));
            return unsafe { self.processor.notify(response.as_ptr()) };
        }
        if frame["body"]["type"] == "reply" && self.refuse_reply.get() {
            kResultFalse
        } else {
            kResultOk
        }
    }
}
pub struct Peer {
    pub interface: ComPtr<IConnectionPoint>,
    pub frames: Rc<RefCell<Vec<Value>>>,
    pub refuse: Rc<Cell<bool>>,
    pub scope: [u8; 16],
}
impl Peer {
    pub fn attach(processor: &ComPtr<IConnectionPoint>, scope: u8) -> Self {
        let frames = Rc::new(RefCell::new(Vec::new()));
        let refuse = Rc::new(Cell::new(false));
        let scope = [scope; 16];
        let interface = ComWrapper::new(Probe {
            processor: Rc::new(processor.clone()),
            scope,
            frames: frames.clone(),
            refuse_reply: refuse.clone(),
        })
        .to_com_ptr::<IConnectionPoint>()
        .unwrap();
        assert!(
            interface.cast::<IEditController>().is_none(),
            "the synthetic host never constructs a controller class"
        );
        unsafe {
            assert_eq!(processor.connect(interface.as_ptr()), kResultOk);
        }
        Self {
            interface,
            frames,
            refuse,
            scope,
        }
    }
    pub fn projection(&self) -> Value {
        self.frames
            .borrow()
            .iter()
            .rev()
            .find(|p| p["body"]["type"] == "projection")
            .unwrap()
            .clone()
    }
    pub fn bootstrap(&self) -> Value {
        serde_json::from_str(self.projection()["body"]["frame"].as_str().unwrap()).unwrap()
    }
    pub fn wire(&self, sequence: u64, id: u64, method: &str, args: Value) -> Value {
        json!({"version":1,"source":self.scope,"target":self.projection()["source"],"sequence":sequence,"body":{"type":"request","frame":json!({"version":1,"attachmentId":self.bootstrap()["attachmentId"],"requestId":id.to_string(),"method":method,"args":args}).to_string()}})
    }
    pub fn send(
        &self,
        p: &ComPtr<IConnectionPoint>,
        sequence: u64,
        method: &str,
        args: Value,
    ) -> Value {
        let packet = message::packet(&self.wire(sequence, sequence, method, args));
        unsafe {
            assert_eq!(p.notify(packet.as_ptr()), kResultOk);
        }
        self.reply(sequence)
    }
    pub fn reply(&self, sequence: u64) -> Value {
        let frames = self.frames.borrow();
        let packet = frames
            .iter()
            .rev()
            .find(|p| p["body"]["type"] == "reply" && p["sequence"] == sequence)
            .expect("correlated response");
        serde_json::from_str(packet["body"]["frame"].as_str().unwrap()).unwrap()
    }
    pub fn complete(
        &self,
        p: &ComPtr<IConnectionPoint>,
        clock: &runloop::LoopState,
        serial: &mut u64,
        job: &str,
    ) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        for _ in 0..500 {
            clock.tick();
            *serial += 1;
            let status = self.send(p, *serial, "getInstrumentSessionJobStatus", json!([job]));
            assert_eq!(status["status"], "accepted", "{status}");
            if status["result"]["state"] == "completed" {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "main-thread service failed to complete: {status}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("finite service attempt bound exhausted")
    }
}
