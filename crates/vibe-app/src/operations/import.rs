//! A finite import cohort in its own slot of the existing application-work owner.
use super::{OperationContext, pending};
use crate::steps::StepsFile;
use crate::{
    AppError, FeatureRequest, ImportAction, Index, InstallRequest, PreparedImport, SettingsRoot,
};
use vibe_wire::generated::{apps_manifest as wire, shared as w};
pub(crate) mod bindings;
pub(crate) mod document;
pub use bindings::ImportScope;
mod completed;
mod full;
mod inspection;
pub(crate) mod receipts;
mod recovery;
pub(crate) use completed::completed_binding;
pub(crate) use inspection::effect_keys;
pub(crate) use recovery::scope_from_state;
pub use recovery::{ImportResumeInfo, inspect_import_resume};
mod full_store;
pub use full::{import_full, resume_full_import};
mod rows;
pub(crate) mod store;
pub(crate) use full_store::load;
pub(crate) use rows::link_child;

/// Install the preclassified finite cohort; its work identity is independent of
/// every package row, so an earlier failed package can later be retried as a dependency.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn import_cohort(
    ctx: &OperationContext<'_>,
    cohort: &mut PreparedImport,
) -> Result<(), AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let mut document = store::begin(ctx, cohort)?;
    let result = drive(lock, ctx, &mut document);
    cohort.state = document.cohort;
    result
}
/// Read the value-free executor and resolver context of one exact cohort.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn import_resume_info(
    settings: &SettingsRoot,
    id: &str,
) -> Result<w::ApplicationImportContext, AppError> {
    Ok(load(settings, id)?.context().clone())
}
/// Resume a user-home cohort. Secret answers belong to an exact child operation;
/// this API deliberately has no unscoped parameter map.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn resume_import(ctx: &OperationContext<'_>, id: &str) -> Result<PreparedImport, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let mut document = store::load(ctx.settings, id)?;
    let saved = &document.context;
    let selected = if saved.hosted || saved.agent.is_none() {
        None
    } else {
        Some(
            ctx.agent
                .selected
                .filter(|a| {
                    Some(&a.id) == saved.agent.as_ref()
                        && (a.source == crate::AgentSource::External) == saved.agent_external
                })
                .ok_or_else(|| {
                    store::failure(
                        ctx.settings,
                        id,
                        "the retained executor is not available from the current configuration",
                    )
                })?,
        )
    };
    let agent = crate::AgentHooks {
        selected,
        hosted: saved.hosted,
        assume_yes: saved.assume_yes || ctx.agent.assume_yes,
        ..ctx.agent
    };
    let continued = OperationContext {
        offline: ctx.offline || saved.offline,
        agent,
        ..*ctx
    };
    drive(lock, &continued, &mut document)?;
    Ok(PreparedImport::retained(document.cohort))
}
pub(crate) fn command(target: &w::ApplicationImportTarget) -> String {
    format!("vibe app import --resume {}", target.operation)
}

