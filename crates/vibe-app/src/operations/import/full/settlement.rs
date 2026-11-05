//! Terminal arrangement, truthful outcomes and waiting data of the one full cohort.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION");
use super::*;

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION"
)]
pub(super) fn finish(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &mut wire::ImportWork,
) -> Result<(), AppError> {
    if document.phase != w::ApplicationImportPhase::Complete {
        let empty = ImportSecrets::default();
        let scope = crate::ImportScope::new(
            document.target.clone(),
            document.cohort.roots.clone(),
            document.cohort.bindings.clone(),
            Default::default(),
            &empty,
        )?;
        verify_scope(ctx.settings, document, scope.fingerprint())?;
        super::super::receipts::recover(lock, ctx.settings, document, scope.fingerprint())?;
        release_refused(ctx, lock, document, scope.fingerprint())?;
        let index = crate::Index::load(ctx.settings)?;
        for key in document.cohort.completed_bindings.keys() {
            if !super::super::receipts::completed(
                ctx.settings,
                document,
                key,
                &index,
                scope.fingerprint(),
            )? {
                return Err(store::failure(
                    ctx.settings,
                    &document.target.operation,
                    "completed binding receipt disappeared",
                ));
            }
        }
        admit_prefix(ctx, document)?;
        document.phase = w::ApplicationImportPhase::Arrangement;
        full_store::save_full(ctx.settings, document)?;
        if !document.cohort.arrangement_done {
            let (collections, pins, launchers) = arrangement(document);
            crate::portable::restore_arrangement_parts(
                ctx.settings,
                &collections,
                &pins,
                &launchers,
            )?;
            document.cohort.arrangement_done = true;
        }
        document.phase = w::ApplicationImportPhase::Complete;
        full_store::save_full(ctx.settings, document)?;
    }
    for result in &document.cohort.results {
        release_complete(ctx, lock, document, result)?;
    }
    for receipt in document.cohort.completed_bindings.values() {
        release_application(ctx, lock, document, &receipt.target, receipt.origin_row)?;
    }
    std::fs::remove_dir_all(store::path(ctx.settings, &document.target.operation)?).map_err(
        |_| {
            store::failure(
                ctx.settings,
                &document.target.operation,
                "cannot retire completed full import",
            )
        },
    )?;
    Ok(())
}
/// Before more work or arrangement, acknowledge every successful prefix record.
pub(super) fn admit_prefix(
    ctx: &OperationContext<'_>,
    document: &wire::ImportWork,
) -> Result<(), AppError> {
    if document.phase == w::ApplicationImportPhase::Complete {
        return Ok(());
    }
    for result in &document.cohort.results {
        if result.action == w::ApplicationPendingImportResultAction::Installed {
            let target = result.target.as_ref().ok_or_else(|| {
                store::failure(
                    ctx.settings,
                    &document.target.operation,
                    "successful prefix lacks its attempt",
                )
            })?;
            pending::ack_import(ctx.settings, target)?;
        }
    }
    Ok(())
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION"
)]
fn release_complete(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &wire::ImportWork,
    result: &w::ApplicationPendingImportResult,
) -> Result<(), AppError> {
    if let (Some(target), Some(row)) = (&result.target, result.row_index) {
        release_application(ctx, lock, document, target, row)?;
    }
    Ok(())
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION"
)]
fn release_application(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &wire::ImportWork,
    target: &w::ApplicationPendingTarget,
    row: u32,
) -> Result<(), AppError> {
    let work = pending::work(ctx.settings, &pending::coordinate(&target.key)?);
    if let Some(state) = crate::steps::StepsFile::load(&work)?
        && state.operation.as_ref().is_some_and(|op| {
            op.target == *target
                && op.parent.as_ref() == Some(&pending::import_reference(&document.target, row))
        })
    {
        pending::complete(lock, ctx.settings, target)?;
    }
    rows::release_target(lock, ctx.settings, &document.target, target, row, false)
}

pub(super) fn advance(
    ctx: &OperationContext<'_>,
    d: &mut wire::ImportWork,
    key: &str,
    option: String,
    target: w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    d.cohort.results.push(w::ApplicationPendingImportResult {
        key: key.into(),
        action: w::ApplicationPendingImportResultAction::Installed,
        option: Some(option),
        code: None,
        message: "installed full configuration".into(),
        target: Some(target),
        row_index: Some(d.cohort.cursor),
    });
    d.cohort.cursor = d
        .cohort
        .cursor
        .checked_add(1)
        .ok_or_else(|| store::failure(ctx.settings, &d.target.operation, "too many roots"))?;
    d.cohort.current = None;
    d.cohort.waiting = None;
    full_store::save_full(ctx.settings, d)
}
fn arrangement(
    d: &wire::ImportWork,
) -> (
    Vec<vibe_wire::generated::apps_manifest::Collection>,
    Vec<vibe_wire::generated::apps_manifest::Pin>,
    Vec<vibe_wire::generated::apps_manifest::Launcher>,
) {
    use vibe_wire::generated::apps_manifest as v;
    (
        d.cohort
            .collections
            .iter()
            .map(|r| v::Collection {
                name: r.name.clone(),
                launchers: r.launchers.clone(),
            })
            .collect(),
        d.cohort
            .pins
            .iter()
            .map(|r| v::Pin {
                target: match r.target {
                    w::ApplicationPendingPinTarget::Taskbar => v::PinTarget::Taskbar,
                    w::ApplicationPendingPinTarget::Favorites => v::PinTarget::Favorites,
                },
                launchers: r.launchers.clone(),
            })
            .collect(),
        d.cohort
            .launchers
            .iter()
            .map(|r| v::Launcher {
                key: r.key.clone(),
                name: r.name.clone(),
                hidden: r.hidden.unwrap_or(false),
                icon: r.icon.clone(),
            })
            .collect(),
    )
}

