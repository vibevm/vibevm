//! Loopback integration: a real pod binary supervising a stub child
//! against an embedded mission-control — the Phase 1 exit criteria.
//!
//! Windows-only on purpose: the stub children are `cmd.exe` one-liners
//! and this campaign validates Windows (plan §10). `ping -n K` sleeps
//! K-1 seconds — the canonical console-free wait.

#![cfg(windows)]

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use fractality_core::api::RegisterRunRequest;
use fractality_core::packet::Packet;
use fractality_core::run::{KillReason, RunRecord, RunState};
use fractality_core::worker::WorkerSpec;
use fractality_mc_client::McClient;
use fractality_mission_control::{Config, start};

fn scratch_home(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-pod-{tag}-{}", ulid::Ulid::new()));
    Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
}

fn stub_packet(title: &str) -> Packet {
    Packet::from_toml_str(&format!(
        r#"
            schema = 1
            [task]
            title = "{title}"
            goal = "loopback stub"
            [workspace]
            mode = "dir"
            [routing]
            profile = "test"
        "#
    ))
    .expect("fixture packet parses")
}

/// A stub worker: prints, then sleeps ~`sleep_secs`, then exits 0.
fn stub_spec(run_dir: &Utf8Path, sleep_secs: u32) -> WorkerSpec {
    let mut env = BTreeMap::new();
    for name in ["PATH", "SystemRoot", "COMSPEC", "TEMP", "TMP"] {
        if let Ok(v) = std::env::var(name) {
            env.insert(name.to_owned(), v);
        }
    }
    WorkerSpec {
        argv: vec![
            "cmd".to_owned(),
            "/C".to_owned(),
            format!(
                "echo hello-from-stub & ping -n {} 127.0.0.1 >nul",
                sleep_secs + 1
            ),
        ],
        env,
        cwd: run_dir.to_owned(),
        stdin: None,
    }
}

