use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use super::super::remote_observation::observe_twice;
use super::*;
use crate::{Token, sha256_digest};

const TOKEN: &str = "fixture-observation-secret";
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Seen {
    path: String,
    authorization: String,
}

fn reply(status: &str, body: &str, extra: &str) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n{extra}\r\n",
        body.len(),
    ).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

fn server(make: impl FnOnce(&str) -> Vec<Vec<u8>>) -> (String, thread::JoinHandle<Vec<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let responses = make(&base);
    let handle = thread::spawn(move || {
        let mut seen = Vec::new();
        for response in responses {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            let path = first.split_whitespace().nth(1).unwrap().to_owned();
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
                    authorization = value.trim().into();
                }
            }
            let _ = socket.write_all(&response);
            seen.push(Seen {
                path,
                authorization,
            });
        }
        seen
    });
    (base, handle)
}

fn client(base: &str) -> GithubReleaseClient {
    GithubReleaseClient::with_endpoints_and_timeouts(
        Token::from_explicit(TOKEN),
        "vibevm",
        "vibevm",
        base,
        base,
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .unwrap()
}

fn tag() -> String {
    format!(
        "{{\"ref\":\"refs/tags/v1.1.0\",\"object\":{{\"type\":\"commit\",\"sha\":\"{COMMIT}\"}}}}"
    )
}
fn release(id: u64, body: &str) -> String {
    format!(
        "{{\"id\":{id},\"tag_name\":\"v1.1.0\",\"draft\":true,\"target_commitish\":\"{COMMIT}\",\"body\":\"{body}\"}}"
    )
}
fn asset(id: u64, name: &str, bytes: &[u8], digest: Option<&str>) -> String {
    let digest = digest.map_or("null".to_owned(), |value| format!("\"{value}\""));
    format!(
        "{{\"id\":{id},\"name\":\"{name}\",\"size\":{},\"digest\":{digest},\"content_type\":\"application/octet-stream\"}}",
        bytes.len()
    )
}

fn present_epoch(body: &str, assets: &[String]) -> Vec<Vec<u8>> {
    vec![
        reply(
            "200 OK",
            "{\"id\":101,\"tag_name\":\"v1.0.1\",\"draft\":false,\"prerelease\":false}",
            "",
        ),
        reply("200 OK", &tag(), ""),
        reply("200 OK", &release(700, body), ""),
        reply("200 OK", &format!("[{}]", assets.join(",")), ""),
    ]
}

#[test]
fn stable_absent_and_fixed_target_guard() {
    let (base, handle) = server(|_| {
        (0..2)
            .flat_map(|_| {
                [
                    reply("404 Not Found", "{}", ""),
                    reply("404 Not Found", "{}", ""),
                    reply("404 Not Found", "{}", ""),
                    reply("200 OK", "[]", ""),
                ]
            })
            .collect()
    });
    let loopback_client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&loopback_client).unwrap();
    let observed = observe_twice(&mut port, COMMIT).unwrap();
    let seen = handle.join().unwrap();
    assert_eq!(seen.len(), 8);
    assert_eq!(seen[0].path, "/repos/vibevm/vibevm/releases/latest");
    assert_eq!(seen[4].path, "/repos/vibevm/vibevm/releases/latest");
    assert!(
        seen.iter()
            .all(|row| row.authorization == format!("Bearer {TOKEN}"))
    );
    assert!(matches!(
        observed.tag,
        vibe_wire::generated::publication_effect_plan::PublicationObservedTag::Absent(_)
    ));
    assert!(matches!(
        observed.release,
        vibe_wire::generated::publication_effect_plan::PublicationObservedRelease::Absent(_)
    ));

    let fixed = GithubReleaseClient::new(Token::from_explicit(TOKEN), "vibevm", "vibevm").unwrap();
    assert!(GithubRemoteReadPort::new_fixed(&fixed).is_ok());
    let foreign =
        GithubReleaseClient::new(Token::from_explicit(TOKEN), "foreign", "vibevm").unwrap();
    assert!(matches!(
        GithubRemoteReadPort::new_fixed(&foreign),
        Err(ObservationError::Refused(_))
    ));
}

#[test]
fn release_without_tag_is_left_for_pure_normalizer_to_refuse() {
    let (base, handle) = server(|_| {
        vec![
            reply(
                "200 OK",
                "{\"id\":101,\"tag_name\":\"v1.0.1\",\"draft\":false,\"prerelease\":false}",
                "",
            ),
            reply("404 Not Found", "{}", ""),
            reply("200 OK", &release(700, "draft"), ""),
            reply("200 OK", "[]", ""),
        ]
    });
    let client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
    let snapshot = port.read_complete().unwrap();
    assert!(snapshot.tag.is_none());
    assert!(snapshot.release.is_some());
    assert!(matches!(
        super::super::remote_observation::normalize_fixed(snapshot, COMMIT),
        Err(ObservationError::Refused(_))
    ));
    assert_eq!(handle.join().unwrap().len(), 4);
}

