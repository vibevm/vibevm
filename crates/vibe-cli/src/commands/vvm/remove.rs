//! `vibe self remove` and `vibe self gc` over immutable instances. Scoped
//! binary-bundle removal can retain `bin/` or source independently; default
//! removal drops both. GC preserves the active instance plus immediate
//! rollback. A committer's external tree is never touched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#remove");

use std::fmt;
use std::fs;

use anyhow::{Context, Result};
use dialoguer::{MultiSelect, Select};
use vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME;

use super::error::VvmError;
use super::model::{self, InstallRecord, InstanceId, Selector, VersionId};
use super::provenance;
use super::store::VersionStore;
use super::{VvmEnv, confirm, forced_kind, require_tty, resolve_installed};
use crate::cli::{VvmGcArgs, VvmRemoveArgs};
use crate::output;

#[path = "remove_guard.rs"]
mod guard;
use guard::{
    RemovalEffects, guard_remove_targets, remove_mirror_after, removes_last_managed,
    scoped_removal_notes,
};

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

fn gc_protected(
    record: &InstallRecord,
    active: &InstallRecord,
    rollback: Option<&InstallRecord>,
    running: Option<&InstallRecord>,
) -> bool {
    same_record(record, active)
        || rollback.is_some_and(|saved| same_record(record, saved))
        || running.is_some_and(|saved| same_record(record, saved))
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
    let store = env.store()?;
    let _lock = super::install::InstallLock::acquire(&store)?;
    let mut state = store.load_state()?;
    let initial_records = state.installs.len();
    let active = store.active()?;
    let running = provenance::running_record(&store)?;

    let targets: Vec<RemoveTarget> = if args.all {
        let ids = distinct_ids(&state);
        if ids.is_empty() {
            ctx.summary("no versions installed.");
            return Ok(());
        }
        if !confirm(
            ctx,
            args.yes,
            &format!("Remove ALL {} version(s)?", ids.len()),
        )? {
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
        pick_ids(ctx, &state)?
            .into_iter()
            .map(RemoveTarget::Version)
            .collect()
    };

    if targets.is_empty() {
        ctx.summary("nothing to remove.");
        return Ok(());
    }
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

    let selected_origin = |origin| {
        state.installs.iter().any(|record| {
            record.origin == origin && effective.iter().any(|target| target.matches(record))
        })
    };
    let selected_external = selected_origin(model::Origin::External);
    let selected_managed = selected_origin(model::Origin::Managed);
    let selected_binary = selected_origin(model::Origin::Binary);

    guard_remove_targets(&store, &state, &effective, scope)?;
    let mirror_candidate = removes_last_managed(&state, &effective, scope);
    if mirror_candidate {
        store.guard_mutation_tree(&store.mirror_dir())?;
    }

    let previous = store.previous()?;
    let pointer_targeted = active
        .as_ref()
        .into_iter()
        .chain(previous.as_ref())
        .any(|record| effective.iter().any(|target| target.matches(record)));
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) && pointer_targeted {
        repair_activation_before_removal(&store, &state, &effective)?;
    }
    let mut effects = RemovalEffects::default();
    for target in &effective {
        effects.merge(remove_target(ctx, &store, &mut state, target, scope)?);
    }
    let remove_mirror = remove_mirror_after(mirror_candidate, scope, &state);
    if remove_mirror {
        let mirror = store.mirror_dir();
        if mirror.exists() {
            fs::remove_dir_all(&mirror).with_context(|| {
                format!("removing shared managed mirror `{}`", mirror.display())
            })?;
            ctx.removed(&mirror.display().to_string());
            effects.any = true;
            effects.managed_source = true;
        }
    }
    if state.installs.len() != initial_records {
        store.save_state(&state)?;
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

enum GcAction {
    Build,
    Prune,
    Cancel,
}

pub(super) fn run_gc_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmGcArgs) -> Result<()> {
    let store = env.store()?;
    let _lock = super::install::InstallLock::acquire(&store)?;
    let action = if args.build {
        GcAction::Build
    } else if args.prune_others {
        GcAction::Prune
    } else {
        gc_menu(ctx)?
    };

    match action {
        GcAction::Cancel => ctx.summary("nothing to do."),
        GcAction::Build => {
            let dir = store.build_dir();
            store.guard_mutation_tree(&dir)?;
            if dir.exists() {
                fs::remove_dir_all(&dir)
                    .with_context(|| format!("removing `{}`", dir.display()))?;
                ctx.summary("cleaned the Rust build cache.");
            } else {
                ctx.summary("build cache already empty.");
            }
        }
        GcAction::Prune => {
            let active = store.active()?.ok_or(VvmError::NoActiveVersion)?;
            let running = provenance::running_record(&store)?;
            let mut state = store.load_state()?;
            let saved_previous = store.previous()?;
            let rollback = saved_previous
                .as_ref()
                .filter(|record| {
                    !same_record(record, &active)
                        && super::ensure_activatable(&store, record).is_ok()
                })
                .cloned()
                .or_else(|| {
                    state
                        .installs
                        .iter()
                        .filter(|record| !same_record(record, &active))
                        .filter(|record| super::ensure_activatable(&store, record).is_ok())
                        .max_by_key(|record| record.instance)
                        .cloned()
                });
            let others: Vec<_> = state
                .installs
                .iter()
                .filter(|record| {
                    !gc_protected(record, &active, rollback.as_ref(), running.as_ref())
                })
                .cloned()
                .collect();
            let active_path = store.instance_dir(&active.version_id(), active.instance);
            let rollback_path = rollback
                .as_ref()
                .map(|record| store.instance_dir(&record.version_id(), record.instance));
            if others.is_empty() {
                store.reset_activation(Some(&active_path), rollback_path.as_deref())?;
                ctx.summary("no instances outside the active + rollback pair to prune.");
                return Ok(());
            }
            for record in &others {
                store.guard_mutation_tree(
                    &store.instance_dir(&record.version_id(), record.instance),
                )?;
            }
            let remove_mirror = !state.installs.iter().any(|record| {
                record.origin == model::Origin::Managed
                    && !others.iter().any(|removed| same_record(record, removed))
            });
            if remove_mirror {
                store.guard_mutation_tree(&store.mirror_dir())?;
            }
            let build_dir = store.build_dir();
            store.guard_mutation_tree(&build_dir)?;
            if !confirm(
                ctx,
                args.yes,
                &format!(
                    "Remove {} instance(s) except active, immediate rollback, and running? This cannot be undone.",
                    others.len()
                ),
            )? {
                ctx.summary("aborted.");
                return Ok(());
            }
            store.reset_activation(Some(&active_path), rollback_path.as_deref())?;
            let mut pruned = 0usize;
            for r in &others {
                let dir = store.instance_dir(&r.version_id(), r.instance);
                store.guard_mutation_tree(&dir)?;
                let forget = if dir.exists() {
                    match fs::remove_dir_all(&dir) {
                        Ok(()) => true,
                        Err(error) => {
                            ctx.summary(&format!("skipped {} (in use?): {error}", dir.display()));
                            false
                        }
                    }
                } else {
                    true
                };
                if forget {
                    state.installs.retain(|candidate| {
                        !(candidate.version_id() == r.version_id()
                            && candidate.instance == r.instance)
                    });
                    pruned += 1;
                }
            }
            if pruned > 0 {
                store.save_state(&state)?;
            }
            if remove_mirror
                && !state
                    .installs
                    .iter()
                    .any(|record| record.origin == model::Origin::Managed)
            {
                let mirror = store.mirror_dir();
                if mirror.exists() {
                    fs::remove_dir_all(&mirror).with_context(|| {
                        format!("removing shared managed mirror `{}`", mirror.display())
                    })?;
                }
            }
            store.guard_mutation_tree(&build_dir)?;
            if build_dir.exists() {
                let _ = fs::remove_dir_all(&build_dir);
            }
            ctx.summary(&format!(
                "pruned {} instance(s); kept active, immediate rollback, and running.",
                pruned
            ));
        }
    }
    Ok(())
}

fn gc_menu(ctx: &output::Context) -> Result<GcAction> {
    require_tty(
        ctx,
        "pass `--build` (clean the Rust build cache) or `--prune-others`",
    )?;
    let items = [
        "Clean the Rust build cache (the shared --target-dir)",
        "Prune all instances except the active",
    ];
    let sel = Select::new()
        .with_prompt("vibe self gc")
        .items(items)
        .default(0)
        .interact()
        .ok();
    Ok(match sel {
        Some(0) => GcAction::Build,
        Some(1) => GcAction::Prune,
        _ => GcAction::Cancel,
    })
}

#[cfg(test)]
#[path = "remove_tests.rs"]
mod tests;
