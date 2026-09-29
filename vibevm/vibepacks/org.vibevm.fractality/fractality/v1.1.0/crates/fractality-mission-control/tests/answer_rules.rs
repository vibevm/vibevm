//! Integration: the Ф5 auto-answer slice — a profile rule answers a
//! matching question without parking the boss (both facts journaled);
//! a non-matching question parks exactly as before.

use camino::Utf8PathBuf;
use fractality_core::api::{PodEvent, PodEventRequest, PodRegisterRequest, RegisterRunRequest};
use fractality_core::ids::PodId;
use fractality_core::packet::Packet;
use fractality_core::run::RunState;
use fractality_mc_client::McClient;
use fractality_mission_control::{Config, start};

fn scratch_home(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-mc-{tag}-{}", ulid::Ulid::new()));
    Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
}

/// A home whose profiles.toml carries one auto-answer rule for the
/// `test` profile the packet fixture routes to.
fn home_with_rule(tag: &str) -> Utf8PathBuf {
    let home = scratch_home(tag);
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
            [[profile.test.permissions.answer_rules]]
            name = "push-allowed-on-work-branches"
            contains = "may i push"
            answer = "Yes — push to your fractality/* work branch; never to main."
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

/// Registers a run and walks it to `running` over the pod leg (the
/// honest bus path: register → pod adopt → starting → running).
async fn running_run(client: &McClient) -> fractality_core::run::RunRecord {
    let run = client
        .register_run(&RegisterRunRequest {
            packet: packet("ask"),
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
async fn a_matching_question_is_answered_without_parking() {
    let home = home_with_rule("rule-hit");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let run = running_run(&client).await;
    let answered = client
        .question(run.run_id, "May I push the branch now?")
        .await
        .expect("question accepted");
    assert_eq!(
        answered.state,
        RunState::Running,
        "the rule answers in the same breath — no waiting_on_boss window for the boss"
    );
    assert_eq!(
        answered.answer.as_deref(),
        Some("Yes — push to your fractality/* work branch; never to main."),
        "the broker's first poll sees the rule's reply"
    );
    assert!(answered.question.is_none(), "the question is consumed");
    assert!(
        run.run_dir.join("answer.md").as_std_path().is_file(),
        "the plane records the answer (I2)"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn a_non_matching_question_parks_for_the_boss() {
    let home = home_with_rule("rule-miss");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let run = running_run(&client).await;
    let parked = client
        .question(run.run_id, "Which module should own the parser?")
        .await
        .expect("question accepted");
    assert_eq!(
        parked.state,
        RunState::WaitingOnBoss,
        "no rule matches — the escalation parks exactly as before Ф5"
    );
    assert_eq!(
        parked.question.as_deref(),
        Some("Which module should own the parser?")
    );
    assert!(parked.answer.is_none());

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
