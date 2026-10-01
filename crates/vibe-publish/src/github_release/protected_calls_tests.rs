use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use super::*;
use crate::Token;

const TOKEN: &str = "fixture-protected-call-secret";
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Received {
    method: String,
    path: String,
    authorization: String,
    body: Vec<u8>,
}

fn receive(mut socket: TcpStream) -> Received {
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(&mut socket);
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap().to_owned();
    let path = parts.next().unwrap().to_owned();
    let mut authorization = String::new();
    let mut size = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("authorization") {
                authorization = value.trim().into();
            }
            if name.eq_ignore_ascii_case("content-length") {
                size = value.trim().parse().unwrap();
            }
        }
    }
    let mut body = vec![0; size];
    reader.read_exact(&mut body).unwrap();
    Received {
        method,
        path,
        authorization,
        body,
    }
}

fn reply(status: &str, body: &str, extra: &str) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n{extra}\r\n",
        body.len(),
    ).into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

fn serve_once(response: Vec<u8>) -> (String, thread::JoinHandle<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let received = receive(socket.try_clone().unwrap());
        let _ = socket.write_all(&response);
        received
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

fn tag_body(reference: &str, commit: &str) -> String {
    format!("{{\"ref\":\"{reference}\",\"object\":{{\"sha\":\"{commit}\",\"type\":\"commit\"}}}}")
}

fn draft_body(id: u64, tag: &str, commit: &str, draft: bool) -> String {
    serde_json::json!({
        "id": id,
        "tag_name": tag,
        "target_commitish": commit,
        "name": DRAFT_NAME,
        "body": fixed_draft_body(),
        "draft": draft,
        "prerelease": false,
    })
    .to_string()
}

fn selected_draft(commit: &str) -> DraftCallFields {
    DraftCallFields {
        target_commit: commit.into(),
        tag: TAG.into(),
        draft: true,
        name: DRAFT_NAME.into(),
        body: fixed_draft_body(),
        prerelease: false,
    }
}

fn changed_draft(field: &str, value: serde_json::Value) -> String {
    let mut row: serde_json::Value =
        serde_json::from_str(&draft_body(701, TAG, COMMIT, true)).unwrap();
    row[field] = value;
    row.to_string()
}

#[test]
fn exact_201_tag_204_delete_and_201_draft_are_response_evidence_only() {
    let (base, server) = serve_once(reply("201 Created", &tag_body(REF, COMMIT), ""));
    let evidence = client(&base).create_fixed_tag_ref_explicit(COMMIT).unwrap();
    let received = server.join().unwrap();
    assert_eq!(received.method, "POST");
    assert_eq!(received.path, "/repos/vibevm/vibevm/git/refs");
    assert_eq!(received.authorization, format!("Bearer {TOKEN}"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&received.body).unwrap(),
        serde_json::json!({"ref": REF, "sha": COMMIT})
    );
    assert_eq!(evidence.object_sha, COMMIT);

    let (base, server) = serve_once(reply("204 No Content", "", ""));
    let evidence = client(&base).delete_prior_release_explicit(700).unwrap();
    let received = server.join().unwrap();
    assert_eq!(
        (received.method.as_str(), received.path.as_str()),
        ("DELETE", "/repos/vibevm/vibevm/releases/700")
    );
    assert_eq!(evidence.release_id, 700);

    let (base, server) = serve_once(reply(
        "201 Created",
        &draft_body(701, TAG, COMMIT, true),
        "",
    ));
    let evidence = client(&base)
        .create_fixed_draft_explicit(&selected_draft(COMMIT))
        .unwrap();
    let received = server.join().unwrap();
    assert_eq!(received.method, "POST");
    assert_eq!(received.path, "/repos/vibevm/vibevm/releases");
    let posted: serde_json::Value = serde_json::from_slice(&received.body).unwrap();
    assert_eq!(posted["tag_name"], TAG);
    assert_eq!(posted["target_commitish"], COMMIT);
    assert_eq!(posted["name"], "vibevm 1.1.0");
    assert_eq!(
        posted["body"],
        format!(
            "Mutable vibevm 1.1.0 distribution. Verify downloads through `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`."
        )
    );
    assert_eq!(posted["draft"], true);
    assert_eq!(posted["prerelease"], false);
    assert_eq!(posted["generate_release_notes"], false);
    assert_eq!((evidence.release_id, evidence.tag.as_str()), (701, TAG));
    assert_eq!(evidence.target_commitish, COMMIT);
    assert_eq!(evidence.name, "vibevm 1.1.0");
    assert_eq!(evidence.body, fixed_draft_body());
    assert!(evidence.draft);
    assert!(!evidence.prerelease);
}

#[test]
fn write_redirects_never_replay_on_another_endpoint() {
    for action in 0..3 {
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let location = format!("http://{}/replayed", target.local_addr().unwrap());
        let (base, server) = serve_once(reply(
            "307 Temporary Redirect",
            "",
            &format!("Location: {location}\r\n"),
        ));
        let client = client(&base);
        let result = match action {
            0 => client.create_fixed_tag_ref_explicit(COMMIT).map(|_| ()),
            1 => client.delete_prior_release_explicit(700).map(|_| ()),
            _ => client
                .create_fixed_draft_explicit(&selected_draft(COMMIT))
                .map(|_| ()),
        };
        assert!(matches!(result, Err(ProtectedCallError::Uncertain(_))));
        server.join().unwrap();
        assert!(
            target.accept().is_err(),
            "redirect target must not receive write"
        );
    }
}

#[test]
fn lost_delete_response_and_foreign_client_do_not_grant_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        let seen = receive(socket);
        seen
    });
    let result = client(&base).delete_prior_release_explicit(700);
    assert!(matches!(result, Err(ProtectedCallError::Uncertain(_))));
    assert_eq!(server.join().unwrap().method, "DELETE");

    let foreign = GithubReleaseClient::with_endpoints_and_timeouts(
        Token::from_explicit(TOKEN),
        "foreign",
        "vibevm",
        "http://127.0.0.1:9",
        "http://127.0.0.1:9",
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(matches!(
        foreign.create_fixed_tag_ref_explicit(COMMIT),
        Err(ProtectedCallError::Refused(_))
    ));
    assert!(matches!(
        foreign.delete_prior_release_explicit(700),
        Err(ProtectedCallError::Refused(_))
    ));

    let local = client("http://127.0.0.1:9");
    let long_oid = "a".repeat(64);
    assert!(matches!(
        local.create_fixed_tag_ref_explicit(&long_oid),
        Err(ProtectedCallError::Refused(_))
    ));
    assert!(matches!(
        local.create_fixed_draft_explicit(&selected_draft(&long_oid)),
        Err(ProtectedCallError::Refused(_))
    ));
}

