//! Bounded observations, never a second outcome journal or an effect authority.
use super::*;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

const ROWS: usize = 32;
const BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceBasis {
    LiveCallbacks,
    RetainedReport,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedPhase {
    phase: String,
    status: Option<String>,
}
impl ObservedPhase {
    pub fn phase(&self) -> &str {
        &self.phase
    }
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }
}
/// A handler's terminal WORK outcome. `ok` does not prove filesystem effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedWork {
    key: String,
    phase: String,
    status: String,
    provider: String,
    handler: String,
}
impl ObservedWork {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn phase(&self) -> &str {
        &self.phase
    }
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn provider(&self) -> &str {
        &self.provider
    }
    pub fn handler(&self) -> &str {
        &self.handler
    }
}

/// Best-effort read of one owner. Status is sampled before the ledger and may
/// race callback/terminal publication. No atomic cross-run snapshot is claimed.
/// Only `status` carries the freshly validated marker's domain effect truth.
/// No constructor, serialization, or capability promotion is provided.
#[derive(Debug, Clone)]
pub struct FixedOwnerEvidenceSnapshot {
    operation_id: &'static str,
    requested_scope: SubmissionScope,
    requested_phase: Phase,
    bound_scope: Option<SubmissionScope>,
    bound_run: Option<String>,
    status: Option<FixedOperationStatus>,
    inspection_unavailable: Option<String>,
    inspection_diagnostic_truncated: bool,
    basis: EvidenceBasis,
    last_phase: Option<ObservedPhase>,
    phase_unavailable: Option<&'static str>,
    phases: Vec<ObservedPhase>,
    work: Vec<ObservedWork>,
    known_total: Option<u64>,
    observed_terminals: Option<u64>,
    partial: bool,
}
impl FixedOwnerEvidenceSnapshot {
    pub fn requested_scope(&self) -> &SubmissionScope {
        &self.requested_scope
    }
    pub fn requested_phase(&self) -> Phase {
        self.requested_phase
    }
    pub fn operation_id(&self) -> &'static str {
        self.operation_id
    }
    pub fn bound_scope(&self) -> Option<&SubmissionScope> {
        self.bound_scope.as_ref()
    }
    pub fn bound_run_id(&self) -> Option<&str> {
        self.bound_run.as_deref()
    }
    pub fn status(&self) -> Option<&FixedOperationStatus> {
        self.status.as_ref()
    }
    pub fn inspection_unavailable(&self) -> Option<&str> {
        self.inspection_unavailable.as_deref()
    }
    pub fn inspection_diagnostic_truncated(&self) -> bool {
        self.inspection_diagnostic_truncated
    }
    pub fn basis(&self) -> EvidenceBasis {
        self.basis
    }
    pub fn last_observed_phase(&self) -> Option<&ObservedPhase> {
        self.last_phase.as_ref()
    }
    pub fn phase_unavailable(&self) -> Option<&str> {
        self.phase_unavailable
    }
    pub fn phases(&self) -> &[ObservedPhase] {
        &self.phases
    }
    pub fn work_outcomes(&self) -> &[ObservedWork] {
        &self.work
    }
    pub fn known_contribution_total(&self) -> Option<u64> {
        self.known_total
    }
    /// Real terminal callback count; unavailable for a cold retained report.
    pub fn observed_terminal_work_count(&self) -> Option<u64> {
        self.observed_terminals
    }
    /// Rows/counts may be incomplete; a failed report never proves a full prefix.
    pub fn partial(&self) -> bool {
        self.partial
    }
}

