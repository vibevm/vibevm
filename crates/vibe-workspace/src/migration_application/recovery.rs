//! Finite checkpoint/observed-state recovery; third states are never replaced.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");

use super::super::{
    OwnedManifestMigrationFile, ReportBuilder,
    identity::{self, StageAction},
};
use super::{
    Checked, boundary, fault, observe_write, publication_failure, residue, retained, set_effect,
    storage::{Store, Target},
    warning,
};
use vibe_safefs::{FileIdentity, StableFileSnapshot};
use vibe_wire::generated::migration::e1::{
    application_journal::{ApplicationJournal, ApplicationJournalPhase as Phase},
    operation_report::*,
    operation_request::{OperationRequestRecover, OperationRequestRecoverAction},
    plan_report::Plan,
};

fn observed(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    journal: &ApplicationJournal,
    plan: &Plan,
    report: &mut ReportBuilder,
) -> Checked<(OperationReportTargetState, Option<StableFileSnapshot>)> {
    let snapshot = match target.observe(grant) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            report.evidence_mut().target_state = OperationReportTargetState::Unavailable;
            return Err(error);
        }
    };
    let state = match &snapshot {
        None => OperationReportTargetState::Absent,
        Some(s) if journal.unix_mode.is_some() && s.unix_mode != journal.unix_mode => {
            OperationReportTargetState::ThirdState
        }
        Some(s) if s.bytes == plan.before.as_bytes() => OperationReportTargetState::ApprovedBefore,
        Some(s) if s.bytes == plan.after.as_bytes() => OperationReportTargetState::ValidatedAfter,
        Some(_) => OperationReportTargetState::ThirdState,
    };
    report.evidence_mut().target_state = state.clone();
    if state == OperationReportTargetState::ThirdState {
        warning(
            report,
            WarningCode::ObservedConflict,
            WarningAction::ManualResolution,
        );
    }
    Ok((state, snapshot))
}

fn ambiguous() -> super::super::ApplicationValidationError {
    fault(
        ApplicationErrorCode::RecoveryAmbiguous,
        "target",
        "checkpoint and observed target do not prove a supported prefix; preserve state for explicit resolution",
    )
}

fn stages(target: &Target, journal: &ApplicationJournal, report: &mut ReportBuilder) {
    for action in [StageAction::Apply, StageAction::Restore] {
        let stage = identity::transaction_stage_name(&journal.intent_id, action);
        if !matches!(
            target.parent.inspect_transaction_stage_state(&stage),
            Ok(None)
        ) {
            residue(
                report,
                ResidueLocation::Project,
                target.stage_relative(&stage),
                ResidueOwnership::Unproven,
            );
            warning(
                report,
                WarningCode::ResidueRetained,
                WarningAction::ManualResolution,
            );
        }
    }
}

pub(super) fn inspect(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    id: &str,
    report: &mut ReportBuilder,
) -> Checked<()> {
    let Some(store) = Store::acquire(grant, id, false, report)? else {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "store",
            "selected recovery store is absent; inspection creates nothing",
        ));
    };
    let Some((journal, plan)) = store.load(grant, id, report)? else {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "journal",
            "valid retained intent is absent; inspection grants no mutation",
        ));
    };
    retained(report, &journal, &plan);
    stages(target, &journal, report);
    let (state, _) = observed(grant, target, &journal, &plan, report)?;
    match (&journal.phase, state) {
        (Phase::Published, OperationReportTargetState::ValidatedAfter) => {
            set_effect(
                report,
                EffectKind::TargetPublication,
                EffectState::Completed,
            );
            report.evidence_mut().outcome = OperationReportOutcome::Completed;
        }
        (Phase::Restored, OperationReportTargetState::ApprovedBefore) => {
            set_effect(report, EffectKind::TargetRestore, EffectState::Completed);
            report.evidence_mut().outcome = OperationReportOutcome::Completed;
        }
        (Phase::Prepared | Phase::Publishing, OperationReportTargetState::ApprovedBefore)
        | (Phase::Publishing, OperationReportTargetState::ValidatedAfter)
        | (
            Phase::Restoring,
            OperationReportTargetState::ApprovedBefore | OperationReportTargetState::ValidatedAfter,
        ) => {
            report.evidence_mut().outcome = OperationReportOutcome::Pending;
        }
        _ => return Err(ambiguous()),
    }
    Ok(())
}

