//! Wire DTOs of the mission-control bus (plan D10), both legs.
//!
//! The bus is versioned localhost HTTP (`/v0/…`, bearer on every call).
//! The **client leg** serves the CLI and the boss; the **pod leg** is how
//! per-run supervisors register, heartbeat, and report events. Every verb
//! resolves through this API — files are the persistence plane, never the
//! medium (invariant I2).
//!
//! Phase 1 implements: health, node, run registration + reads, shutdown,
//! pod register/heartbeat/event. Spawn, kill, questions, results, and
//! metrics join in Phases 2–4b; their DTOs arrive with them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{PodId, RunId, SessionId};
use crate::node::{NodeIdentity, ScopeInfo};
use crate::packet::Packet;
use crate::run::{KillReason, RunRecord, RunState, UsageTotals};
use crate::session::{SessionNote, SessionRecord};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Path prefix of the current API generation.
pub const API_PREFIX: &str = "/v0";

/// Uniform error body for every non-2xx answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

// ---------------------------------------------------------------- client leg

/// `GET /v0/health`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `"ok"` when the daemon answers at all.
    pub status: String,
    pub version: String,
    pub pid: u32,
    pub started_ts_ms: u64,
    pub node_id: String,
    /// Registry size, total and non-terminal.
    pub runs_total: u64,
    pub runs_open: u64,
}

/// `GET /v0/node`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeResponse {
    pub node: NodeIdentity,
    pub scopes: Vec<ScopeInfo>,
}

/// `POST /v0/runs` — register a run from a packet; with `spawn = true`
/// mission-control also provisions the workspace (D8) and launches the
/// pod (D3). Registration-only is the primitive underneath — it is what
/// tests and manual pod driving use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisterRunRequest {
    pub packet: Packet,
    /// Parent run for nested delegation (Phase 4 populates it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<RunId>,
    /// Provision + launch the pod (the product path; `fractality run`
    /// sets it). Defaults to false so raw registration stays available.
    #[serde(default)]
    pub spawn: bool,
    /// Boss session this run is attributed to (Campaign 2 D2: the CLI
    /// reads `FRACTALITY_BOSS_SESSION` and stamps it here). A label —
    /// an unknown session id never fails the registration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_session: Option<SessionId>,
}

/// `GET /v0/runs` — the list, newest last (tail-friendly, D17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunListResponse {
    pub runs: Vec<RunRecord>,
}

/// `GET /v0/runs/:id/tree` — the call tree rooted at a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    pub run: RunRecord,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

/// `POST /v0/runs/:id/kill` — kill one run, optionally its whole subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KillRequest {
    /// Kill the call tree rooted at the run (depth-first from the root).
    #[serde(default)]
    pub recursive: bool,
}

/// What happened to one run of a kill request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillOutcome {
    /// The run transitioned to `killed`; its pod is being signaled.
    Killed,
    /// The run was already terminal — nothing to do.
    AlreadyTerminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KillResult {
    pub run_id: RunId,
    pub outcome: KillOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KillResponse {
    /// Root first, then descendants in tree order (recursive mode).
    pub results: Vec<KillResult>,
}

/// `POST /v0/runs/:id/question` — park the run on a question (D18).
/// Called by the ask_boss broker from inside the worker's MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionRequest {
    pub question: String,
}

/// `POST /v0/runs/:id/answer` — answer and resume (D18). The broker
/// collects the answer from the run record and returns it as the
/// worker's tool result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnswerRequest {
    pub answer: String,
}

/// `GET /v0/metrics` — aggregates over the whole registry (D16: every
/// telemetry consumer reads exactly this; no shadow accounting).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub totals: MetricsBucket,
    /// Runs per lifecycle state (all seven states, present when nonzero).
    pub by_state: BTreeMap<String, u64>,
    pub by_profile: BTreeMap<String, MetricsBucket>,
    pub by_model: BTreeMap<String, MetricsBucket>,
    /// UTC day (`YYYY-MM-DD`) of run creation.
    pub by_day: BTreeMap<String, MetricsBucket>,
}

/// One aggregate bucket of the metrics answer.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MetricsBucket {
    pub runs: u64,
    pub completed: u64,
    pub failed: u64,
    pub killed: u64,
    /// Non-terminal runs (queued through waiting_on_boss).
    pub open: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub total_cost_usd: f64,
    /// Wall time of terminal runs (start → last update), milliseconds.
    pub wall_ms: u64,
    /// The D12 quota counter, summed from run usage.
    pub web_tool_calls: u64,
}

