//! Worker credibility on the boss surface (PP-002 / DEF-C2-2b-full).
//!
//! Ф6 measured that a cold boss's strongest lever on delegation is its
//! *belief* that workers can self-verify: with the staging toolchain broken,
//! bosses in both arms rationally kept all work ("workers can't self-verify
//! here"). This makes that belief **provable instead of asserted** — a fact
//! a boss surface can cite, backed by a real recorded acceptance and carrying
//! its age ("workers ran acceptance green here, last proven <when>").
//!
//! D7 binds: strictly factual. A credibility claim exists ONLY when a run
//! actually completed with its acceptance green; absent that, the surface
//! says nothing — it never invents credibility. This cell is the query (the
//! "compute the fact" half); the surface wiring that renders it on the
//! scoreboard / cold board / mid-work nudge is a following slice, and reads
//! this — one telemetry source, no shadow store (I3).

use serde::{Deserialize, Serialize};

use crate::run::{RunRecord, RunState};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// A dated, factual proof that workers can self-verify here: distilled from
/// the most recent run whose acceptance ran and passed in full. The surface
/// renders the age from `proven_ts_ms` ("last proven <when>"); this carries
/// the raw timestamp, never a verdict on staleness — that is the surface's
/// call (the PP-002 staleness rule lives where the fact is shown, not here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredibilityFact {
    /// When the green acceptance was proven — the proving run's
    /// `updated_ts_ms` (its completion moment).
    pub proven_ts_ms: u64,
    /// The routing profile whose worker proved it (e.g. `glm`).
    pub profile: String,
    /// The acceptance that passed: `acceptance_passed == acceptance_total`,
    /// `acceptance_total > 0`.
    pub acceptance_passed: u32,
    pub acceptance_total: u32,
}

/// The worker-credibility fact from a run snapshot (PP-002): the most recent
/// COMPLETED run whose acceptance ran and passed IN FULL
/// (`acceptance_total > 0 && acceptance_passed == acceptance_total`). `None`
/// when nothing proves it — the surface then shows no credibility line (D7:
/// never assert what a recorded fact does not back). "Green" is deliberately
/// *full* acceptance: a partially-passing run has not proven self-verification.
/// Pure over the snapshot; total (no panic), the caller renders the age.
pub fn worker_credibility(runs: &[RunRecord]) -> Option<CredibilityFact> {
    runs.iter()
        .filter(|r| r.state == RunState::Completed)
        .filter_map(|r| r.collected.as_ref().map(|c| (r, c)))
        .filter(|(_, c)| c.acceptance_total > 0 && c.acceptance_passed == c.acceptance_total)
        .max_by_key(|(r, _)| r.updated_ts_ms)
        .map(|(r, c)| CredibilityFact {
            proven_ts_ms: r.updated_ts_ms,
            profile: r.profile.clone(),
            acceptance_passed: c.acceptance_passed,
            acceptance_total: c.acceptance_total,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{BudgetSpec, WorkspaceMode};
    use crate::run::{Collected, UsageTotals};

    /// A run with the fields the credibility query reads; the rest are inert
    /// defaults (mirrors the journal_fold test builder).
    fn run(id: &str, state: RunState, ts: u64, accept: Option<(u32, u32)>) -> RunRecord {
        RunRecord {
            run_id: id.parse().expect("ulid"),
            title: "t".into(),
            state,
            profile: "glm".into(),
            model: "small".into(),
            workspace_mode: WorkspaceMode::Dir,
            parent: None,
            origin_session: None,
            depth: 0,
            spawn_requested: true,
            verifier: false,
            advice: false,
            budget: BudgetSpec::default(),
            node_id: "n".into(),
            run_dir: "runs/x".into(),
            created_ts_ms: ts,
            updated_ts_ms: ts,
            started_ts_ms: None,
            pod: None,
            worker_pid: None,
            exit_code: None,
            failure: None,
            kill_reason: None,
            usage: UsageTotals::default(),
            collected: accept.map(|(passed, total)| Collected {
                result_source: "worker".into(),
                result: None,
                result_path: None,
                acceptance_passed: passed,
                acceptance_total: total,
                acceptance_skipped: None,
            }),
            question: None,
            answer: None,
            escalation: None,
        }
    }

    const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
    const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA2";
    const C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA3";

    #[test]
    fn no_runs_means_no_credibility() {
        assert_eq!(worker_credibility(&[]), None);
    }

    #[test]
    fn only_a_completed_full_green_run_proves_it() {
        // Completed but no acceptance (total 0) → not proof.
        assert_eq!(
            worker_credibility(&[run(A, RunState::Completed, 10, Some((0, 0)))]),
            None,
            "no acceptance ran → nothing proven"
        );
        // Completed but only partial acceptance → not proof (D7: full green).
        assert_eq!(
            worker_credibility(&[run(A, RunState::Completed, 10, Some((1, 2)))]),
            None,
            "partial acceptance is not self-verification"
        );
        // Not completed, even with green acceptance → not proof.
        assert_eq!(
            worker_credibility(&[run(A, RunState::Failed, 10, Some((2, 2)))]),
            None,
            "a non-completed run never proves credibility"
        );
        // Completed + full green → the fact.
        let fact = worker_credibility(&[run(A, RunState::Completed, 10, Some((3, 3)))])
            .expect("a full-green completed run proves credibility");
        assert_eq!(fact.proven_ts_ms, 10);
        assert_eq!((fact.acceptance_passed, fact.acceptance_total), (3, 3));
        assert_eq!(fact.profile, "glm");
    }

    #[test]
    fn picks_the_most_recent_green_run_over_later_non_green() {
        let older_green = run(A, RunState::Completed, 10, Some((1, 1)));
        let newer_green = run(B, RunState::Completed, 30, Some((2, 2)));
        let newest_failed = run(C, RunState::Failed, 40, Some((5, 5))); // newer but not proof
        let fact = worker_credibility(&[older_green, newer_green, newest_failed])
            .expect("has a green run");
        assert_eq!(
            fact.proven_ts_ms, 30,
            "the most recent GREEN run wins — not the newer failed one"
        );
        assert_eq!(fact.acceptance_total, 2);
    }
}
