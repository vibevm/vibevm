//! The existing steps2 work owner: exact home/key/generation and retained operation input.
use crate::steps::StepsFile;
use crate::{
    AppError, Index, InstallRequest, PackageCoordinate, Plan, ResolvedPackage, SettingsRoot, Start,
    StepError,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use vibe_core::manifest::InstallerOptionDecl;
use vibe_wire::generated::shared as w;
mod checkpoint;
mod work_retirement;
pub(super) use work_retirement::refused_owner;
mod continuation;
pub(crate) use continuation::{
    continuation_command, continuation_target, owner_path, retained_for,
};
mod source;
pub(super) use checkpoint::{Checkpoint, begin, generation, lookup};
pub(super) use source::{read_source, source};
mod dependencies;
mod drive;
mod identity;
pub(super) use identity::published as record_identity;
mod import;
mod input;
pub(super) use dependencies::{ack_child, bound_child, link_child, resolve_binding};
pub use drive::{ResumeInfo, ResumeRequest, resume, resume_info};
pub(super) use drive::{continue_target, manual};
pub(super) use import::{ack_import, finish_import_row};

#[derive(Clone, Copy)]
pub(super) enum Parent<'a> {
    Dependency {
        target: &'a w::ApplicationPendingTarget,
        requirement: &'a str,
    },
    ImportRow {
        target: &'a w::ApplicationImportTarget,
        row: u32,
    },
}
impl Parent<'_> {
    pub(super) fn reference(&self) -> w::ApplicationContinuationTarget {
        match self {
            Self::Dependency { target, .. } => application_reference(target),
            Self::ImportRow { target, row } => import_reference(target, *row),
        }
    }
}
pub(crate) fn application_reference(
    target: &w::ApplicationPendingTarget,
) -> w::ApplicationContinuationTarget {
    w::ApplicationContinuationTarget::Application(Box::new(
        w::ApplicationContinuationTargetApplication {
            target: target.clone(),
        },
    ))
}
pub(crate) fn import_reference(
    target: &w::ApplicationImportTarget,
    row: u32,
) -> w::ApplicationContinuationTarget {
    w::ApplicationContinuationTarget::Import(Box::new(w::ApplicationContinuationTargetImport {
        target: target.clone(),
        row,
    }))
}

pub(crate) fn failure(
    settings: &SettingsRoot,
    package: &PackageCoordinate,
    reason: String,
) -> AppError {
    StepError::ResumeMismatch {
        work: settings
            .app_work_dir()
            .join(package.directory_name())
            .display()
            .to_string(),
        reason,
    }
    .into()
}
pub(super) fn lossless(path: &Path) -> Result<String, AppError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        StepError::ResumeMismatch {
            work: path.display().to_string(),
            reason: "continuation path is not lossless UTF-8".into(),
        }
        .into()
    })
}
pub(crate) fn actual_home(settings: &SettingsRoot) -> Result<String, AppError> {
    let path = settings
        .path()
        .canonicalize()
        .map_err(|e| StepError::ResumeMismatch {
            work: settings.path().display().to_string(),
            reason: format!("cannot resolve actual application home: {e}"),
        })?;
    lossless(&path)
}
pub(crate) fn work(settings: &SettingsRoot, package: &PackageCoordinate) -> PathBuf {
    settings.app_work_dir().join(package.directory_name())
}
pub(crate) fn load(
    settings: &SettingsRoot,
    package: &PackageCoordinate,
    expected: Option<&str>,
) -> Result<StepsFile, AppError> {
    let state = StepsFile::load(&work(settings, package))?.ok_or_else(|| {
        failure(
            settings,
            package,
            "no pending work exists for this exact package key".into(),
        )
    })?;
    let op=state.operation.as_ref().ok_or_else(||failure(settings,package,"this is legacy protocol1 work with effective-only provenance; use the established manual --resume route".into()))?;
    if op.target.home != actual_home(settings)? {
        return Err(failure(
            settings,
            package,
            "pending work belongs to another actual application home".into(),
        ));
    }
    if state.package != *package || op.target.key != package.key() {
        return Err(failure(
            settings,
            package,
            "pending work belongs to another package key".into(),
        ));
    }
    if expected.is_some_and(|id| id != op.target.operation) {
        return Err(failure(
            settings,
            package,
            "the requested operation generation is stale or belongs to another execution".into(),
        ));
    }
    Ok(state)
}
pub(super) fn kind_name(kind: &w::ApplicationPendingOperationKind) -> &'static str {
    match kind {
        w::ApplicationPendingOperationKind::Install => "install",
        w::ApplicationPendingOperationKind::Update => "update",
        w::ApplicationPendingOperationKind::Modify => "modify",
    }
}

