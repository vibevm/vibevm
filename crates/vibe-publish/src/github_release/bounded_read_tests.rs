use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use super::*;
use crate::Token;

const TOKEN: &str = "fixture-bounded-json-secret";
const TAG: &str = "v1.1.0";

#[derive(Debug)]
struct Seen {
    path: String,
    authorization: String,
}

fn reply(status: &str, body: &str, extra: &str) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n{extra}\r\n",
        body.len(),
    ).into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

fn scripted(
    make_replies: impl FnOnce(&str) -> Vec<Vec<u8>>,
) -> (String, thread::JoinHandle<Vec<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let replies = make_replies(&base);
    let server = thread::spawn(move || {
        let mut seen = Vec::new();
        for response in replies {
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
                    authorization = value.trim().to_owned();
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
    (base, server)
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

fn tag_json() -> String {
    format!(
        "{{\"ref\":\"refs/tags/{TAG}\",\"object\":{{\"type\":\"commit\",\"sha\":\"{}\"}}}}",
        "a".repeat(40)
    )
}

fn release_json(id: u64, tag: &str, draft: bool) -> String {
    format!("{{\"id\":{id},\"tag_name\":\"{tag}\",\"draft\":{draft}}}")
}

fn asset_json(id: u64, name: &str) -> String {
    format!("{{\"id\":{id},\"name\":\"{name}\",\"size\":7,\"digest\":null}}")
}

#[test]
fn exact_tag_404_is_absent_and_redirect_never_forwards_token() {
    let (base, server) = scripted(|_| vec![reply("404 Not Found", "{}", "")]);
    assert!(client(&base).read_tag_ref_bounded(TAG).unwrap().is_none());
    let seen = server.join().unwrap();
    assert_eq!(seen[0].path, "/repos/vibevm/vibevm/git/ref/tags/v1.1.0");
    assert_eq!(seen[0].authorization, format!("Bearer {TOKEN}"));

    let (base, server) = scripted(|_| vec![reply("200 OK", &tag_json(), "")]);
    let tag = client(&base).read_tag_ref_bounded(TAG).unwrap().unwrap();
    server.join().unwrap();
    assert_eq!(tag.object.sha, "a".repeat(40));

    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let location = format!("http://{}/elsewhere", target.local_addr().unwrap());
    let (base, server) =
        scripted(|_| vec![reply("302 Found", "", &format!("Location: {location}\r\n"))]);
    assert!(matches!(
        client(&base).read_tag_ref_bounded(TAG),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();
    assert!(
        target.accept().is_err(),
        "JSON redirect target must not receive token"
    );
}

#[test]
fn draft_release_404_requires_exhausted_authenticated_list() {
    let (base, server) = scripted(|base| {
        vec![
            reply("404 Not Found", "{}", ""),
            reply(
                "200 OK",
                &format!("[{}]", release_json(500, "v1.0.0", false)),
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases?per_page=100&page=2>; rel=\"next\"\r\n"
                ),
            ),
            reply("200 OK", &format!("[{}]", release_json(700, TAG, true)), ""),
        ]
    });
    let release = client(&base)
        .read_release_by_tag_bounded(TAG)
        .unwrap()
        .unwrap();
    let seen = server.join().unwrap();
    assert_eq!(release.id, 700);
    assert!(release.draft);
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[1].path, "/repos/vibevm/vibevm/releases?per_page=100");
    assert!(seen[2].path.contains("page=2"));
    assert!(
        seen.iter()
            .all(|row| row.authorization == format!("Bearer {TOKEN}"))
    );

    let (base, server) =
        scripted(|_| vec![reply("404 Not Found", "{}", ""), reply("200 OK", "[]", "")]);
    assert!(
        client(&base)
            .read_release_by_tag_bounded(TAG)
            .unwrap()
            .is_none()
    );
    server.join().unwrap();
}

#[test]
fn two_asset_pages_are_complete_unique_and_sorted_independent_of_listing_order() {
    let (base, server) = scripted(|base| {
        vec![
            reply(
                "200 OK",
                &format!(
                    "[{},{}]",
                    asset_json(703, "z.zip"),
                    asset_json(701, "b.json")
                ),
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100&page=2>; rel=\"next\"\r\n"
                ),
            ),
            reply("200 OK", &format!("[{}]", asset_json(702, "a.zip")), ""),
        ]
    });
    let assets = client(&base).read_all_assets_bounded(700).unwrap();
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        assets
            .iter()
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a.zip", "b.json", "z.zip"]
    );
    assert_eq!(
        assets.iter().map(|asset| asset.id).collect::<Vec<_>>(),
        vec![702, 701, 703]
    );
}

#[test]
fn malformed_overcap_duplicate_loop_and_foreign_next_refuse() {
    let (base, server) = scripted(|_| vec![reply("200 OK", "{bad", "")]);
    assert!(matches!(
        client(&base).read_tag_ref_bounded(TAG),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) =
        scripted(|_| vec![reply("200 OK", "{\"id\":700,\"tag_name\":\"v1.1.0\"}", "")]);
    assert!(matches!(
        client(&base).read_release_by_tag_bounded(TAG),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let huge = "x".repeat(TAG_JSON_CAP as usize + 1);
    let (base, server) = scripted(|_| vec![reply("200 OK", &huge, "")]);
    assert!(matches!(
        client(&base).read_tag_ref_bounded(TAG),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let huge_page = "x".repeat(PAGE_JSON_CAP as usize + 1);
    let (base, server) = scripted(|_| vec![reply("200 OK", &huge_page, "")]);
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) = scripted(|_| {
        vec![reply(
            "200 OK",
            "[{\"id\":701,\"name\":\"a.zip\",\"size\":7,\"digest\":\"sha256:BAD\"}]",
            "",
        )]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) = scripted(|base| {
        vec![
            reply(
                "200 OK",
                &format!("[{}]", asset_json(701, "same.zip")),
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100&page=2>; rel=\"next\"\r\n"
                ),
            ),
            reply("200 OK", &format!("[{}]", asset_json(701, "other.zip")), ""),
        ]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) = scripted(|base| {
        vec![
            reply(
                "200 OK",
                "[]",
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100&page=2>; rel=\"next\"\r\n"
                ),
            ),
            reply(
                "200 OK",
                "[]",
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100>; rel=\"next\"\r\n"
                ),
            ),
        ]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) = scripted(|_| {
        vec![reply(
            "200 OK",
            "[]",
            "Link: <http://127.0.0.1:9/foreign>; rel=\"next\"\r\n",
        )]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    let (base, server) = scripted(|base| {
        vec![reply(
            "200 OK",
            "[]",
            &format!(
                "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100&page=2>; rel=next\r\n"
            ),
        )]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();

    for query in [
        "per_page=100&page=3",
        "per_page=100&page=2&debug=1",
        "per_page=100&per_page=100&page=2",
    ] {
        let (base, server) = scripted(|base| {
            vec![reply(
                "200 OK",
                "[]",
                &format!(
                    "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?{query}>; rel=\"next\"\r\n"
                ),
            )]
        });
        assert!(matches!(
            client(&base).read_all_assets_bounded(700),
            Err(BoundedReadError::Uncertain(_))
        ));
        server.join().unwrap();
    }
    let (base, server) = scripted(|base| {
        vec![reply(
            "200 OK",
            "[]",
            &format!(
                "Link: <{base}/repos/vibevm/vibevm/releases/700/assets?per_page=100&page=2>; rel=\"next prev\"\r\n"
            ),
        )]
    });
    assert!(matches!(
        client(&base).read_all_assets_bounded(700),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();
}

#[test]
fn token_echo_and_release_404_without_list_never_claim_absent() {
    let (base, server) = scripted(|_| vec![reply("500 Server Error", TOKEN, "")]);
    let error = client(&base).read_tag_ref_bounded(TAG).unwrap_err();
    server.join().unwrap();
    assert!(matches!(error, BoundedReadError::Uncertain(_)));
    assert!(!error.to_string().contains(TOKEN));

    let (base, server) = scripted(|_| {
        vec![
            reply("404 Not Found", "{}", ""),
            reply("500 Server Error", TOKEN, ""),
        ]
    });
    assert!(matches!(
        client(&base).read_release_by_tag_bounded(TAG),
        Err(BoundedReadError::Uncertain(_))
    ));
    server.join().unwrap();
}
