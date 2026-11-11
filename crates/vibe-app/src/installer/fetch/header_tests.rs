//! This invocation's extra request headers on a declared input
//! (`PROP-075#DOWNLOAD-HEADERS`): both fetch branches send them, a source
//! that is not an HTTP URL gets none, nothing that is written down ever
//! carries a value, and a redirect out of the origin is refused rather
//! than followed with the credential in hand.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use vibe_core::download_headers::DownloadHeaders;
use vibe_core::progress::Progress;
use vibe_core::progress::transfer::{TransferJournal, Transfers};

use super::super::FetchWatch;
use super::*;

const SECRET: &str = "s3cr3t-gateway-token";

fn declared() -> DownloadHeaders {
    DownloadHeaders::parse(
        &format!("# the gateway\nAuthorization: Bearer {SECRET}\nX-Mirror: eu\n"),
        "headers.txt",
    )
    .unwrap()
}

/// Answer each connection with `reply(n)`, recording every request head.
fn serve(
    listener: TcpListener,
    reply: impl Fn(usize) -> String + Send + 'static,
) -> Arc<Mutex<Vec<String>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for (index, stream) in listener.incoming().flatten().enumerate() {
            let mut stream = stream;
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            log.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            let _ = stream.write_all(reply(index).as_bytes());
        }
    });
    seen
}

fn payload(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// The operation an ordinary `install -g` builds: no storage of its own
/// and a neutral policy, which is the unbound incumbent.
fn operation() -> vibe_package_source::SourceOperation {
    vibe_package_source::source_operation(
        &vibe_package_source::PackageSourceOptions::default(),
        &vibe_core::manifest::Manifest::default(),
        std::path::Path::new("."),
        vibe_core::GlobalRegistrySource::ready(vibe_core::GlobalRegistryConfig::default()),
    )
    .unwrap()
}

fn carries_the_credential(request: &str) -> bool {
    request.contains(SECRET)
}

/// The managed branch — the one every `install -g` takes — sends the
/// declared headers, and nothing it writes down carries a value.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
fn the_managed_branch_sends_the_headers_and_journals_none_of_them() {
    let body = "payload";
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!(
        "http://{}/mirror/payload.bin",
        listener.local_addr().unwrap()
    );
    let seen = serve(listener, move |_| payload(body));

    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let task = Progress::default()
        .with_transfers(Some(Transfers::new(
            Some(journal.clone()),
            None,
            "install -g",
        )))
        .task("fetch");
    let operation = operation();
    let client = operation.http_client(std::time::Duration::from_secs(30));
    let headers = declared();
    let watch = FetchWatch {
        http_client: Some(&client),
        download_headers: Some(&headers),
        task: Some(&task),
        ..FetchWatch::default()
    };
    fetch_verified(
        &source,
        &hex(&Sha256::digest(body.as_bytes())),
        home.path(),
        &home.path().join("out"),
        false,
        &watch,
    )
    .unwrap();
    task.finish();

    let requests = seen.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert!(carries_the_credential(&requests[0]), "{:?}", requests[0]);
    assert!(requests[0].contains("x-mirror: eu") || requests[0].contains("X-Mirror: eu"));

    let entry = &journal.history()[0];
    let recorded = format!("{entry:?}");
    assert!(!recorded.contains(SECRET), "{recorded}");
    assert!(
        !std::fs::read_to_string(home.path().join("transfers/history.jsonl"))
            .unwrap()
            .contains(SECRET)
    );
}

/// The direct branch — a fetch handed no managed client — sends them too.
#[test]
fn the_direct_branch_sends_the_headers() {
    let body = "payload";
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!(
        "http://{}/mirror/payload.bin",
        listener.local_addr().unwrap()
    );
    let seen = serve(listener, move |_| payload(body));

    let home = tempfile::tempdir().unwrap();
    let headers = declared();
    fetch_verified(
        &source,
        &hex(&Sha256::digest(body.as_bytes())),
        home.path(),
        &home.path().join("out"),
        false,
        &FetchWatch {
            download_headers: Some(&headers),
            ..FetchWatch::default()
        },
    )
    .unwrap();
    let requests = seen.lock().unwrap().clone();
    assert!(carries_the_credential(&requests[0]), "{:?}", requests[0]);
}

/// A source that is not an HTTP URL is a file in the package: it is read,
/// not requested, so the headers of this invocation reach nothing at all.
#[test]
fn a_package_relative_source_is_read_with_no_request_and_no_header() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("input.bin"), b"payload").unwrap();
    let headers = declared();
    fetch_verified(
        "input.bin",
        &hex(&Sha256::digest(b"payload")),
        home.path(),
        &home.path().join("out"),
        false,
        &FetchWatch {
            download_headers: Some(&headers),
            ..FetchWatch::default()
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read(home.path().join("out")).unwrap(),
        b"payload".to_vec()
    );
}

