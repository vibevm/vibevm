//! Integration: the D-C3-6 escalate channel — a running run hands its task
//! UP the tree; the run ends `escalated`, the record carries reason/needs,
//! and escalation.md lands on the plane (I2). A run that has not engaged
//! the task (queued) cannot escalate.

use camino::Utf8PathBuf;
use fractality_core::api::{PodEvent, PodEventRequest, PodRegisterRequest, RegisterRunRequest};
use fractality_core::ids::PodId;
use fractality_core::packet::Packet;
use fractality_core::run::RunState;
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
            [profile.test]
            backend = "claude-code"
            base_url = "http://localhost:9"
            token_file = "unused.token"
            [profile.test.models]
            big = "m-big"
            small = "m-small"
            haiku_slot = "m-small"
        "#,
    )
    .expect("profiles written");
    home
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

/// Registers a run and walks it to `running` over the pod leg (the honest
/// bus path: register → pod adopt → starting → running).
async fn running_run(client: &McClient) -> fractality_core::run::RunRecord {
    let run = client
        .register_run(&RegisterRunRequest {
            packet: packet("silo-task"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers");
    let pod_id = PodId::generate();
    client
        .pod_register(&PodRegisterRequest {
            pod_id,
            pod_pid: 4242,
            run_id: run.run_id,
        })
        .await
        .expect("pod adopts");
    for state in [RunState::Starting, RunState::Running] {
        client
            .pod_event(
                pod_id,
                &PodEventRequest {
                    run_id: run.run_id,
                    event: PodEvent::State {
                        state,
                        detail: None,
                    },
                },
            )
            .await
            .expect("state advances");
    }
    client.run(run.run_id).await.expect("fetch")
}

#[tokio::test]
async fn a_running_run_escalates_to_the_top_with_reason_and_needs() {
    let home = scratch_home("escalate-hit");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let run = running_run(&client).await;
    let escalated = client
        .escalate(
            run.run_id,
            "cross-chunk reasoning — any split loses the answer",
            "route to the largest-window profile",
        )
        .await
        .expect("escalate accepted");
    assert_eq!(escalated.state, RunState::Escalated);
    let esc = escalated
        .escalation
        .as_ref()
        .expect("escalation record set");
    assert_eq!(
        esc.reason,
        "cross-chunk reasoning — any split loses the answer"
    );
    assert_eq!(esc.needs, "route to the largest-window profile");
    // The plane carries the fact too (I2: the bus delivers, the file records).
    let plane = escalated.run_dir.join("escalation.md");
    let body = std::fs::read_to_string(plane.as_std_path()).expect("escalation.md written");
    assert!(
        body.contains("cross-chunk reasoning"),
        "reason on the plane"
    );
    assert!(body.contains("largest-window"), "needs on the plane");

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn escalate_on_a_queued_run_is_refused() {
    let home = scratch_home("escalate-miss");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    // A freshly-registered run is `queued` — not a live state, so it cannot
    // escalate: nothing has engaged the task yet (that is a gate-time
    // route/escalate verdict, not a run outcome).
    let run = client
        .register_run(&RegisterRunRequest {
            packet: packet("premature"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers");
    let err = client
        .escalate(run.run_id, "cannot do it here", "a bigger window")
        .await
        .expect_err("a queued run cannot escalate");
    assert!(
        matches!(err, ClientError::Api { status: 409, .. }),
        "expected 409 conflict, got {err:?}"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
