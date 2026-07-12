//! Integration: the PP-003 / D-C3-7 advisor bar (RD-10). An ADVICE call
//! (`output.advice`) is admitted only when its CALLER (parent run) clears
//! the `advisor_enabled` capability bar — `caller_class >= medium`. A weak
//! caller is refused 400; a medium/strong caller and a boss (no parent)
//! are admitted. The advice run itself transfers no ownership.

use camino::Utf8PathBuf;
use fractality_core::api::RegisterRunRequest;
use fractality_core::ids::RunId;
use fractality_core::packet::Packet;
use fractality_core::run::{RunRecord, RunState};
use fractality_mc_client::{ClientError, McClient};
use fractality_mission_control::{Config, start};

fn scratch_home(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-mc-{tag}-{}", ulid::Ulid::new()));
    let home = Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir");
    std::fs::create_dir_all(home.as_std_path()).expect("home dir");
    std::fs::write(
        home.join("profiles.toml").as_std_path(),
        r#"
            schema = 1
            [profile.weakp]
            backend = "claude-code"
            base_url = "http://localhost:9"
            token_file = "unused.token"
            capability_class = "weak"
            [profile.weakp.models]
            big = "m-big"
            small = "m-small"
            haiku_slot = "m-small"
            [profile.strongp]
            backend = "claude-code"
            base_url = "http://localhost:9"
            token_file = "unused.token"
            capability_class = "strong"
            [profile.strongp.models]
            big = "m-big"
            small = "m-small"
            haiku_slot = "m-small"
        "#,
    )
    .expect("profiles written");
    home
}

/// A run under `profile` — the CALLER whose class the bar reads.
fn caller_packet(profile: &str) -> Packet {
    Packet::from_toml_str(&format!(
        r#"
            schema = 1
            [task]
            title = "caller"
            goal = "owns a task, may consult an advisor"
            [workspace]
            mode = "dir"
            [routing]
            profile = "{profile}"
        "#
    ))
    .expect("caller packet parses")
}

/// An advice call — worker-shaped, `output.advice = true`, no ownership.
fn advice_packet() -> Packet {
    Packet::from_toml_str(
        r#"
            schema = 1
            [task]
            title = "advice"
            goal = "a bounded judgment for the caller"
            [output]
            advice = true
            [workspace]
            mode = "dir"
            [routing]
            profile = "strongp"
        "#,
    )
    .expect("advice packet parses")
}

async fn register(
    client: &McClient,
    packet: Packet,
    parent: Option<RunId>,
) -> Result<RunRecord, ClientError> {
    client
        .register_run(&RegisterRunRequest {
            packet,
            parent,
            origin_session: None,
            spawn: false,
        })
        .await
}

#[tokio::test]
async fn a_weak_caller_may_not_consult_an_advisor() {
    let home = scratch_home("advisor-weak");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    // A weak-class caller run, then an advice call attributed to it.
    let caller = register(&client, caller_packet("weakp"), None)
        .await
        .expect("weak caller registers");
    let err = register(&client, advice_packet(), Some(caller.run_id))
        .await
        .expect_err("a weak caller's advice call is refused");
    assert!(
        matches!(err, ClientError::Api { status: 400, .. }),
        "expected 400, got {err:?}"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn a_strong_caller_and_a_boss_may_consult_an_advisor() {
    let home = scratch_home("advisor-ok");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    // A strong caller clears the bar.
    let caller = register(&client, caller_packet("strongp"), None)
        .await
        .expect("strong caller registers");
    let advised = register(&client, advice_packet(), Some(caller.run_id))
        .await
        .expect("a strong caller's advice call is admitted");
    assert_eq!(advised.state, RunState::Queued);
    assert!(advised.advice, "the run is marked an advice call");

    // A boss-spawned advice call (no parent) is the human at the top —
    // always above the bar.
    let boss_advice = register(&client, advice_packet(), None)
        .await
        .expect("a boss advice call is admitted");
    assert!(boss_advice.advice);

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
