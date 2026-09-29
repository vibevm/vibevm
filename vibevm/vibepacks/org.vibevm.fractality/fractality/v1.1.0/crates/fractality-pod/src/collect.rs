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
    /// D-C3-2: raw JSON Schema the result is validated against at this
    /// seam. `None` = no schema gate (the historical behavior).
    pub(crate) output_schema: Option<String>,
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

/// D-C3-2 schema-validation verdict for a worker's result. `checked` is
/// false when the packet declared no `output_schema`; otherwise `valid`
/// says whether the result conformed, and `violations` carries the
/// retry-feedback message (the Ф0 s1 shape: `at <JSON-Pointer>:
/// <message>`).
pub(crate) struct SchemaVerdict {
    pub(crate) checked: bool,
    pub(crate) valid: bool,
    pub(crate) violations: Vec<String>,
}

impl SchemaVerdict {
    fn no_gate() -> Self {
        Self {
            checked: false,
            valid: true,
            violations: Vec::new(),
        }
    }
    fn fail(violation: String) -> Self {
        Self {
            checked: true,
            valid: false,
            violations: vec![violation],
        }
    }
}

/// Validates the collected result against the packet's `output_schema`
/// (D-C3-2). Format-gate-then-quality (FD-15): a malformed schema, a
/// missing result, or a non-JSON result is itself a violation — the
/// worker is told exactly what to fix on its one retry (Ф1.2c). A `None`
/// schema means no gate: always valid, `checked = false`.
pub(crate) fn validate_output_schema(
    result_path: Option<&Utf8Path>,
    schema_json: Option<&str>,
) -> SchemaVerdict {
    let Some(schema_json) = schema_json else {
        return SchemaVerdict::no_gate();
    };
    let schema: serde_json::Value = match serde_json::from_str(schema_json) {
        Ok(v) => v,
        Err(e) => return SchemaVerdict::fail(format!("output_schema is not valid JSON: {e}")),
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => {
            return SchemaVerdict::fail(format!("output_schema is not a valid JSON Schema: {e}"));
        }
    };
    let Some(path) = result_path else {
        return SchemaVerdict::fail("no result file to validate against output_schema".to_owned());
    };
    let text = match std::fs::read_to_string(path.as_std_path()) {
        Ok(t) => t,
        Err(e) => return SchemaVerdict::fail(format!("cannot read result `{path}`: {e}")),
    };
    let instance: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return SchemaVerdict::fail(format!(
                "result is not valid JSON (an output_schema gate requires a JSON result): {e}"
            ));
        }
    };
    let violations: Vec<String> = validator
        .iter_errors(&instance)
        .map(|err| format!("at `{}`: {}", err.instance_path(), err))
        .collect();
    SchemaVerdict {
        checked: true,
        valid: violations.is_empty(),
        violations,
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
        // A capped command must die with its verdict: when the timeout
        // drops the output future, the child is reaped rather than left
        // running unsupervised (tokio's default keeps it alive).
        cmd.kill_on_drop(true);
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
        "web_tool_calls": summary.map_or(0, |s| s.totals.web_tool_calls),
        "malformed_lines": summary.map_or(0, |s| s.malformed_lines),
        "num_turns": summary.and_then(|s| s.num_turns),
        "is_error": summary.is_some_and(|s| s.is_error),
        "result_source": result_source,
        "event_counts": summary.map(|s| s.event_counts.clone()).unwrap_or_default(),
        "tool_counts": summary.map(|s| s.tool_counts.clone()).unwrap_or_default(),
        "ts_ms": now_ms(),
    });
    let path = run_dir.join(USAGE_FILE);
    let body =
        serde_json::to_string_pretty(&usage).map_err(|e| format!("encoding usage.json: {e}"))?;
    std::fs::write(path.as_std_path(), body).map_err(|e| format!("writing `{path}`: {e}"))
}

/// Everything `status.json` records about how the run ended — one
/// bundle so the call site reads as a record, not an argument train.
pub(crate) struct StatusRecord<'a> {
    pub(crate) exit_code: Option<i32>,
    pub(crate) worker_pid: u32,
    pub(crate) result_source: &'a str,
    pub(crate) acceptance: &'a [AcceptanceVerdict],
    pub(crate) acceptance_skipped: Option<&'a str>,
    /// Set when the pod itself killed the worker; outranks the exit code.
    pub(crate) killed_reason: Option<&'a str>,
    /// D-C3-2 output_schema verdict for the result (no gate when the
    /// packet declared no schema).
    pub(crate) schema: &'a SchemaVerdict,
}

