//! Progress-observed `vibe self gc` over the guarded VVM store.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#GC-STAGES");

use std::fs;

use anyhow::{Context, Result};
use dialoguer::Select;
use vibe_core::progress::{Progress, ProgressTask};

use super::*;
use crate::cli::VvmGcArgs;
use crate::commands::vvm::error::VvmError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::commands::vvm) enum GcOutcome {
    Finished,
    Skipped(&'static str),
}

enum GcAction {
    Build,
    Prune,
    Cancel,
}

pub(super) fn gc_protected(
    record: &InstallRecord,
    active: &InstallRecord,
    rollback: Option<&InstallRecord>,
    running: Option<&InstallRecord>,
) -> bool {
    same_record(record, active)
        || rollback.is_some_and(|saved| same_record(record, saved))
        || running.is_some_and(|saved| same_record(record, saved))
}

pub(in crate::commands::vvm) fn run_gc_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmGcArgs,
) -> Result<GcOutcome> {
    let progress = ctx.progress();
    run_gc_with_progress(ctx, env, args, &progress)
}

fn run_gc_with_progress(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmGcArgs,
    progress: &Progress,
) -> Result<GcOutcome> {
    let inspection = progress.task("Inspecting version store");
    if let Some(root) = &env.root {
        inspection.detail(format!("store: {}", root.display()));
    }
    let store = observed_value(&inspection, env.store().map_err(anyhow::Error::from))?;
    let lock = observed_value(
        &inspection,
        super::super::install::InstallLock::acquire(&store),
    )?;
    inspection.detail("exclusive install lock acquired");
    inspection.finish();
    let _lock = lock;

    let selection = progress.task("Selecting garbage collection action");
    let action = if args.build {
        GcAction::Build
    } else if args.prune_others {
        GcAction::Prune
    } else {
        match ctx.suspend_progress(|| gc_menu(ctx)) {
            Ok(action) => action,
            Err(error) => {
                selection.fail(error.to_string());
                return Err(error);
            }
        }
    };
    selection.detail(match action {
        GcAction::Build => "selected Rust build cache cleanup",
        GcAction::Prune => "selected unused instance pruning",
        GcAction::Cancel => "selection cancelled",
    });
    selection.finish();

    match action {
        GcAction::Cancel => {
            ctx.summary("nothing to do.");
            Ok(GcOutcome::Skipped("cancelled"))
        }
        GcAction::Build => clean_build_cache(ctx, &store, progress),
        GcAction::Prune => prune_others(ctx, &store, args.yes, progress),
    }
}

fn clean_build_cache(
    ctx: &output::Context,
    store: &VersionStore,
    progress: &Progress,
) -> Result<GcOutcome> {
    let dir = store.build_dir();
    let preflight = progress.task("Preflighting Rust build cache");
    preflight.detail(format!("path: {}", dir.display()));
    if let Err(error) = store.guard_mutation_tree(&dir) {
        preflight.fail(error.to_string());
        return Err(error.into());
    }
    preflight.finish();

    let removal = progress.task("Removing Rust build cache");
    removal.detail(format!("path: {}", dir.display()));
    if !dir.exists() {
        removal.skip("already empty");
        ctx.summary("build cache already empty.");
        return Ok(GcOutcome::Skipped("build cache already empty"));
    }
    match fs::remove_dir_all(&dir).with_context(|| format!("removing `{}`", dir.display())) {
        Ok(()) => {
            removal.finish();
            ctx.summary("cleaned the Rust build cache.");
            Ok(GcOutcome::Finished)
        }
        Err(error) => {
            removal.fail(error.to_string());
            Err(error)
        }
    }
}