pub(super) fn recover(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    request: &OperationRequestRecover,
    report: &mut ReportBuilder,
) -> Checked<()> {
    let Some(store) = Store::acquire(grant, &request.operation_id, false, report)? else {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "store",
            "selected recovery store is absent",
        ));
    };
    let Some((journal, plan)) = store.load(grant, &request.operation_id, report)? else {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "journal",
            "valid retained intent is absent; orphan images cannot authorize recovery",
        ));
    };
    if journal.intent_id != request.expected_intent_id {
        return Err(fault(
            ApplicationErrorCode::IntentMismatch,
            "expected_intent_id",
            "selected recovery intent differs from retained intent",
        ));
    }
    retained(report, &journal, &plan);
    match request.action {
        OperationRequestRecoverAction::Complete => {
            complete(grant, target, &store, journal, plan, report)
        }
        OperationRequestRecoverAction::Restore => {
            restore(grant, target, &store, journal, plan, report)
        }
    }
}

pub(super) fn complete(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    store: &Store,
    journal: ApplicationJournal,
    plan: Plan,
    report: &mut ReportBuilder,
) -> Checked<()> {
    stages(target, &journal, report);
    let (state, _) = observed(grant, target, &journal, &plan, report)?;
    if plan.before == plan.after {
        if state != OperationReportTargetState::ApprovedBefore {
            return Err(ambiguous());
        }
        report.evidence_mut().outcome = OperationReportOutcome::NoOp;
        return Ok(());
    }
    match (&journal.phase, state) {
        (Phase::Prepared | Phase::Publishing, OperationReportTargetState::ApprovedBefore) => {
            publish(grant, target, store, journal, plan, report, None)
        }
        (Phase::Publishing, OperationReportTargetState::ValidatedAfter) => {
            set_effect(
                report,
                EffectKind::TargetPublication,
                EffectState::Completed,
            );
            checkpoint_phase(
                store,
                grant,
                journal,
                Phase::Published,
                report,
                "before_published_journal",
                "after_published_journal",
            )?;
            report.evidence_mut().outcome = OperationReportOutcome::Completed;
            Ok(())
        }
        (Phase::Published, OperationReportTargetState::ValidatedAfter) => {
            set_effect(
                report,
                EffectKind::TargetPublication,
                EffectState::Completed,
            );
            report.evidence_mut().outcome = OperationReportOutcome::NoOp;
            Ok(())
        }
        (Phase::Restored, OperationReportTargetState::ApprovedBefore) => {
            set_effect(report, EffectKind::TargetRestore, EffectState::Completed);
            report.evidence_mut().outcome = OperationReportOutcome::NoOp;
            Ok(())
        }
        _ => Err(ambiguous()),
    }
}

fn checkpoint_phase(
    store: &Store,
    grant: &OwnedManifestMigrationFile,
    previous: ApplicationJournal,
    phase: Phase,
    report: &mut ReportBuilder,
    before: &'static str,
    after: &'static str,
) -> Checked<ApplicationJournal> {
    let mut next = previous.clone();
    next.phase = phase;
    store.checkpoint(&next, Some(&previous), grant, report, before, after)?;
    Ok(next)
}

pub(super) fn publish(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    store: &Store,
    journal: ApplicationJournal,
    plan: Plan,
    report: &mut ReportBuilder,
    expected_identity: Option<FileIdentity>,
) -> Checked<()> {
    let journal = if journal.phase == Phase::Prepared {
        checkpoint_phase(
            store,
            grant,
            journal,
            Phase::Publishing,
            report,
            "before_publishing_journal",
            "after_publishing_journal",
        )?
    } else {
        journal
    };
    target_write(
        grant,
        target,
        store,
        &journal,
        &plan,
        report,
        StageAction::Apply,
        expected_identity,
    )?;
    checkpoint_phase(
        store,
        grant,
        journal,
        Phase::Published,
        report,
        "before_published_journal",
        "after_published_journal",
    )?;
    report.evidence_mut().outcome = OperationReportOutcome::Completed;
    Ok(())
}

fn restore(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    store: &Store,
    journal: ApplicationJournal,
    plan: Plan,
    report: &mut ReportBuilder,
) -> Checked<()> {
    stages(target, &journal, report);
    let (state, _) = observed(grant, target, &journal, &plan, report)?;
    match (&journal.phase, state) {
        (Phase::Restored, OperationReportTargetState::ApprovedBefore) => {
            set_effect(report, EffectKind::TargetRestore, EffectState::Completed);
            report.evidence_mut().outcome = OperationReportOutcome::NoOp;
            Ok(())
        }
        (Phase::Prepared | Phase::Publishing, OperationReportTargetState::ApprovedBefore)
        | (Phase::Restoring, OperationReportTargetState::ApprovedBefore) => {
            checkpoint_phase(
                store,
                grant,
                journal,
                Phase::Restored,
                report,
                "before_restored_journal",
                "after_restored_journal",
            )?;
            report.evidence_mut().outcome = OperationReportOutcome::Completed;
            Ok(())
        }
        (
            Phase::Publishing | Phase::Published | Phase::Restoring,
            OperationReportTargetState::ValidatedAfter,
        ) => {
            let journal = if journal.phase != Phase::Restoring {
                checkpoint_phase(
                    store,
                    grant,
                    journal,
                    Phase::Restoring,
                    report,
                    "before_restoring_journal",
                    "after_restoring_journal",
                )?
            } else {
                journal
            };
            target_write(
                grant,
                target,
                store,
                &journal,
                &plan,
                report,
                StageAction::Restore,
                None,
            )?;
            checkpoint_phase(
                store,
                grant,
                journal,
                Phase::Restored,
                report,
                "before_restored_journal",
                "after_restored_journal",
            )?;
            report.evidence_mut().outcome = OperationReportOutcome::Completed;
            Ok(())
        }
        _ => Err(ambiguous()),
    }
}

