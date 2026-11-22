//! Integration: the boss-session cell end to end — begin idempotence,
//! note folding, delegation slate-cleaning, session metrics, and
//! survival across a daemon restart.

use camino::Utf8PathBuf;
use fractality_core::api::{RegisterRunRequest, SessionBeginRequest, SessionBeginResponse};
use fractality_core::ids::SessionId;
use fractality_core::packet::Packet;
use fractality_core::session::SessionNote;
use fractality_mc_client::{ClientError, McClient};
use fractality_mission_control::{Config, start};

fn scratch_home(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-mc-{tag}-{}", ulid::Ulid::new()));
    Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
}

fn packet(title: &str) -> Packet {
    Packet::from_toml_str(&format!(
        r#"
            schema = 1
            [task]
            title = "{title}"
            goal = "integration fixture"
            [workspace]
            mode = "dir"
            [routing]
            profile = "test"
        "#
    ))
    .expect("fixture packet parses")
}

async fn begin(
    client: &McClient,
    home: &camino::Utf8Path,
    external_id: &str,
) -> SessionBeginResponse {
    client
        .session_begin(&SessionBeginRequest {
            harness: "claude-code".into(),
            external_id: external_id.into(),
            cwd: home.to_owned(),
        })
        .await
        .expect("begin")
}

#[tokio::test]
async fn begin_is_idempotent_while_open_and_mints_anew_after_end() {
    let home = scratch_home("begin");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let first = begin(&client, &home, "cc-1").await;
    assert!(!first.resumed, "fresh begin mints, not resumes");
    let second = begin(&client, &home, "cc-1").await;
    assert!(
        second.resumed,
        "same (harness, external_id) resumes while open"
    );
    let id = first.session.session_id;
    assert_eq!(
        second.session.session_id, id,
        "open resume keeps one record"
    );

    client.session_end(id).await.expect("end");
    let open = client.sessions(true).await.expect("open list");
    assert!(
        open.iter().all(|s| s.session_id != id),
        "ended sessions are excluded from the open-only listing"
    );

    let third = begin(&client, &home, "cc-1").await;
    assert!(!third.resumed, "a closed pair mints anew");
    assert_ne!(
        third.session.session_id, id,
        "after end the same pair gets a different session id"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn work_tool_notes_fold_and_a_delegation_cleans_the_slate() {
    let home = scratch_home("fold");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let id = begin(&client, &home, "cc-fold").await.session.session_id;
    let work = SessionNote::WorkTool {
        tool: "Bash".into(),
        duration_ms: 100,
    };
    client.session_note(id, work.clone()).await.expect("note 1");
    client.session_note(id, work).await.expect("note 2");

    let rec = client.session(id).await.expect("fetch");
    assert_eq!(rec.counters.work_tools_since_delegation, 2);
    assert_eq!(rec.counters.work_tools_total, 2);
    assert_eq!(rec.counters.work_tool_ms_total, 200);

    client
        .register_run(&RegisterRunRequest {
            packet: packet("delegated"),
            parent: None,
            origin_session: Some(id),
            spawn: false,
        })
        .await
        .expect("registers");

    let rec = client.session(id).await.expect("fetch after delegation");
    assert_eq!(
        rec.counters.delegations, 1,
        "the run's origin stamps a delegation"
    );
    assert_eq!(
        rec.counters.work_tools_since_delegation, 0,
        "BD1: delegating cleans the slate"
    );
    assert_eq!(
        rec.counters.work_tools_total, 2,
        "history survives the slate reset"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn session_metrics_buckets_only_the_sessions_runs() {
    let home = scratch_home("metrics");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let a = begin(&client, &home, "cc-a").await.session.session_id;
    let b = begin(&client, &home, "cc-b").await.session.session_id;

    client
        .register_run(&RegisterRunRequest {
            packet: packet("run-a"),
            parent: None,
            origin_session: Some(a),
            spawn: false,
        })
        .await
        .expect("registers a");
    client
        .register_run(&RegisterRunRequest {
            packet: packet("run-b"),
            parent: None,
            origin_session: Some(b),
            spawn: false,
        })
        .await
        .expect("registers b");
    client
        .register_run(&RegisterRunRequest {
            packet: packet("run-none"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers unattributed");

    let metrics = client.session_metrics(a).await.expect("metrics for a");
    assert_eq!(metrics.runs.runs, 1, "only a's attributed run is bucketed");
    assert!(metrics.parked.is_empty(), "no runs parked on a question");

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn notes_on_unknown_sessions_answer_404() {
    let home = scratch_home("note404");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let unknown = SessionId::generate();
    let err = client
        .session_note(
            unknown,
            SessionNote::WorkTool {
                tool: "Bash".into(),
                duration_ms: 0,
            },
        )
        .await
        .expect_err("unknown session is rejected");
    match err {
        ClientError::Api { status, .. } => assert_eq!(status, 404, "unknown session is a 404"),
        other => panic!("expected ClientError::Api 404, got {other:?}"),
    }

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn sessions_survive_a_daemon_restart() {
    let home = scratch_home("restart");

    let first = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");
    let id = begin(&client, &home, "cc-restart").await.session.session_id;
    client
        .session_note(
            id,
            SessionNote::WorkTool {
                tool: "Bash".into(),
                duration_ms: 50,
            },
        )
        .await
        .expect("note");
    // Graceful stop drops the lockfile but leaves the journal on disk.
    first.stop().await;

    let second = start(Config::new(home.clone()))
        .await
        .expect("restarts over a clean lockfile");
    let client2 = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live again");

    let rec = client2
        .session(id)
        .await
        .expect("session replayed from journal");
    assert_eq!(
        rec.counters.work_tools_total, 1,
        "the folded counter survives"
    );
    assert!(
        rec.is_open(),
        "the session is still open across the restart"
    );

    second.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn session_end_is_idempotent() {
    let home = scratch_home("end");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let id = begin(&client, &home, "cc-end").await.session.session_id;
    client.session_end(id).await.expect("first end");
    client
        .session_end(id)
        .await
        .expect("second end (idempotent)");

    let rec = client.session(id).await.expect("fetch");
    assert!(rec.ended_ts_ms.is_some(), "the session is ended");
    assert!(!rec.is_open(), "an echoed end keeps the record closed");

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn run_with_an_unknown_origin_session_still_registers() {
    let home = scratch_home("unknown-origin");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let unknown = SessionId::generate();
    let run = client
        .register_run(&RegisterRunRequest {
            packet: packet("best-effort"),
            parent: None,
            origin_session: Some(unknown),
            spawn: false,
        })
        .await
        .expect("registers despite an unknown origin session");
    assert_eq!(
        run.origin_session,
        Some(unknown),
        "the best-effort label is stamped on the run"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