fn prune_others(
    ctx: &output::Context,
    store: &VersionStore,
    yes: bool,
    progress: &Progress,
) -> Result<GcOutcome> {
    let protections = progress.task("Selecting protected instances");
    let selected = (|| -> Result<_> {
        let active = store.active()?.ok_or(VvmError::NoActiveVersion)?;
        let running = provenance::running_record(store)?;
        let state = store.load_state()?;
        let saved_previous = store.previous()?;
        let rollback = saved_previous
            .as_ref()
            .filter(|record| {
                !same_record(record, &active)
                    && super::super::ensure_activatable(store, record).is_ok()
            })
            .cloned()
            .or_else(|| {
                state
                    .installs
                    .iter()
                    .filter(|record| !same_record(record, &active))
                    .filter(|record| super::super::ensure_activatable(store, record).is_ok())
                    .max_by_key(|record| record.instance)
                    .cloned()
            });
        let others = state
            .installs
            .iter()
            .filter(|record| !gc_protected(record, &active, rollback.as_ref(), running.as_ref()))
            .cloned()
            .collect::<Vec<_>>();
        Ok((active, running, state, rollback, others))
    })();
    let (active, running, mut state, rollback, others) = match selected {
        Ok(selected) => selected,
        Err(error) => {
            protections.fail(error.to_string());
            return Err(error);
        }
    };
    protections.detail(format!("active: {}", active.selector()));
    if let Some(record) = &rollback {
        protections.detail(format!("rollback: {}", record.selector()));
    } else {
        protections.detail("rollback: none");
    }
    if let Some(record) = &running {
        protections.detail(format!("running: {}", record.selector()));
    }
    protections.detail(format!("candidates: {}", others.len()));
    protections.finish();

    let active_path = store.instance_dir(&active.version_id(), active.instance);
    let rollback_path = rollback
        .as_ref()
        .map(|record| store.instance_dir(&record.version_id(), record.instance));
    if others.is_empty() {
        record_activation(store, &active_path, rollback_path.as_deref(), progress)?;
        ctx.summary("no instances outside the active + rollback pair to prune.");
        return Ok(GcOutcome::Skipped("nothing to prune"));
    }

    let remove_mirror = !state.installs.iter().any(|record| {
        record.origin == model::Origin::Managed
            && !others.iter().any(|removed| same_record(record, removed))
    });
    let build_dir = store.build_dir();
    preflight_prune(store, &others, remove_mirror, &build_dir, progress)?;

    let confirmation = progress.task("Confirming instance pruning");
    let confirmed = ctx.suspend_progress(|| {
        confirm(
            ctx,
            yes,
            &format!(
                "Remove {} instance(s) except active, immediate rollback, and running? This cannot be undone.",
                others.len()
            ),
        )
    });
    let confirmed = match confirmed {
        Ok(confirmed) => confirmed,
        Err(error) => {
            confirmation.fail(error.to_string());
            return Err(error.into());
        }
    };
    if !confirmed {
        confirmation.skip("declined");
        ctx.summary("aborted.");
        return Ok(GcOutcome::Skipped("declined"));
    }
    confirmation.finish();

    record_activation(store, &active_path, rollback_path.as_deref(), progress)?;

    let removals = progress.task("Removing unused instances");
    removals.set_progress(0, Some(others.len() as u64), "instances");
    let mut pruned = 0usize;
    let mut retained = 0usize;
    for (index, record) in others.iter().enumerate() {
        let component = removals
            .progress()
            .task(format!("Removing {}", record.selector()));
        let dir = store.instance_dir(&record.version_id(), record.instance);
        component.detail(format!("path: {}", dir.display()));
        if let Err(error) = store.guard_mutation_tree(&dir) {
            component.fail(error.to_string());
            removals.fail("candidate safety check failed");
            return Err(error.into());
        }
        let forget = if dir.exists() {
            match fs::remove_dir_all(&dir) {
                Ok(()) => {
                    component.finish();
                    true
                }
                Err(error) => {
                    component.fail(format!("retained for later retry: {error}"));
                    ctx.summary(&format!("skipped {} (in use?): {error}", dir.display()));
                    retained += 1;
                    false
                }
            }
        } else {
            component.skip("already absent; forgetting inventory record");
            true
        };
        if forget {
            state.installs.retain(|candidate| {
                !(candidate.version_id() == record.version_id()
                    && candidate.instance == record.instance)
            });
            pruned += 1;
        }
        removals.set_progress((index + 1) as u64, Some(others.len() as u64), "instances");
    }
    if retained == 0 {
        removals.finish();
    } else {
        removals.skip(format!("{retained} retained for later retry"));
    }

    save_inventory(store, &state, pruned, progress)?;
    remove_unused_mirror(store, &state, remove_mirror, progress)?;
    let build_cache_failed = clean_pruned_build_cache(store, &build_dir, progress)?;

    ctx.summary(&format!(
        "pruned {} instance(s); kept active, immediate rollback, and running.",
        pruned
    ));
    Ok(prune_outcome(retained, build_cache_failed))
}

