//! Explicit one-file application under a participating-client barrier.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");

use super::{
    ApplicationValidationError as Fault, OwnedManifestMigrationFile, ReportBuilder, ValidatedApply,
    ValidatedOperation, identity, validation,
};
use std::path::Path;
use vibe_safefs::{DirectoryDurability, DurableWrite, PublishError, PublishStage};
use vibe_wire::generated::migration::e1::{
    application_journal::{ApplicationJournal, ApplicationJournalPhase, Authority, AuthorityScope},
    operation_report::*,
};

#[path = "recovery.rs"]
mod recovery;
#[path = "storage.rs"]
mod storage;
use storage::{Store, Target};
pub(super) type Checked<T> = Result<T, Fault>;

pub(super) fn fault(
    code: ApplicationErrorCode,
    field: &'static str,
    reason: &'static str,
) -> Fault {
    Fault::new(code, field, reason)
}

#[cfg(test)]
thread_local! { static STOP_AT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
pub(super) fn stop_at(name: String) {
    STOP_AT.with(|slot| *slot.borrow_mut() = Some(name));
}
pub(super) fn boundary(name: &'static str) {
    #[cfg(test)]
    STOP_AT.with(|slot| {
        if slot.borrow().as_deref() == Some(name) {
            std::process::exit(73);
        }
    });
    #[cfg(not(test))]
    let _ = name;
}

pub(super) fn set_effect(b: &mut ReportBuilder, kind: EffectKind, state: EffectState) {
    let rows = &mut b.evidence_mut().effects;
    if let Some(row) = rows.iter_mut().find(|row| row.kind == kind) {
        if row.state != EffectState::Uncertain || state == EffectState::Uncertain {
            row.state = state;
        }
    } else {
        rows.push(Effect { kind, state });
    }
}
pub(super) fn warning(b: &mut ReportBuilder, code: WarningCode, action: WarningAction) {
    let row = Warning { code, action };
    if !b.evidence_mut().warnings.contains(&row) {
        b.evidence_mut().warnings.push(row);
    }
}
pub(super) fn residue(
    b: &mut ReportBuilder,
    location: ResidueLocation,
    relative_path: String,
    ownership: ResidueOwnership,
) {
    let row = Residue {
        location,
        relative_path,
        ownership,
    };
    if !b.evidence_mut().residue.contains(&row) {
        b.evidence_mut().residue.push(row);
    }
}
pub(super) fn effect_state(b: &mut ReportBuilder, kind: &EffectKind) -> EffectState {
    b.evidence_mut()
        .effects
        .iter()
        .find(|e| &e.kind == kind)
        .map_or(EffectState::NotStarted, |e| e.state.clone())
}
pub(super) fn unconfirmed(b: &mut ReportBuilder) {
    b.evidence_mut().guarantees.durability = GuaranteesDurability::Unconfirmed;
    warning(
        b,
        WarningCode::DurabilityUnconfirmed,
        WarningAction::Inspect,
    );
}
pub(super) fn observe_durability(b: &mut ReportBuilder, d: DirectoryDurability) -> Checked<()> {
    match d {
        DirectoryDurability::Synced => {
            let known_unconfirmed = b
                .evidence_mut()
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::DurabilityUnconfirmed);
            if b.evidence_mut().guarantees.durability == GuaranteesDurability::Unconfirmed
                && !known_unconfirmed
            {
                b.evidence_mut().guarantees.durability =
                    GuaranteesDurability::FileAndDirectorySynced;
            }
        }
        DirectoryDurability::JournalRecoverable | DirectoryDurability::Unsupported(_) => {
            let known_unconfirmed = b
                .evidence_mut()
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::DurabilityUnconfirmed);
            if !known_unconfirmed {
                b.evidence_mut().guarantees.durability = GuaranteesDurability::JournalRecoverable;
            }
            warning(b, WarningCode::DurabilityReduced, WarningAction::Inspect);
        }
        DirectoryDurability::Failed(_) => {
            b.evidence_mut().guarantees.durability = GuaranteesDurability::Unconfirmed;
            warning(
                b,
                WarningCode::DurabilityUnconfirmed,
                WarningAction::ManualResolution,
            );
            return Err(fault(
                ApplicationErrorCode::SynchronizationFailed,
                "durability",
                "an actual namespace synchronization failed",
            ));
        }
    }
    Ok(())
}
pub(super) fn observe_write(b: &mut ReportBuilder, w: &DurableWrite) -> Checked<()> {
    if !w.file_synced {
        b.evidence_mut().guarantees.durability = GuaranteesDurability::Unconfirmed;
        return Err(fault(
            ApplicationErrorCode::SynchronizationFailed,
            "durability",
            "file data synchronization was not confirmed",
        ));
    }
    observe_durability(b, w.parent)?;
    for sync in &w.directory_syncs {
        observe_durability(b, sync.durability)?;
    }
    Ok(())
}
pub(super) fn publication_failure(
    b: &mut ReportBuilder,
    kind: EffectKind,
    prior: EffectState,
    error: &PublishError,
    location: ResidueLocation,
    root: &Path,
) {
    set_effect(
        b,
        kind,
        if error.stage == PublishStage::PossiblyPublished {
            EffectState::Uncertain
        } else {
            prior
        },
    );
    if error.stage == PublishStage::PossiblyPublished {
        b.evidence_mut().guarantees.durability = GuaranteesDurability::Unconfirmed;
        warning(b, WarningCode::PublicationUncertain, WarningAction::Inspect);
    }
    for path in &error.created_directories {
        if let Ok(relative) = path.strip_prefix(root) {
            let value = relative.to_string_lossy().replace('\\', "/");
            if vibe_safefs::split_relative(&value).is_ok() {
                residue(b, location.clone(), value, ResidueOwnership::Unproven);
            }
        }
    }
    if !error.created_directories.is_empty() {
        set_effect(b, EffectKind::StoreBootstrap, EffectState::Completed);
        unconfirmed(b);
        warning(
            b,
            WarningCode::ResidueRetained,
            WarningAction::ManualResolution,
        );
    }
}
pub(super) fn identities(
    b: &mut ReportBuilder,
    plan: &vibe_wire::generated::migration::e1::plan_report::Plan,
) {
    let r = b.evidence_mut();
    r.before_content_id = Some(plan.before_identity.clone());
    r.after_content_id = Some(plan.after_identity.clone());
    r.diff_id = Some(plan.diff_identity.clone());
}
pub(super) fn retained(
    b: &mut ReportBuilder,
    j: &ApplicationJournal,
    plan: &vibe_wire::generated::migration::e1::plan_report::Plan,
) {
    identities(b, plan);
    b.evidence_mut().intent_id = Some(j.intent_id.clone());
    b.evidence_mut().guarantees.preservation = if j.unix_mode.is_some() {
        GuaranteesPreservation::BytesAndUnixMode
    } else {
        GuaranteesPreservation::BytesOnly
    };
    if j.unix_mode.is_none() {
        warning(b, WarningCode::MetadataScopeReduced, WarningAction::Inspect);
    }
}
fn finish(
    mut b: ReportBuilder,
    result: Checked<()>,
) -> Result<OperationReport, super::report::ReportValidationFailure> {
    if let Err(error) = result {
        let uncertain = b
            .evidence_mut()
            .effects
            .iter()
            .any(|e| e.state == EffectState::Uncertain);
        b.error(
            error.generated_error(),
            if uncertain {
                OperationReportOutcome::Uncertain
            } else {
                OperationReportOutcome::Refused
            },
        );
    }
    b.into_report()
}

