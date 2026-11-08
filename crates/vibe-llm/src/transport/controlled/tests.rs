use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use vibe_core::cancellation::CancellationToken;

use crate::{ApiKey, ChatTransport, Endpoint, ReqwestChatTransport, TransportError};

/// What the loopback provider does after it has read one complete request.
#[derive(Clone, Copy)]
enum Answer {
    /// Never answer; hold the connection until the client closes it.
    HoldHeaders,
    /// Send headers and part of a declared body, then hold.
    HoldBody,
    /// Answer `200` with this many body bytes.
    Body(usize),
}

struct Fixture {
    endpoint: Endpoint,
    /// One message per complete request the provider read.
    requests: Mutex<mpsc::Receiver<()>>,
    /// Sent when the provider observed the client close a held connection.
    closed: mpsc::Receiver<()>,
    listener: TcpListener,
}

impl Fixture {
    fn serve(answer: Answer) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accept = listener.try_clone().unwrap();
        let (request_tx, requests) = mpsc::channel();
        let (closed_tx, closed) = mpsc::channel();
        thread::spawn(move || {
            let Ok((mut stream, _)) = accept.accept() else {
                return;
            };
            read_request(&mut stream);
            let _ = request_tx.send(());
            match answer {
                Answer::HoldHeaders => {}
                Answer::HoldBody => {
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                          Content-Length: 4096\r\n\r\n{\"partial\":",
                    );
                    let _ = stream.flush();
                }
                Answer::Body(length) => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                         Content-Length: {length}\r\nConnection: close\r\n\r\n"
                    );
                    let _ = stream.write_all(&vec![b'x'; length]);
                    let _ = stream.flush();
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                }
            }
            // A held connection ends only when the client closes it.
            let mut sink = [0_u8; 256];
            while matches!(stream.read(&mut sink), Ok(read) if read > 0) {}
            let _ = closed_tx.send(());
        });
        let endpoint =
            Endpoint::parse(&format!("http://{address}/v1/chat/completions"), false).unwrap();
        Self {
            endpoint,
            requests: Mutex::new(requests),
            closed,
            listener,
        }
    }

    /// No second connection ever reached the provider: nothing was retried.
    fn assert_no_further_connection(&self) {
        self.listener.set_nonblocking(true).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert!(
            self.listener.accept().is_err(),
            "the paid request was retried"
        );
    }
}

fn read_request(stream: &mut TcpStream) {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            return;
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
            return;
        }
    }
}

/// Request a stop as soon as the provider has the request, and measure how
/// long the transport takes to return after it.
fn stop_in_flight(fixture: &Fixture) -> (Result<(), TransportError>, Duration, CancellationToken) {
    let transport = ReqwestChatTransport::new().unwrap();
    let token = CancellationToken::new();
    let stopper = token.clone();
    let (started_tx, started) = mpsc::channel();
    let requests = &fixture.requests;
    let stop = thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            requests
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .unwrap();
            // Let the client settle into waiting on the held exchange.
            thread::sleep(Duration::from_millis(100));
            let stopped_at = Instant::now();
            stopper.cancel();
            started_tx.send(stopped_at).unwrap();
        });
        let result = transport
            .post_json_controlled(&fixture.endpoint, None, b"{\"canary\":1}", &token)
            .map(|_| ());
        let returned = Instant::now();
        waiter.join().unwrap();
        let stopped_at = started.recv().unwrap();
        (result, returned.duration_since(stopped_at))
    });
    (stop.0, stop.1, token)
}

#[test]
fn a_stop_closes_a_request_whose_headers_never_arrive() {
    let fixture = Fixture::serve(Answer::HoldHeaders);
    let (result, latency, token) = stop_in_flight(&fixture);
    assert_eq!(result.unwrap_err(), TransportError::Cancelled);
    assert!(latency < Duration::from_secs(2), "{latency:?}");
    assert!(token.acknowledged());
    assert_eq!(token.llm_effect(), "unknown");
    // The provider saw the connection closed by the client: no request is
    // left running behind the returned call.
    fixture
        .closed
        .recv_timeout(Duration::from_secs(2))
        .expect("the in-flight connection was closed");
    fixture.assert_no_further_connection();
    assert_eq!(token.publication_state(), "not-started");
}

