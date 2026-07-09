//! Phase 3 collection: settling what a finished run leaves behind.
//!
//! The worker was told (invocation preamble, D7) to write its report to
//! the packet-named result file; this cell records whether it did
//! ([`collect_result`] — `worker` | `extracted` | `none`, never
//! guessed), proves the deliverable with the packet's acceptance
//! commands ([`run_acceptance`] — pass/fail *recorded*, evidence in
//! `acceptance.log`), and writes the persistence-plane records
//! (`usage.json`, `status.json` — D4; flat fields for grep-ability,
//! D17). The acceptance verdicts live on the plane for the v0.1 single
//! box; promoting them onto the bus (a `Collected` pod event folding
//! into the run record) is named Phase 4 work — the swarm needs remote
//! reads, the sync loop does not.

use camino::{Utf8Path, Utf8PathBuf};
use fractality_backend_claude_code::stream::StreamSummary;
use fractality_core::ids::RunId;
use fractality_core::time::now_ms;

specmark::scope!("spec://fractality/PROP-001#architecture");

const STATUS_FILE: &str = "status.json";
const USAGE_FILE: &str = "usage.json";

/// The collection contract resolved from the packet: where the worker's
/// result file must land, what it is called (packet `output.result`),
/// and which acceptance commands prove the work (packet
/// `task.acceptance`). Product mode only — the raw `--spec` seam has no
/// packet.
pub(crate) struct Collection {
    pub(crate) workspace_dir: Utf8PathBuf,
    pub(crate) result_file: String,
    pub(crate) acceptance: Vec<String>,
}

/// Settles the result-file contract (D4/D7): the worker was told to
/// write `result_file` in its workspace; when it did not, fall back to
/// the transcript's final message — and always record which happened
/// (and where, so readers need no packet to find it).
pub(crate) fn collect_result(
    collection: Option<&Collection>,
    summary: Option<&StreamSummary>,
) -> (&'static str, Option<Utf8PathBuf>) {
    let Some(c) = collection else {
        // Raw --spec seam: no packet, no contract to settle.
        return ("none", None);
    };
    let path = c.workspace_dir.join(&c.result_file);
    match std::fs::metadata(path.as_std_path()) {
        Ok(m) if m.len() > 0 => ("worker", Some(path)),
        _ => {
            let text = summary.and_then(|s| s.final_text.as_deref());
            match text.filter(|t| !t.trim().is_empty()) {
                Some(text) => match std::fs::write(path.as_std_path(), text) {
                    Ok(()) => {
                        tracing::info!(%path, "result extracted from the final message");
                        ("extracted", Some(path))
                    }
                    Err(e) => {
                        tracing::error!(%path, error = %e, "result extraction failed");
                        ("none", None)
                    }
                },
                None => ("none", None),
            }
        }
    }
}

/// One acceptance command's verdict (Phase 3: pass/fail is *recorded*,
/// never assumed). `exit_code: None` means the command did not produce
/// an exit code — killed on timeout or unspawnable.
pub(crate) struct AcceptanceVerdict {
    pub(crate) command: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) ok: bool,
    pub(crate) duration_ms: u64,
}

/// Hard per-command cap until packet budgets enforce their own (Phase
/// 4): a hung acceptance command must not hang the pod forever.
pub(crate) const ACCEPTANCE_CAP: std::time::Duration = std::time::Duration::from_secs(600);