/// The admitting caller supplies the grant independently of this JSON payload.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
pub fn execute_manifest_migration_json(
    text: &str,
    grant: &OwnedManifestMigrationFile,
) -> Result<OperationReport, super::report::ReportValidationFailure> {
    let operation = match validation::decode_operation_json(text, grant) {
        Ok(operation) => operation,
        Err(error) => return Ok(ReportBuilder::preflight(&error)),
    };
    let (kind, id) = match &operation {
        ValidatedOperation::Apply(v) => (
            OperationReportOperation::Apply,
            v.request.operation_id.clone(),
        ),
        ValidatedOperation::Inspect(v) => {
            (OperationReportOperation::Inspect, v.operation_id.clone())
        }
        ValidatedOperation::Recover(v) => {
            (OperationReportOperation::Recover, v.operation_id.clone())
        }
    };
    let mut report = ReportBuilder::new(Some(kind), Some(id));
    let target = match Target::open(grant) {
        Ok(target) => target,
        Err(error) => return finish(report, Err(error)),
    };
    let result = match operation {
        ValidatedOperation::Apply(apply) => apply_plan(grant, &target, apply, &mut report),
        ValidatedOperation::Inspect(inspect) => {
            recovery::inspect(grant, &target, &inspect.operation_id, &mut report)
        }
        ValidatedOperation::Recover(recover) => {
            recovery::recover(grant, &target, &recover, &mut report)
        }
    };
    finish(report, result)
}

