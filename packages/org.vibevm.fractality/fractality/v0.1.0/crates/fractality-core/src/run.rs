//! Run state machine and the run record.
//!
//! A run is one worker lifecycle under one pod (D3). States are flat and
//! machine-readable; rich detail (exit codes, failure text, kill reasons,
//! usage) lives in dedicated fields of [`RunRecord`], so `ps --state=…`
//! filters stay trivial and the journal stays greppable (D17).

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::fileref::FileRef;
use crate::ids::{PodId, RunId};
use crate::packet::{BudgetSpec, WorkspaceMode};

specmark::scope!("spec://fractality/PROP-001#model");

/// Lifecycle state of a run.
///
/// Legal transitions (everything else is a bug and is refused):
///
/// ```text
/// queued          → starting | failed | killed
/// starting        → running | completed | failed | killed
/// running         → waiting_on_boss | completed | failed | killed
/// waiting_on_boss → running | completed | failed | killed
/// completed | failed | killed → (terminal)
/// ```
///
/// `starting → completed` is legal because a worker may exit before its
/// first heartbeat lands; `waiting_on_boss → completed` because a parked
/// worker may die while parked (D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Starting,
    Running,
    WaitingOnBoss,
    Completed,
    Failed,
    Killed,
}

impl RunState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunState::Completed | RunState::Failed | RunState::Killed
        )
    }

    /// Whether the state machine allows `self → next`.
    pub fn can_transition_to(self, next: RunState) -> bool {
        use RunState::*;
        match self {
            Queued => matches!(next, Starting | Failed | Killed),
            Starting => matches!(next, Running | Completed | Failed | Killed),
            Running => matches!(next, WaitingOnBoss | Completed | Failed | Killed),
            WaitingOnBoss => matches!(next, Running | Completed | Failed | Killed),
            Completed | Failed | Killed => false,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RunState::Queued => "queued",
            RunState::Starting => "starting",
            RunState::Running => "running",
            RunState::WaitingOnBoss => "waiting_on_boss",
            RunState::Completed => "completed",
            RunState::Failed => "failed",
            RunState::Killed => "killed",
        }
    }
}

impl std::fmt::Display for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RunState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(RunState::Queued),
            "starting" => Ok(RunState::Starting),
            "running" => Ok(RunState::Running),
            "waiting_on_boss" => Ok(RunState::WaitingOnBoss),
            "completed" => Ok(RunState::Completed),
            "failed" => Ok(RunState::Failed),
            "killed" => Ok(RunState::Killed),
            other => Err(format!(
                "unknown run state `{other}` (expected one of: queued, starting, running, \
                 waiting_on_boss, completed, failed, killed)"
            )),
        }
    }
}

/// Why a run ended in `killed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillReason {
    /// Operator or boss asked for it (`fractality kill`).
    Manual,
    /// A packet budget was exceeded (D7 / Phase 4).
    Budget,
    /// The supervising pod vanished and its worker was reaped (D3/D9).
    PodLost,
}

impl std::fmt::Display for KillReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            KillReason::Manual => "manual",
            KillReason::Budget => "budget",
            KillReason::PodLost => "pod_lost",
        };
        f.write_str(s)
    }
}

/// Cumulative usage totals for one run, as reported by the pod from the
/// worker's stream-json `usage` fields (F4 proved they are present).
/// Snapshots are cumulative, so replaying the latest one is idempotent.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    /// As reported by the worker CLI; informational for flat-rate plans (D6).
    pub total_cost_usd: f64,
    /// Stream events observed (the D14 fallback metric when usage is absent).
    pub events: u64,
    /// Web-ish tool calls observed in the transcript (the D12 quota
    /// counter; the parser classifies by tool name).
    #[serde(default)]
    pub web_tool_calls: u64,
}

/// The pod currently bound to a run (D3: one pod per run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodBinding {
    pub pod_id: PodId,
    pub pod_pid: u32,
}

/// What collection settled for a finished run (Phase 4: the verdicts ride
/// the bus and fold into the record — remote readers never need the run
/// dir; the plane files stay the D17 last resort).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collected {
    /// How the result file came to be: `worker` | `extracted` | `none`.
    pub result_source: String,
    /// Claim-check reference to the result (D19), minted by
    /// mission-control when the path lies inside a known scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<FileRef>,
    /// Absolute result path on the executing node (same-box convenience;
    /// the FileRef is the portable form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_path: Option<Utf8PathBuf>,
    pub acceptance_passed: u32,
    pub acceptance_total: u32,
    /// Why acceptance did not run, when it did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_skipped: Option<String>,
}