/// Runs the packet's acceptance commands sequentially in the workspace
/// (pod env, worker cwd; shell form per platform). Combined output
/// appends to `acceptance.log` in the run dir — the verdicts stay lean
/// in status.json, the evidence stays greppable on the plane (D17).
/// `cap` bounds each command (callers pass [`ACCEPTANCE_CAP`]; tests
/// pass something humane).
pub(crate) async fn run_acceptance(
    run_dir: &Utf8Path,
    workspace_dir: &Utf8Path,
    commands: &[String],
    cap: std::time::Duration,
) -> Vec<AcceptanceVerdict> {
    let log_path = run_dir.join("acceptance.log");
    let mut log = String::new();
    let mut verdicts = Vec::with_capacity(commands.len());
    for command in commands {
        let started = std::time::Instant::now();
        let mut cmd = if cfg!(windows) {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(command);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(command);
            c
        };
        cmd.current_dir(workspace_dir.as_std_path());
        let outcome = tokio::time::timeout(cap, cmd.output()).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        let (exit_code, ok, evidence) = match outcome {
            Ok(Ok(output)) => (
                output.status.code(),
                output.status.success(),
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ),
            Ok(Err(e)) => (None, false, format!("(spawn failed: {e})\n")),
            Err(_) => (
                None,
                false,
                format!("(killed: exceeded {}s cap)\n", cap.as_secs()),
            ),
        };
        log.push_str(&format!(
            "=== {} (exit {:?}, {} ms) ===\n{evidence}\n",
            command, exit_code, duration_ms
        ));
        tracing::info!(
            command,
            ?exit_code,
            ok,
            duration_ms,
            "acceptance command finished"
        );
        verdicts.push(AcceptanceVerdict {
            command: command.clone(),
            exit_code,
            ok,
            duration_ms,
        });
    }
    if let Err(e) = std::fs::write(log_path.as_std_path(), &log) {
        tracing::warn!(%log_path, error = %e, "acceptance log write failed");
    }
    verdicts
}

/// `usage.json` — the metering record of the persistence plane (D4),
/// flat fields for grep-ability (D17).
pub(crate) fn write_usage_json(
    run_dir: &Utf8Path,
    run_id: RunId,
    summary: Option<&StreamSummary>,
    result_source: &str,
    result_path: Option<&Utf8Path>,
) -> Result<(), String> {
    let usage = serde_json::json!({
        "schema": 1,
        "run_id": run_id,
        "model": summary.and_then(|s| s.model.clone()),
        "result_path": result_path.map(|p| p.to_string()),
        "input_tokens": summary.map_or(0, |s| s.totals.input_tokens),
        "output_tokens": summary.map_or(0, |s| s.totals.output_tokens),
        "cache_creation_input_tokens": summary.map_or(0, |s| s.totals.cache_creation_input_tokens),
        "cache_read_input_tokens": summary.map_or(0, |s| s.totals.cache_read_input_tokens),
        "total_cost_usd": summary.map_or(0.0, |s| s.totals.total_cost_usd),
        "events": summary.map_or(0, |s| s.totals.events),
        "malformed_lines": summary.map_or(0, |s| s.malformed_lines),
        "num_turns": summary.and_then(|s| s.num_turns),
        "is_error": summary.is_some_and(|s| s.is_error),
        "result_source": result_source,
        "event_counts": summary.map(|s| s.event_counts.clone()).unwrap_or_default(),
        "ts_ms": now_ms(),
    });
    let path = run_dir.join(USAGE_FILE);
    let body =
        serde_json::to_string_pretty(&usage).map_err(|e| format!("encoding usage.json: {e}"))?;
    std::fs::write(path.as_std_path(), body).map_err(|e| format!("writing `{path}`: {e}"))
}

/// `status.json` — the run dir's persistence-plane record (D4).
pub(crate) fn write_status(
    run_dir: &Utf8Path,
    run_id: RunId,
    exit_code: Option<i32>,
    worker_pid: u32,
    result_source: &str,
    acceptance: &[AcceptanceVerdict],
    acceptance_skipped: Option<&str>,
) -> Result<(), String> {
    let acceptance: Vec<serde_json::Value> = acceptance
        .iter()
        .map(|v| {
            serde_json::json!({
                "command": v.command,
                "exit_code": v.exit_code,
                "ok": v.ok,
                "duration_ms": v.duration_ms,
            })
        })
        .collect();
    let status = serde_json::json!({
        "schema": 1,
        "run_id": run_id,
        "state": if exit_code == Some(0) { "completed" } else { "failed" },
        "exit_code": exit_code,
        "worker_pid": worker_pid,
        "result_source": result_source,
        "acceptance": acceptance,
        "acceptance_skipped": acceptance_skipped,
        "ts_ms": now_ms(),
    });
    let path = run_dir.join(STATUS_FILE);
    let body =
        serde_json::to_string_pretty(&status).map_err(|e| format!("encoding status.json: {e}"))?;
    std::fs::write(path.as_std_path(), body).map_err(|e| format!("writing `{path}`: {e}"))
}
