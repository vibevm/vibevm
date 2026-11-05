//! A cohort row owns only its exact execution attempt, never a package slot forever.
use super::*;
use crate::{ResolvedPackage, SettingsRoot};
use vibe_core::manifest::InstallerOptionDecl;

pub(super) fn admit(
    ctx: &OperationContext<'_>,
    row: &w::ApplicationPendingImportRow,
) -> Result<(ResolvedPackage, InstallerOptionDecl), AppError> {
    let resolved = pending::read_source(ctx, &row.source, &row.key)?;
    let option = resolved
        .options
        .iter()
        .find(|o| o.id == row.installer && o.platform == row.platform && o.arch == row.arch)
        .cloned()
        .ok_or_else(|| {
            pending::failure(
                ctx.settings,
                &resolved.package,
                "retained import installer disappeared".into(),
            )
        })?;
    if crate::portable::import_definition(&resolved, &option)? != row.definition {
        return Err(pending::failure(
            ctx.settings,
            &resolved.package,
            "retained import declaration changed; start a fresh import after review".into(),
        ));
    }
    Ok((resolved, option))
}
pub(super) fn bound(
    ctx: &OperationContext<'_>,
    document: &store::ImportWork,
    row: &w::ApplicationPendingImportRow,
    option: &InstallerOptionDecl,
) -> Result<Option<w::ApplicationPendingTarget>, AppError> {
    let package = pending::coordinate(&row.key)?;
    let current = document.cohort.current.as_ref();
    let Some(state) = StepsFile::load(&pending::work(ctx.settings, &package))? else {
        if current.is_some() {
            return Err(store::failure(
                ctx.settings,
                &document.target.operation,
                "current row work disappeared",
            ));
        }
        return Ok(None);
    };
    let Some(op) = state.operation.as_ref() else {
        return Err(pending::failure(ctx.settings,&package,"legacy unfinished work cannot be adopted as this import row; finish or explicitly restart it separately".into()));
    };
    let expected = pending::import_reference(&document.target, document.cohort.cursor);
    if current.is_none()
        && op.phase == w::ApplicationPendingOperationPhase::Complete
        && op.parent.as_ref() != Some(&expected)
    {
        return Ok(None);
    }
    let portable = crate::import_row(row);
    let imported = crate::portable::answers(
        &portable,
        crate::import_choice(&portable, std::slice::from_ref(option)),
        Some(option),
    );
    let context = op.parent.as_ref() == Some(&expected)
        && op.kind == w::ApplicationPendingOperationKind::Install
        && op.source == row.source
        && op.request.installer == row.installer
        && op.request.platform == row.platform
        && op.request.arch == row.arch
        && op.request.enabled == imported.enable
        && op.request.disabled == imported.disable
        && op.request.public_set == row.parameters
        && op.request.hidden == row.hidden
        && !op.request.force;
    if !context || current.is_some_and(|t| t != &op.target) {
        return Err(pending::failure(
            ctx.settings,
            &package,
            format!(
                "work is not this exact import row/attempt; continue `{}` separately",
                pending::command(&op.target)
            ),
        ));
    }
    pending::load(ctx.settings, &package, Some(&op.target.operation))?;
    if current.is_none() {
        link_child(
            ctx.settings,
            &document.target,
            document.cohort.cursor,
            &op.target,
        )?;
    }
    Ok(Some(op.target.clone()))
}
pub(crate) fn link_child(
    settings: &SettingsRoot,
    parent: &w::ApplicationImportTarget,
    row: u32,
    child: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let mut document = super::full_store::load(settings, &parent.operation)?;
    if document.target() != parent
        || document.cursor() != row
        || document
            .root_key(row as usize)
            .is_none_or(|key| key != child.key)
        || document.current().is_some_and(|t| t != child)
    {
        return Err(store::failure(
            settings,
            &parent.operation,
            "child differs from the exact current cohort row/attempt",
        ));
    }
    let state = pending::load(
        settings,
        &pending::coordinate(&child.key)?,
        Some(&child.operation),
    )?;
    if state.operation.as_ref().and_then(|op| op.parent.as_ref())
        != Some(&pending::import_reference(parent, row))
    {
        return Err(store::failure(
            settings,
            &parent.operation,
            "child has no matching backward row ownership",
        ));
    }
    document.set_current(Some(child.clone()));
    super::full_store::save(settings, &document)
}

pub(super) fn release(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    owner: &w::ApplicationImportTarget,
    result: &w::ApplicationPendingImportResult,
    refused: bool,
) -> Result<(), AppError> {
    let Some(target) = &result.target else {
        return Ok(());
    };
    let Some(row) = result.row_index else {
        return Err(store::failure(
            settings,
            &owner.operation,
            "execution result has no original row index",
        ));
    };
    release_target(lock, settings, owner, target, row, refused)
}

pub(super) fn release_target(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    owner: &w::ApplicationImportTarget,
    target: &w::ApplicationPendingTarget,
    row: u32,
    refused: bool,
) -> Result<(), AppError> {
    let package = pending::coordinate(&target.key)?;
    let work = pending::work(settings, &package);
    let Some(state) = StepsFile::load(&work)? else {
        return Ok(());
    };
    // A terminal refused attempt may already have been retried. Never follow its
    // old correlation into a new generation at this package's ordinary slot.
    if state
        .operation
        .as_ref()
        .is_none_or(|op| op.target != *target)
    {
        return Ok(());
    }
    if state.operation.as_ref().and_then(|op| op.parent.as_ref())
        != Some(&pending::import_reference(owner, row))
    {
        return Ok(());
    }
    if refused {
        pending::refused_owner(lock, settings, &state)?;
    }
    let mut state = pending::load(settings, &package, Some(&target.operation))?;
    let op = state.pending_mut(settings)?;
    if op.phase != w::ApplicationPendingOperationPhase::Complete {
        return Err(store::failure(
            settings,
            &owner.operation,
            "cannot release an unfinished successful row",
        ));
    }
    op.parent = None;
    if refused || op.request.keep_work {
        state.save(&work)?;
    } else {
        std::fs::remove_dir_all(&work).map_err(|_| {
            store::failure(
                settings,
                &owner.operation,
                "cannot release completed row work",
            )
        })?;
    }
    Ok(())
}