/// `status.json` — the run dir's persistence-plane record (D4). The
/// state is `completed`/`failed` from the exit code — unless the pod
/// itself killed the worker (`killed_reason`), which outranks whatever
/// exit code the terminated tree happened to produce.
pub(crate) fn write_status(
    run_dir: &Utf8Path,
    run_id: RunId,
    record: StatusRecord<'_>,
) -> Result<(), String> {
    let acceptance: Vec<serde_json::Value> = record
        .acceptance
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
    let state = match (record.killed_reason, record.exit_code) {
        (Some(_), _) => "killed",
        (None, Some(0)) => "completed",
        _ => "failed",
    };
    let status = serde_json::json!({
        "schema": 1,
        "run_id": run_id,
        "state": state,
        "exit_code": record.exit_code,
        "worker_pid": record.worker_pid,
        "result_source": record.result_source,
        "acceptance": acceptance,
        "acceptance_skipped": record.acceptance_skipped,
        "kill_reason": record.killed_reason,
        "schema_gate": {
            "checked": record.schema.checked,
            "valid": record.schema.valid,
            "violations": record.schema.violations,
        },
        "ts_ms": now_ms(),
    });
    let path = run_dir.join(STATUS_FILE);
    let body =
        serde_json::to_string_pretty(&status).map_err(|e| format!("encoding status.json: {e}"))?;
    std::fs::write(path.as_std_path(), body).map_err(|e| format!("writing `{path}`: {e}"))
}

#[cfg(test)]
mod acceptance_tests {
    use camino::Utf8PathBuf;
    use std::time::Duration;

    use super::run_acceptance;

    fn make_dirs(tag: &str) -> (Utf8PathBuf, Utf8PathBuf) {
        let base =
            std::env::temp_dir().join(format!("fractality-acc-{}-{}", std::process::id(), tag));
        let run_dir = Utf8PathBuf::from_path_buf(base.clone()).expect("utf-8 temp dir");
        let work_dir = run_dir.join("work");
        std::fs::create_dir_all(work_dir.as_std_path()).expect("create work dir");
        (run_dir, work_dir)
    }

    /// A command that exits zero produces a passing verdict.
    #[tokio::test]
    async fn exit_zero_command_passes() {
        let (run_dir, work_dir) = make_dirs("exit-zero");
        let cmds = vec!["exit 0".to_string()];
        let verdicts = run_acceptance(&run_dir, &work_dir, &cmds, Duration::from_secs(10)).await;
        assert_eq!(verdicts.len(), 1);
        let v = &verdicts[0];
        assert_eq!(v.command, "exit 0");
        assert!(v.ok, "exit 0 must be ok");
        assert_eq!(v.exit_code, Some(0));
        std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    }

    /// A non-zero exit is recorded as a failure with the exact code.
    #[tokio::test]
    async fn nonzero_exit_fails_with_the_code() {
        let (run_dir, work_dir) = make_dirs("nonzero");
        let cmds = vec!["exit 7".to_string()];
        let verdicts = run_acceptance(&run_dir, &work_dir, &cmds, Duration::from_secs(10)).await;
        assert_eq!(verdicts.len(), 1);
        let v = &verdicts[0];
        assert!(!v.ok, "exit 7 must not be ok");
        assert_eq!(v.exit_code, Some(7));
        std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    }

