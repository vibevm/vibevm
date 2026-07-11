//! Integration: the daemon end to end — bus auth, run registration and
//! reads, journal persistence across a restart, stale-lock handling.

use camino::Utf8PathBuf;
use fractality_core::api::RegisterRunRequest;
use fractality_core::packet::Packet;
use fractality_core::run::RunState;
use fractality_mc_client::McClient;
use fractality_mc_client::lock::Lockfile;
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

#[tokio::test]
async fn bus_requires_the_bearer() {
    let home = scratch_home("auth");
    let server = start(Config::new(home.clone())).await.expect("starts");

    let lock = Lockfile::read(&home)
        .expect("lock reads")
        .expect("lock exists");
    let url = format!("http://127.0.0.1:{}/v0/health", lock.port);
    let resp = reqwest::get(&url).await.expect("request goes through");
    assert_eq!(resp.status().as_u16(), 401, "no bearer, no answer");

    let authed = McClient::from_lockfile(&lock).expect("client builds");
    let health = authed.health().await.expect("bearer opens the bus");
    assert_eq!(health.status, "ok");
    assert_eq!(health.pid, std::process::id());

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn register_read_and_tree_work_over_the_bus() {
    let home = scratch_home("crud");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    let parent = client
        .register_run(&RegisterRunRequest {
            packet: packet("parent"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers");
    assert_eq!(parent.state, RunState::Queued);
    assert!(
        parent.run_dir.join("packet.toml").as_std_path().is_file(),
        "the packet lands in the run dir at registration (D4)"
    );

    let child = client
        .register_run(&RegisterRunRequest {
            packet: packet("child"),
            parent: Some(parent.run_id),
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers child");

    let listed = client.runs(None, None).await.expect("lists");
    assert_eq!(listed.len(), 2);
    assert_eq!(
        listed.last().expect("nonempty").run_id,
        child.run_id,
        "newest last (D17)"
    );

    let queued = client
        .runs(Some(RunState::Queued), None)
        .await
        .expect("filter works");
    assert_eq!(queued.len(), 2);

    let fetched = client.run(parent.run_id).await.expect("get works");
    assert_eq!(fetched.title, "parent");

    let tree = client.tree(parent.run_id).await.expect("tree works");
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].run.run_id, child.run_id);

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// The depth guard (D-C3-3) refuses a spawn that would nest past the cap,
/// at the door — before preflight or any pod is provisioned. A grandchild
/// registered with `spawn = true` sits at depth 2, past the default medium
/// cap of 1 (no profiles.toml here, so the guard charges the conservative
/// `medium` fallback), and the request is refused with no fourth run left
/// behind.
#[tokio::test]
async fn spawn_past_the_depth_cap_is_refused_at_the_door() {
    let home = scratch_home("depthcap");
    let server = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect works")
        .expect("daemon is live");

    // root (depth 0) → child (depth 1): plain registrations, no pods.
    let root = client
        .register_run(&RegisterRunRequest {
            packet: packet("root"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers root");
    let child = client
        .register_run(&RegisterRunRequest {
            packet: packet("child"),
            parent: Some(root.run_id),
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers child");

    // grandchild at depth 2 with spawn = true: the guard fires first.
    let refused = client
        .register_run(&RegisterRunRequest {
            packet: packet("grandchild"),
            parent: Some(child.run_id),
            origin_session: None,
            spawn: true,
        })
        .await;
    assert!(
        refused.is_err(),
        "a grandchild spawn past the cap must be refused"
    );

    // The refusal is at the door: no fourth run was ever created.
    let listed = client.runs(None, None).await.expect("lists");
    assert_eq!(
        listed.len(),
        2,
        "only root and child exist after the refusal"
    );

    server.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn registry_survives_a_daemon_restart_via_replay() {
    let home = scratch_home("replay");

    let first = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");
    let run = client
        .register_run(&RegisterRunRequest {
            packet: packet("survivor"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers");
    // Crash semantics: no graceful stop, lockfile left behind.
    first.kill_for_test().await;

    let second = start(Config::new(home.clone()))
        .await
        .expect("restarts over stale lock");
    let client2 = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live again");
    let recovered = client2.run(run.run_id).await.expect("replayed");
    assert_eq!(recovered.title, "survivor");
    assert_eq!(recovered.state, RunState::Queued);

    second.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn torn_journal_tail_does_not_block_startup() {
    let home = scratch_home("torn");

    let first = start(Config::new(home.clone())).await.expect("starts");
    let client = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");
    client
        .register_run(&RegisterRunRequest {
            packet: packet("kept"),
            parent: None,
            origin_session: None,
            spawn: false,
        })
        .await
        .expect("registers");
    first.kill_for_test().await;

    // Death mid-write: a half JSON line with no newline.
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(home.join("journal").join("events.jsonl").as_std_path())
            .expect("journal exists");
        f.write_all(b"{\"ts_ms\":9,\"event\":\"regis")
            .expect("torn write");
    }

    let second = start(Config::new(home.clone()))
        .await
        .expect("starts despite torn tail");
    let client2 = McClient::connect(&home)
        .await
        .expect("connect")
        .expect("live");
    assert_eq!(client2.runs(None, None).await.expect("lists").len(), 1);

    second.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn second_daemon_on_the_same_home_refuses() {
    let home = scratch_home("dup");
    let first = start(Config::new(home.clone())).await.expect("starts");
    let err = match start(Config::new(home.clone())).await {
        Err(e) => e,
        Ok(second) => {
            second.stop().await;
            panic!("second daemon on the same home must refuse");
        }
    };
    assert!(err.to_string().contains("already running"), "{err}");
    first.stop().await;
    std::fs::remove_dir_all(home.as_std_path()).ok();
}

#[tokio::test]
async fn graceful_stop_removes_the_lockfile() {
    let home = scratch_home("lock");
    let server = start(Config::new(home.clone())).await.expect("starts");
    assert!(Lockfile::read(&home).expect("reads").is_some());
    server.stop().await;
    assert!(
        Lockfile::read(&home).expect("reads").is_none(),
        "stop cleans the lock"
    );
    std::fs::remove_dir_all(home.as_std_path()).ok();
}
