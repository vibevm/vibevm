//! Truthful generated report construction; errors retain observed effects.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use super::{ApplicationValidationError as Fault, validation::hex64};
use vibe_wire::generated::migration::e1::operation_report::*;

/// Internal construction failure retains all observed evidence for the owner.
/// It is not another serialized machine document or semantic autocorrection.
#[derive(Debug, thiserror::Error)]
#[error("{fault}")]
pub struct ReportValidationFailure {
    pub report: OperationReport,
    pub fault: Fault,
}

pub struct ReportBuilder {
    pub(super) report: OperationReport,
}
impl ReportBuilder {
    pub fn new(operation: Option<OperationReportOperation>, operation_id: Option<String>) -> Self {
        Self {
            report: OperationReport {
                schema: 1,
                operation,
                operation_id,
                outcome: OperationReportOutcome::Refused,
                target_state: OperationReportTargetState::Unavailable,
                before_content_id: None,
                after_content_id: None,
                diff_id: None,
                intent_id: None,
                error: None,
                effects: Vec::new(),
                warnings: Vec::new(),
                residue: Vec::new(),
                guarantees: Guarantees {
                    platform: platform(),
                    concurrency: GuaranteesConcurrency::Unestablished,
                    barrier_binding: None,
                    uncooperating_writers_excluded: false,
                    durability: GuaranteesDurability::Unconfirmed,
                    preservation: GuaranteesPreservation::BytesOnly,
                },
            },
        }
    }
    /// Runtime may populate ONLY measured evidence. The final semantic validator
    /// checks coherence; mutable access does not confer any filesystem authority.
    pub fn evidence_mut(&mut self) -> &mut OperationReport {
        &mut self.report
    }
    pub fn error(&mut self, error: ApplicationError, outcome: OperationReportOutcome) {
        self.report.error = Some(error);
        self.report.outcome = outcome;
        // Effects/warnings/residue survive failures, including publication errors.
    }
    pub fn into_report(self) -> Result<OperationReport, ReportValidationFailure> {
        match validate_operation_report(&self.report) {
            Ok(()) => Ok(self.report),
            Err(fault) => Err(ReportValidationFailure {
                report: self.report,
                fault,
            }),
        }
    }
    pub fn preflight(error: &Fault) -> OperationReport {
        let mut result = Self::new(error.operation.clone(), error.operation_id.clone());
        result.report.error = Some(error.generated_error());
        result.report
    }
}
fn platform() -> GuaranteesPlatform {
    if cfg!(target_os = "linux") {
        GuaranteesPlatform::Linux
    } else if cfg!(target_os = "macos") {
        GuaranteesPlatform::Macos
    } else if cfg!(windows) {
        GuaranteesPlatform::Windows
    } else {
        GuaranteesPlatform::Other
    }
}
fn invalid(field: &'static str, reason: &'static str) -> Fault {
    Fault::new(ApplicationErrorCode::InvalidRequest, field, reason)
}