#[test]
fn wrong_e2_successor_fields_refuse_before_any_http_dispatch() {
    let local = client("http://127.0.0.1:9");
    for field in ["target", "tag", "draft", "name", "body", "prerelease"] {
        let mut selected = selected_draft(COMMIT);
        match field {
            "target" => selected.target_commit = "not-a-commit".into(),
            "tag" => selected.tag = "v9.9.9".into(),
            "draft" => selected.draft = false,
            "name" => selected.name = "foreign".into(),
            "body" => selected.body = "foreign".into(),
            "prerelease" => selected.prerelease = true,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                local.create_fixed_draft_explicit(&selected),
                Err(ProtectedCallError::Refused(_))
            ),
            "{field}"
        );
    }
}

#[test]
fn selected_valid_commit_is_sent_from_e2_fields_not_a_transport_default() {
    let other_commit = "b".repeat(40);
    let selected = selected_draft(&other_commit);
    let (base, server) = serve_once(reply(
        "201 Created",
        &draft_body(702, TAG, &other_commit, true),
        "",
    ));
    let evidence = client(&base)
        .create_fixed_draft_explicit(&selected)
        .unwrap();
    let received = server.join().unwrap();
    let posted: serde_json::Value = serde_json::from_slice(&received.body).unwrap();
    assert_eq!(posted["target_commitish"], other_commit);
    assert_eq!(evidence.target_commitish, other_commit);
    assert_eq!(evidence.release_id, 702);
}

#[test]
fn malformed_overcap_and_foreign_responses_are_uncertain_without_token_leak() {
    for body in [
        "{bad".to_owned(),
        tag_body("refs/tags/v9.9.9", COMMIT),
        tag_body(REF, &"b".repeat(40)),
        "x".repeat(TAG_RESPONSE_CAP as usize + 1),
    ] {
        let (base, server) = serve_once(reply("201 Created", &body, ""));
        let result = client(&base).create_fixed_tag_ref_explicit(COMMIT);
        assert!(matches!(result, Err(ProtectedCallError::Uncertain(_))));
        server.join().unwrap();
    }
    for body in [
        "{bad".to_owned(),
        draft_body(0, TAG, COMMIT, true),
        draft_body(701, "v9.9.9", COMMIT, true),
        draft_body(701, TAG, COMMIT, false),
        draft_body(701, TAG, &"b".repeat(40), true),
        changed_draft("name", serde_json::json!("foreign")),
        changed_draft("body", serde_json::json!("foreign")),
        changed_draft("prerelease", serde_json::json!(true)),
        "x".repeat(RELEASE_RESPONSE_CAP as usize + 1),
    ] {
        let (base, server) = serve_once(reply("201 Created", &body, ""));
        let result = client(&base).create_fixed_draft_explicit(&selected_draft(COMMIT));
        assert!(matches!(result, Err(ProtectedCallError::Uncertain(_))));
        server.join().unwrap();
    }
    let (base, server) = serve_once(reply("500 Server Error", TOKEN, ""));
    let error = client(&base)
        .delete_prior_release_explicit(700)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(error, ProtectedCallError::Uncertain(_)));
    assert!(!error.to_string().contains(TOKEN));
}