pub(super) fn skip(
    ctx: &OperationContext<'_>,
    d: &mut wire::ImportWork,
    key: &str,
    message: &str,
) -> Result<(), AppError> {
    d.cohort.results.push(w::ApplicationPendingImportResult {
        key: key.into(),
        action: w::ApplicationPendingImportResultAction::Skipped,
        option: None,
        code: None,
        message: message.into(),
        target: None,
        row_index: Some(d.cohort.cursor),
    });
    d.cohort.cursor = d
        .cohort
        .cursor
        .checked_add(1)
        .ok_or_else(|| store::failure(ctx.settings, &d.target.operation, "too many roots"))?;
    d.cohort.current = None;
    d.cohort.waiting = None;
    full_store::save_full(ctx.settings, d)
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
pub(super) fn wait_error(
    ctx: &OperationContext<'_>,
    document: &mut wire::ImportWork,
    error: AppError,
) -> Result<(), AppError> {
    if let AppError::Configuration(
        crate::configuration::ConfigurationError::ImportInputRequired {
            key, parameters, ..
        },
    ) = &error
    {
        document.cohort.waiting = Some(w::ApplicationImportWaitingInput {
            key: key.clone(),
            parameters: parameters.clone(),
        });
        full_store::save_full(ctx.settings, document)?;
    }
    Err(error)
}

/// Only a terminal failure of this root advances it. A descendant's failed or
/// parked attempt remains resumable through its exact owner chain.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn settle_error(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &mut wire::ImportWork,
    key: &str,
    fingerprint: &str,
    error: AppError,
) -> Result<(), AppError> {
    let own_failure = matches!(&error, AppError::Step(crate::StepError::Failed {key: failed,..} | crate::StepError::FinishFailed {key: failed,..}) if failed == key);
    if !own_failure {
        return wait_error(ctx, document, error);
    }
    let row = document.cohort.cursor;
    let target = document.cohort.current.clone();
    let option = document
        .cohort
        .bindings
        .get(key)
        .map(|b| b.installer.clone());
    document
        .cohort
        .results
        .push(w::ApplicationPendingImportResult {
            key: key.into(),
            action: w::ApplicationPendingImportResultAction::Refused,
            option,
            code: Some(error.code().into()),
            message: super::super::refusal_message(key, &error, target.as_ref()),
            target,
            row_index: Some(row),
        });
    document.cohort.cursor = row.checked_add(1).ok_or_else(|| {
        store::failure(ctx.settings, &document.target.operation, "too many roots")
    })?;
    document.cohort.current = None;
    document.cohort.waiting = None;
    // The terminal outcome precedes receipt minting and retirement. Recovery
    // never needs to replay a failed recipe to discover its previous result.
    full_store::save_full(ctx.settings, document)?;
    release_refused(ctx, lock, document, fingerprint)
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn release_refused(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &mut wire::ImportWork,
    fingerprint: &str,
) -> Result<(), AppError> {
    for position in 0..document.cohort.results.len() {
        let result = document.cohort.results[position].clone();
        if result.action != w::ApplicationPendingImportResultAction::Refused {
            continue;
        }
        let Some(target) = &result.target else {
            continue;
        };
        let work = pending::work(ctx.settings, &pending::coordinate(&target.key)?);
        if let Some(state) = crate::steps::StepsFile::load(&work)?
            && state.operation.as_ref().is_some_and(|op| {
                op.target == *target && op.phase != w::ApplicationPendingOperationPhase::Complete
            })
        {
            super::super::receipts::preserve(lock, ctx.settings, document, &state, fingerprint)?;
        }
        rows::release(lock, ctx.settings, &document.target, &result, true)?;
        // Retain the exact Refused attempt as history for a same-cohort retry.
        // It is never a successful publication receipt.
    }
    Ok(())
}

/// Bind retained attempts before new input obligations or cleanup checkpoints.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTEXT-IDENTITY"
)]
pub(super) fn verify_scope(
    settings: &crate::SettingsRoot,
    document: &wire::ImportWork,
    fingerprint: &str,
) -> Result<(), AppError> {
    if document.phase == w::ApplicationImportPhase::Complete {
        return Ok(());
    }
    for target in document.cohort.current.iter().chain(
        document
            .cohort
            .results
            .iter()
            .filter(|r| r.action == w::ApplicationPendingImportResultAction::Installed)
            .filter_map(|r| r.target.as_ref()),
    ) {
        let state = pending::load(
            settings,
            &pending::coordinate(&target.key)?,
            Some(&target.operation),
        )?;
        if !pending::matches_import_scope(&state, fingerprint) {
            return Err(store::failure(
                settings,
                &document.target.operation,
                "immutable full import meaning differs from its retained execution context",
            ));
        }
    }
    Ok(())
}
