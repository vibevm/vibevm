use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use vibe_safefs::{Project, StreamOutcome};

use super::*;
use crate::Token;

const NAME: &str = ".vibe-0123456789abcdef0123456789abcdef-00-abcdef0123456789";
const TOKEN: &str = "fixture-secret-do-not-leak";

#[derive(Debug)]
struct Received {
    path: String,
    authorization: String,
    content_length: usize,
    body: Vec<u8>,
}

fn request(mut stream: TcpStream) -> Received {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(&mut stream);
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let path = first.split_whitespace().nth(1).unwrap().to_string();
    let mut authorization = String::new();
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "authorization" => authorization = value.trim().to_string(),
            "content-length" => content_length = value.trim().parse().unwrap(),
            _ => {}
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    Received {
        path,
        authorization,
        content_length,
        body,
    }
}

fn response(status: &str, extra_headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n{extra_headers}\r\n{body}",
        body.len()
    )
}

fn serve_once(reply: String) -> (String, thread::JoinHandle<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let received = request(stream.try_clone().unwrap());
        stream.write_all(reply.as_bytes()).unwrap();
        received
    });
    (url, handle)
}

fn client(url: &str) -> GithubReleaseClient {
    GithubReleaseClient::with_endpoints_and_timeouts(
        Token::from_explicit(TOKEN),
        "vibevm",
        "vibevm",
        url,
        url,
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .unwrap()
}

fn stream(
    bytes: &[u8],
) -> (
    tempfile::TempDir,
    VerifiedStreamReader,
    StreamCompletion,
    String,
) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("asset"), bytes).unwrap();
    let project = Project::open(root.path()).unwrap();
    let digest = sha256_digest(bytes);
    let (reader, completion) = project
        .open_verified_stream_bounded(
            "asset",
            &digest[7..],
            bytes.len() as u64,
            bytes.len() as u64,
        )
        .unwrap();
    (root, reader, completion, digest)
}

#[test]
fn exact_body_over_64k_returns_only_response_evidence() {
    let bytes = vec![0x5a; 128 * 1024 + 17];
    let (_root, reader, completion, digest) = stream(&bytes);
    let reply = response(
        "201 Created",
        "",
        &format!(
            "{{\"id\":701,\"name\":\"{NAME}\",\"size\":{},\"digest\":\"{digest}\"}}",
            bytes.len(),
        ),
    );
    let (url, server) = serve_once(reply);
    let evidence = client(&url)
        .upload_temporary_asset_streamed(
            700,
            NAME,
            "application/zip",
            bytes.len() as u64,
            &digest,
            reader,
            completion.clone(),
        )
        .unwrap();
    let received = server.join().unwrap();
    assert_eq!(received.body, bytes);
    assert_eq!(received.content_length, bytes.len());
    assert!(
        received
            .path
            .contains("/repos/vibevm/vibevm/releases/700/assets?name=")
    );
    assert_eq!(received.authorization, format!("Bearer {TOKEN}"));
    assert_eq!(evidence.asset_id, 701);
    assert_eq!(evidence.temporary_name, NAME);
    assert_eq!(evidence.sha256, digest);
    assert!(
        matches!(completion.outcome(), StreamOutcome::Complete { bytes, .. } if bytes == 128 * 1024 + 17)
    );
}

#[test]
fn redirect_never_replays_post_or_grants_landed() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let location = format!("http://{}/redirected", target.local_addr().unwrap());
    let reply = response(
        "307 Temporary Redirect",
        &format!("Location: {location}\r\n"),
        "",
    );
    let (url, server) = serve_once(reply);
    let bytes = b"small exact asset";
    let (_root, reader, completion, digest) = stream(bytes);
    let result = client(&url).upload_temporary_asset_streamed(
        700,
        NAME,
        "application/octet-stream",
        bytes.len() as u64,
        &digest,
        reader,
        completion,
    );
    assert!(matches!(result, Err(StreamedUploadError::Uncertain(_))));
    assert_eq!(server.join().unwrap().body, bytes);
    assert!(
        target.accept().is_err(),
        "redirect target must not receive a POST"
    );
}

#[test]
fn early_close_and_changed_short_source_are_uncertain() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        drop(stream);
    });
    let bytes = vec![0x31; 128 * 1024];
    let (_root, reader, completion, digest) = stream(&bytes);
    let result = client(&url).upload_temporary_asset_streamed(
        700,
        NAME,
        "application/zip",
        bytes.len() as u64,
        &digest,
        reader,
        completion,
    );
    server.join().unwrap();
    assert!(matches!(result, Err(StreamedUploadError::Uncertain(_))));

    let (_root, reader, completion, digest) = stream(&bytes);
    std::fs::write(_root.path().join("asset"), b"short").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut buffer = [0u8; 4096];
        while matches!(socket.read(&mut buffer), Ok(n) if n > 0) {}
    });
    let result = client(&url).upload_temporary_asset_streamed(
        700,
        NAME,
        "application/zip",
        bytes.len() as u64,
        &digest,
        reader,
        completion.clone(),
    );
    server.join().unwrap();
    assert!(matches!(result, Err(StreamedUploadError::Uncertain(_))));
    assert!(matches!(completion.outcome(), StreamOutcome::Uncertain(_)));
}

#[test]
fn absent_or_wrong_response_digest_and_token_echo_are_uncertain_without_leak() {
    let bytes = b"small exact asset";
    let digest = sha256_digest(bytes);
    for returned in ["null".to_owned(), format!("\"sha256:{}\"", "f".repeat(64))] {
        let reply = response(
            "201 Created",
            "",
            &format!(
                "{{\"id\":701,\"name\":\"{NAME}\",\"size\":{},\"digest\":{returned}}}",
                bytes.len(),
            ),
        );
        let (url, server) = serve_once(reply);
        let (_root, reader, completion, _) = stream(bytes);
        let result = client(&url).upload_temporary_asset_streamed(
            700,
            NAME,
            "application/zip",
            bytes.len() as u64,
            &digest,
            reader,
            completion,
        );
        assert!(matches!(result, Err(StreamedUploadError::Uncertain(_))));
        server.join().unwrap();
    }
    for body in [
        "{malformed".to_owned(),
        format!(
            "{{\"id\":0,\"name\":\"{NAME}\",\"size\":{},\"digest\":\"{digest}\"}}",
            bytes.len()
        ),
        format!(
            "{{\"id\":701,\"name\":\"foreign\",\"size\":{},\"digest\":\"{digest}\"}}",
            bytes.len()
        ),
    ] {
        let (url, server) = serve_once(response("201 Created", "", &body));
        let (_root, reader, completion, _) = stream(bytes);
        let result = client(&url).upload_temporary_asset_streamed(
            700,
            NAME,
            "application/zip",
            bytes.len() as u64,
            &digest,
            reader,
            completion,
        );
        assert!(matches!(result, Err(StreamedUploadError::Uncertain(_))));
        server.join().unwrap();
    }
    let (url, server) = serve_once(response("500 Server Error", "", TOKEN));
    let (_root, reader, completion, _) = stream(bytes);
    let error = client(&url)
        .upload_temporary_asset_streamed(
            700,
            NAME,
            "application/zip",
            bytes.len() as u64,
            &digest,
            reader,
            completion,
        )
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(error, StreamedUploadError::Uncertain(_)));
    assert!(!error.to_string().contains(TOKEN));
}