fn apply_plan(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    apply: ValidatedApply,
    report: &mut ReportBuilder,
) -> Checked<()> {
    identities(report, &apply.plan);
    if apply.plan.before.len() > grant.journal_read_limit()
        || apply.plan.after.len() > grant.journal_read_limit()
    {
        return Err(fault(
            ApplicationErrorCode::PreservationUnavailable,
            "plan",
            "approved bytes exceed the admitting caller's observation ceiling",
        ));
    }
    let store =
        Store::acquire(grant, &apply.request.operation_id, true, report)?.ok_or_else(|| {
            fault(
                ApplicationErrorCode::JournalIo,
                "store",
                "create-enabled store returned absence",
            )
        })?;
    if let Some((journal, plan)) = store.load(grant, &apply.request.operation_id, report)? {
        if journal.approved_request_json != apply.request.approved_request_json
            || journal.before_content_id != apply.request.before_content_id
            || journal.after_content_id != apply.request.after_content_id
            || journal.diff_id != apply.request.diff_id
        {
            return Err(fault(
                ApplicationErrorCode::IntentMismatch,
                "intent",
                "operation id is already bound to a different immutable approval",
            ));
        }
        retained(report, &journal, &plan);
        return recovery::complete(grant, target, &store, journal, plan, report);
    }
    let snapshot = target.observe(grant)?.ok_or_else(|| {
        report.evidence_mut().target_state = OperationReportTargetState::Absent;
        fault(
            ApplicationErrorCode::SourceChanged,
            "target",
            "approved target is absent",
        )
    })?;
    if snapshot.bytes != apply.plan.before.as_bytes() {
        report.evidence_mut().target_state = OperationReportTargetState::ThirdState;
        warning(
            report,
            WarningCode::ObservedConflict,
            WarningAction::ManualResolution,
        );
        return Err(fault(
            ApplicationErrorCode::SourceChanged,
            "target",
            "current bytes differ from the approved source",
        ));
    }
    report.evidence_mut().target_state = OperationReportTargetState::ApprovedBefore;
    report.evidence_mut().guarantees.preservation = if snapshot.unix_mode.is_some() {
        GuaranteesPreservation::BytesAndUnixMode
    } else {
        GuaranteesPreservation::BytesOnly
    };
    if snapshot.unix_mode.is_none() {
        warning(
            report,
            WarningCode::MetadataScopeReduced,
            WarningAction::Inspect,
        );
    }
    if apply.is_no_op {
        report.evidence_mut().guarantees.durability = GuaranteesDurability::Unconfirmed;
        report.evidence_mut().outcome = OperationReportOutcome::NoOp;
        return Ok(());
    }
    let a = &apply.request.authority;
    let mut journal = ApplicationJournal {
        schema: 1,
        protocol_revision: 1,
        operation_id: apply.request.operation_id,
        intent_id: String::new(),
        phase: ApplicationJournalPhase::Prepared,
        authority: Authority {
            project_root: a.project_root.clone(),
            recovery_store_root: a.recovery_store_root.clone(),
            target_relative: a.target_relative.clone(),
            scope: AuthorityScope::OwnedAuthoredFile,
        },
        project_binding: store.project_binding.clone(),
        store_binding: store.store_binding.clone(),
        approved_request_json: apply.request.approved_request_json,
        before_content_id: apply.plan.before_identity.clone(),
        after_content_id: apply.plan.after_identity.clone(),
        diff_id: apply.plan.diff_identity.clone(),
        before_raw_sha256: snapshot.sha256,
        after_raw_sha256: identity::raw_sha256(apply.plan.after.as_bytes()),
        before_length: snapshot.size.to_string(),
        after_length: apply.plan.after.len().to_string(),
        unix_mode: snapshot.unix_mode,
    };
    journal.intent_id = identity::intent_identity(&journal);
    validation::validate_journal(&journal, grant)?;
    let encoded = serde_json::to_vec(&journal).map_err(|_| {
        fault(
            ApplicationErrorCode::JournalInvalid,
            "journal",
            "checkpoint encoding failed",
        )
    })?;
    if encoded.len() > grant.journal_read_limit() {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "journal",
            "checkpoint exceeds the admitting caller's observation ceiling",
        ));
    }
    store.retain_image(&snapshot.bytes, grant, report)?;
    store.checkpoint(
        &journal,
        None,
        grant,
        report,
        "before_prepared_journal",
        "after_prepared_journal",
    )?;
    retained(report, &journal, &apply.plan);
    recovery::publish(
        grant,
        target,
        &store,
        journal,
        apply.plan,
        report,
        Some(snapshot.identity),
    )
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
