//! The Phase 4 swarm verbs (D13): `spawn` / `wait` / `tree` / `kill`,
//! plus the exit-code family and parent resolution they share with the
//! sync `run` loop.

use fractality_core::run::{RunRecord, RunState};
use fractality_mc_client::{McClient, connect_or_start};

use crate::{EXIT_INFRA, EXIT_NEGATIVE, EXIT_OK, err_code, fail_code, out, resolve_run};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The D17 exit-code family for a terminal run: 0 completed · 1 failed ·
/// 3 policy-killed (budget, manual) · 2 infrastructure (pod lost).
pub(crate) fn state_code(r: &RunRecord) -> u8 {
    match r.state {
        RunState::Completed => EXIT_OK,
        RunState::Killed => match r.kill_reason {
            Some(fractality_core::run::KillReason::PodLost) => EXIT_INFRA,
            _ => 3,
        },
        _ => EXIT_NEGATIVE,
    }
}

/// The nesting default (Phase 4): a worker's own spawns attach to its
/// run via the env the pod injected.
fn parent_from_env() -> Option<String> {
    std::env::var("FRACTALITY_RUN_ID")
        .ok()
        .filter(|v| !v.is_empty())
}

/// The attribution default (Campaign 2 D2): a boss session's spawns
/// carry its id via the env the harness adapter exported at
/// SessionStart. Same worker-context seam as `FRACTALITY_RUN_ID` —
/// this file is the recorded env root for both. A malformed value is
/// dropped with a warning, never fatal (attribution is a label).
pub(crate) fn origin_session_from_env() -> Option<fractality_core::ids::SessionId> {
    let raw = std::env::var(fractality_core::session::BOSS_SESSION_ENV)
        .ok()
        .filter(|v| !v.is_empty())?;
    match raw.parse() {
        Ok(id) => Some(id),
        Err(_) => {
            eprintln!(
                "fractality: {} `{raw}` is not a session id; run not attributed",
                fractality_core::session::BOSS_SESSION_ENV
            );
            None
        }
    }
}

/// Resolves the parent for a new run: explicit flag wins, then the
/// worker-context env (FRACTALITY_RUN_ID), then none. An explicit value
/// may be a unique prefix; the env value must be exact (the pod wrote it).
pub(crate) async fn resolve_parent(
    client: &McClient,
    explicit: Option<&str>,
) -> Result<Option<fractality_core::ids::RunId>, (u8, String)> {
    match explicit {
        Some(raw) => {
            let run = resolve_run(client, raw).await?;
            Ok(Some(run.run_id))
        }
        None => match parent_from_env() {
            Some(raw) => raw.parse().map(Some).map_err(|_| {
                (
                    EXIT_NEGATIVE,
                    format!("FRACTALITY_RUN_ID `{raw}` is not a run id"),
                )
            }),
            None => Ok(None),
        },
    }
}

/// `fractality spawn --packet <file>`: fire-and-return (D13).
pub(crate) async fn spawn(
    home: &camino::Utf8Path,
    packet_path: &camino::Utf8Path,
    parent: Option<&str>,
    json: bool,
) -> u8 {
    let text = match std::fs::read_to_string(packet_path.as_std_path()) {
        Ok(t) => t,
        Err(e) => return fail_code(EXIT_NEGATIVE, &format!("reading `{packet_path}`: {e}")),
    };
    let packet = match fractality_core::Packet::from_toml_str(&text) {
        Ok(p) => p,
        Err(e) => return fail_code(EXIT_NEGATIVE, &e.to_string()),
    };
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let parent = match resolve_parent(&client, parent).await {
        Ok(p) => p,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client
        .register_run(&fractality_core::api::RegisterRunRequest {
            packet,
            parent,
            spawn: true,
            origin_session: origin_session_from_env(),
        })
        .await
    {
        Ok(run) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run)
                        .unwrap_or_else(|e| format!("{{\"error\":\"json: {e}\"}}"))
                );
            } else {
                // One id on stdout: `id=$(fractality spawn …)` just works.
                println!("{}", run.run_id);
                eprintln!("state={} dir={}", run.state, run.run_dir);
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality wait <id>…`: shell-wait semantics (D13/D17).
pub(crate) async fn wait(home: &camino::Utf8Path, raw_ids: &[String], timeout_secs: u64) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let mut ids = Vec::with_capacity(raw_ids.len());
    for raw in raw_ids {
        match resolve_run(&client, raw).await {
            Ok(r) => ids.push(r.run_id),
            Err((code, message)) => return fail_code(code, &message),
        }
    }
    let deadline = (timeout_secs > 0)
        .then(|| std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs));
    let mut last_code = EXIT_OK;
    for id in ids {
        let settled = loop {
            match client.run(id).await {
                Ok(r) if r.state.is_terminal() => break r,
                Ok(_) => {
                    if let Some(d) = deadline
                        && std::time::Instant::now() >= d
                    {
                        return fail_code(
                            EXIT_INFRA,
                            &format!("timeout: run {id} is still not terminal"),
                        );
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => return fail_code(err_code(&e), &e.to_string()),
            }
        };
        println!(
            "{} {} exit={}",
            settled.run_id,
            settled.state,
            settled
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".into())
        );
        last_code = state_code(&settled);
    }
    last_code
}

/// `fractality tree [<id>]`: one tree or the whole forest.
pub(crate) async fn tree(home: &camino::Utf8Path, raw_id: Option<&str>, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match raw_id {
        Some(raw) => {
            let root = match resolve_run(&client, raw).await {
                Ok(r) => r,
                Err((code, message)) => return fail_code(code, &message),
            };
            match client.tree(root.run_id).await {
                Ok(node) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&node).expect("tree serializes")
                        );
                    } else {
                        out::print_tree(&node, 0);
                    }
                    EXIT_OK
                }
                Err(e) => fail_code(err_code(&e), &e.to_string()),
            }
        }
        None => {
            // Forest: every root, assembled client-side from one list
            // call (roots are runs with no parent).
            let runs = match client.runs(None, None).await {
                Ok(r) => r,
                Err(e) => return fail_code(err_code(&e), &e.to_string()),
            };
            let roots: Vec<_> = runs.iter().filter(|r| r.parent.is_none()).collect();
            if json {
                let mut nodes = Vec::new();
                for root in &roots {
                    if let Ok(node) = client.tree(root.run_id).await {
                        nodes.push(node);
                    }
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&nodes).expect("forest serializes")
                );
                return EXIT_OK;
            }
            for root in roots {
                match client.tree(root.run_id).await {
                    Ok(node) => out::print_tree(&node, 0),
                    Err(e) => return fail_code(err_code(&e), &e.to_string()),
                }
            }
            EXIT_OK
        }
    }
}

/// `fractality kill <id> [--tree]` (D13).
pub(crate) async fn kill(home: &camino::Utf8Path, raw_id: &str, tree: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let root = match resolve_run(&client, raw_id).await {
        Ok(r) => r,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client.kill(root.run_id, tree).await {
        Ok(resp) => {
            let mut root_killed = false;
            for result in &resp.results {
                let verdict = match result.outcome {
                    fractality_core::api::KillOutcome::Killed => "killed",
                    fractality_core::api::KillOutcome::AlreadyTerminal => "already-terminal",
                };
                if result.run_id == root.run_id
                    && result.outcome == fractality_core::api::KillOutcome::Killed
                {
                    root_killed = true;
                }
                println!("{} {}", result.run_id, verdict);
            }
            if root_killed { EXIT_OK } else { EXIT_NEGATIVE }
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}