/// `POST /v0/sessions` — register (or resume) a boss session
/// (Campaign 2 D2/D3). Idempotent per `(harness, external_id)`: while
/// such a session is open, begin returns it instead of minting a twin —
/// Claude Code fires SessionStart on `startup|resume|clear|compact`,
/// and all four must land on one record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBeginRequest {
    /// Harness label, e.g. `claude-code` (I4: data, not a code path).
    pub harness: String,
    /// The harness's own session identifier (CC session UUID).
    pub external_id: String,
    pub cwd: camino::Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBeginResponse {
    pub session: SessionRecord,
    /// True when an open session matched and was returned as-is.
    pub resumed: bool,
}

/// `POST /v0/sessions/:id/events` — record one initiative fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEventRequest {
    pub note: SessionNote,
}

/// `GET /v0/sessions` — newest last (D17), optional `?open=true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionRecord>,
}

/// `GET /v0/sessions/:id/metrics` — the session-scoped scoreboard
/// facts: the record (with its initiative counters) plus the metrics
/// bucket folded over the runs this session originated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMetricsResponse {
    pub session: SessionRecord,
    pub runs: MetricsBucket,
    /// Open questions on runs of this session (id + age), the D5
    /// injection facts.
    #[serde(default)]
    pub parked: Vec<ParkedQuestion>,
}

/// One parked worker question, as the scoreboard shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParkedQuestion {
    pub run_id: RunId,
    pub question: String,
    /// Milliseconds the run has been waiting.
    pub waiting_ms: u64,
}

/// `POST /v0/shutdown`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShutdownResponse {
    pub shutting_down: bool,
}

// ------------------------------------------------------------------- pod leg

/// `POST /v0/pods/register` — a pod binds itself to its run. Idempotent:
/// re-registration after a mission-control restart is the adoption path
/// (D3), not an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodRegisterRequest {
    pub pod_id: PodId,
    pub pod_pid: u32,
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodRegisterResponse {
    /// True when the run was known and non-terminal — the pod is adopted.
    pub adopted: bool,
    pub heartbeat_interval_ms: u64,
}

/// `POST /v0/pods/:id/heartbeat`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodHeartbeat {
    pub run_id: RunId,
    /// The state the pod believes the run is in (echo; MC logs drift).
    pub state: RunState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_pid: Option<u32>,
}

/// Heartbeat answer — the pod's command channel. Kill directives ride
/// here (Phase 4); restart directives are future work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PodCommand {
    None,
    /// Kill the worker tree now. The reason is informational for the
    /// pod's log — the authoritative `killed` record is journaled by
    /// mission-control at decision time.
    Kill {
        reason: KillReason,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodHeartbeatResponse {
    pub command: PodCommand,
}

/// `POST /v0/pods/:id/event` — the pod-reported run lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodEventRequest {
    pub run_id: RunId,
    pub event: PodEvent,
}

/// What a pod can report about its worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PodEvent {
    /// The worker process is up.
    Spawned { worker_pid: u32 },
    /// A state assertion (e.g. `running`, later `waiting_on_boss`).
    State {
        state: RunState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Cumulative usage snapshot parsed from the worker stream.
    Usage { usage: UsageTotals },
    /// Collection settled (Phase 4): result provenance + acceptance
    /// verdicts. The pod ships the node-local path; mission-control
    /// mints the FileRef (it owns scopes and stamps etags, D19).
    Collected {
        result_source: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result_path: Option<camino::Utf8PathBuf>,
        acceptance_passed: u32,
        acceptance_total: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        acceptance_skipped: Option<String>,
    },
    /// The worker exited (`None` = killed by signal).
    Exit { exit_code: Option<i32> },
    /// A pod-side fault; `terminal` says whether the run dies of it.
    Fault { message: String, terminal: bool },
}

/// Uniform "accepted" body for pod-leg posts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_event_wire_form_uses_kind_tags() {
        let e = PodEventRequest {
            run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
            event: PodEvent::Exit { exit_code: Some(0) },
        };
        let json = serde_json::to_string(&e).expect("serializes");
        assert_eq!(
            json,
            r#"{"run_id":"01ARZ3NDEKTSV4RRFFQ69G5FAV","event":{"kind":"exit","exit_code":0}}"#
        );
    }

    #[test]
    fn pod_command_none_is_a_plain_string() {
        let r = PodHeartbeatResponse {
            command: PodCommand::None,
        };
        assert_eq!(
            serde_json::to_string(&r).expect("serializes"),
            r#"{"command":"none"}"#
        );
    }
}