fn drive(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    document: &mut store::ImportWork,
) -> Result<(), AppError> {
    // Recover the narrow outcome-before-release crash cut before another row may
    // reuse a failed package slot. Release only the exact old attempt, if present.
    release_refused(lock, ctx.settings, document)?;
    if document.phase != w::ApplicationImportPhase::Complete {
        for result in document
            .cohort
            .results
            .iter()
            .filter(|r| r.action == w::ApplicationPendingImportResultAction::Installed)
        {
            if let Some(target) = &result.target {
                pending::ack_import(ctx.settings, target)?;
            }
        }
    }

    while document.phase == w::ApplicationImportPhase::Running {
        let Some(row) = document
            .cohort
            .rows
            .get(document.cohort.cursor as usize)
            .cloned()
        else {
            document.phase = w::ApplicationImportPhase::Arrangement;
            store::save(ctx.settings, document)?;
            break;
        };
        let (resolved, option) = rows::admit(ctx, &row)?;
        let mut target = rows::bound(ctx, document, &row, &option)?;
        let existing = target.is_some();
        let result = if let Some(child) = &target {
            document.cohort.current = Some(child.clone());
            let state = pending::load(ctx.settings, &resolved.package, Some(&child.operation))?;
            if state
                .operation
                .as_ref()
                .is_some_and(|op| op.phase == w::ApplicationPendingOperationPhase::Complete)
            {
                pending::ack_import(ctx.settings, child)
            } else {
                pending::continue_target(lock, ctx, child, &mut vec![]).map(|_| ())
            }
        } else {
            let index = Index::load(ctx.settings)?;
            let portable = crate::import_row(&row);
            match crate::import_action(
                &index,
                &portable,
                &resolved.options,
                document.cohort.skip_installed,
            ) {
                ImportAction::Skip(reason) => {
                    advance(
                        ctx.settings,
                        document,
                        result(
                            &row,
                            w::ApplicationPendingImportResultAction::Skipped,
                            None,
                            reason,
                            None,
                        ),
                    )?;
                    continue;
                }
                ImportAction::Refuse(error) => {
                    advance(ctx.settings, document, refused(&row, &error, None))?;
                    continue;
                }
                ImportAction::Install(install) => {
                    let request = InstallRequest {
                        choice: install.choice,
                        features: FeatureRequest {
                            enable: install.enable,
                            disable: install.disable,
                            profile: None,
                        },
                        parameters: install.parameters,
                        accept_license: document.cohort.accept_license && row.license_accepted,
                        hidden: row.hidden,
                        ..Default::default()
                    };
                    let outcome = super::install_locked(
                        lock,
                        ctx,
                        &resolved,
                        &request,
                        &mut vec![],
                        None,
                        Some(pending::Parent::ImportRow {
                            target: &document.target,
                            row: document.cohort.cursor,
                        }),
                    );
                    // A checkpoint can have been created even when execution failed.
                    // Its exact backward row reference is the only recovery evidence.
                    if let Some(state) =
                        StepsFile::load(&pending::work(ctx.settings, &resolved.package))?
                        && let Some(op) = state.operation.filter(|op| {
                            op.parent.as_ref()
                                == Some(&pending::import_reference(
                                    &document.target,
                                    document.cohort.cursor,
                                ))
                        })
                    {
                        target = Some(op.target);
                    }
                    outcome.map(|_| ())
                }
            }
        };
        if let Some(child) = &target {
            document.cohort.current = Some(child.clone());
        }
        match result {
            Ok(()) => {
                if let Some(child) = &target {
                    pending::ack_import(ctx.settings, child)?;
                }
                advance(
                    ctx.settings,
                    document,
                    self::result(
                        &row,
                        w::ApplicationPendingImportResultAction::Installed,
                        None,
                        "installed".into(),
                        target,
                    ),
                )?;
            }
            Err(error) => {
                if matches!(&error, AppError::Agent(crate::AgentError::Parked(_)))
                    || error.code() == "resume_mismatch"
                    || existing && error.class() == crate::ErrorClass::Refusal
                {
                    return Err(error);
                }
                if let Some(child) = &target {
                    let state =
                        pending::load(ctx.settings, &resolved.package, Some(&child.operation))?;
                    if state.operation.as_ref().is_some_and(|op| {
                        matches!(
                            op.phase,
                            w::ApplicationPendingOperationPhase::Removing
                                | w::ApplicationPendingOperationPhase::Placing
                        )
                    }) {
                        return Err(error);
                    }
                }
                // The failed attempt is recorded first. Only then can its slot
                // become available for a later dependency or duplicate row retry.
                advance(ctx.settings, document, refused(&row, &error, target))?;
                release_refused(lock, ctx.settings, document)?;
            }
        }
    }
    if document.phase == w::ApplicationImportPhase::Arrangement {
        for result in document
            .cohort
            .results
            .iter()
            .filter(|r| r.action == w::ApplicationPendingImportResultAction::Installed)
        {
            if let Some(target) = &result.target {
                pending::ack_import(ctx.settings, target)?;
            }
        }
        if !document.cohort.arrangement_done {
            let index = Index::load(ctx.settings)?;
            let mut notes = Vec::new();
            for key in &document.cohort.auto {
                notes.push(note(
                    key.clone(),
                    if index.installations.contains_key(key) {
                        "a requirement; installed with what requires it"
                    } else {
                        "a requirement nothing in the set required"
                    }
                    .into(),
                ));
            }
            let (collection, pin, launcher) = crate::portable::import_arrangement(&document.cohort);
            let set = wire::AppsManifest {
                protocol: crate::APPS_PROTOCOL.into(),
                exported: String::new(),
                home: wire::AppsManifestHome::User,
                platform: String::new(),
                arch: String::new(),
                application: vec![],
                collection,
                pin,
                launcher,
            };
            for (key, message) in crate::portable::restore_arrangement_locked(ctx.settings, &set)? {
                notes.push(note(key, message));
            }
            document.cohort.results.extend(notes);
            document.cohort.arrangement_done = true;
        }
        document.phase = w::ApplicationImportPhase::Complete;
        store::save(ctx.settings, document)?;
    }
    for result in &document.cohort.results {
        rows::release(lock, ctx.settings, &document.target, result, false)?;
    }
    std::fs::remove_dir_all(store::path(ctx.settings, &document.target.operation)?).map_err(
        |_| {
            store::failure(
                ctx.settings,
                &document.target.operation,
                "cannot remove settled cohort work",
            )
        },
    )?;
    Ok(())
}
fn advance(
    settings: &SettingsRoot,
    document: &mut store::ImportWork,
    mut outcome: w::ApplicationPendingImportResult,
) -> Result<(), AppError> {
    outcome.row_index = Some(document.cohort.cursor);
    document.cohort.results.push(outcome);
    document.cohort.cursor = document.cohort.cursor.checked_add(1).ok_or_else(|| {
        store::failure(settings, &document.target.operation, "row cursor overflow")
    })?;
    document.cohort.current = None;
    store::save(settings, document)
}
fn release_refused(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    document: &mut store::ImportWork,
) -> Result<(), AppError> {
    let mut changed = false;
    for result in &mut document.cohort.results {
        if result.action == w::ApplicationPendingImportResultAction::Refused
            && result.target.is_some()
        {
            rows::release(lock, settings, &document.target, result, true)?;
            result.target = None;
            changed = true;
        }
    }
    if changed {
        store::save(settings, document)?;
    }
    Ok(())
}
fn result(
    row: &w::ApplicationPendingImportRow,
    action: w::ApplicationPendingImportResultAction,
    code: Option<String>,
    message: String,
    target: Option<w::ApplicationPendingTarget>,
) -> w::ApplicationPendingImportResult {
    w::ApplicationPendingImportResult {
        key: row.key.clone(),
        option: Some(row.installer.clone()),
        action,
        code,
        message,
        target,
        row_index: None,
    }
}
fn refused(
    row: &w::ApplicationPendingImportRow,
    error: &AppError,
    target: Option<w::ApplicationPendingTarget>,
) -> w::ApplicationPendingImportResult {
    result(
        row,
        w::ApplicationPendingImportResultAction::Refused,
        Some(error.code().into()),
        refusal_message(&row.key, error, target.as_ref()),
        target,
    )
}
fn refusal_message(
    key: &str,
    error: &AppError,
    target: Option<&w::ApplicationPendingTarget>,
) -> String {
    match error {
        AppError::Step(crate::StepError::Failed { step, kind, .. })
        | AppError::Step(crate::StepError::FinishFailed { step, kind, .. }) => format!(
            "{kind} step `{step}` failed ({}). Historical application attempt {}; inspect `vibe app runs {} -g` for retained step output before choosing a fresh retry",
            error.code(),
            target.map_or("not checkpointed", |t| t.operation.as_str()),
            key
        ),
        _ if error.code() == "choice_differs" => error.to_string(),
        _ => format!(
            "import row refused ({}); inspect `vibe app errors` for this refusal and `vibe app runs {} -g` for any retained execution log",
            error.code(),
            key
        ),
    }
}

fn note(key: String, message: String) -> w::ApplicationPendingImportResult {
    w::ApplicationPendingImportResult {
        key,
        action: w::ApplicationPendingImportResultAction::Skipped,
        option: None,
        code: None,
        message,
        target: None,
        row_index: None,
    }
}

#[cfg(test)]
mod tests;
