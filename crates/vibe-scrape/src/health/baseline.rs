//! Strict and structured no-regression judgment.

use std::collections::BTreeMap;

use super::model::*;

pub fn judge(
    baseline: BaselinePolicy,
    before: &PhaseHealthResult,
    after: &PhaseHealthResult,
) -> BaselineDecision {
    if before.phase != HealthPhase::Before {
        return BaselineDecision::RefuseBefore;
    }
    if after.phase != HealthPhase::After
        || before.plan_id != after.plan_id
        || !same_panel(before, after)
    {
        return BaselineDecision::RollbackAfter;
    }
    match baseline {
        BaselinePolicy::Strict => judge_strict(before, after),
        BaselinePolicy::NoRegression => judge_no_regression(before, after),
    }
}

fn judge_strict(before: &PhaseHealthResult, after: &PhaseHealthResult) -> BaselineDecision {
    if !before.checks.iter().all(strictly_acceptable) {
        return BaselineDecision::RefuseBefore;
    }
    if !after.checks.iter().all(strictly_acceptable) {
        return BaselineDecision::RollbackAfter;
    }
    if reduced(before) || reduced(after) {
        BaselineDecision::AcceptReduced
    } else {
        BaselineDecision::AcceptFull
    }
}

fn judge_no_regression(before: &PhaseHealthResult, after: &PhaseHealthResult) -> BaselineDecision {
    let mut compared = 0_usize;
    for (before, after) in before.checks.iter().zip(&after.checks) {
        match (&before.state, &after.state) {
            (CheckState::Skipped { reason: left }, CheckState::Skipped { reason: right })
                if left == right => {}
            (
                CheckState::Completed(HealthVerdict::Structured(left)),
                CheckState::Completed(HealthVerdict::Structured(right)),
            ) => {
                compared += 1;
                if !structured_no_regression(left, right) {
                    return BaselineDecision::RollbackAfter;
                }
            }
            // Opaque built-in/exit results and applicability changes cannot be
            // turned into a no-regression baseline.
            (CheckState::Skipped { .. }, _) => return BaselineDecision::RollbackAfter,
            (CheckState::Completed(HealthVerdict::Structured(_)), _) => {
                return BaselineDecision::RollbackAfter;
            }
            _ => return BaselineDecision::RefuseBefore,
        }
    }
    if compared == 0 {
        return BaselineDecision::RefuseBefore;
    }
    if reduced(before)
        || reduced(after)
        || after.checks.iter().any(|check| {
            matches!(
                &check.state,
                CheckState::Completed(HealthVerdict::Structured(value))
                    if value.status != HealthStatus::Pass
            )
        })
    {
        BaselineDecision::AcceptReduced
    } else {
        BaselineDecision::AcceptFull
    }
}

fn structured_no_regression(before: &StructuredVerdict, after: &StructuredVerdict) -> bool {
    if before.status == HealthStatus::Pass && after.status != HealthStatus::Pass {
        return false;
    }
    let before_findings = before
        .findings
        .iter()
        .map(|finding| (finding.id.as_str(), finding.severity))
        .collect::<BTreeMap<_, _>>();
    after.findings.iter().all(|finding| {
        before_findings
            .get(finding.id.as_str())
            .is_some_and(|before| finding.severity <= *before)
    })
}

fn strictly_acceptable(check: &CheckResult) -> bool {
    match &check.state {
        CheckState::Skipped { .. } => true,
        CheckState::Completed(HealthVerdict::Pass) => true,
        CheckState::Completed(HealthVerdict::Structured(value)) => {
            value.status == HealthStatus::Pass
        }
    }
}

fn reduced(phase: &PhaseHealthResult) -> bool {
    phase.assurance_reduced
        || phase
            .checks
            .iter()
            .any(|check| matches!(check.state, CheckState::Skipped { .. }))
}

fn same_panel(before: &PhaseHealthResult, after: &PhaseHealthResult) -> bool {
    before.checks.len() == after.checks.len()
        && before
            .checks
            .iter()
            .zip(&after.checks)
            .all(|(left, right)| left.id == right.id)
}
