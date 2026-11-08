//! Release acceptance for operation-owned cancellation of stalled HTTP.
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};
use vibe_core::{
    cancellation::CancellationToken,
    progress::{
        Progress,
        transfer::{Destination, Purpose, Transfer, TransferRecord},
    },
};

fn stalled_server(body: bool) -> (String, mpsc::Receiver<()>, std::thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/data", listener.local_addr().unwrap());
    let (started, ready) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = [0; 4096];
        let _ = stream.read(&mut request).unwrap();
        if body {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n")
                .unwrap();
        }
        started.send(()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            match stream.read(&mut request) {
                Ok(0) => return true,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => return true,
            }
        }
        let bytes: &[u8] = if body {
            b"data"
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        };
        let _ = stream.write_all(bytes);
        false
    });
    (url, ready, server)
}

#[test]
fn cancellation_wakes_a_stalled_header_request() {
    let (url, ready, server) = stalled_server(false);
    let token = CancellationToken::new();
    crate::git_backend::transfers::observe_by_default(
        Progress::default().with_cancellation(token.clone()),
    );
    let signal = token.clone();
    let cancel = std::thread::spawn(move || {
        ready.recv().unwrap();
        signal.cancel();
    });
    let client = ManagedHttpClient::bound(
        Duration::from_secs(3),
        std::sync::Arc::new(HttpAdmission::new([] as [&str; 0]).unwrap()),
    )
    .unwrap();
    let start = Instant::now();
    let outcome = client.get_anonymous(&url, &token);
    let elapsed = start.elapsed();
    cancel.join().unwrap();
    assert!(
        server.join().unwrap(),
        "managed request left its connection open after cancellation"
    );
    crate::git_backend::transfers::observe_by_default(Progress::default());
    assert!(outcome.is_err());
    assert!(
        elapsed < Duration::from_millis(750),
        "stop waited for stalled headers: {elapsed:?}"
    );
}

#[test]
fn cancellation_wakes_a_stalled_response_body() {
    let (url, ready, server) = stalled_server(true);
    let token = CancellationToken::new();
    let signal = token.clone();
    let cancel = std::thread::spawn(move || {
        ready.recv().unwrap();
        std::thread::sleep(Duration::from_millis(150));
        signal.cancel();
    });
    let client = ManagedHttpClient::bound(
        Duration::from_secs(3),
        std::sync::Arc::new(HttpAdmission::new([] as [&str; 0]).unwrap()),
    )
    .unwrap();
    let response = client.get_anonymous(&url, &token).unwrap();
    let task = Progress::default().with_cancellation(token).task("body");
    let mut reader = Transfer::begin(
        TransferRecord::http(Purpose::IndexFile, &url, Destination::Cache),
        Some(&task),
        None,
    )
    .reader(response);
    let start = Instant::now();
    let outcome = reader.read_to_end(&mut Vec::new());
    let elapsed = start.elapsed();
    cancel.join().unwrap();
    assert!(
        server.join().unwrap(),
        "managed request left its connection open after cancellation"
    );
    assert!(outcome.is_err());
    assert!(
        elapsed < Duration::from_millis(750),
        "stop waited for stalled body: {elapsed:?}"
    );
}
