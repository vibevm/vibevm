use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use super::*;
use crate::{Token, sha256_digest};

const TOKEN: &str = "fixture-stream-read-secret";

#[derive(Debug)]
struct RequestSeen {
    path: String,
    authorization: String,
}

fn serve(listener: TcpListener, reply: Vec<u8>) -> thread::JoinHandle<RequestSeen> {
    thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        let path = first.split_whitespace().nth(1).unwrap().to_string();
        let mut authorization = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some((key, value)) = line.split_once(':')
                && key.eq_ignore_ascii_case("authorization")
            {
                authorization = value.trim().to_string();
            }
        }
        socket.write_all(&reply).unwrap();
        RequestSeen {
            path,
            authorization,
        }
    })
}

fn fixed_reply(status: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut reply = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len(),
    )
    .into_bytes();
    reply.extend_from_slice(body);
    reply
}

fn chunked_reply(bytes: &[u8]) -> Vec<u8> {
    let mut reply =
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    for part in bytes.chunks(17_113) {
        reply.extend_from_slice(format!("{:x}\r\n", part.len()).as_bytes());
        reply.extend_from_slice(part);
        reply.extend_from_slice(b"\r\n");
    }
    reply.extend_from_slice(b"0\r\n\r\n");
    reply
}

fn endpoint(reply: Vec<u8>) -> (String, thread::JoinHandle<RequestSeen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (url, serve(listener, reply))
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

fn verify(
    url: &str,
    bytes: &[u8],
    max: u64,
) -> Result<DraftAssetDigestEvidence, DraftAssetDigestError> {
    client(url).verify_draft_asset_digest_streamed(
        701,
        bytes.len() as u64,
        &sha256_digest(bytes),
        max,
    )
}

#[test]
fn exact_body_over_64k_and_chunked_body_without_length_are_verified() {
    let bytes = vec![0x5a; 131_089];
    for reply in [fixed_reply("200 OK", &bytes, ""), chunked_reply(&bytes)] {
        let (url, server) = endpoint(reply);
        let evidence = verify(&url, &bytes, bytes.len() as u64).unwrap();
        let seen = server.join().unwrap();
        assert_eq!(seen.path, "/repos/vibevm/vibevm/releases/assets/701");
        assert_eq!(seen.authorization, format!("Bearer {TOKEN}"));
        assert_eq!(evidence.asset_id, 701);
        assert_eq!(evidence.size_bytes, bytes.len() as u64);
        assert_eq!(evidence.sha256, sha256_digest(&bytes));
    }
}

#[test]
fn early_close_oversize_and_same_size_wrong_bytes_refuse_evidence() {
    let expected = vec![0x51; 131_089];
    let mut short_reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        expected.len(),
    )
    .into_bytes();
    short_reply.extend_from_slice(&expected[..1024]);
    let (url, server) = endpoint(short_reply);
    assert!(matches!(
        verify(&url, &expected, expected.len() as u64),
        Err(DraftAssetDigestError::Uncertain(_))
    ));
    server.join().unwrap();

    let (url, server) = endpoint(chunked_reply(&[0x51; 70_000]));
    assert!(matches!(
        client(&url).verify_draft_asset_digest_streamed(
            701,
            65_536,
            &sha256_digest(&vec![0x51; 65_536]),
            65_536,
        ),
        Err(DraftAssetDigestError::Uncertain(_))
    ));
    server.join().unwrap();

    let wrong = vec![0x52; expected.len()];
    let (url, server) = endpoint(fixed_reply("200 OK", &wrong, ""));
    assert!(matches!(
        verify(&url, &expected, expected.len() as u64),
        Err(DraftAssetDigestError::Uncertain(_))
    ));
    server.join().unwrap();
}

#[test]
fn cross_origin_redirect_strips_token_and_hashes_final_body() {
    let bytes = vec![0x33; 80_017];
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_url = format!("http://{}", target.local_addr().unwrap());
    let target_server = serve(target, fixed_reply("200 OK", &bytes, ""));
    let redirect = fixed_reply(
        "302 Found",
        b"",
        &format!("Location: {target_url}/blob\r\n"),
    );
    let (source_url, source_server) = endpoint(redirect);
    let evidence = verify(&source_url, &bytes, bytes.len() as u64).unwrap();
    let source = source_server.join().unwrap();
    let target = target_server.join().unwrap();
    assert_eq!(source.authorization, format!("Bearer {TOKEN}"));
    assert_eq!(target.path, "/blob");
    assert!(
        target.authorization.is_empty(),
        "cross-origin target must not receive token"
    );
    assert_eq!(evidence.sha256, sha256_digest(&bytes));
}

#[test]
fn non_success_or_declared_oversize_never_proves_asset_and_error_hides_token() {
    let bytes = b"expected asset bytes";
    let (url, server) = endpoint(fixed_reply("404 Not Found", b"", ""));
    assert!(matches!(
        verify(&url, bytes, bytes.len() as u64),
        Err(DraftAssetDigestError::Uncertain(_))
    ));
    server.join().unwrap();
    let (url, server) = endpoint(fixed_reply("500 Server Error", TOKEN.as_bytes(), ""));
    let error = verify(&url, bytes, bytes.len() as u64).unwrap_err();
    server.join().unwrap();
    assert!(matches!(error, DraftAssetDigestError::Uncertain(_)));
    assert!(!error.to_string().contains(TOKEN));

    let (url, server) = endpoint(fixed_reply("200 OK", &vec![0x41; 70_000], ""));
    let error = client(&url)
        .verify_draft_asset_digest_streamed(
            701,
            65_536,
            &sha256_digest(&vec![0x41; 65_536]),
            65_536,
        )
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(error, DraftAssetDigestError::Uncertain(_)));
}
