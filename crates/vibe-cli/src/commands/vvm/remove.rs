//! `vibe self remove` and `vibe self gc` over immutable instances. Scoped
//! binary-bundle removal can retain `bin/` or source independently; default
//! removal drops both. GC preserves the active instance plus immediate
//! rollback. A committer's external tree is never touched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#remove");

use std::fmt;
use std::fs;

use anyhow::{Context, Result};
use dialoguer::MultiSelect;
use vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME;

use super::model::{self, InstallRecord, InstanceId, Selector, VersionId};
use super::provenance;
use super::store::VersionStore;
use super::{VvmEnv, confirm, forced_kind, require_tty, resolve_installed};
use crate::cli::VvmRemoveArgs;
use crate::output;

#[path = "remove_guard.rs"]
mod guard;
use guard::{
    RemovalEffects, guard_remove_targets, remove_mirror_after, removes_last_managed,
    scoped_removal_notes,
};

#[path = "gc.rs"]
mod gc;
#[cfg(test)]
use gc::gc_protected;
pub(super) use gc::{GcOutcome, run_gc_cmd};

/// What `self remove` deletes for a version (PROP-019 §2.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoveScope {
    Bin,
    Src,
    Both,
}

fn removal_scope(bin: bool, src: bool) -> RemoveScope {
    match (bin, src) {
        (true, false) => RemoveScope::Bin,
        (false, true) => RemoveScope::Src,
        _ => RemoveScope::Both,
    }
}

/// Distinct version ids in the inventory, in first-seen order.
fn distinct_ids(state: &model::State) -> Vec<VersionId> {
    let mut ids: Vec<VersionId> = Vec::new();
    for r in &state.installs {
        let id = r.version_id();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoveTarget {
    Version(VersionId),
    Instance(InstanceId),
}

impl RemoveTarget {
    fn version_id(&self) -> &VersionId {
        match self {
            RemoveTarget::Version(id) => id,
            RemoveTarget::Instance(id) => &id.version,
        }
    }

    fn matches(&self, record: &InstallRecord) -> bool {
        match self {
            RemoveTarget::Version(id) => &record.version_id() == id,
            RemoveTarget::Instance(id) => {
                record.version_id() == id.version && record.instance == id.instance
            }
        }
    }
}

impl fmt::Display for RemoveTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoveTarget::Version(id) => id.fmt(f),
            RemoveTarget::Instance(id) => id.fmt(f),
        }
    }
}

fn protection_reason<'a>(
    target: &RemoveTarget,
    active: Option<&'a InstallRecord>,
    running: Option<&'a InstallRecord>,
) -> Option<(&'static str, &'a InstallRecord)> {
    running
        .filter(|record| target.matches(record))
        .map(|record| ("running", record))
        .or_else(|| {
            active
                .filter(|record| target.matches(record))
                .map(|record| ("active", record))
        })
}

fn protection_blocks(reason: &str, force: bool) -> bool {
    reason == "running" || !force
}