/// A download that fails says why, and says it without the credential —
/// not in the message, not in the debug rendering, not in the journal.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
fn a_failing_download_puts_no_header_value_in_its_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!(
        "http://{}/mirror/payload.bin",
        listener.local_addr().unwrap()
    );
    serve(listener, |_| {
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
    });

    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let task = Progress::default()
        .with_transfers(Some(Transfers::new(
            Some(journal.clone()),
            None,
            "install -g",
        )))
        .task("fetch");
    let operation = operation();
    let client = operation.http_client(std::time::Duration::from_secs(30));
    let headers = declared();
    let error = fetch_verified(
        &source,
        &"0".repeat(64),
        home.path(),
        &home.path().join("out"),
        false,
        &FetchWatch {
            http_client: Some(&client),
            download_headers: Some(&headers),
            task: Some(&task),
            ..FetchWatch::default()
        },
    )
    .unwrap_err();
    task.finish();

    let shown = format!("{error} {error:?}");
    assert!(
        shown.contains("503"),
        "the refusal names the failure: {shown}"
    );
    assert!(!shown.contains(SECRET), "{shown}");
    let history = home.path().join("transfers/history.jsonl");
    let journaled = std::fs::read(&history).unwrap_or_default();
    assert!(!String::from_utf8_lossy(&journaled).contains(SECRET));
}

/// nvm refuses a cross-origin redirect outright when `$NVM_AUTH_HEADER`
/// is set, because the downloader would resend the credential. The direct
/// branch has no walk of its own, so it refuses in the same way — and the
/// other origin is never asked.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
fn the_direct_branch_refuses_a_redirect_out_of_the_declared_origin() {
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    let away = format!("http://{}/payload.bin", second.local_addr().unwrap());
    let source = format!("http://{}/mirror/payload.bin", first.local_addr().unwrap());
    let elsewhere = away.clone();
    serve(first, move |_| {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {elsewhere}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
    });
    let seen = serve(second, |_| payload("payload"));

    let home = tempfile::tempdir().unwrap();
    let headers = declared();
    let error = fetch_verified(
        &source,
        &hex(&Sha256::digest(b"payload")),
        home.path(),
        &home.path().join("out"),
        false,
        &FetchWatch {
            download_headers: Some(&headers),
            ..FetchWatch::default()
        },
    )
    .unwrap_err();
    let shown = format!("{error}");
    assert!(shown.contains("another origin"), "{shown}");
    assert!(!shown.contains(SECRET), "{shown}");
    assert!(
        seen.lock().unwrap().is_empty(),
        "the other origin is never asked"
    );
}

/// With no headers declared the redirect policy is untouched: the same
/// cross-origin redirect is followed exactly as it always was.
#[test]
fn without_headers_a_redirect_is_followed_as_before() {
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    let away = format!("http://{}/payload.bin", second.local_addr().unwrap());
    let source = format!("http://{}/mirror/payload.bin", first.local_addr().unwrap());
    serve(first, move |_| {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {away}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
    });
    serve(second, |_| payload("payload"));

    let home = tempfile::tempdir().unwrap();
    fetch_verified(
        &source,
        &hex(&Sha256::digest(b"payload")),
        home.path(),
        &home.path().join("out"),
        false,
        &FetchWatch::default(),
    )
    .unwrap();
}
