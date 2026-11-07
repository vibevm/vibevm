//! Durable transfer of successful descendants away from a refused ancestor.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE");
use super::{full_store, pending, store};
use crate::{
    AppError, ImportScope, ImportSecrets, Index, MutationLock, SettingsRoot, steps::StepsFile,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::{application::e3::import_work as wire, shared as w};
use w::{
    ApplicationImportCompletedBinding as Receipt, ApplicationImportCompletedBindingState as State,
};

fn bad(settings: &SettingsRoot, d: &wire::ImportWork, reason: &str) -> AppError {
    store::failure(settings, &d.target.operation, reason)
}

/// No filesystem access or source/secret reconstruction is needed for admission.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn validate(d: &wire::ImportWork) -> Result<(), String> {
    if d.cohort.completed_bindings.is_empty() {
        return Ok(());
    }
    let secrets = ImportSecrets::default();
    let scope = ImportScope::new(
        d.target.clone(),
        d.cohort.roots.clone(),
        d.cohort.bindings.clone(),
        BTreeMap::new(),
        &secrets,
    )
    .map_err(|_| "receipt scope is invalid".to_owned())?;
    for (key, r) in &d.cohort.completed_bindings {
        let target_ok = |t: &w::ApplicationPendingTarget| {
            t.home == d.target.home
                && t.project.is_none()
                && pending::valid_operation(&t.operation)
                && pending::coordinate(&t.key).is_ok()
                && d.cohort.bindings.contains_key(&t.key)
        };
        let origin = d.cohort.results.iter().any(|v| {
            v.action == w::ApplicationPendingImportResultAction::Refused
                && v.row_index == Some(r.origin_row)
                && v.key == r.origin_root.key
                && (v.target.as_ref() == Some(&r.origin_root)
                    || v.target.is_none() && r.state == State::Detached)
        });
        let previous_owner_ok = match &r.previous_parent_owner {
            w::ApplicationContinuationTarget::Import(owner) => {
                owner.target == d.target
                    && owner.row == r.origin_row
                    && r.previous_parent == r.origin_root
            }
            w::ApplicationContinuationTarget::Application(owner) => {
                target_ok(&owner.target) && owner.target != r.previous_parent
            }
        };
        if key != &r.target.key
            || !target_ok(&r.target)
            || !target_ok(&r.origin_root)
            || !target_ok(&r.previous_parent)
            || r.target == r.origin_root
            || r.target == r.previous_parent
            || r.scope != scope.fingerprint()
            || r.publication_record.is_empty()
            || d.cohort.roots.get(r.origin_row as usize) != Some(&r.origin_root.key)
            || !origin
            || !previous_owner_ok
        {
            return Err("completed binding receipt has inconsistent exact identities".into());
        }
        let mut queue = vec![r.origin_root.key.as_str()];
        let mut reachable = BTreeSet::new();
        while let Some(key) = queue.pop() {
            if reachable.insert(key)
                && let Some(binding) = d.cohort.bindings.get(key)
            {
                queue.extend(binding.application.requires.iter().map(|e| e.key.as_str()));
            }
        }
        if !reachable.contains(r.target.key.as_str())
            || !reachable.contains(r.previous_parent.key.as_str())
            || d.cohort
                .bindings
                .get(&r.previous_parent.key)
                .is_none_or(|b| !b.application.requires.iter().any(|e| e.key == r.target.key))
        {
            return Err("receipt is outside its origin root dependency closure".into());
        }
    }
    Ok(())
}

/// Structural ownership is usable at every durable transfer cut. Runtime callers
/// still verify the child's exact work, parent, scope and published result.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(crate) fn owns(d: &wire::ImportWork, row: u32, target: &w::ApplicationPendingTarget) -> bool {
    d.cohort
        .completed_bindings
        .get(&target.key)
        .is_some_and(|r| r.origin_row == row && r.target == *target)
}

