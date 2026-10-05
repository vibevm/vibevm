//! Sampled evidence only: matching values never reserve, renew, dispatch or grant.
//! A borrowed immutable snapshot describes its sample, not current filesystem
//! freshness. A service must query the incumbent afresh before publication.
use crate::machine_submission::{EvidenceBasis, FixedOperationStatus, FixedOwnerEvidenceSnapshot};
use std::{fmt, io::Write};
use vibe_wire::generated::lifecycle_outcome_marker as marker;
pub use vibe_wire::generated::machine::e1::lifecycle_submission_view::*;

pub const MAX_FRAME_BYTES: usize = 128 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    FrameLimit,
    InvalidShape,
    UnsupportedEpoch,
    InvalidSource(&'static str),
    SnapshotMismatch,
}
impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifecycle view refused: {self:?}")
    }
}
impl std::error::Error for ViewError {}
type Check<T = ()> = Result<T, ViewError>;
fn require(ok: bool, reason: &'static str) -> Check {
    if ok {
        Ok(())
    } else {
        Err(ViewError::InvalidSource(reason))
    }
}
fn bounded(s: &str, cap: usize) -> Check {
    require(
        !s.is_empty() && s.len() <= cap && !s.chars().any(char::is_control),
        "identity string bound",
    )
}
fn copy(s: &str, cap: usize) -> Check<String> {
    bounded(s, cap)?;
    Ok(s.to_owned())
}
fn scope(s: &vibe_lifecycle::SubmissionScope) -> Check<Scope> {
    Ok(Scope {
        context: copy(&s.context, 256)?,
        target: copy(&s.target, 256)?,
        principal: copy(&s.principal, 256)?,
    })
}
fn phase(p: &crate::machine_submission::ObservedPhase) -> Check<Phase> {
    Ok(Phase {
        phase: copy(p.phase(), 256)?,
        status: p.status().map(|s| copy(s, 128)).transpose()?,
    })
}
fn diagnostic(s: &str, already_truncated: bool) -> (String, bool) {
    let mut end = s.len().min(4096);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_owned(), already_truncated || end < s.len())
}
fn hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn run(s: &str) -> Check {
    require(hex(s, 32), "run ID")
}
fn size(v: &LifecycleSubmissionView) -> Check<usize> {
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_FRAME_BYTES)
                .ok_or_else(|| std::io::Error::other("view frame cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut c = Counter(0);
    serde_json::to_writer(&mut c, v).map_err(|_| ViewError::FrameLimit)?;
    Ok(c.0)
}
/// Projection failure leaves the original borrowed snapshot with its caller.
/// No report bytes, stdout or error payloads are copied into the view.
pub fn project_snapshot(s: &FixedOwnerEvidenceSnapshot) -> Check<LifecycleSubmissionView> {
    require(
        matches!(
            (s.requested_phase().as_str(), s.operation_id()),
            ("validate", "rust:vibe-lifecycle:validate")
                | ("install", "rust:vibe-lifecycle:install-exact-local")
        ),
        "operation identity",
    )?;
    require(s.phases().len() + s.work_outcomes().len() <= 32, "row cap")?;
    // Validate all source identities and their aggregate storage before cloning.
    let mut identity_bytes = 0usize;
    for scope in std::iter::once(s.requested_scope()).chain(s.bound_scope()) {
        for value in [&scope.context, &scope.target, &scope.principal] {
            bounded(value, 256)?;
        }
    }
    bounded(s.requested_phase().as_str(), 256)?;
    if let Some(reason) = s.phase_unavailable() {
        bounded(reason, 4096)?;
    }
    for p in s.phases().iter().chain(s.last_observed_phase()) {
        bounded(p.phase(), 256)?;
        identity_bytes += p.phase().len();
        if let Some(status) = p.status() {
            bounded(status, 128)?;
            identity_bytes += status.len();
        }
    }
    for w in s.work_outcomes() {
        for (value, cap) in [
            (w.key(), 1024),
            (w.phase(), 256),
            (w.status(), 128),
            (w.provider(), 1024),
            (w.handler(), 1024),
        ] {
            bounded(value, cap)?;
            identity_bytes += value.len();
        }
    }
    require(identity_bytes <= 32 * 1024, "identity storage cap")?;
    if let Some(id) = s.bound_run_id() {
        run(id)?;
    }
    let status = match (s.status(), s.inspection_unavailable()) {
        (None, Some(reason)) => {
            require(!reason.is_empty(), "empty inspection diagnostic")?;
            let (reason, reason_truncated) =
                diagnostic(reason, s.inspection_diagnostic_truncated());
            Status::InspectionUnavailable(Box::new(StatusInspectionUnavailable {
                reason,
                reason_truncated,
            }))
        }
        (Some(FixedOperationStatus::Starting), None) => {
            // The owner samples status before binding/ledger. Ready may bind a
            // real C1 run between those reads; preserve it without promoting
            // the sampled Starting status to execution or terminal evidence.
            Status::Starting(Box::new(StatusStarting {}))
        }
        (Some(FixedOperationStatus::Running { run_id }), None) => {
            run(run_id)?;
            require(s.bound_run_id() == Some(run_id.as_str()), "running binding")?;
            Status::Running(Box::new(StatusRunning {}))
        }
        (Some(FixedOperationStatus::RecoveryRequired { run_id, reason }), None) => {
            require(run_id.as_deref() == s.bound_run_id(), "recovery binding")?;
            require(!reason.is_empty(), "empty recovery diagnostic")?;
            let (reason, reason_truncated) = diagnostic(reason, false);
            Status::RecoveryRequired(Box::new(StatusRecoveryRequired {
                reason,
                reason_truncated,
            }))
        }
        (
            Some(FixedOperationStatus::ResultAvailable {
                run_id, marker: m, ..
            }),
            None,
        ) => {
            run(run_id)?;
            require(
                m.schema == 1 && m.run_id == *run_id && s.bound_run_id() == Some(run_id.as_str()),
                "marker run/epoch",
            )?;
            require(
                m.reservation_ref
                    .strip_prefix("sha256:")
                    .is_some_and(|v| hex(v, 64)),
                "reservation hash",
            )?;
            let report = match &m.report {
                marker::ReportEvidence::Available(r) => {
                    require(
                        hex(&r.sha256, 64) && r.bytes <= 8 * 1024 * 1024,
                        "report metadata",
                    )?;
                    Report::Available(Box::new(ReportAvailable {
                        sha256: r.sha256.clone(),
                        bytes: r.bytes,
                    }))
                }
                marker::ReportEvidence::Unavailable(r) => {
                    Report::Unavailable(Box::new(ReportUnavailable {
                        reason: match r.reason {
                            marker::ReportEvidenceUnavailableReason::NotProduced => {
                                ReportUnavailableReason::NotProduced
                            }
                            marker::ReportEvidenceUnavailableReason::PublicationUncertain => {
                                ReportUnavailableReason::PublicationUncertain
                            }
                            marker::ReportEvidenceUnavailableReason::ValidationFailed => {
                                ReportUnavailableReason::ValidationFailed
                            }
                            marker::ReportEvidenceUnavailableReason::PayloadUnavailable => {
                                ReportUnavailableReason::PayloadUnavailable
                            }
                        },
                    }))
                }
            };
            Status::ResultAvailable(Box::new(StatusResultAvailable {
                outcome: match m.outcome {
                    marker::LifecycleOutcomeMarkerOutcome::Completed => {
                        StatusResultAvailableOutcome::Completed
                    }
                    marker::LifecycleOutcomeMarkerOutcome::Failed => {
                        StatusResultAvailableOutcome::Failed
                    }
                    marker::LifecycleOutcomeMarkerOutcome::Parked => {
                        StatusResultAvailableOutcome::Parked
                    }
                    marker::LifecycleOutcomeMarkerOutcome::Uncertain => {
                        StatusResultAvailableOutcome::Uncertain
                    }
                },
                effect_state: match m.effect_state {
                    marker::LifecycleOutcomeMarkerEffectState::None => {
                        StatusResultAvailableEffectState::None
                    }
                    marker::LifecycleOutcomeMarkerEffectState::Landed => {
                        StatusResultAvailableEffectState::Landed
                    }
                    marker::LifecycleOutcomeMarkerEffectState::Uncertain => {
                        StatusResultAvailableEffectState::Uncertain
                    }
                },
                reservation_ref: m.reservation_ref.clone(),
                report,
            }))
        }
        _ => return Err(ViewError::InvalidSource("inspection/status coherence")),
    };
    require(
        s.bound_run_id().is_none() || s.bound_scope().is_some(),
        "known run scope",
    )?;
    require(
        s.last_observed_phase().is_some() != s.phase_unavailable().is_some(),
        "phase availability",
    )?;
    require(
        s.known_contribution_total()
            .zip(s.observed_terminal_work_count())
            .is_none_or(|(t, n)| n <= t || s.partial()),
        "counter coherence",
    )?;
    let requested_phase = s.requested_phase().as_str();
    let v = LifecycleSubmissionView {
        schema: 1,
        operation_id: s.operation_id().to_owned(),
        requested_scope: scope(s.requested_scope())?,
        requested_phase: copy(requested_phase, 256)?,
        bound_scope: s.bound_scope().map(scope).transpose()?,
        run_id: s.bound_run_id().map(str::to_owned),
        basis: match s.basis() {
            EvidenceBasis::LiveCallbacks => LifecycleSubmissionViewBasis::LiveCallbacks,
            EvidenceBasis::RetainedReport => LifecycleSubmissionViewBasis::RetainedReport,
            EvidenceBasis::Unavailable => LifecycleSubmissionViewBasis::Unavailable,
        },
        last_phase: s.last_observed_phase().map(phase).transpose()?,
        phase_unavailable_reason: s.phase_unavailable().map(|s| copy(s, 4096)).transpose()?,
        phases: s.phases().iter().map(phase).collect::<Check<_>>()?,
        work_outcomes: s
            .work_outcomes()
            .iter()
            .map(|w| {
                Ok(Work {
                    key: copy(w.key(), 1024)?,
                    phase: copy(w.phase(), 256)?,
                    status: copy(w.status(), 128)?,
                    provider: copy(w.provider(), 1024)?,
                    handler: copy(w.handler(), 1024)?,
                })
            })
            .collect::<Check<_>>()?,
        known_contribution_total: s.known_contribution_total().map(|n| n.to_string()),
        observed_terminal_work_count: s.observed_terminal_work_count().map(|n| n.to_string()),
        partial: s.partial(),
        status,
    };
    size(&v)?;
    Ok(v)
}
/// Publication compares every field with the actual opaque sample; generated
/// shape by itself proves neither completion nor absence of effects.
pub fn validate(v: &LifecycleSubmissionView, s: &FixedOwnerEvidenceSnapshot) -> Check {
    size(v)?;
    if v.schema != 1 {
        return Err(ViewError::UnsupportedEpoch);
    }
    if *v != project_snapshot(s)? {
        return Err(ViewError::SnapshotMismatch);
    }
    Ok(())
}
pub fn parse(bytes: &[u8], s: &FixedOwnerEvidenceSnapshot) -> Check<LifecycleSubmissionView> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ViewError::FrameLimit);
    }
    let v = serde_json::from_slice(bytes).map_err(|_| ViewError::InvalidShape)?;
    validate(&v, s)?;
    Ok(v)
}
pub fn serialize(v: &LifecycleSubmissionView, s: &FixedOwnerEvidenceSnapshot) -> Check<Vec<u8>> {
    validate(v, s)?;
    let mut bytes = Vec::with_capacity(size(v)?);
    serde_json::to_writer(&mut bytes, v).map_err(|_| ViewError::InvalidShape)?;
    Ok(bytes)
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
