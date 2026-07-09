//! Integration: the real `fractality` binary against an embedded
//! daemon — verbs, output shape, and the D17 exit-code contract.

use camino::Utf8PathBuf;
use fractality_core::api::RegisterRunRequest;
use fractality_core::packet::Packet;
use fractality_mc_client::McClient;
use fractality_mission_control::{Config, start};

fn scratch_home(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-cli-{tag}-{}", ulid::Ulid::new()));
    Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
}

fn cli(home: &camino::Utf8Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_fractality"))
        .arg("--home")
        .arg(home.as_str())
        .args(args)
        .output()
        .expect("fractality binary runs")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test]
async fn the_read_verbs_speak_d17() {
    let home = scratch_home("verbs");
    let server = start(Config::new(home.clone())).await.expect("mc starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    // status: running -> exit 0, one line.
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["mc", "status"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout(&out).starts_with("running pid="),
        "got: {}",
        stdout(&out)
    );

    // ps on an empty registry: header only.
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["ps"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.starts_with("RUN_ID"), "header first: {text}");
    assert_eq!(text.lines().count(), 1, "no runs, no rows");

    // Register one run over the bus, then read it back through the CLI.
    let run = client
        .register_run(&RegisterRunRequest {
            packet: Packet::from_toml_str(
                r#"
                    schema = 1
                    [task]
                    title = "cli-fixture"
                    goal = "cli test"
                    [workspace]
                    mode = "dir"
                    [routing]
                    profile = "test"
                "#,
            )
            .expect("packet parses"),
            parent: None,
            spawn: false,
        })
        .await
        .expect("registers");

    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["ps", "-q"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out).trim(), run.run_id.to_string(), "-q: ids only");

    // show accepts a unique prefix and renders key: value lines.
    let prefix = run.run_id.to_string()[..8].to_owned();
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        let prefix = prefix.clone();
        move || cli(&home, &["show", &prefix])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("title:      cli-fixture"), "{text}");
    assert!(text.contains("state:      queued"), "{text}");

    // show --json is machine-readable and round-trips the record.
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        let id = run.run_id.to_string();
        move || cli(&home, &["show", &id, "--json"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    let parsed: fractality_core::run::RunRecord =
        serde_json::from_str(stdout(&out).trim()).expect("json parses");
    assert_eq!(parsed.run_id, run.run_id);

    // Negatives: unknown id -> 1; bogus state -> 1.
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["show", "01ZZZZZZZZZZZZZZZZZZZZZZZZ"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        out.status.code(),
        Some(1),
        "unknown run is a truthful negative"
    );

    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["ps", "--state", "bogus"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(1));

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn mc_stop_drains_the_daemon_and_status_reports_stopped() {
    let home = scratch_home("stop");
    let server = start(Config::new(home.clone())).await.expect("mc starts");

    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["mc", "stop"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("mc stopped"), "{}", stdout(&out));

    // The serve loop has drained; join the embedded server and clean up.
    server.stop().await;

    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["mc", "status"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(1), "stopped is exit 1 for status");
    assert_eq!(stdout(&out).trim(), "stopped");

    // stop again: idempotent, exit 0.
    let out = tokio::task::spawn_blocking({
        let home = home.clone();
        move || cli(&home, &["mc", "stop"])
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("not running"));

    std::fs::remove_dir_all(home.as_std_path()).ok();
}