fn target_write(
    grant: &OwnedManifestMigrationFile,
    target: &Target,
    store: &Store,
    journal: &ApplicationJournal,
    plan: &Plan,
    report: &mut ReportBuilder,
    action: StageAction,
    expected_identity: Option<FileIdentity>,
) -> Checked<()> {
    let (state, snapshot) = observed(grant, target, journal, plan, report)?;
    let (expected_state, bytes, kind, before_hook, after_hook) = match action {
        StageAction::Apply => (
            OperationReportTargetState::ApprovedBefore,
            plan.after.as_bytes(),
            EffectKind::TargetPublication,
            "before_target_publish",
            "after_target_publish",
        ),
        StageAction::Restore => (
            OperationReportTargetState::ValidatedAfter,
            plan.before.as_bytes(),
            EffectKind::TargetRestore,
            "before_target_restore",
            "after_target_restore",
        ),
    };
    if state != expected_state {
        return Err(ambiguous());
    }
    let image = store.read(&store.image_path(), grant)?.ok_or_else(|| {
        fault(
            ApplicationErrorCode::JournalInvalid,
            "before_image",
            "retained approved image is absent before mutation",
        )
    })?;
    if image.bytes != plan.before.as_bytes()
        || image.sha256 != journal.before_raw_sha256
        || image.size.to_string() != journal.before_length
    {
        return Err(fault(
            ApplicationErrorCode::JournalInvalid,
            "before_image",
            "retained approved image changed before mutation",
        ));
    }
    store.confirm_image(&image, grant, report)?;
    let snapshot = snapshot.ok_or_else(ambiguous)?;
    if expected_identity.is_some_and(|identity| identity != snapshot.identity) {
        return Err(fault(
            ApplicationErrorCode::SourceChanged,
            "target",
            "source identity changed during durable preparation",
        ));
    }
    store.still_named()?;
    grant.validate_recovery_store_policy()?;
    boundary(before_hook);
    // Reobserve after the interruption seam and immediately before publication.
    let current = target.observe(grant)?.ok_or_else(ambiguous)?;
    if current != snapshot {
        return Err(fault(
            ApplicationErrorCode::SourceChanged,
            "target",
            "source snapshot changed before publication",
        ));
    }
    let stage = identity::transaction_stage_name(&journal.intent_id, action);
    let prior = super::effect_state(report, &kind);
    set_effect(report, kind.clone(), EffectState::Pending);
    let result = grant.project().write_atomic_transactional_in_with_mode(
        &target.parent,
        &target.name,
        bytes,
        journal.unix_mode,
        &stage,
    );
    match result {
        Ok(write) => {
            set_effect(report, kind.clone(), EffectState::Completed);
            if let Err(error) = observe_write(report, &write) {
                set_effect(report, kind, EffectState::Uncertain);
                let _ = observed(grant, target, journal, plan, report);
                stages(target, journal, report);
                return Err(error);
            }
        }
        Err(error) => {
            publication_failure(
                report,
                kind,
                prior,
                &error,
                ResidueLocation::Project,
                grant.project().root_path(),
            );
            let _ = observed(grant, target, journal, plan, report);
            stages(target, journal, report);
            return Err(fault(
                ApplicationErrorCode::PublicationIo,
                "target",
                "target publication failed; reported effects preserve its actual stage",
            ));
        }
    }
    boundary(after_hook);
    let (state, _) = observed(grant, target, journal, plan, report)?;
    let expected = match action {
        StageAction::Apply => OperationReportTargetState::ValidatedAfter,
        StageAction::Restore => OperationReportTargetState::ApprovedBefore,
    };
    if state != expected {
        set_effect(report, kind, EffectState::Uncertain);
        warning(
            report,
            WarningCode::PublicationUncertain,
            WarningAction::Inspect,
        );
        return Err(fault(
            ApplicationErrorCode::PublicationIo,
            "target",
            "visible target failed exact post-publication verification",
        ));
    }
    if matches!(action, StageAction::Apply) {
        vibe_core::manifest::Manifest::parse_str(&plan.after).map_err(|_| {
            fault(
                ApplicationErrorCode::PlanRefused,
                "target",
                "verified after document is not admitted by the current reader",
            )
        })?;
    }
    Ok(())
}