/// Everything mission-control knows about one run. This is both the
/// registry entry and the `registered` journal snapshot (D9), and the wire
/// shape for list/show (D10) — one type, no summary/detail drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: RunId,
    pub title: String,
    pub state: RunState,
    /// Routing profile name (D6), e.g. `glm`.
    pub profile: String,
    /// Model slot within the profile, e.g. `big` / `small`.
    pub model: String,
    pub workspace_mode: WorkspaceMode,
    /// Parent run for nested delegation (Phase 4); the call tree edges.
    pub parent: Option<RunId>,
    /// Boss session that originated this run (Campaign 2 D2). A label
    /// for attribution and per-session metrics — never a reference:
    /// a dangling id is harmless and never invalidates the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_session: Option<crate::ids::SessionId>,
    /// Nesting depth (0 = boss-spawned; parent's depth + 1 otherwise).
    #[serde(default)]
    pub depth: u32,
    /// True when the caller asked mission-control to provision + launch
    /// (the product path). Only such runs pass through admission; raw
    /// registrations stay a driving primitive for tests and manual pods.
    #[serde(default)]
    pub spawn_requested: bool,
    /// The packet's hard budget, denormalized for the watchdog (a value
    /// of 0 in any field means "unlimited" for that axis).
    #[serde(default)]
    pub budget: BudgetSpec,
    /// Node the run executes on (D19).
    pub node_id: String,
    /// The run directory — the persistence plane for this run (D4).
    pub run_dir: Utf8PathBuf,
    pub created_ts_ms: u64,
    pub updated_ts_ms: u64,
    /// When the run left the queue (the budget wall-clock anchor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_ts_ms: Option<u64>,
    pub pod: Option<PodBinding>,
    pub worker_pid: Option<u32>,
    pub exit_code: Option<i32>,
    /// Human-readable failure cause when `state = failed`.
    pub failure: Option<String>,
    pub kill_reason: Option<KillReason>,
    pub usage: UsageTotals,
    /// Collection outcome (result + acceptance), once the pod reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collected: Option<Collected>,
    /// The open question while `waiting_on_boss` (D18); cleared by the
    /// answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// The most recent answer (what the broker returns to the worker);
    /// cleared when a new question parks the run again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [RunState; 7] = [
        RunState::Queued,
        RunState::Starting,
        RunState::Running,
        RunState::WaitingOnBoss,
        RunState::Completed,
        RunState::Failed,
        RunState::Killed,
    ];

    #[test]
    fn terminal_states_have_no_exits() {
        for from in [RunState::Completed, RunState::Failed, RunState::Killed] {
            for to in ALL {
                assert!(
                    !from.can_transition_to(to),
                    "{from} must not transition to {to}"
                );
            }
        }
    }

    #[test]
    fn transition_matrix_matches_the_documented_machine() {
        use RunState::*;
        let legal: &[(RunState, RunState)] = &[
            (Queued, Starting),
            (Queued, Failed),
            (Queued, Killed),
            (Starting, Running),
            (Starting, Completed),
            (Starting, Failed),
            (Starting, Killed),
            (Running, WaitingOnBoss),
            (Running, Completed),
            (Running, Failed),
            (Running, Killed),
            (WaitingOnBoss, Running),
            (WaitingOnBoss, Completed),
            (WaitingOnBoss, Failed),
            (WaitingOnBoss, Killed),
        ];
        for from in ALL {
            for to in ALL {
                let expected = legal.contains(&(from, to));
                assert_eq!(
                    from.can_transition_to(to),
                    expected,
                    "transition {from} -> {to}"
                );
            }
        }
    }

    #[test]
    fn state_names_round_trip_through_parse() {
        for s in ALL {
            let parsed: RunState = s.as_str().parse().expect("round trip");
            assert_eq!(parsed, s);
        }
        assert!("bogus".parse::<RunState>().is_err());
    }

    #[test]
    fn state_serde_uses_snake_case_strings() {
        let json = serde_json::to_string(&RunState::WaitingOnBoss).expect("serializes");
        assert_eq!(json, "\"waiting_on_boss\"");
    }
}