/// Caller holds the mutation lock and has already classified the root as terminal
/// Refused. Only successful descendants are captured; the root is never blessed.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn preserve(
    lock: &MutationLock,
    settings: &SettingsRoot,
    d: &mut wire::ImportWork,
    failed: &StepsFile,
    fingerprint: &str,
) -> Result<(), AppError> {
    recover(lock, settings, d, fingerprint)?;
    let failed_op = failed.pending(settings)?;
    let root = &failed_op.target;
    let Some(w::ApplicationContinuationTarget::Import(parent)) = &failed_op.parent else {
        return Err(bad(settings, d, "refused root has no exact cohort owner"));
    };
    let row = parent.row;
    if parent.target != d.target
        || d.cohort.roots.get(row as usize) != Some(&root.key)
        || !d.cohort.results.iter().any(|r| {
            r.row_index == Some(row)
                && r.key == root.key
                && r.target.as_ref() == Some(root)
                && r.action == w::ApplicationPendingImportResultAction::Refused
        })
    {
        return Err(bad(
            settings,
            d,
            "refused root lacks its durably settled exact outcome",
        ));
    }
    let state = pending::load(
        settings,
        &pending::coordinate(&root.key)?,
        Some(&root.operation),
    )?;
    let op = state.pending(settings)?;
    if op.target != *root
        || op.parent.as_ref() != Some(&pending::import_reference(&d.target, row))
        || op.phase == w::ApplicationPendingOperationPhase::Complete
        || !pending::matches_import_scope(&state, fingerprint)
    {
        return Err(bad(
            settings,
            d,
            "refused root lineage, phase or scope differs",
        ));
    }
    let index = Index::load(settings)?;
    let empty = ImportSecrets::default();
    let imported = ImportScope::new(
        d.target.clone(),
        d.cohort.roots.clone(),
        d.cohort.bindings.clone(),
        Default::default(),
        &empty,
    )?;
    let capture = Capture {
        imported: &imported,
        settings,
        d,
        index: &index,
        root,
        row,
        fingerprint,
    };
    let mut collected = Collected::default();
    collect(&capture, &state, &mut collected)?;
    let mut staged = d.clone();
    for (key, receipt) in collected.receipts {
        if staged
            .cohort
            .completed_bindings
            .insert(key, receipt)
            .is_some()
        {
            return Err(bad(
                settings,
                d,
                "another receipt already reserves this binding",
            ));
        }
    }
    // No child ownership changes occur before this atomic durable save.
    full_store::save_full(settings, &staged)?;
    *d = staged;
    recover(lock, settings, d, fingerprint)
}