#[test]
fn stable_present_reordered_assets_keep_one_generation() {
    let bytes = b"seven!!";
    let digest = sha256_digest(bytes);
    let first = vec![
        asset(702, "z.zip", bytes, Some(&digest)),
        asset(701, "a.zip", bytes, Some(&digest)),
    ];
    let second = vec![first[1].clone(), first[0].clone()];
    let (base, handle) = server(|_| {
        [
            present_epoch("same", &first),
            present_epoch("same", &second),
        ]
        .concat()
    });
    let client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
    let observed = observe_twice(&mut port, COMMIT).unwrap();
    assert_eq!(handle.join().unwrap().len(), 8);
    assert!(observed.generation_ref.starts_with("sha256:"));
    let vibe_wire::generated::publication_effect_plan::PublicationObservedRelease::Present(row) =
        observed.release
    else {
        panic!()
    };
    assert_eq!(row.assets.len(), 2);
    assert_eq!(row.assets[0].asset_id, "701");
}

#[test]
fn missing_metadata_digest_is_streamed_by_same_id_in_each_epoch() {
    let bytes = b"seven!!";
    let row = asset(701, "a.zip", bytes, None);
    let binary = String::from_utf8(bytes.to_vec()).unwrap();
    let (base, handle) = server(|_| {
        (0..2)
            .flat_map(|_| {
                let mut reads = present_epoch("same", std::slice::from_ref(&row));
                reads.push(reply("200 OK", &binary, ""));
                reads
            })
            .collect()
    });
    let client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
    let observed = observe_twice(&mut port, COMMIT).unwrap();
    let seen = handle.join().unwrap();
    assert_eq!(seen.len(), 10);
    assert_eq!(
        seen.iter()
            .filter(|row| row.path.ends_with("/releases/assets/701"))
            .count(),
        2
    );
    let vibe_wire::generated::publication_effect_plan::PublicationObservedRelease::Present(row) =
        observed.release
    else {
        panic!()
    };
    assert_eq!(row.assets[0].sha256, sha256_digest(bytes));
}

#[test]
fn changed_release_or_asset_between_epochs_reports_drift() {
    let bytes = b"seven!!";
    let digest = sha256_digest(bytes);
    let row = asset(701, "a.zip", bytes, Some(&digest));
    for second in [
        present_epoch("changed", std::slice::from_ref(&row)),
        present_epoch("same", &[asset(702, "a.zip", bytes, Some(&digest))]),
    ] {
        let (base, handle) =
            server(|_| [present_epoch("same", std::slice::from_ref(&row)), second].concat());
        let client = client(&base);
        let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
        assert_eq!(
            observe_twice(&mut port, COMMIT),
            Err(ObservationError::Drift)
        );
        handle.join().unwrap();
    }
}

#[test]
fn malformed_overcap_foreign_pagination_and_token_echo_are_unavailable() {
    let huge = "x".repeat(1024 * 1024 + 1);
    for response in [
        reply("200 OK", "{bad", ""),
        reply("200 OK", &huge, ""),
        reply("500 Server Error", TOKEN, ""),
    ] {
        let (base, handle) = server(|_| vec![response]);
        let client = client(&base);
        let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
        let error = port.read_complete().unwrap_err();
        handle.join().unwrap();
        assert!(matches!(error, ObservationError::Unavailable(_)));
        assert!(!error.to_string().contains(TOKEN));
    }
    let row = asset(701, "a.zip", b"seven!!", Some(&sha256_digest(b"seven!!")));
    let (base, handle) = server(|_| {
        vec![
            reply(
                "200 OK",
                "{\"id\":101,\"tag_name\":\"v1.0.1\",\"draft\":false,\"prerelease\":false}",
                "",
            ),
            reply("200 OK", &tag(), ""),
            reply("200 OK", &release(700, "same"), ""),
            reply(
                "200 OK",
                &format!("[{row}]"),
                "Link: <http://127.0.0.1:9/foreign>; rel=\"next\"\r\n",
            ),
        ]
    });
    let foreign_client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&foreign_client).unwrap();
    assert!(matches!(
        port.read_complete(),
        Err(ObservationError::Unavailable(_))
    ));
    handle.join().unwrap();

    let (base, handle) = server(|_| {
        vec![
            reply(
                "200 OK",
                "{\"id\":101,\"tag_name\":\"v1.0.1\",\"draft\":false,\"prerelease\":false}",
                "",
            ),
            reply("200 OK", &tag(), ""),
            reply("200 OK", &release(700, "same"), ""),
            reply(
                "200 OK",
                &format!("[{}]", asset(701, "a.zip", b"seven!!", None)),
                "",
            ),
            reply("404 Not Found", "{}", ""),
        ]
    });
    let client = client(&base);
    let mut port = GithubRemoteReadPort::new_loopback(&client).unwrap();
    assert!(matches!(
        port.read_complete(),
        Err(ObservationError::Unavailable(_))
    ));
    assert_eq!(handle.join().unwrap().len(), 5);
}