/// Apply bin/source/both to each selected immutable instance. Binary bundles
/// own their `bin/` and `source/` independently; external worktrees are never
/// touched. Whole-instance removal remains best-effort around running locks.
fn remove_target(
    ctx: &output::Context,
    store: &VersionStore,
    state: &mut model::State,
    target: &RemoveTarget,
    scope: RemoveScope,
) -> Result<RemovalEffects> {
    let id = target.version_id();
    let records = store
        .instances_of(id)?
        .into_iter()
        .filter(|record| target.matches(record))
        .collect::<Vec<_>>();
    let mut effects = RemovalEffects::default();
    for record in &records {
        let home = store.instance_dir(id, record.instance);
        store.guard_mutation_path(&home)?;
        match scope {
            RemoveScope::Both => {
                store.guard_mutation_tree(&home)?;
                let forget = if home.exists() {
                    match fs::remove_dir_all(&home) {
                        Ok(()) => {
                            ctx.removed(&home.display().to_string());
                            effects.any = true;
                            true
                        }
                        Err(error) => {
                            ctx.summary(&format!("skipped {} (in use?): {error}", home.display()));
                            effects.failed = true;
                            false
                        }
                    }
                } else {
                    effects.any = true;
                    true
                };
                if forget {
                    state.installs.retain(|candidate| {
                        !(candidate.version_id() == *id && candidate.instance == record.instance)
                    });
                }
            }
            RemoveScope::Bin => {
                let bin = store.instance_bin_dir(id, record.instance);
                store.guard_mutation_tree(&bin)?;
                if bin.exists() {
                    fs::remove_dir_all(&bin)
                        .with_context(|| format!("removing `{}`", bin.display()))?;
                    ctx.removed(&bin.display().to_string());
                    effects.any = true;
                    effects.bin = true;
                }
                let legacy = home.join(super::store::BINARY_NAME);
                store.guard_mutation_path(&legacy)?;
                if legacy.is_file() {
                    fs::remove_file(&legacy)
                        .with_context(|| format!("removing `{}`", legacy.display()))?;
                    ctx.removed(&legacy.display().to_string());
                    effects.any = true;
                    effects.bin = true;
                }
            }
            RemoveScope::Src if record.origin == model::Origin::Binary => {
                let source = store.instance_source_dir(id, record.instance);
                store.guard_mutation_tree(&source)?;
                if source.exists() {
                    fs::remove_dir_all(&source)
                        .with_context(|| format!("removing `{}`", source.display()))?;
                    ctx.removed(&source.display().to_string());
                    effects.any = true;
                    effects.binary_source = true;
                }
                let archive = home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME);
                store.guard_mutation_path(&archive)?;
                if archive.is_file() {
                    fs::remove_file(&archive)
                        .with_context(|| format!("removing `{}`", archive.display()))?;
                    ctx.removed(&archive.display().to_string());
                    effects.any = true;
                    effects.binary_source = true;
                }
            }
            RemoveScope::Src => {}
        }
    }
    if !effects.any {
        ctx.summary(&format!("{target}: nothing on disk to remove"));
    }
    Ok(effects)
}