#[derive(Clone)]
pub(super) struct Ledger {
    pub(super) total: Option<u64>,
    terminals: Option<u64>,
    callbacks: bool,
    last: Option<ObservedPhase>,
    phase_unavailable: Option<&'static str>,
    phases: Vec<ObservedPhase>,
    work: Vec<ObservedWork>,
    bytes: usize,
    partial: bool,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            total: None,
            terminals: Some(0),
            callbacks: false,
            last: None,
            phase_unavailable: Some("no phase callback or validated report step"),
            phases: Vec::new(),
            work: Vec::new(),
            bytes: 0,
            partial: false,
        }
    }
}
fn valid(value: &str, cap: usize) -> bool {
    !value.is_empty() && value.len() <= cap && !value.chars().any(char::is_control)
}
impl Ledger {
    fn reserve(&mut self, bytes: usize) -> bool {
        if self.phases.len() + self.work.len() >= ROWS || bytes > BYTES - 384 - self.bytes {
            self.partial = true;
            return false;
        }
        self.bytes += bytes;
        true
    }
    pub(super) fn phase(&mut self, phase: &str, status: Option<&str>) {
        self.callbacks = true;
        if !valid(phase, 256) {
            self.last = None;
            self.phase_unavailable = Some("invalid or overlong observed phase");
            self.partial = true;
            return;
        }
        let status = match status {
            Some(value) if !valid(value, 128) => {
                self.partial = true;
                None
            }
            other => other,
        };
        // Last phase is independently bounded, even after row/identity loss.
        self.last = Some(ObservedPhase {
            phase: phase.to_owned(),
            status: status.map(str::to_owned),
        });
        self.phase_unavailable = None;
        if self.reserve(phase.len() + status.map_or(0, str::len)) {
            self.phases.push(self.last.as_ref().unwrap().clone());
        }
    }
    pub(super) fn terminal(&mut self, report: &LifecycleContributionReport) {
        self.callbacks = true;
        self.terminals = self.terminals.and_then(|value| value.checked_add(1));
        if self.terminals.is_none()
            || self
                .total
                .zip(self.terminals)
                .is_some_and(|(total, count)| count > total)
        {
            self.partial = true;
        }
        self.row(report);
    }
    fn row(&mut self, row: &LifecycleContributionReport) {
        let fields = [
            (&row.key, 1024),
            (&row.phase, 256),
            (&row.status, 128),
            (&row.provider, 1024),
            (&row.handler, 1024),
        ];
        if fields.iter().any(|(value, cap)| !valid(value, *cap)) {
            self.partial = true;
            return;
        }
        let bytes = fields.iter().map(|(value, _)| value.len()).sum();
        if self.reserve(bytes) {
            self.work.push(ObservedWork {
                key: row.key.clone(),
                phase: row.phase.clone(),
                status: row.status.clone(),
                provider: row.provider.clone(),
                handler: row.handler.clone(),
            });
        }
    }
}

pub(super) fn sample(
    shared: &Arc<Shared>,
    scope: &SubmissionScope,
    inspection: Result<FixedOperationStatus>,
) -> FixedOwnerEvidenceSnapshot {
    let binding = shared.record.lock().unwrap().binding.clone();
    let ledger = shared.ledger.lock().unwrap().clone();
    // A verified cold replay/recovery can establish the C1 subject without
    // minting a fresh live Ready binding or granting control.
    let known_run = binding
        .as_ref()
        .map(|b| b.run_id.clone())
        .or_else(|| match &inspection {
            Ok(FixedOperationStatus::Running { run_id })
            | Ok(FixedOperationStatus::ResultAvailable { run_id, .. })
            | Ok(FixedOperationStatus::RecoveryRequired {
                run_id: Some(run_id),
                ..
            }) => Some(run_id.clone()),
            _ => None,
        });
    let known_scope = binding
        .as_ref()
        .map(|b| b.coordinates.scope.clone())
        .or_else(|| known_run.as_ref().map(|_| scope.clone()));
    project(
        FIXED_OPERATION,
        scope,
        shared.requested_phase,
        known_scope,
        known_run,
        ledger,
        inspection,
    )
}

// Only the trusted cold owner calls this after incumbent C1/capsule/marker inspection.
pub(in crate::machine_submission) fn cold_snapshot(
    scope: &SubmissionScope,
    status: FixedOperationStatus,
) -> Result<FixedOwnerEvidenceSnapshot> {
    cold_snapshot_for_phase(scope, Phase::Validate, FIXED_OPERATION, status)
}

pub(in crate::machine_submission) fn cold_snapshot_for_phase(
    scope: &SubmissionScope,
    phase: Phase,
    operation_id: &'static str,
    status: FixedOperationStatus,
) -> Result<FixedOwnerEvidenceSnapshot> {
    ensure!(
        (phase == Phase::Validate && operation_id == FIXED_OPERATION)
            || (phase == Phase::Install
                && operation_id == "rust:vibe-lifecycle:install-exact-local"),
        "cold evidence operation/phase mismatch"
    );
    if let FixedOperationStatus::ResultAvailable {
        report_bytes: Some(bytes),
        ..
    } = &status
    {
        let report: LifecycleReport = serde_json::from_slice(bytes)?;
        ensure!(
            report.command == "lifecycle" && report.requested == phase.as_str(),
            "cold evidence refuses a different lifecycle phase"
        );
    }
    let run = match &status {
        FixedOperationStatus::ResultAvailable { run_id, .. }
        | FixedOperationStatus::RecoveryRequired {
            run_id: Some(run_id),
            ..
        } => Some(run_id.clone()),
        _ => None,
    };
    Ok(project(
        operation_id,
        scope,
        phase,
        run.as_ref().map(|_| scope.clone()),
        run,
        Ledger::default(),
        Ok(status),
    ))
}

