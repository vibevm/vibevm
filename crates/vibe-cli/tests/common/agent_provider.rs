//! A loopback OpenAI-compatible endpoint that counts requests.
//!
//! Shared by the create-phase e2e binaries so both drive the **production**
//! `vibe-llm` transport rather than a test-only hook: keyless HTTP is legal on
//! literal loopback, so the shipped configuration, endpoint policy and
//! provider mapping are all exercised. The hit counter is what makes "refused
//! before spend" a measurement instead of a claim.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct MockProvider {
    address: SocketAddr,
    hits: Arc<AtomicUsize>,
    /// Exact request bodies, so a test can prove WHICH prompt bytes were sent
    /// — the only way to tell one provider instance's document from another's.
    bodies: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    /// Set when a held request's connection was closed by the client.
    closed_by_client: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// How long a holding provider keeps a request open before giving up on the
/// client closing it. Far beyond any stop latency a test accepts.
const HOLD_LIMIT: Duration = Duration::from_secs(20);

impl MockProvider {
    pub fn serving(assistant_content: &str) -> Self {
        Self::start(assistant_content, usize::MAX)
    }

    /// A provider that reads each request and then never answers, holding
    /// the connection until the client closes it — a paid request in flight.
    pub fn holding() -> Self {
        Self::start("", 0)
    }

    /// A provider that answers the first `answered` requests with
    /// `assistant_content` and holds every later one in flight: an earlier
    /// agent step completes, a later one is still paying when a stop lands.
    pub fn answering_then_holding(assistant_content: &str, answered: usize) -> Self {
        Self::start(assistant_content, answered)
    }

    /// Whether a held request's connection was closed by the client.
    pub fn closed_by_client(&self) -> bool {
        self.closed_by_client.load(Ordering::SeqCst)
    }

    fn start(assistant_content: &str, answered: usize) -> Self {
        let body = serde_json::to_vec(&serde_json::json!({
            "id": "chatcmpl-create-1",
            "model": "demo-chat-model",
            "choices": [{ "message": { "role": "assistant", "content": assistant_content } }],
            "usage": { "prompt_tokens": 42, "completion_tokens": 9, "total_tokens": 51 },
        }))
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let bodies: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let closed_by_client = Arc::new(AtomicBool::new(false));
        let thread = {
            let hits = Arc::clone(&hits);
            let bodies = Arc::clone(&bodies);
            let stop = Arc::clone(&stop);
            let closed = Arc::clone(&closed_by_client);
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let request = drain_request(&mut stream);
                            let seen = hits.fetch_add(1, Ordering::SeqCst);
                            bodies.lock().unwrap().push(request);
                            if seen >= answered {
                                if held_until_closed(&mut stream, &stop) {
                                    closed.store(true, Ordering::SeqCst);
                                }
                                continue;
                            }
                            let _ = write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                                 Content-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(&body);
                            let _ = stream.flush();
                            // Close the write half first and let the client
                            // finish reading: dropping a socket with unread
                            // bytes still queued makes Windows send RST and
                            // discard the response the client is waiting for.
                            let _ = stream.shutdown(std::net::Shutdown::Write);
                            let _ = stream.read(&mut [0_u8; 64]);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        // A transient accept failure must not silently retire
                        // the endpoint: that would surface as an unrelated
                        // "provider unreachable" in whichever case ran next.
                        Err(_) => std::thread::sleep(Duration::from_millis(5)),
                    }
                }
            })
        };
        Self {
            address,
            hits,
            bodies,
            stop,
            closed_by_client,
            thread: Some(thread),
        }
    }

    /// The exact request bodies received, oldest first.
    pub fn bodies(&self) -> Vec<String> {
        self.bodies.lock().unwrap().clone()
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}/v1/chat/completions", self.address)
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

impl Drop for MockProvider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Hold an answered-never connection; `true` when the client closed it.
fn held_until_closed(stream: &mut TcpStream, stop: &AtomicBool) -> bool {
    let deadline = Instant::now() + HOLD_LIMIT;
    let mut sink = [0_u8; 256];
    while Instant::now() < deadline && !stop.load(Ordering::SeqCst) {
        match stream.read(&mut sink) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return true,
        }
    }
    false
}

/// Read the complete request before answering. The accepted socket inherits
/// the listener's non-blocking mode, so it is put back into blocking mode
/// first: a `WouldBlock` treated as end-of-request would answer before reading,
/// and the resulting reset would look exactly like an unreachable provider.
fn drain_request(stream: &mut TcpStream) -> String {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() >= deadline {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        let read = match stream.read(&mut chunk) {
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(_) => return String::from_utf8_lossy(&bytes).into_owned(),
        };
        if read == 0 {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        bytes.extend_from_slice(&chunk[..read]);
        let Some(head) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&bytes[..head]).to_ascii_lowercase();
        let declared = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if bytes.len() >= head + 4 + declared {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Append a user-level `[llm]` section pointing at a loopback endpoint.
pub fn configure_provider(user: &vibe_test_support::UserScratch, endpoint: &str) {
    let path = user.settings.join("config.toml");
    let mut body = std::fs::read_to_string(&path).unwrap_or_default();
    body.push_str(&format!(
        "\n[llm]\nprovider = \"openai-compatible\"\nmodel = \"demo-chat-model\"\n\
         endpoint = \"{endpoint}\"\n",
    ));
    std::fs::write(path, body).unwrap();
}