    /// Multiple commands produce verdicts in submission order; the log
    /// preserves that order too.
    #[tokio::test]
    async fn verdicts_keep_command_order() {
        let (run_dir, work_dir) = make_dirs("order");
        let cmds = vec!["echo alpha".to_string(), "exit 3".to_string()];
        let verdicts = run_acceptance(&run_dir, &work_dir, &cmds, Duration::from_secs(10)).await;
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts[0].ok, "echo alpha must be ok");
        assert!(!verdicts[1].ok, "exit 3 must not be ok");
        let log =
            std::fs::read_to_string(run_dir.join("acceptance.log").as_std_path()).expect("log");
        let idx_alpha = log.find("=== echo alpha").expect("alpha header in log");
        let idx_exit = log.find("=== exit 3").expect("exit 3 header in log");
        assert!(
            idx_alpha < idx_exit,
            "alpha header must appear before exit 3 header in the log"
        );
        std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    }

    /// A hung command is killed when the cap expires, yielding no exit
    /// code and a failure verdict.
    #[tokio::test]
    async fn cap_kills_a_hung_command() {
        let (run_dir, work_dir) = make_dirs("cap-kill");
        let cmd = if cfg!(windows) {
            "ping -n 30 127.0.0.1 -w 1000 >nul"
        } else {
            "sleep 30"
        };
        let cmds = vec![cmd.to_string()];
        let cap = Duration::from_millis(400);
        let verdicts = run_acceptance(&run_dir, &work_dir, &cmds, cap).await;
        assert_eq!(verdicts.len(), 1);
        let v = &verdicts[0];
        assert_eq!(
            v.exit_code, None,
            "timed-out command must have exit_code None"
        );
        assert!(!v.ok, "timed-out command must not be ok");
        assert!(
            v.duration_ms < 5000,
            "verdict duration_ms ({}) must be well below 5 s -- the cap did not bite",
            v.duration_ms
        );
        let log =
            std::fs::read_to_string(run_dir.join("acceptance.log").as_std_path()).expect("log");
        assert!(
            log.contains("killed: exceeded"),
            "log must mention kill reason"
        );
        std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    }

    /// Evidence from a command is written into acceptance.log and is
    /// greppable.
    #[tokio::test]
    async fn evidence_lands_in_the_log() {
        let (run_dir, work_dir) = make_dirs("evidence");
        let cmds = vec!["echo fractality-evidence-marker".to_string()];
        let verdicts = run_acceptance(&run_dir, &work_dir, &cmds, Duration::from_secs(10)).await;
        assert_eq!(verdicts.len(), 1);
        let log =
            std::fs::read_to_string(run_dir.join("acceptance.log").as_std_path()).expect("log");
        assert!(
            log.contains("fractality-evidence-marker"),
            "log must contain the echoed marker"
        );
        std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    }
}

#[cfg(test)]
mod schema_tests {
    use camino::Utf8PathBuf;

    use super::validate_output_schema;

    fn write_result(tag: &str, body: &str) -> Utf8PathBuf {
        let base =
            std::env::temp_dir().join(format!("fractality-schema-{}-{}", std::process::id(), tag));
        std::fs::create_dir_all(&base).expect("mkdir");
        let path = Utf8PathBuf::from_path_buf(base)
            .expect("utf-8 temp dir")
            .join("result.json");
        std::fs::write(path.as_std_path(), body).expect("write result");
        path
    }

    const SCHEMA: &str = r#"{
        "type": "object",
        "required": ["summary", "status"],
        "properties": {
            "summary": { "type": "string" },
            "status": { "type": "string", "enum": ["ok", "failed"] }
        }
    }"#;

    /// No output_schema means no gate — always valid, never checked.
    #[test]
    fn no_schema_is_no_gate() {
        let v = validate_output_schema(None, None);
        assert!(!v.checked, "no schema declared → gate did not run");
        assert!(v.valid);
        assert!(v.violations.is_empty());
    }

    /// A conforming JSON result passes the gate.
    #[test]
    fn conforming_result_passes() {
        let path = write_result("ok", r#"{"summary":"did it","status":"ok"}"#);
        let v = validate_output_schema(Some(&path), Some(SCHEMA));
        assert!(v.checked);
        assert!(v.valid, "unexpected violations: {:?}", v.violations);
        std::fs::remove_dir_all(path.parent().unwrap().as_std_path()).ok();
    }

    /// A violating result fails and the report names the failing field —
    /// the retry-feedback message a worker gets for its one retry.
    #[test]
    fn violating_result_reports_each_failure() {
        let path = write_result("bad", r#"{"summary":"did it"}"#); // status missing
        let v = validate_output_schema(Some(&path), Some(SCHEMA));
        assert!(v.checked);
        assert!(!v.valid);
        assert!(
            v.violations.iter().any(|s| s.contains("status")),
            "expected a status violation, got {:?}",
            v.violations
        );
        std::fs::remove_dir_all(path.parent().unwrap().as_std_path()).ok();
    }

    /// A non-JSON result (e.g. a markdown report) fails the format gate
    /// first (FD-15) — an output_schema gate requires a JSON result.
    #[test]
    fn non_json_result_is_a_violation() {
        let path = write_result("md", "# just markdown, not JSON\n");
        let v = validate_output_schema(Some(&path), Some(SCHEMA));
        assert!(v.checked);
        assert!(!v.valid);
        assert!(
            v.violations.iter().any(|s| s.contains("not valid JSON")),
            "got {:?}",
            v.violations
        );
        std::fs::remove_dir_all(path.parent().unwrap().as_std_path()).ok();
    }
}