// Stop formatting at the UTF-8 byte cap, before an unbounded allocation.
fn diagnostic(error: &anyhow::Error) -> (String, bool) {
    use std::fmt::Write;
    struct Bounded {
        text: String,
        truncated: bool,
    }
    impl std::fmt::Write for Bounded {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let remaining = 4096 - self.text.len();
            if value.len() <= remaining {
                self.text.push_str(value);
                return Ok(());
            }
            let mut end = remaining;
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            self.text.push_str(&value[..end]);
            self.truncated = true;
            Err(std::fmt::Error)
        }
    }
    let mut writer = Bounded {
        text: String::new(),
        truncated: false,
    };
    let _ = write!(&mut writer, "{error:#}");
    (writer.text, writer.truncated)
}
fn project(
    operation_id: &'static str,
    scope: &SubmissionScope,
    requested_phase: Phase,
    bound_scope: Option<SubmissionScope>,
    bound_run: Option<String>,
    mut ledger: Ledger,
    inspection: Result<FixedOperationStatus>,
) -> FixedOwnerEvidenceSnapshot {
    let (status, inspection_unavailable, inspection_diagnostic_truncated) = match inspection {
        Ok(status) => (Some(status), None, false),
        Err(error) => {
            let (cause, truncated) = diagnostic(&error);
            (None, Some(cause), truncated)
        }
    };
    let mut basis = if ledger.callbacks {
        EvidenceBasis::LiveCallbacks
    } else {
        EvidenceBasis::Unavailable
    };
    // Only bytes returned by successful incumbent inspection are decoded here.
    // They are transient projections: neither marker nor report is cached.
    if !ledger.callbacks {
        if let Some(FixedOperationStatus::ResultAvailable {
            report_bytes: Some(bytes),
            marker,
            ..
        }) = &status
        {
            if let Ok(report) = serde_json::from_slice::<LifecycleReport>(bytes) {
                basis = EvidenceBasis::RetainedReport;
                for step in &report.steps {
                    ledger.phase(&step.phase, Some(&step.status));
                }
                for row in &report.contributions {
                    ledger.row(row);
                }
                ledger.terminals = None;
                ledger.total = None; // Report rows are not a measured execution-plan total.
                ledger.partial |= marker.outcome != MarkerOutcome::Completed;
            }
        }
    }
    let final_available = matches!(&status, Some(FixedOperationStatus::ResultAvailable { marker, .. }) if marker.outcome == MarkerOutcome::Completed);
    let partial = !final_available
        || ledger.partial
        || basis == EvidenceBasis::Unavailable
        || (basis == EvidenceBasis::LiveCallbacks
            && ledger
                .total
                .zip(ledger.terminals)
                .is_none_or(|(total, count)| total != count));
    FixedOwnerEvidenceSnapshot {
        operation_id,
        requested_scope: scope.clone(),
        requested_phase,
        bound_scope,
        bound_run,
        status,
        inspection_unavailable,
        inspection_diagnostic_truncated,
        basis,
        last_phase: ledger.last,
        phase_unavailable: ledger.phase_unavailable,
        phases: ledger.phases,
        work: ledger.work,
        known_total: ledger.total,
        observed_terminals: if basis == EvidenceBasis::LiveCallbacks {
            ledger.terminals
        } else {
            None
        },
        partial,
    }
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod install_operation_identity_tests {
    use super::*;

    #[test]
    fn unsealed_install_snapshot_cannot_claim_exact_local_operation() {
        let scope = SubmissionScope {
            context: "context".into(),
            target: "target".into(),
            principal: "principal".into(),
        };
        let unsealed = project(
            FIXED_OPERATION,
            &scope,
            Phase::Install,
            None,
            None,
            Ledger::default(),
            Ok(FixedOperationStatus::Starting),
        );
        assert_eq!(unsealed.operation_id(), FIXED_OPERATION);
        assert!(crate::machine_operation::lifecycle_view::project_snapshot(&unsealed).is_err());
        assert!(
            cold_snapshot_for_phase(
                &scope,
                Phase::Validate,
                "rust:vibe-lifecycle:install-exact-local",
                FixedOperationStatus::Starting,
            )
            .is_err()
        );
    }
}