#[derive(Clone, Copy)]
struct Capture<'a> {
    settings: &'a SettingsRoot,
    d: &'a wire::ImportWork,
    index: &'a Index,
    root: &'a w::ApplicationPendingTarget,
    row: u32,
    fingerprint: &'a str,
    imported: &'a ImportScope<'a>,
}
#[derive(Default)]
struct Collected {
    seen: BTreeSet<(String, String)>,
    receipts: BTreeMap<String, Receipt>,
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn collect(
    capture: &Capture<'_>,
    state: &StepsFile,
    collected: &mut Collected,
) -> Result<(), AppError> {
    let Capture {
        settings,
        d,
        index,
        root,
        row,
        fingerprint,
        imported,
    } = *capture;
    let op = state.pending(settings)?;
    if !collected
        .seen
        .insert((op.target.key.clone(), op.target.operation.clone()))
        || !pending::matches_import_scope(state, fingerprint)
        || matches!(
            op.phase,
            w::ApplicationPendingOperationPhase::Removing
                | w::ApplicationPendingOperationPhase::Placing
        )
        || op
            .retirements
            .iter()
            .any(|r| r.state == w::ApplicationPendingRetirementState::Removing)
    {
        return Err(bad(
            settings,
            d,
            "refused subtree is cyclic, changed or has uncertain effects",
        ));
    }
    for edge in &op.children {
        let Some(target) = &edge.target else { continue };
        let child = pending::load(
            settings,
            &pending::coordinate(&target.key)?,
            Some(&target.operation),
        )?;
        let child_op = child.pending(settings)?;
        if child_op.target != *target
            || child_op.parent.as_ref() != Some(&pending::application_reference(&op.target))
        {
            return Err(bad(
                settings,
                d,
                "descendant generation or previous parent differs",
            ));
        }
        let imported_edge = imported
            .dependency(
                &op.target.key,
                &edge.requirement,
                match edge.scope {
                    w::ApplicationPendingChildScope::Build => {
                        vibe_core::manifest::RequirementScope::Build
                    }
                    w::ApplicationPendingChildScope::Runtime => {
                        vibe_core::manifest::RequirementScope::Runtime
                    }
                },
                edge.step.as_deref(),
            )?
            .is_some_and(|binding| binding.application.key == target.key);
        if child_op.phase == w::ApplicationPendingOperationPhase::Complete && imported_edge {
            let publication = child_op.publication_record.clone().ok_or_else(|| {
                bad(
                    settings,
                    d,
                    "completed descendant lacks publication identity",
                )
            })?;
            if !edge.completed
                || edge.key != target.key
                || edge.record_identity.as_ref() != Some(&publication)
            {
                return Err(bad(
                    settings,
                    d,
                    "completed descendant lacks its exact acknowledged edge",
                ));
            }
            let r = Receipt {
                target: target.clone(),
                publication_record: publication,
                scope: fingerprint.into(),
                origin_root: root.clone(),
                origin_row: row,
                previous_parent: op.target.clone(),
                previous_parent_owner: op.parent.clone().ok_or_else(|| {
                    bad(settings, d, "previous parent has no exact retained owner")
                })?,
                state: State::Captured,
            };
            prove(settings, d, index, &r, fingerprint, false)?;
            if collected.receipts.insert(target.key.clone(), r).is_some() {
                return Err(bad(
                    settings,
                    d,
                    "descendant binding has multiple retained attempts",
                ));
            }
        } else if edge.completed && child_op.phase != w::ApplicationPendingOperationPhase::Complete
        {
            return Err(bad(settings, d, "acknowledged descendant is not complete"));
        }
        collect(capture, &child, collected)?;
    }
    Ok(())
}

/// Recover only captured-before-move, moved-before-save and unlinked-before-save
/// cuts. Once Detached is durable, the old failed generation may be retried.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn recover(
    _lock: &MutationLock,
    settings: &SettingsRoot,
    d: &mut wire::ImportWork,
    fingerprint: &str,
) -> Result<(), AppError> {
    validate(d).map_err(|_| bad(settings, d, "invalid completed binding receipts"))?;
    let index = Index::load(settings)?;
    let keys: Vec<_> = d.cohort.completed_bindings.keys().cloned().collect();
    // Reparent the whole captured set before detaching any old forward edge.
    for key in &keys {
        let r = d.cohort.completed_bindings[key].clone();
        let mut state = prove(settings, d, &index, &r, fingerprint, false)?;
        if r.state == State::Captured {
            check_old_edge(settings, d, &r, false)?;
            state.pending_mut(settings)?.parent =
                Some(pending::import_reference(&d.target, r.origin_row));
            state.save(&pending::work(settings, &state.package))?;
            set_state(settings, d, key, State::Reparented)?;
        }
    }
    for key in keys {
        let r = d.cohort.completed_bindings[&key].clone();
        if r.state == State::Reparented {
            let mut parent = check_old_edge(settings, d, &r, true)?;
            parent
                .pending_mut(settings)?
                .children
                .retain(|edge| edge.target.as_ref() != Some(&r.target));
            if !parent
                .pending(settings)?
                .children
                .iter()
                .any(|edge| edge.key == r.target.key)
            {
                // The child now owns this observation in its retained steps/index.
                parent.source_evidence.remove(&r.target.key);
            }
            parent.save(&pending::work(settings, &parent.package))?;
            set_state(settings, d, &key, State::Detached)?;
        }
    }
    Ok(())
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn set_state(
    settings: &SettingsRoot,
    d: &mut wire::ImportWork,
    key: &str,
    state: State,
) -> Result<(), AppError> {
    let mut staged = d.clone();
    staged
        .cohort
        .completed_bindings
        .get_mut(key)
        .ok_or_else(|| bad(settings, d, "enumerated completed binding disappeared"))?
        .state = state;
    full_store::save_full(settings, &staged)?;
    *d = staged;
    Ok(())
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn check_old_edge(
    settings: &SettingsRoot,
    d: &wire::ImportWork,
    r: &Receipt,
    absent_ok: bool,
) -> Result<StepsFile, AppError> {
    let state = pending::load(
        settings,
        &pending::coordinate(&r.previous_parent.key)?,
        Some(&r.previous_parent.operation),
    )?;
    let op = state.pending(settings)?;
    let owner_ok = op.parent.as_ref() == Some(&r.previous_parent_owner)
        || d.cohort
            .completed_bindings
            .get(&r.previous_parent.key)
            .is_some_and(|parent| {
                parent.target == r.previous_parent
                    && parent.origin_root == r.origin_root
                    && parent.origin_row == r.origin_row
                    && op.parent.as_ref()
                        == Some(&pending::import_reference(&d.target, parent.origin_row))
            });
    let edges: Vec<_> = op
        .children
        .iter()
        .filter(|e| e.key == r.target.key || e.target.as_ref() == Some(&r.target))
        .collect();
    if op.target != r.previous_parent
        || !owner_ok
        || !pending::matches_import_scope(&state, &r.scope)
        || !(edges.is_empty() && absent_ok
            || edges.len() == 1
                && edges[0].target.as_ref() == Some(&r.target)
                && edges[0].completed
                && edges[0].record_identity.as_ref() == Some(&r.publication_record))
    {
        return Err(bad(settings, d, "receipt previous parent edge changed"));
    }
    Ok(state)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn prove(
    settings: &SettingsRoot,
    d: &wire::ImportWork,
    index: &Index,
    r: &Receipt,
    fingerprint: &str,
    detached: bool,
) -> Result<StepsFile, AppError> {
    let state = pending::load(
        settings,
        &pending::coordinate(&r.target.key)?,
        Some(&r.target.operation),
    )?;
    let op = state.pending(settings)?;
    let imported = pending::import_reference(&d.target, r.origin_row);
    let previous = pending::application_reference(&r.previous_parent);
    let parent_ok = op.parent.as_ref() == Some(&imported)
        || r.state == State::Captured && op.parent.as_ref() == Some(&previous);
    family_ready(settings, d, index, &state.package, &r.target)?;
    let record = index
        .installations
        .get(&r.target.key)
        .ok_or_else(|| bad(settings, d, "retained completed binding disappeared"))?;
    let identity = pending::record_identity(record)
        .map_err(|_| bad(settings, d, "cannot identify retained completed binding"))?;
    if op.target != r.target
        || !parent_ok
        || r.scope != fingerprint
        || detached && r.state != State::Detached
        || !pending::matches_import_scope(&state, fingerprint)
        || op.phase != w::ApplicationPendingOperationPhase::Complete
        || op.publication_record.as_ref() != Some(&r.publication_record)
        || identity != r.publication_record
        || record.pending.is_some()
        || op
            .retirements
            .iter()
            .any(|v| v.state != w::ApplicationPendingRetirementState::Removed)
    {
        return Err(bad(
            settings,
            d,
            "receipt work, ownership, scope or publication differs",
        ));
    }
    Ok(state)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
pub(super) fn completed(
    settings: &SettingsRoot,
    d: &wire::ImportWork,
    key: &str,
    index: &Index,
    fingerprint: &str,
) -> Result<bool, AppError> {
    let Some(r) = d.cohort.completed_bindings.get(key) else {
        return Ok(false);
    };
    validate(d).map_err(|_| bad(settings, d, "invalid completed binding receipts"))?;
    prove(settings, d, index, r, fingerprint, true)?;
    Ok(true)
}

// An index hit cannot hide an unindexed newer attempt. Use the canonical
// package-directory owner, and never decode source/executor inputs here.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn family_ready(
    settings: &SettingsRoot,
    d: &wire::ImportWork,
    index: &Index,
    package: &crate::PackageCoordinate,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let name = package.directory_name();
    let prefix = name
        .strip_suffix(&package.version)
        .ok_or_else(|| bad(settings, d, "invalid canonical receipt work identity"))?;
    let entries = std::fs::read_dir(settings.app_work_dir())
        .map_err(|_| bad(settings, d, "cannot inspect retained binding work"))?;
    for entry in entries {
        let entry =
            entry.map_err(|_| bad(settings, d, "cannot enumerate retained binding work"))?;
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            return Err(bad(settings, d, "retained binding family has unsafe work"));
        }
        let state = StepsFile::load(&entry.path())
            .map_err(|_| bad(settings, d, "retained binding family has unreadable work"))?
            .ok_or_else(|| bad(settings, d, "retained binding family has unknown work"))?;
        if !state.package.same_package(package)
            || entry.path() != pending::work(settings, &state.package)
        {
            return Err(bad(
                settings,
                d,
                "retained binding family work identity differs",
            ));
        }
        if let Some(op) = &state.operation {
            pending::load(settings, &state.package, Some(&op.target.operation))?;
            if op.target != *target
                && (op.phase != w::ApplicationPendingOperationPhase::Complete
                    || op.parent.is_some()
                    || op.publication_record.is_none())
            {
                return Err(bad(
                    settings,
                    d,
                    "retained binding has unfinished or owned replacement work",
                ));
            }
        } else if !index.installations.contains_key(&state.package.key())
            || state.steps.iter().any(|s| {
                !matches!(
                    s.outcome,
                    crate::StepOutcome::Done | crate::StepOutcome::Skipped
                )
            })
        {
            return Err(bad(
                settings,
                d,
                "retained binding has unproven legacy replacement work",
            ));
        }
    }
    if index
        .installations
        .values()
        .any(|v| v.package.same_package(package) && v.pending.is_some())
    {
        return Err(bad(
            settings,
            d,
            "retained binding family has pending publication",
        ));
    }
    Ok(())
}
