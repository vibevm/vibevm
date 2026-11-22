//! The Phase 4 swarm verbs (D13): `spawn` / `wait` / `tree` / `kill`,
//! plus the exit-code family and parent resolution they share with the
//! sync `run` loop.

use fractality_core::run::{RunRecord, RunState};
use fractality_mc_client::{McClient, connect_or_start};

use crate::{EXIT_INFRA, EXIT_NEGATIVE, EXIT_OK, err_code, fail_code, out, resolve_run};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The D17 exit-code family for a terminal run: 0 completed · 1 failed ·
/// 3 policy-killed (budget, manual) · 2 infrastructure (pod lost) ·
/// 5 escalated (the task was handed UP the tree — NOT a failure, D-C3-6).
/// A parent awaiting a child keys off 5 to climb the escalation rather
/// than treat it as a failed run. (4 is reserved for a run left parked
/// past its wait budget — see `run_once`.)
pub(crate) fn state_code(r: &RunRecord) -> u8 {
    match r.state {
        RunState::Completed => EXIT_OK,
        RunState::Killed => match r.kill_reason {
            Some(fractality_core::run::KillReason::PodLost) => EXIT_INFRA,
            _ => 3,
        },
        RunState::Escalated => 5,
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

/// `fractality run --packet <file>`: the sync delegation loop (D13) —
/// register + spawn, block to a terminal state, print the one-screen
/// summary. On an `output_schema` violation it re-dispatches ONCE with the
/// violation report folded into the retry's context (D-C3-2 retry-on-
/// violation, deferred here from Ф1.2b): the need-gate's re-spawn at the
/// sync orchestration layer. Lives with the other run verbs rather than in
/// `main`, which stays dispatch-only.
pub(crate) async fn run_packet(
    home: &camino::Utf8Path,
    packet_path: &camino::Utf8Path,
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
    // Client-side wait cap: the packet's wall budget plus grace. Budget
    // enforcement (the kill) is Phase 4; until then an overrun stops the
    // WAIT loudly, never silently.
    let wait_cap = std::time::Duration::from_secs(packet.budget.wall_secs + 60);

    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let parent = match resolve_parent(&client, None).await {
        Ok(p) => p,
        Err((code, message)) => return fail_code(code, &message),
    };

    let overall = std::time::Instant::now();
    let first = match run_once(&client, packet.clone(), parent, wait_cap).await {
        Ok(r) => r,
        Err(code) => return code,
    };
    // Retry-on-violation (D-C3-2): a single re-dispatch when the result
    // failed its output_schema gate, with the violations folded into the
    // retry's context notes. Checked only on the FIRST attempt, so the
    // retry runs at most once — its own result stands whatever it is.
    let final_run = match retry_report(&first.run_dir) {
        Some(report) => {
            eprintln!(
                "run {} failed its output_schema gate; re-dispatching once with the violation report",
                first.run_id
            );
            let mut retry = packet;
            let prior = retry.context.notes.take().unwrap_or_default();
            retry.context.notes = Some(
                format!(
                    "{prior}\n\n[fractality retry] the prior attempt's result failed \
                     output_schema validation — fix these and return a conforming result:\n{report}"
                )
                .trim()
                .to_owned(),
            );
            match run_once(&client, retry, parent, wait_cap).await {
                Ok(r) => r,
                Err(code) => return code,
            }
        }
        None => first,
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&final_run)
                .unwrap_or_else(|e| format!("{{\"error\":\"json: {e}\"}}"))
        );
    } else {
        out::print_run_summary(&final_run, overall.elapsed());
    }
    state_code(&final_run)
}

