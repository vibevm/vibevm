use super::*;

#[derive(Debug)]
struct HttpSeen {
    method: String,
    path: String,
    pending: bool,
}

fn serve_tag(
    commit: String,
    pending: std::path::PathBuf,
    lost: bool,
) -> (String, std::thread::JoinHandle<Vec<HttpSeen>>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let mut rows = Vec::new();
        let mut posted = false;
        let expected = if lost { 9 } else { 17 };
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            let (mut socket, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if rows.len() >= expected {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("loopback accept: {error}"),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            let mut parts = first.split_whitespace();
            let method = parts.next().unwrap().to_owned();
            let path = parts.next().unwrap().to_owned();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let pending_seen = pending.is_file();
            let response = match (method.as_str(), path.as_str()) {
                ("GET", "/repos/vibevm/vibevm/releases/latest") => (
                    "200 OK",
                    "{\"id\":101,\"tag_name\":\"v1.0.1\",\"draft\":false,\"prerelease\":false}"
                        .to_owned(),
                ),
                ("GET", "/repos/vibevm/vibevm/git/ref/tags/v1.1.0") if posted => (
                    "200 OK",
                    format!(
                        "{{\"ref\":\"refs/tags/v1.1.0\",\"object\":{{\"type\":\"commit\",\"sha\":\"{commit}\"}}}}"
                    ),
                ),
                ("GET", "/repos/vibevm/vibevm/git/ref/tags/v1.1.0") => {
                    ("404 Not Found", "{}".into())
                }
                ("GET", "/repos/vibevm/vibevm/releases/tags/v1.1.0") => {
                    ("404 Not Found", "{}".into())
                }
                ("GET", "/repos/vibevm/vibevm/releases?per_page=100") => ("200 OK", "[]".into()),
                ("POST", "/repos/vibevm/vibevm/git/refs") => {
                    assert_eq!(
                        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                        serde_json::json!({"ref":"refs/tags/v1.1.0","sha":commit})
                    );
                    posted = true;
                    (
                        "201 Created",
                        format!(
                            "{{\"ref\":\"refs/tags/v1.1.0\",\"object\":{{\"type\":\"commit\",\"sha\":\"{commit}\"}}}}"
                        ),
                    )
                }
                _ => panic!("unexpected loopback request {method} {path}"),
            };
            rows.push(HttpSeen {
                method: method.clone(),
                path,
                pending: pending_seen,
            });
            if method == "POST" && lost {
                continue;
            } // committed remote state, lost reply
            let bytes = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                response.0,
                response.1.len(),
                response.1
            );
            socket.write_all(bytes.as_bytes()).unwrap();
        }
        rows
    });
    (base, handle)
}

fn loopback_prefix_case(lost: bool) {
    use crate::{Token, github_release::GithubReleaseClient};
    use std::time::Duration;
    let mut tiny = TinyPayloadWorld::new();
    let captured = crate::publication_full_plan::remote_observation::normalize_fixed(
        crate::publication_full_plan::remote_observation::RemoteSnapshot {
            latest: crate::github_release::LatestReleaseObservation::Present {
                release_id: 101,
                tag: "v1.0.1".into(),
            },
            tag: None,
            release: None,
            assets: None,
            streamed_digests: Vec::new(),
        },
        &tiny.subject.source.commit,
    )
    .unwrap();
    tiny.plan = crate::publication_effect_plan::plan_from_captured_remote(
        &tiny.subject,
        &expected(),
        &captured,
    )
    .unwrap()
    .plan;
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    tiny.seed_approval(&lease, &scope);
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let expected = expected();
    let output = Project::open(tiny.output.path()).unwrap();
    prepare::prepare_local_full_plan(
        &lease,
        &namespace,
        &scope,
        "loopback-prefix",
        &tiny.subject,
        &tiny.plan,
        &expected,
        prepare::trusted(
            tiny.settings.path(),
            &lease,
            &scope,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let home = tiny.approval_home();
    let SessionOpen::Ready(session) = ReadyRun::open(
        &lease,
        &namespace,
        &scope,
        "loopback-prefix",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap() else {
        panic!()
    };
    let pending = root.path().join(format!(
        ".vibe/publication-operations/{}.full-plan-api-000-pending.json",
        session.run_id
    ));
    let (base, server) = serve_tag(tiny.subject.source.commit.clone(), pending.clone(), lost);
    let client = GithubReleaseClient::with_endpoints_and_timeouts(
        Token::from_explicit("fixture-owner-secret"),
        "vibevm",
        "vibevm",
        &base,
        &base,
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .unwrap();
    let result = owner::drive_one_loopback(&session, &client).unwrap();
    let seen = server.join().unwrap();
    if lost {
        assert!(matches!(
            result,
            OwnerStep::Uncertain {
                at: CallRef {
                    phase: Phase::TagRelease,
                    ordinal: 0
                },
                ..
            }
        ));
        assert!(matches!(
            owner::drive_one_loopback(&session, &client).unwrap(),
            OwnerStep::Uncertain { .. }
        ));
    } else {
        assert!(
            matches!(
                result,
                OwnerStep::Advanced {
                    attempted: CallRef {
                        phase: Phase::TagRelease,
                        ordinal: 0
                    },
                    next: CallRef {
                        phase: Phase::TagRelease,
                        ordinal: 1
                    },
                    ..
                }
            ),
            "unexpected owner result: {result:?}; HTTP: {seen:?}"
        );
    }
    let run_id = session.run_id.clone();
    drop(session);
    assert_eq!(seen.len(), if lost { 9 } else { 17 });
    assert_eq!(seen.iter().filter(|row| row.method == "POST").count(), 1);
    assert!(
        seen.iter()
            .all(|row| row.method == "POST" || row.method == "GET")
    );
    assert_eq!(seen[0].path, "/repos/vibevm/vibevm/releases/latest");
    assert_eq!(
        seen.iter()
            .filter(|row| row.path.ends_with("/releases/latest"))
            .count(),
        if lost { 2 } else { 4 }
    );
    assert_eq!(
        seen.iter()
            .filter(|row| row.path.ends_with("/git/ref/tags/v1.1.0"))
            .count(),
        if lost { 2 } else { 4 }
    );
    assert_eq!(seen[8].method, "POST");
    assert!(seen[8].pending, "HTTP write preceded durable Pending");
    assert!(pending.is_file());
    let landed = root.path().join(format!(
        ".vibe/publication-operations/{run_id}.full-plan-api-000-landed.json"
    ));
    assert_eq!(landed.is_file(), !lost);
    drop(lease);
    let reopened = LifecycleLease::acquire(root.path()).unwrap();
    let cold = ReadyRun::open(
        &reopened,
        &namespace,
        &scope,
        "loopback-prefix",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap();
    if lost {
        assert!(matches!(cold, SessionOpen::Uncertain { .. }));
    } else {
        assert!(matches!(cold, SessionOpen::Ready(_)));
    }
}

#[test]
fn fixed_owner_loopback_one_tag_post_lands_after_independent_reads() {
    loopback_prefix_case(false);
}

#[test]
fn fixed_owner_loopback_lost_tag_reply_keeps_pending_without_repost() {
    loopback_prefix_case(true);
}
