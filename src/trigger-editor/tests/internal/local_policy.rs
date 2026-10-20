use super::{EditorAssets, NativeEditor, common};
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub(super) fn external_fetch_is_blocked_by_enforcing_csp(parent: u32) {
    let endpoint = Endpoint::start();
    endpoint.calibrate();
    let package = common::Package::new();
    for path in common::FILES {
        package.write(path, b"fixture");
    }
    package.write("index.html", br#"<!doctype html><link rel="stylesheet" href="/app.css"><div id="control" style="width:37px">local policy</div><script type="module" src="/app.js"></script>"#);
    package.write("app.css", b"#control { height:43px; }");
    package.write(
        "app.js",
        include_str!("../fixtures/local-policy.mjs")
            .replace("__LOCAL_POLICY_ENDPOINT__", &endpoint.origin())
            .as_bytes(),
    );
    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let owner = NativeEditor::create(
        parent,
        EditorAssets::open(package.root()).unwrap(),
        move |message| sink.borrow_mut().push(message),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !messages.borrow().iter().any(|message| message == "done") && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let observed = messages.borrow();
    let requests = endpoint.requests.lock().unwrap();
    eprintln!("local-policy browser receipt: {observed:?}; owned endpoint requests: {requests:?}");
    assert!(
        observed.iter().any(|message| message == "done"),
        "normal packaged page completes its policy exercise"
    );
    assert!(
        observed
            .iter()
            .any(|message| message == "local|404|43px|37px"),
        "packaged module, stylesheet, inline styling and local fetch remain usable"
    );
    let expected_violation = format!(
        "policy|connect-src|enforce|{}|dandrum://localhost/index.html",
        endpoint.origin()
    );
    assert!(
        observed
            .iter()
            .any(|message| message == &expected_violation),
        "an enforcing connect-src violation identifies the blocked endpoint origin and local document"
    );
    assert!(
        observed
            .iter()
            .any(|message| message == "external|rejected"),
        "the page fetch itself is rejected"
    );
    assert_eq!(
        &*requests,
        &["GET /calibration HTTP/1.1"],
        "CSP blocks the request before it reaches the independently calibrated endpoint"
    );
    drop(owner);
}

/// An owned loopback-only endpoint, independent of WebKit's load/error signals.
/// Its nonblocking accept loop and finite read timeout also bound failure cleanup.
struct Endpoint {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Endpoint {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let gate = stop.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = requests.clone();
        let worker = std::thread::spawn(move || {
            while !gate.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        stream
                            .set_write_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        let mut reader = BufReader::new((&stream).take(8192));
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() && !line.is_empty() {
                            let first_line = line.trim_end().to_owned();
                            let mut complete = false;
                            loop {
                                line.clear();
                                match reader.read_line(&mut line) {
                                    Ok(0) | Err(_) => break,
                                    Ok(_) if line == "\r\n" => {
                                        complete = true;
                                        break;
                                    }
                                    Ok(_) => {}
                                }
                            }
                            if !complete {
                                continue;
                            }
                            received.lock().unwrap().push(first_line);
                            let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 21\r\nConnection: close\r\n\r\nlocal-policy-sentinel";
                            stream.write_all(response).unwrap();
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("owned endpoint accept failed: {error}"),
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    fn calibrate(&self) {
        let mut client = TcpStream::connect_timeout(&self.address, Duration::from_secs(1)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        client
            .write_all(b"GET /calibration HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with("\r\n\r\nlocal-policy-sentinel"));
        assert_eq!(
            &*self.requests.lock().unwrap(),
            &["GET /calibration HTTP/1.1"]
        );
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}