pub(super) fn run_remove_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmRemoveArgs,
) -> Result<()> {
    let progress = ctx.progress();
    let inspection = progress.task("Inspecting removal inventory");
    let inspected = (|| -> Result<_> {
        let store = env.store()?;
        if let Some(root) = &env.root {
            inspection.detail(format!("store: {}", root.display()));
        }
        let lock = super::install::InstallLock::acquire(&store)?;
        let state = store.load_state()?;
        let active = store.active()?;
        let running = provenance::running_record(&store)?;
        Ok((store, lock, state, active, running))
    })();
    let (store, _lock, mut state, active, running) = match inspected {
        Ok(value) => {
            inspection.finish();
            value
        }
        Err(error) => {
            inspection.fail(error.to_string());
            return Err(error);
        }
    };
    let initial_records = state.installs.len();

    let selection = progress.task("Selecting removal targets");
    let targets: Vec<RemoveTarget> = if args.all {
        let ids = distinct_ids(&state);
        if ids.is_empty() {
            selection.skip("no versions installed");
            ctx.summary("no versions installed.");
            return Ok(());
        }
        if !ctx.suspend_progress(|| {
            confirm(
                ctx,
                args.yes,
                &format!("Remove ALL {} version(s)?", ids.len()),
            )
        })? {
            selection.skip("declined");
            ctx.summary("aborted.");
            return Ok(());
        }
        ids.into_iter().map(RemoveTarget::Version).collect()
    } else if let Some(raw) = args.selector.as_deref() {
        let selector = model::Selector::parse(raw, forced_kind(&args.kind))?;
        let record = resolve_installed(&state, &selector, raw)?;
        vec![match selector {
            Selector::Exact(instance) => RemoveTarget::Instance(instance),
            _ => RemoveTarget::Version(record.version_id()),
        }]
    } else {
        ctx.suspend_progress(|| pick_ids(ctx, &state))?
            .into_iter()
            .map(RemoveTarget::Version)
            .collect()
    };

    if targets.is_empty() {
        selection.skip("no targets selected");
        ctx.summary("nothing to remove.");
        return Ok(());
    }
    let selected_count = targets.len();
    let scope = removal_scope(args.bin, args.src);
    let mut effective = Vec::new();
    for target in targets {
        let protected = protection_reason(&target, active.as_ref(), running.as_ref());
        if let Some((reason, record)) = protected
            && protection_blocks(reason, args.force)
        {
            if reason == "running" {
                ctx.summary(&format!(
                    "skipped running {} — the executing instance cannot remove itself",
                    record.selector()
                ));
            } else {
                ctx.summary(&format!(
                    "skipped active {} — use --force to remove it",
                    record.selector()
                ));
            }
            continue;
        }
        effective.push(target);
    }
    selection.detail(format!(
        "selected: {}; removable: {}",
        selected_count,
        effective.len()
    ));
    selection.finish();

    let selected_origin = |origin| {
        state.installs.iter().any(|record| {
            record.origin == origin && effective.iter().any(|target| target.matches(record))
        })
    };
    let selected_external = selected_origin(model::Origin::External);
    let selected_managed = selected_origin(model::Origin::Managed);
    let selected_binary = selected_origin(model::Origin::Binary);

    let preflight = progress.task("Preflighting removal targets");
    for target in &effective {
        preflight.detail(format!("target: {target}"));
    }
    if let Err(error) = guard_remove_targets(&store, &state, &effective, scope) {
        preflight.fail(error.to_string());
        return Err(error);
    }
    let mirror_candidate = removes_last_managed(&state, &effective, scope);
    if mirror_candidate {
        let mirror = store.mirror_dir();
        preflight.detail(format!("managed mirror: {}", mirror.display()));
        if let Err(error) = store.guard_mutation_tree(&mirror) {
            preflight.fail(error.to_string());
            return Err(error.into());
        }
    }
    preflight.finish();

    let previous = store.previous()?;
    let pointer_targeted = active
        .as_ref()
        .into_iter()
        .chain(previous.as_ref())
        .any(|record| effective.iter().any(|target| target.matches(record)));
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) && pointer_targeted {
        let repair = progress.task("Repairing activation pointers");
        match repair_activation_before_removal(&store, &state, &effective) {
            Ok(()) => repair.finish(),
            Err(error) => {
                repair.fail(error.to_string());
                return Err(error);
            }
        }
    }
    let mut effects = RemovalEffects::default();
    let removals = progress.task("Removing selected versions");
    removals.set_progress(0, Some(effective.len() as u64), "targets");
    for (index, target) in effective.iter().enumerate() {
        let component = removals.progress().task(format!("Removing {target}"));
        let target_effects = match remove_target(ctx, &store, &mut state, target, scope) {
            Ok(target_effects) => target_effects,
            Err(error) => {
                component.fail(error.to_string());
                removals.fail("target removal failed");
                return Err(error);
            }
        };
        if target_effects.failed {
            component.fail("one or more instances retained for later retry");
        } else if target_effects.any {
            component.finish();
        } else {
            component.skip("nothing on disk to remove");
        }
        effects.merge(target_effects);
        removals.set_progress((index + 1) as u64, Some(effective.len() as u64), "targets");
    }
    if effective.is_empty() {
        removals.skip("all selected targets are protected");
    } else if effects.failed {
        removals.skip("one or more instances retained for later retry");
    } else {
        removals.finish();
    }
    let remove_mirror = remove_mirror_after(mirror_candidate, scope, &state);
    if remove_mirror {
        let mirror = store.mirror_dir();
        let mirror_task = progress.task("Removing unused managed source mirror");
        mirror_task.detail(format!("path: {}", mirror.display()));
        if mirror.exists() {
            if let Err(error) = fs::remove_dir_all(&mirror)
                .with_context(|| format!("removing shared managed mirror `{}`", mirror.display()))
            {
                mirror_task.fail(error.to_string());
                return Err(error);
            }
            ctx.removed(&mirror.display().to_string());
            effects.any = true;
            effects.managed_source = true;
            mirror_task.finish();
        } else {
            mirror_task.skip("already absent");
        }
    }
    if state.installs.len() != initial_records {
        let saving = progress.task("Saving version inventory");
        saving.detail(format!("state: {}", store.state_path().display()));
        match store.save_state(&state) {
            Ok(()) => saving.finish(),
            Err(error) => {
                saving.fail(error.to_string());
                return Err(error.into());
            }
        }
    } else {
        let saving = progress.task("Saving version inventory");
        saving.skip("inventory unchanged");
    }
    if !effective.is_empty() {
        for note in scoped_removal_notes(
            scope,
            effects,
            selected_external,
            selected_managed,
            selected_binary,
            mirror_candidate,
        ) {
            ctx.summary(note);
        }
    }
    ctx.summary("done.");
    Ok(())
}