fn preflight_prune(
    store: &VersionStore,
    others: &[InstallRecord],
    remove_mirror: bool,
    build_dir: &std::path::Path,
    progress: &Progress,
) -> Result<()> {
    let preflight = progress.task("Preflighting garbage collection targets");
    let result = (|| -> Result<()> {
        for record in others {
            let dir = store.instance_dir(&record.version_id(), record.instance);
            preflight.detail(format!("instance: {}", dir.display()));
            store.guard_mutation_tree(&dir)?;
        }
        if remove_mirror {
            let mirror = store.mirror_dir();
            preflight.detail(format!("managed mirror: {}", mirror.display()));
            store.guard_mutation_tree(&mirror)?;
        }
        preflight.detail(format!("build cache: {}", build_dir.display()));
        store.guard_mutation_tree(build_dir)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            preflight.finish();
            Ok(())
        }
        Err(error) => {
            preflight.fail(error.to_string());
            Err(error)
        }
    }
}

fn record_activation(
    store: &VersionStore,
    active: &std::path::Path,
    rollback: Option<&std::path::Path>,
    progress: &Progress,
) -> Result<()> {
    let recording = progress.task("Recording activation pointers");
    recording.detail(format!("active: {}", active.display()));
    if let Some(path) = rollback {
        recording.detail(format!("rollback: {}", path.display()));
    }
    match store.reset_activation(Some(active), rollback) {
        Ok(()) => {
            recording.finish();
            Ok(())
        }
        Err(error) => {
            recording.fail(error.to_string());
            Err(error.into())
        }
    }
}

fn save_inventory(
    store: &VersionStore,
    state: &model::State,
    pruned: usize,
    progress: &Progress,
) -> Result<()> {
    let saving = progress.task("Saving version inventory");
    saving.detail(format!("state: {}", store.state_path().display()));
    if pruned == 0 {
        saving.skip("inventory unchanged");
        return Ok(());
    }
    match store.save_state(state) {
        Ok(()) => {
            saving.finish();
            Ok(())
        }
        Err(error) => {
            saving.fail(error.to_string());
            Err(error.into())
        }
    }
}

fn remove_unused_mirror(
    store: &VersionStore,
    state: &model::State,
    remove_mirror: bool,
    progress: &Progress,
) -> Result<()> {
    if !remove_mirror
        || state
            .installs
            .iter()
            .any(|record| record.origin == model::Origin::Managed)
    {
        return Ok(());
    }
    let mirror = store.mirror_dir();
    let removal = progress.task("Removing unused managed source mirror");
    removal.detail(format!("path: {}", mirror.display()));
    if !mirror.exists() {
        removal.skip("already absent");
        return Ok(());
    }
    match fs::remove_dir_all(&mirror)
        .with_context(|| format!("removing shared managed mirror `{}`", mirror.display()))
    {
        Ok(()) => {
            removal.finish();
            Ok(())
        }
        Err(error) => {
            removal.fail(error.to_string());
            Err(error)
        }
    }
}

fn clean_pruned_build_cache(
    store: &VersionStore,
    build_dir: &std::path::Path,
    progress: &Progress,
) -> Result<bool> {
    let cleanup = progress.task("Removing Rust build cache");
    cleanup.detail(format!("path: {}", build_dir.display()));
    if let Err(error) = store.guard_mutation_tree(build_dir) {
        cleanup.fail(error.to_string());
        return Err(error.into());
    }
    if !build_dir.exists() {
        cleanup.skip("already empty");
        return Ok(false);
    }
    Ok(match fs::remove_dir_all(build_dir) {
        Ok(()) => {
            cleanup.finish();
            false
        }
        Err(error) => {
            cleanup.fail(format!("best-effort cleanup failed: {error}"));
            true
        }
    })
}

fn prune_outcome(retained: usize, build_cache_failed: bool) -> GcOutcome {
    if retained > 0 || build_cache_failed {
        GcOutcome::Skipped("completed with retained items")
    } else {
        GcOutcome::Finished
    }
}

fn observed_value<T>(task: &ProgressTask, result: Result<T>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            task.fail(error.to_string());
            Err(error)
        }
    }
}

fn gc_menu(ctx: &output::Context) -> Result<GcAction> {
    require_tty(
        ctx,
        "pass `--build` (clean the Rust build cache) or `--prune-others`",
    )?;
    let items = [
        "Clean the Rust build cache (the shared --target-dir)",
        "Prune all instances except active, immediate rollback, and running",
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
#[path = "gc_tests.rs"]
mod tests;
