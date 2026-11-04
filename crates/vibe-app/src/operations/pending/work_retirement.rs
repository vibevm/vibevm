//! Retire exact owned work before its parent generation disappears.
use super::*;

pub(super) struct ChildWork {
    state: StepsFile,
    path: PathBuf,
}
/// Read and validate the whole retained subtree before unlinking anything.
pub(super) fn prepare(
    settings: &SettingsRoot,
    parent: &StepsFile,
    terminal: bool,
) -> Result<Vec<ChildWork>, AppError> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    if let Some(op) = &parent.operation {
        seen.insert((op.target.key.clone(), op.target.operation.clone()));
    }
    collect(settings, parent, terminal, &mut seen, &mut out)?;
    Ok(out)
}
fn collect(
    settings: &SettingsRoot,
    parent: &StepsFile,
    terminal: bool,
    seen: &mut std::collections::BTreeSet<(String, String)>,
    out: &mut Vec<ChildWork>,
) -> Result<(), AppError> {
    let Some(owner) = &parent.operation else {
        return Ok(());
    };
    let released = terminal || owner.phase == w::ApplicationPendingOperationPhase::Complete;
    for target in owner.children.iter().filter_map(|b| b.target.as_ref()) {
        let package = coordinate(&target.key)?;
        let path = work(settings, &package);
        let Some(child) = StepsFile::load(&path)? else {
            if released {
                continue;
            }
            return Err(failure(
                settings,
                &package,
                "a retained child disappeared before its owner completed".into(),
            ));
        };
        let Some(op) = &child.operation else {
            return Err(failure(
                settings,
                &package,
                "retained child was replaced by unverifiable legacy work".into(),
            ));
        };
        if op.target != *target {
            if released {
                continue;
            }
            return Err(failure(
                settings,
                &package,
                "retained child generation changed before its owner completed".into(),
            ));
        }
        if op.parent.as_ref() != Some(&application_reference(&owner.target)) {
            if released
                && op.parent.is_none()
                && op.phase == w::ApplicationPendingOperationPhase::Complete
            {
                continue;
            }
            return Err(failure(
                settings,
                &package,
                "cannot retire work owned by a different parent".into(),
            ));
        }
        load(settings, &package, Some(&target.operation))?;
        if !seen.insert((target.key.clone(), target.operation.clone())) {
            continue;
        }
        if matches!(
            op.phase,
            w::ApplicationPendingOperationPhase::Removing
                | w::ApplicationPendingOperationPhase::Placing
        ) {
            return Err(failure(
                settings,
                &package,
                format!(
                    "child has an uncertain removal/placement outcome; inspect `{}` before explicitly abandoning the whole owner",
                    command(target)
                ),
            ));
        }
        if !terminal && op.phase != w::ApplicationPendingOperationPhase::Complete {
            return Err(failure(
                settings,
                &package,
                "owner cannot complete while a retained child is unfinished".into(),
            ));
        }
        collect(settings, &child, terminal, seen, out)?;
        out.push(ChildWork { state: child, path });
    }
    Ok(())
}
/// The same retirement policy serves successful completion, terminal refusal,
/// and an explicit Fresh/from-step replacement. It never follows newer work.
pub(super) fn release(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    children: Vec<ChildWork>,
) -> Result<(), AppError> {
    for child in children {
        let target = &child.state.pending(settings)?.target;
        finish_import_row(lock, settings, target)?;
        let mut current = load(settings, &child.state.package, Some(&target.operation))?;
        let op = current.pending_mut(settings)?;
        op.parent = None;
        let keep = op.request.keep_work
            || current.steps.iter().any(|s| {
                matches!(
                    s.outcome,
                    crate::StepOutcome::Failed
                        | crate::StepOutcome::Interrupted
                        | crate::StepOutcome::Delegated
                )
            });
        if keep {
            current.save(&child.path)?;
        } else {
            fs::remove_dir_all(&child.path)
                .map_err(|e| failure(settings, &child.state.package, e.to_string()))?;
        }
    }
    Ok(())
}
pub(super) fn before_replacement(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    old: &StepsFile,
) -> Result<(), AppError> {
    let Some(op) = &old.operation else {
        return Ok(());
    };
    if op
        .retirements
        .iter()
        .any(|row| row.state == w::ApplicationPendingRetirementState::Removing)
    {
        return Err(failure(settings,&old.package,"this operation retains an uncertain old-version removal; inspect its uninstall log and explicitly abandon work with the correctly scoped `app work --clean` command before a fresh request".into()));
    }
    if let Some(owner) = retained_for(settings, old)? {
        return Err(failure(
            settings,
            &old.package,
            format!(
                "this generation is retained for `{}`; continue or explicitly clean that owner before replacing the child",
                continuation_command(&owner)
            ),
        ));
    }
    let children = prepare(settings, old, true)?;
    finish_import_row(lock, settings, &op.target)?;
    release(lock, settings, children)
}
pub(in crate::operations) fn refused_owner(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    old: &StepsFile,
) -> Result<(), AppError> {
    let Some(op) = &old.operation else {
        return Ok(());
    };
    let children = prepare(settings, old, true)?;
    finish_import_row(lock, settings, &op.target)?;
    release(lock, settings, children)
}