fn repair_activation_before_removal(
    store: &VersionStore,
    state: &model::State,
    targets: &[RemoveTarget],
) -> Result<()> {
    let survivors: Vec<&InstallRecord> = state
        .installs
        .iter()
        .filter(|record| !targets.iter().any(|target| target.matches(record)))
        .filter(|record| super::ensure_activatable(store, record).is_ok())
        .collect();
    let active = store.active()?;
    let previous = store.previous()?;
    let current = active
        .as_ref()
        .and_then(|candidate| {
            survivors
                .iter()
                .copied()
                .find(|record| same_record(record, candidate))
        })
        .or_else(|| {
            previous.as_ref().and_then(|candidate| {
                survivors
                    .iter()
                    .copied()
                    .find(|record| same_record(record, candidate))
            })
        })
        .or_else(|| {
            survivors
                .iter()
                .copied()
                .max_by_key(|record| record.instance)
        });
    let rollback = survivors
        .iter()
        .copied()
        .find(|record| {
            previous
                .as_ref()
                .is_some_and(|candidate| same_record(record, candidate))
                && current.is_none_or(|active| !same_record(record, active))
        })
        .or_else(|| {
            survivors
                .iter()
                .copied()
                .filter(|record| current.is_none_or(|active| !same_record(record, active)))
                .max_by_key(|record| record.instance)
        });
    let current_path =
        current.map(|record| store.instance_dir(&record.version_id(), record.instance));
    let previous_path =
        rollback.map(|record| store.instance_dir(&record.version_id(), record.instance));
    store.reset_activation(current_path.as_deref(), previous_path.as_deref())?;
    Ok(())
}

fn same_record(left: &InstallRecord, right: &InstallRecord) -> bool {
    left.version_id() == right.version_id() && left.instance == right.instance
}

/// Interactively pick version ids to remove. A non-TTY / unattended run with
/// no selector is an error, not a wipe (PROP-019 §2.9).
fn pick_ids(ctx: &output::Context, state: &model::State) -> Result<Vec<VersionId>> {
    let ids = distinct_ids(state);
    if ids.is_empty() {
        ctx.summary("no versions installed.");
        return Ok(Vec::new());
    }
    require_tty(
        ctx,
        "no version selected: pass a selector (e.g. `vibe self remove tag:1.2.3`) or `--all`",
    )?;
    let labels: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    let chosen = MultiSelect::new()
        .with_prompt("Select versions to remove (space toggles, enter confirms)")
        .items(&labels)
        .interact()
        .unwrap_or_default();
    Ok(chosen.into_iter().map(|i| ids[i].clone()).collect())
}

#[cfg(test)]
#[path = "remove_tests.rs"]
mod tests;