async fn register_and_launch_pod(
    client: &McClient,
    home: &Utf8Path,
    title: &str,
    sleep_secs: u32,
) -> (RunRecord, std::process::Child) {
    let run = client
        .register_run(&RegisterRunRequest {
            packet: stub_packet(title),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("run registers");
    let spec = stub_spec(&run.run_dir, sleep_secs);
    let spec_path = run.run_dir.join("worker-spec.toml");
    std::fs::write(
        spec_path.as_std_path(),
        spec.to_toml_string().expect("spec renders"),
    )
    .expect("spec written");

    // The pod's own diagnostics land in the run dir (mirrors the Phase 2
    // pod.log design) — a dead-silent pod is undebuggable.
    let pod_log = std::fs::File::create(run.run_dir.join("pod-stderr.log").as_std_path())
        .expect("pod log file");
    let pod = std::process::Command::new(env!("CARGO_BIN_EXE_fractality-pod"))
        .args([
            "--home",
            home.as_str(),
            "--run-id",
            &run.run_id.to_string(),
            "--run-dir",
            run.run_dir.as_str(),
            "--spec",
            spec_path.as_str(),
        ])
        .env("FRACTALITY_LOG", "debug")
        .stdout(std::process::Stdio::null())
        .stderr(pod_log)
        .spawn()
        .expect("pod binary spawns");
    (run, pod)
}

async fn wait_for_state(
    home: &Utf8Path,
    run_id: fractality_core::ids::RunId,
    want: RunState,
    within: std::time::Duration,
) -> RunRecord {
    let deadline = std::time::Instant::now() + within;
    loop {
        if let Ok(Some(client)) = McClient::connect(home).await
            && let Ok(run) = client.run(run_id).await
        {
            if run.state == want {
                return run;
            }
            assert!(
                !run.state.is_terminal(),
                "run went terminal ({}) while waiting for {want}: {run:?}",
                run.state
            );
        }
        assert!(
            std::time::Instant::now() < deadline,
            "run {run_id} did not reach {want} within {within:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn stub_run_completes_with_transcript_and_status() {
    let home = scratch_home("happy");
    let server = start(Config::new(home.clone())).await.expect("mc starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    let (run, mut pod) = register_and_launch_pod(&client, &home, "happy", 1).await;
    let done = wait_for_state(
        &home,
        run.run_id,
        RunState::Completed,
        std::time::Duration::from_secs(20),
    )
    .await;

    assert_eq!(done.exit_code, Some(0));
    assert!(done.worker_pid.is_some(), "spawned event recorded the pid");
    assert!(done.pod.is_some(), "pod binding recorded");

    let stdout = std::fs::read_to_string(run.run_dir.join("worker-stdout.jsonl").as_std_path())
        .expect("transcript exists");
    assert!(
        stdout.contains("hello-from-stub"),
        "stdout streamed to disk"
    );
    assert!(
        run.run_dir.join("status.json").as_std_path().is_file(),
        "status.json persisted (D4)"
    );

    let pod_exit = pod.wait().expect("pod exits");
    assert!(pod_exit.success(), "pod exits 0 after clean supervision");

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// Spawns a REAL daemon process on `home`. In-process embedding cannot
/// emulate a crash: aborting the serve future drops the listener but
/// hyper's per-connection tasks keep answering pooled keep-alive
/// connections — a "dead" daemon that still talks (exactly the artifact
/// that once let a pod deliver its exit report to a killed generation).
/// A killed *process* severs every connection, like a real crash.
fn spawn_mc(home: &Utf8Path, log_name: &str) -> std::process::Child {
    let mc_bin = std::path::Path::new(env!("CARGO_BIN_EXE_fractality-pod"))
        .parent()
        .expect("bin dir")
        .join("fractality-mission-control.exe");
    let log = std::fs::File::create(home.join(log_name).as_std_path()).expect("mc log");
    std::process::Command::new(mc_bin)
        .env("FRACTALITY_HOME", home.as_str())
        .env("FRACTALITY_LOG", "info")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(log)
        .spawn()
        .expect("mission-control binary spawns (built by `cargo test --workspace`)")
}

async fn wait_mc_live(home: &Utf8Path) -> McClient {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Ok(Some(client)) = McClient::connect(home).await {
            return client;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "daemon did not come up within 10s"
        );
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
}

/// The P8 early signal: crash the daemon process mid-run, start a new
/// generation on the same home — the pod keeps supervising, re-registers,
/// and the run completes with zero manual repair.
#[tokio::test]
async fn run_survives_a_daemon_kill_and_restart() {
    let home = scratch_home("restart");
    std::fs::create_dir_all(home.as_std_path()).expect("mkdir home");

    let mut first = spawn_mc(&home, "mc-gen1.log");
    let client = wait_mc_live(&home).await;

    let (run, mut pod) = register_and_launch_pod(&client, &home, "survivor", 6).await;
    wait_for_state(
        &home,
        run.run_id,
        RunState::Running,
        std::time::Duration::from_secs(15),
    )
    .await;

    // The crash: kill the process. Every connection dies with it; the
    // stale lockfile stays on disk.
    first.kill().expect("gen-1 killed");
    let _ = first.wait();

    let mut second = spawn_mc(&home, "mc-gen2.log");
    let done = wait_for_state(
        &home,
        run.run_id,
        RunState::Completed,
        std::time::Duration::from_secs(30),
    )
    .await;
    assert_eq!(done.exit_code, Some(0));

    let pod_exit = pod.wait().expect("pod exits");
    assert!(
        pod_exit.success(),
        "pod delivered its exit report to the new generation"
    );

    // Teardown: graceful shutdown of gen 2.
    let client2 = wait_mc_live(&home).await;
    let _ = client2.shutdown().await;
    let _ = second.wait();
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// The F5 guarantee, pinned as a regression test: killing the pod takes
/// the worker down with it (KILL_ON_JOB_CLOSE), and the daemon reaps the
/// run as killed(pod_lost).
#[tokio::test]
async fn killing_the_pod_reaps_the_worker_and_the_run() {
    let home = scratch_home("orphan");
    let server = start(Config::new(home.clone())).await.expect("mc starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");

    let (run, mut pod) = register_and_launch_pod(&client, &home, "orphan", 30).await;
    let running = wait_for_state(
        &home,
        run.run_id,
        RunState::Running,
        std::time::Duration::from_secs(15),
    )
    .await;
    let worker_pid = running.worker_pid.expect("worker pid known");

    pod.kill().expect("pod killed");
    let _ = pod.wait();

    // The job object must reap the worker within moments of the pod dying.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let mut system = sysinfo::System::new();
        let target = sysinfo::Pid::from_u32(worker_pid);
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
        if system.process(target).is_none() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "worker {worker_pid} still alive 5s after its pod died — F5 regressed"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // And the reaper marks the run killed(pod_lost) once the heartbeat
    // goes stale (15s) and the pid probe fails: budget ≈ stale + sweep.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
    let final_run = loop {
        let current = client.run(run.run_id).await.expect("run readable");
        if current.state.is_terminal() {
            break current;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "run not reaped 40s after pod loss"
        );
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    };
    assert_eq!(final_run.state, RunState::Killed);
    assert_eq!(final_run.kill_reason, Some(KillReason::PodLost));

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
