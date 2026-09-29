//! Integration: the Ф5 cold-verifier suppression (FD-9 / §10.2). An
//! acceptance/verifier packet (`output.verifier`) is admitted only when its
//! `context.context_from` names real work — a run that produced a result.
//! A verifier over an empty or resultless tree is refused 400 at the door.

use camino::Utf8PathBuf;
use fractality_core::api::{PodEvent, PodEventRequest, PodRegisterRequest, RegisterRunRequest};
use fractality_core::ids::{PodId, RunId};
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

/// A plain work packet.
fn work_packet() -> Packet {
    Packet::from_toml_str(
        r#"
            schema = 1
            [task]
            title = "work"
            goal = "produce a result"
            [workspace]
            mode = "dir"
            [routing]
            profile = "test"
        "#,
    )
    .expect("work packet parses")
}

/// A verifier packet whose `context_from` names the given work runs.
fn verifier_packet(context_from: &[RunId]) -> Packet {
    let refs = context_from
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Packet::from_toml_str(&format!(
        r#"
            schema = 1
            [task]
            title = "verify"
            goal = "check the work"
            acceptance = ["exit 0"]
            [context]
            context_from = [{refs}]
            [output]
            verifier = true
            [workspace]
            mode = "dir"
            [routing]
            profile = "test"
        "#
    ))
    .expect("verifier packet parses")
}

/// Registers a run and drives it (over the pod leg) to `completed` with a
/// worker-sourced result — i.e. real work under review.
async fn completed_work_with_result(client: &McClient) -> RunId {
    let run = client
        .register_run(&RegisterRunRequest {
            packet: work_packet(),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers work");
    let pod_id = PodId::generate();
    client
        .pod_register(&PodRegisterRequest {
            pod_id,
            pod_pid: 4242,
            run_id: run.run_id,
        })
        .await
        .expect("pod adopts");
    client
        .pod_event(
            pod_id,
            &PodEventRequest {
                run_id: run.run_id,
                event: PodEvent::State {
                    state: RunState::Running,
                    detail: None,
                },
            },
        )
        .await
        .expect("running");
    client
        .pod_event(
            pod_id,
            &PodEventRequest {
                run_id: run.run_id,
                event: PodEvent::Collected {
                    result_source: "worker".to_owned(),
                    result_path: None,
                    acceptance_passed: 0,
                    acceptance_total: 0,
                    acceptance_skipped: None,
                },
            },
        )
        .await
        .expect("collected");
    client
        .pod_event(
            pod_id,
            &PodEventRequest {
                run_id: run.run_id,
                event: PodEvent::Exit { exit_code: Some(0) },
            },
        )
        .await
        .expect("exit");
    run.run_id
}

async fn register(
    client: &McClient,
    packet: Packet,
) -> Result<fractality_core::run::RunRecord, ClientError> {
    client
        .register_run(&RegisterRunRequest {
            packet,
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
}

#[tokio::test]
async fn a_verifier_over_real_work_is_admitted() {
    let home = scratch_home("verifier-warm");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    let work = completed_work_with_result(&client).await;
    let rec = register(&client, verifier_packet(&[work]))
        .await
        .expect("a verifier over real work is admitted");
    assert_eq!(rec.state, RunState::Queued);
    assert!(
        rec.verifier,
        "the run is marked a verifier (denormalized from output.verifier)"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn a_cold_verifier_is_refused() {
    let home = scratch_home("verifier-cold");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    // Empty tree: no context_from at all.
    let err = register(&client, verifier_packet(&[]))
        .await
        .expect_err("a verifier over nothing is refused");
    assert!(
        matches!(err, ClientError::Api { status: 400, .. }),
        "expected 400, got {err:?}"
    );

    // Resultless tree: context_from names a run that never produced work.
    let phantom: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid");
    let err = register(&client, verifier_packet(&[phantom]))
        .await
        .expect_err("a verifier over a resultless run is refused");
    assert!(
        matches!(err, ClientError::Api { status: 400, .. }),
        "expected 400, got {err:?}"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