pub(crate) fn coordinate(key: &str) -> Result<PackageCoordinate, AppError> {
    let requested = crate::qualified_ref(key)?;
    let (_, version) = key
        .rsplit_once('@')
        .ok_or_else(|| StepError::ResumeMismatch {
            work: key.into(),
            reason: "resume requires an exact package version".into(),
        })?;
    semver::Version::parse(version).map_err(|_| StepError::ResumeMismatch {
        work: key.into(),
        reason: "resume requires GROUP/NAME@VERSION, not a version range".into(),
    })?;
    Ok(PackageCoordinate {
        group: requested
            .group
            .ok_or_else(|| StepError::ResumeMismatch {
                work: key.into(),
                reason: "resume key lacks its group".into(),
            })?
            .to_string(),
        name: requested.name.to_string(),
        version: version.into(),
    })
}

/// Render only the value-free target; never original argv or input values.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn command(target: &w::ApplicationPendingTarget) -> String {
    fn quoted(value: &str) -> String {
        if !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_./:=@,+".contains(c))
        {
            return value.into();
        }
        if cfg!(windows) {
            format!("'{}'", value.replace('\'', "''"))
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    }
    let mut words = vec![
        "vibe".into(),
        "app".into(),
        "resume".into(),
        target.key.clone(),
        "--operation".into(),
        target.operation.clone(),
    ];
    match &target.project {
        Some(path) => {
            words.push("--path".into());
            words.push(path.clone());
        }
        None => words.push("-g".into()),
    }
    words
        .iter()
        .map(|s| quoted(s))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn phase(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    phase: w::ApplicationPendingOperationPhase,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    if let Some(operation) = &mut state.operation {
        operation.phase = phase;
    }
    state.save(&work(settings, &package))
}

pub(super) fn validate_plan(
    ctx: &super::OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    plan: &Plan,
    state: &StepsFile,
) -> Result<(), AppError> {
    let op = state.operation.as_ref().ok_or_else(|| {
        failure(
            ctx.settings,
            &resolved.package,
            "legacy work has no retained execution identity".into(),
        )
    })?;
    if matches!(
        op.phase,
        w::ApplicationPendingOperationPhase::Removing
            | w::ApplicationPendingOperationPhase::Placing
    ) {
        return Err(failure(ctx.settings,&resolved.package,"the interrupted removal or placement has an uncertain outcome; inspect and recover explicitly before starting fresh work".into()));
    }
    let index = Index::load(ctx.settings)?;
    if !was_published(ctx.settings, &index, op)?
        && let Some(key) = &op.predecessor
        && identity::predecessor(&index, key)
            .map_err(|e| failure(ctx.settings, &resolved.package, e))?
            != op.predecessor_identity
    {
        return Err(failure(
            ctx.settings,
            &resolved.package,
            "the exact predecessor changed before continuation".into(),
        ));
    }
    if identity::execution(
        ctx.settings,
        resolved,
        option,
        plan,
        request.force,
        kind_name(&op.kind),
    )? != op.execution_identity
    {
        return Err(failure(ctx.settings,&resolved.package,"the retained public execution context or checked prompt changed; start fresh reconfiguration".into()));
    }
    Ok(())
}

pub(super) fn before_dependencies(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let state = load(settings, &package, Some(&target.operation))?;
    if state
        .operation
        .as_ref()
        .is_some_and(|op| op.phase == w::ApplicationPendingOperationPhase::Prepared)
    {
        phase(
            settings,
            target,
            w::ApplicationPendingOperationPhase::Dependencies,
        )?;
    }
    Ok(())
}

pub(super) fn complete(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    finish(lock, settings, target, false)
}
pub(super) fn complete_failed(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    finish(lock, settings, target, true)
}
fn finish(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    failed: bool,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    let index = Index::load(settings)?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing operation".into()))?;
    let expected = op.publication_record.as_ref().ok_or_else(|| {
        failure(
            settings,
            &package,
            "operation has no pre-publication result identity".into(),
        )
    })?;
    if index
        .installations
        .get(&target.key)
        .is_none_or(|r| r.pending.is_some())
        || identity::predecessor(&index, &target.key)
            .map_err(|e| failure(settings, &package, e))?
            .as_ref()
            != Some(expected)
    {
        return Err(failure(
            settings,
            &package,
            "the published installation differs from this operation's exact expected result".into(),
        ));
    }

    let acceptable = op.retirements.iter().all(|r| {
        r.state
            == if failed {
                w::ApplicationPendingRetirementState::Queued
            } else {
                w::ApplicationPendingRetirementState::Removed
            }
    });
    if !acceptable {
        return Err(failure(
            settings,
            &package,
            "post-publication retirement obligations are not settled".into(),
        ));
    }
    let keep = op.request.keep_work || op.parent.is_some();
    let children = work_retirement::prepare(settings, &state, false)?;
    state.pending_mut(settings)?.phase = w::ApplicationPendingOperationPhase::Complete;
    state.save(&work(settings, &package))?;
    work_retirement::release(lock, settings, children)?;
    if !keep {
        fs::remove_dir_all(work(settings, &package))
            .map_err(|e| failure(settings, &package, e.to_string()))?;
    }
    Ok(())
}

pub(super) fn publishing(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    record: &crate::Installation,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing operation".into()))?;
    op.publication_record =
        Some(identity::published(record).map_err(|e| failure(settings, &package, e))?);
    op.phase = w::ApplicationPendingOperationPhase::Publishing;
    state.save(&work(settings, &package))
}

pub(super) fn keep_work(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    keep: bool,
) -> Result<(), AppError> {
    if !keep {
        return Ok(());
    }
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    if let Some(op) = &mut state.operation
        && !op.request.keep_work
    {
        op.request.keep_work = true;
        state.save(&work(settings, &package))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;

pub(super) fn was_published(
    settings: &SettingsRoot,
    index: &Index,
    op: &w::ApplicationPendingOperation,
) -> Result<bool, AppError> {
    let package = coordinate(&op.target.key)?;
    let marked = index
        .installations
        .get(&op.target.key)
        .is_some_and(|r| r.pending.as_ref() == Some(&op.target));
    if !marked
        && !matches!(
            op.phase,
            w::ApplicationPendingOperationPhase::Publishing
                | w::ApplicationPendingOperationPhase::Published
                | w::ApplicationPendingOperationPhase::Retiring
        )
    {
        return Ok(false);
    }
    let matches = op.publication_record.is_some()
        && identity::predecessor(index, &op.target.key)
            .map_err(|e| failure(settings, &package, e))?
            == op.publication_record;
    if (marked
        || matches!(
            op.phase,
            w::ApplicationPendingOperationPhase::Published
                | w::ApplicationPendingOperationPhase::Retiring
        ))
        && !matches
    {
        return Err(failure(
            settings,
            &package,
            "the published record changed after this operation parked".into(),
        ));
    }
    Ok(matches)
}

/// Save only current resupply obligations before suffix effects. Original request
/// origins still describe the initial/prefix binding; no answer or digest is saved.
pub(super) fn bind_resupply(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    provided: &[String],
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing resupply owner".into()))?;
    let mut changed = false;
    for id in provided {
        if !op.secret_required.contains(id) {
            op.secret_required.push(id.clone());
            changed = true;
        }
    }
    if changed {
        op.secret_required.sort();
        state.save(&work(settings, &package))?;
    }
    Ok(())
}

/// Filesystem-safe execution generation, shared by package and cohort owners.
pub(crate) fn valid_operation(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
        && !matches!(id, "." | "..")
}