/// One register-spawn-and-wait cycle: registers the packet as a spawn,
/// blocks to a terminal state, and returns the settled record. Early exits
/// — parked past budget, wall-budget overrun, transport fault — come back
/// as `Err(exit_code)`, already printed.
pub(crate) async fn run_once(
    client: &McClient,
    packet: fractality_core::Packet,
    parent: Option<fractality_core::ids::RunId>,
    wait_cap: std::time::Duration,
) -> Result<RunRecord, u8> {
    let started = std::time::Instant::now();
    let run = client
        .register_run(&fractality_core::api::RegisterRunRequest {
            packet,
            parent,
            spawn: true,
            origin_session: origin_session_from_env(),
        })
        .await
        .map_err(|e| fail_code(err_code(&e), &e.to_string()))?;
    eprintln!("run {} spawned (dir {})", run.run_id, run.run_dir);

    let mut parked_notice = false;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match client.run(run.run_id).await {
            Ok(r) if r.state.is_terminal() => {
                if parked_notice {
                    eprintln!("run {} resumed and finished", run.run_id);
                }
                return Ok(r);
            }
            Ok(r) if r.state == RunState::WaitingOnBoss => {
                if !parked_notice {
                    parked_notice = true;
                    eprintln!(
                        "run {} PARKED on a question: {}\n  answer with: fractality answer {} \"<text>\"",
                        r.run_id,
                        r.question.as_deref().unwrap_or("-"),
                        r.run_id,
                    );
                }
                if started.elapsed() > wait_cap {
                    // D17 exit family 4: parked past its wait — the run
                    // stays alive for a later answer; this loop stops.
                    return Err(fail_code(
                        4,
                        &format!(
                            "run {} is still parked on its question past the wall budget; \
                             it keeps waiting — `fractality questions` to triage",
                            run.run_id
                        ),
                    ));
                }
            }
            Ok(_) if started.elapsed() > wait_cap => {
                return Err(fail_code(
                    EXIT_INFRA,
                    &format!(
                        "run {} outlived its wall budget plus grace and is still not \
                         terminal — the mission-control watchdog should have killed it; \
                         inspect with `fractality show {}` and `mc.log`",
                        run.run_id, run.run_id
                    ),
                ));
            }
            Ok(_) => continue,
            Err(e) => return Err(fail_code(err_code(&e), &e.to_string())),
        }
    }
}

/// The output_schema violation report for a finished run, read from its
/// run dir's `status.json` (Ф1.2b writes the `schema_gate` there). `Some`
/// only when the gate was checked AND failed; `None` when it passed, was
/// not checked (no schema ⇒ no retry), or the file is unreadable.
fn retry_report(run_dir: &camino::Utf8Path) -> Option<String> {
    let text = std::fs::read_to_string(run_dir.join("status.json").as_std_path()).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&text).ok()?;
    let gate = doc.get("schema_gate")?;
    if gate.get("checked")?.as_bool()? && !gate.get("valid")?.as_bool()? {
        let violations = gate
            .get("violations")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Some(violations)
    } else {
        None
    }
}

/// `fractality wait <id>…`: shell-wait semantics (D13/D17). The descent
/// await verbs (D-C3-4) share this one command: passing ids is `await
/// named`; the default joins on ALL of them (blocks until every awaited
/// run is terminal, exit mirrors the last); `--any` races them and
/// returns on the FIRST to settle — the parallel-siblings idiom where a
/// parent proceeds on the first sibling done.
pub(crate) async fn wait(
    home: &camino::Utf8Path,
    raw_ids: &[String],
    timeout_secs: u64,
    any: bool,
) -> u8 {
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

    // `await any`: race the awaited runs; the first terminal one wins and
    // its outcome is the exit code. The siblings keep running — the caller
    // decides whether to kill them (a merge node's job, a later slice).
    if any {
        loop {
            let mut winner = None;
            for id in &ids {
                match client.run(*id).await {
                    Ok(r) if r.state.is_terminal() => {
                        winner = Some(r);
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => return fail_code(err_code(&e), &e.to_string()),
                }
            }
            if let Some(r) = winner {
                println!(
                    "{} {} exit={}",
                    r.run_id,
                    r.state,
                    r.exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".into())
                );
                return state_code(&r);
            }
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                return fail_code(
                    EXIT_INFRA,
                    "timeout: no awaited run reached a terminal state",
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> camino::Utf8PathBuf {
        let dir = std::env::temp_dir().join(format!("fractality-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        camino::Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
    }

    fn write_status(dir: &camino::Utf8Path, body: &str) {
        std::fs::write(dir.join("status.json").as_std_path(), body).expect("write status.json");
    }

    #[test]
    fn retry_report_fires_only_on_a_checked_failed_gate() {
        let dir = scratch();

        // Checked + failed → the violations come back for the retry.
        write_status(
            &dir,
            r#"{"schema_gate":{"checked":true,"valid":false,"violations":["at /x: missing"]}}"#,
        );
        assert_eq!(retry_report(&dir).as_deref(), Some("at /x: missing"));

        // Checked + passed → no retry.
        write_status(
            &dir,
            r#"{"schema_gate":{"checked":true,"valid":true,"violations":[]}}"#,
        );
        assert!(retry_report(&dir).is_none(), "a passing gate never retries");

        // No gate (packet had no output_schema) → no retry.
        write_status(
            &dir,
            r#"{"schema_gate":{"checked":false,"valid":true,"violations":[]}}"#,
        );
        assert!(
            retry_report(&dir).is_none(),
            "an unchecked gate never retries"
        );

        // Missing status.json → no retry, never a panic.
        std::fs::remove_file(dir.join("status.json").as_std_path()).ok();
        assert!(
            retry_report(&dir).is_none(),
            "a missing status file never retries"
        );

        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }
}