#[test]
fn a_stop_closes_a_response_whose_body_stalls() {
    let fixture = Fixture::serve(Answer::HoldBody);
    let (result, latency, token) = stop_in_flight(&fixture);
    assert_eq!(result.unwrap_err(), TransportError::Cancelled);
    assert!(latency < Duration::from_secs(2), "{latency:?}");
    // The provider answered with a status, so the request certainly reached it.
    assert_eq!(token.llm_effect(), "completed");
    fixture
        .closed
        .recv_timeout(Duration::from_secs(2))
        .expect("the in-flight connection was closed");
    fixture.assert_no_further_connection();
}

#[test]
fn an_already_acknowledged_stop_never_contacts_the_provider() {
    let fixture = Fixture::serve(Answer::Body(2));
    let transport = ReqwestChatTransport::new().unwrap();
    let token = CancellationToken::new();
    token.cancel();
    let error = transport
        .post_json_controlled(&fixture.endpoint, None, b"{}", &token)
        .unwrap_err();
    assert_eq!(error, TransportError::Cancelled);
    assert_eq!(token.llm_effect(), "not-started");
    fixture.assert_no_further_connection();
}

#[test]
fn a_controlled_exchange_keeps_the_response_cap_and_reports_completion() {
    let transport = ReqwestChatTransport::new().unwrap();
    let exact = Fixture::serve(Answer::Body(crate::MAX_CHAT_RESPONSE_BYTES));
    let token = CancellationToken::new();
    let response = transport
        .post_json_controlled(&exact.endpoint, None, b"{}", &token)
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.body().len(), crate::MAX_CHAT_RESPONSE_BYTES);
    assert_eq!(token.llm_effect(), "completed");
    assert!(!token.acknowledged());

    let over = Fixture::serve(Answer::Body(crate::MAX_CHAT_RESPONSE_BYTES + 1));
    let error = transport
        .post_json_controlled(&over.endpoint, None, b"{}", &token)
        .unwrap_err();
    assert_eq!(
        error,
        TransportError::ResponseTooLarge {
            limit: crate::MAX_CHAT_RESPONSE_BYTES
        }
    );
}

#[test]
fn a_controlled_exchange_keeps_the_https_only_credential_gate() {
    let fixture = Fixture::serve(Answer::Body(2));
    let transport = ReqwestChatTransport::new().unwrap();
    let token = CancellationToken::new();
    let key = ApiKey::new("sk-controlled-canary").unwrap();
    let error = transport
        .post_json_controlled(&fixture.endpoint, Some(&key), b"{}", &token)
        .unwrap_err();
    assert_eq!(error, TransportError::CredentialRequiresHttps);
    assert_eq!(token.llm_effect(), "not-started");
    fixture.assert_no_further_connection();
}

#[test]
fn a_refused_connection_is_not_an_unknown_provider_effect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let endpoint =
        Endpoint::parse(&format!("http://{address}/v1/chat/completions"), false).unwrap();
    let transport = ReqwestChatTransport::new().unwrap();
    let token = CancellationToken::new();
    let error = transport
        .post_json_controlled(&endpoint, None, b"{}", &token)
        .unwrap_err();
    assert_eq!(error, TransportError::RequestFailed);
    assert_eq!(token.llm_effect(), "not-started");
}

/// An adapter with no interruption of its own refuses a controlled token
/// instead of silently running an unstoppable paid request.
#[test]
fn an_adapter_without_interruption_refuses_before_contact() {
    struct Blocking;
    impl ChatTransport for Blocking {
        fn post_json(
            &self,
            _: &Endpoint,
            _: Option<&ApiKey>,
            _: &[u8],
        ) -> Result<crate::TransportResponse, TransportError> {
            panic!("the paid request must not start");
        }
    }
    let endpoint = Endpoint::parse("http://127.0.0.1:9/v1/chat/completions", false).unwrap();
    let error = Blocking
        .post_json_controlled(&endpoint, None, b"{}", &CancellationToken::new())
        .unwrap_err();
    assert_eq!(error, TransportError::ControlUnavailable);
}