/// Logical result consistency, not a substitute for native evidence. In
/// particular a refused/uncertain report may retain completed earlier effects.
pub fn validate_operation_report(r: &OperationReport) -> Result<(), Fault> {
    if r.schema != 1 {
        return Err(invalid("schema", "unsupported operation report schema"));
    }
    if r.guarantees.uncooperating_writers_excluded {
        return Err(invalid(
            "guarantees.uncooperating_writers_excluded",
            "protocol never excludes uncooperating writers",
        ));
    }
    if r.operation_id
        .as_ref()
        .is_some_and(|id| id.trim().is_empty())
    {
        return Err(invalid(
            "operation_id",
            "established operation id must be nonblank",
        ));
    }
    for id in [
        &r.intent_id,
        &r.before_content_id,
        &r.after_content_id,
        &r.diff_id,
    ]
    .into_iter()
    .flatten()
    {
        if !hex64(id) {
            return Err(invalid(
                "identities",
                "established identity is not canonical lowercase hex",
            ));
        }
    }
    let ids = [&r.before_content_id, &r.after_content_id, &r.diff_id];
    if ids.iter().any(|id| id.is_some()) && ids.iter().any(|id| id.is_none()) {
        return Err(invalid(
            "identities",
            "planner identities must be established together",
        ));
    }
    if matches!(
        r.target_state,
        OperationReportTargetState::ApprovedBefore | OperationReportTargetState::ValidatedAfter
    ) && ids.iter().any(|id| id.is_none())
    {
        return Err(invalid(
            "identities",
            "recognized target bytes require all planner identities",
        ));
    }
    match r.guarantees.concurrency {
        GuaranteesConcurrency::Unestablished if r.guarantees.barrier_binding.is_some() => {
            return Err(invalid(
                "guarantees.barrier_binding",
                "unestablished concurrency cannot claim a barrier",
            ));
        }
        GuaranteesConcurrency::ParticipatingClients
            if r.guarantees
                .barrier_binding
                .as_ref()
                .is_none_or(|s| s.trim().is_empty()) =>
        {
            return Err(invalid(
                "guarantees.barrier_binding",
                "participating concurrency requires a barrier binding",
            ));
        }
        _ => {}
    }
    let any_effect = r.effects.iter().any(|e| e.state != EffectState::NotStarted);
    if !any_effect && r.guarantees.durability != GuaranteesDurability::Unconfirmed {
        return Err(invalid(
            "guarantees.durability",
            "no observed boundary can establish durability",
        ));
    }
    for (index, effect) in r.effects.iter().enumerate() {
        if r.effects[..index].iter().any(|e| e.kind == effect.kind) {
            return Err(invalid("effects", "effect kind appears more than once"));
        }
        if matches!(
            effect.kind,
            EffectKind::TargetPublication | EffectKind::TargetRestore
        ) && effect.state != EffectState::NotStarted
            && (r.intent_id.is_none()
                || r.before_content_id.is_none()
                || r.guarantees.concurrency != GuaranteesConcurrency::ParticipatingClients)
        {
            return Err(invalid(
                "effects",
                "target effect requires retained intent and participating barrier",
            ));
        }
    }
    if !matches!(r.outcome, OperationReportOutcome::Refused)
        && (r.operation.is_none() || r.operation_id.is_none())
    {
        return Err(invalid(
            "operation",
            "non-preflight outcome requires recognized operation identity",
        ));
    }
    if matches!(
        r.outcome,
        OperationReportOutcome::Completed | OperationReportOutcome::NoOp
    ) {
        if r.error.is_some()
            || r.effects
                .iter()
                .any(|e| matches!(e.state, EffectState::Pending | EffectState::Uncertain))
        {
            return Err(invalid(
                "outcome",
                "successful outcome cannot carry error or unresolved effects",
            ));
        }
        if !matches!(
            r.target_state,
            OperationReportTargetState::ApprovedBefore | OperationReportTargetState::ValidatedAfter
        ) {
            return Err(invalid(
                "target_state",
                "success requires verified recognized target bytes",
            ));
        }
        // Completed target effects can describe the VERIFIED retained prefix,
        // while this replay performs no new target write. Runtime proves that.
        if r.outcome == OperationReportOutcome::NoOp
            && r.effects.iter().any(|e| {
                e.state == EffectState::Completed
                    && match e.kind {
                        EffectKind::TargetPublication => {
                            r.target_state != OperationReportTargetState::ValidatedAfter
                        }
                        EffectKind::TargetRestore => {
                            r.target_state != OperationReportTargetState::ApprovedBefore
                        }
                        _ => false,
                    }
            })
        {
            return Err(invalid(
                "effects",
                "no_op retained target effect does not match verified target state",
            ));
        }
    }
    if r.effects.iter().any(|e| e.state == EffectState::Uncertain)
        && r.outcome != OperationReportOutcome::Uncertain
    {
        return Err(invalid(
            "outcome",
            "uncertain effect requires uncertain outcome",
        ));
    }
    for residue in &r.residue {
        if vibe_safefs::split_relative(&residue.relative_path).is_ok() {
            continue;
        }
        // Only a deterministic stage for THIS established intent is reportable.
        // Reporting it, including unproven residue, grants no removal authority.
        let Some(intent) = &r.intent_id else {
            return Err(invalid(
                "residue.relative_path",
                "residue path is not portable",
            ));
        };
        let (parent, leaf) = residue
            .relative_path
            .rsplit_once('/')
            .map_or((None, residue.relative_path.as_str()), |(parent, leaf)| {
                (Some(parent), leaf)
            });
        let expected = [
            super::identity::StageAction::Apply,
            super::identity::StageAction::Restore,
        ]
        .iter()
        .any(|action| leaf == super::identity::transaction_stage_name(intent, *action));
        if residue.location != ResidueLocation::Project || !expected {
            return Err(invalid(
                "residue.relative_path",
                "residue is not this intent's project stage",
            ));
        }
        if let Some(parent) = parent {
            vibe_safefs::split_relative(&format!("{parent}/stage-parent-check"))
                .map_err(|_| invalid("residue.relative_path", "stage parent is not portable"))?;
        }
    }
    Ok(())
}
