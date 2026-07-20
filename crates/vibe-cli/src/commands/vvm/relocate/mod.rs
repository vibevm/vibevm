//! `vibe self relocate` — repoint source provenance to a moved checkout and
//! clear the instances built from the abandoned tree (PROP-019 §2.17).
//!
//! A committer's checkout is not pinned in place. When it moves, every
//! *external* instance's remembered `source_path` (§2.16) goes stale — a later
//! linked-source rebuild would miss — and the instances built from the old tree
//! clutter `self ls`. Relocate rewrites the remembered paths to the new tree and
//! removes the now-stale built instances, behind an interactive warning or
//! `--yes`. The active instance is kept (its source is repointed, never
//! deleted); removing a version is `self remove`'s job (§2.9).

specmark::scope!("spec://vibevm/common/PROP-019#relocate");

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use specmark::spec;
use thiserror::Error;

use super::model::{self, InstallRecord, Origin};
use super::selfloc::same_location;
use super::source::{external_path, find_source_root};
use super::store::VersionStore;
use super::{VvmEnv, confirm};
use crate::cli::VvmRelocateArgs;
use crate::output;

/// The relocate layer's decision failures (PROP-019 §2.17): a target that is
/// not a vibevm source tree, or nothing on the install side that records a
/// source tree to relocate from. Store / IO failures pass through transparently
/// (their own Class-F messages already cite the requirement).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#relocate")]
pub(crate) enum RelocateError {
    #[error(
        "`{path}` is not a vibevm source tree \
         (violates spec://vibevm/common/PROP-019#relocate; \
          fix: pass the path of a checkout carrying the workspace Cargo.toml and crates/vibe-cli)"
    )]
    NotASourceTree { path: PathBuf },

    #[error(
        "no installed version records a source tree to relocate from \
         (violates spec://vibevm/common/PROP-019#relocate; \
          fix: install from a checkout first (`vibe self install`), or pass `--from <old-path>`)"
    )]
    NoOldSource,
}

/// A version id + its instance number — the unit the plan addresses.
type InstanceRef = (model::VersionId, u64);

/// What `self relocate` will do (PROP-019 §2.17): a pure projection of the
/// inventory, the old/new paths, and the active instance. [`apply_relocate`]
/// turns it into filesystem + state changes. Building it separately lets
/// `--dry-run` print the plan and the oracle test assert it partitions the
/// inventory without writing anything.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/common/PROP-019#relocate")]
pub(crate) struct RelocatePlan {
    /// The old source path (as recorded), repointed away from.
    pub old: PathBuf,
    /// The new source path (canonical, `\\?\`-stripped — the form install
    /// records), repointed to.
    pub new: String,
    /// Active instances whose `source_path` is `old` — repointed, kept.
    pub repoint: Vec<InstanceRef>,
    /// Non-active instances built from `old` — dir removed, record forgotten.
    pub delete: Vec<InstanceRef>,
    /// Records that are neither external-from-`old` (informational).
    pub untouched: usize,
}

impl RelocatePlan {
    /// Whether the plan mutates anything.
    fn is_empty(&self) -> bool {
        self.repoint.is_empty() && self.delete.is_empty()
    }
}

/// The most common recorded `source_path` among external installs — the old
/// location a single moved checkout leaves behind (PROP-019 §2.17). `None` when
/// no external install records a source tree.
fn infer_old_source(state: &model::State) -> Option<PathBuf> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in &state.installs {
        if r.origin == Origin::External
            && let Some(p) = &r.source_path
        {
            *counts.entry(p.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(p, _)| PathBuf::from(p))
}

/// Partition the inventory into the relocate plan (PROP-019 §2.17). Pure: reads
/// `state`, `old`, `new`, and the active instance; writes nothing.
///
/// Every external record whose `source_path` is `old` is either **repointed**
/// (when it is the active instance — kept) or **deleted** (otherwise); every
/// other record is `untouched`. The active instance is never deleted.
fn plan_relocate(
    state: &model::State,
    old: &Path,
    new: &str,
    active: Option<&InstallRecord>,
) -> RelocatePlan {
    let mut repoint = Vec::new();
    let mut delete = Vec::new();
    let mut untouched = 0;
    for r in &state.installs {
        let matches_old = r.origin == Origin::External
            && r.source_path
                .as_deref()
                .is_some_and(|p| same_location(p, old));
        if !matches_old {
            untouched += 1;
        } else if active
            .is_some_and(|a| a.version_id() == r.version_id() && a.instance == r.instance)
        {
            repoint.push((r.version_id(), r.instance));
        } else {
            delete.push((r.version_id(), r.instance));
        }
    }
    RelocatePlan {
        old: old.to_path_buf(),
        new: new.to_string(),
        repoint,
        delete,
        untouched,
    }
}

/// How many instance dirs were actually removed vs. skipped (locked), from the
/// last [`apply_relocate`] — drives the honest end-of-run summary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ApplyCounts {
    removed: u32,
    skipped: u32,
}

/// Rewrite the inventory for a relocate plan (PROP-019 §2.17). Pure over
/// `state`: forgets a deleted instance's record **unless** it is in `kept`
/// (its dir was locked — the record stays so a later run, `self gc`, or
/// `self remove` can finish the job; §2.10), and repoints the active
/// instance's `source_path` to the new path. One read, one atomic write at the
/// call site (PROP-019 §2.4).
fn rewrite_state(
    state: &mut model::State,
    plan: &RelocatePlan,
    kept: &[InstanceRef],
) -> ApplyCounts {
    state.installs.retain(|r| {
        let deleted = plan
            .delete
            .iter()
            .any(|(id, n)| r.version_id() == *id && r.instance == *n);
        // A deleted instance is dropped UNLESS its dir was locked (kept) —
        // keeping the record avoids an orphaned dir nothing later collects.
        !deleted
            || kept
                .iter()
                .any(|(id, n)| r.version_id() == *id && r.instance == *n)
    });
    let new = plan.new.clone();
    for r in state.installs.iter_mut() {
        if plan
            .repoint
            .iter()
            .any(|(id, n)| r.version_id() == *id && r.instance == *n)
        {
            r.source_path = Some(new.clone());
        }
    }
    ApplyCounts {
        removed: (plan.delete.len() - kept.len()) as u32,
        skipped: kept.len() as u32,
    }
}

/// Apply a relocate plan (PROP-019 §2.17): remove the stale instance dirs
/// (best-effort — a dir locked by a running process is skipped and its record
/// kept for a later sweep, §2.10), then rewrite `state.toml` once via
/// [`rewrite_state`].
fn apply_relocate(
    ctx: &output::Context,
    store: &VersionStore,
    plan: &RelocatePlan,
) -> Result<ApplyCounts> {
    // 1. Remove the stale instance directories (the filesystem half). A locked
    //    dir is reported and KEPT (not forgotten) so it is not orphaned.
    let mut kept: Vec<InstanceRef> = Vec::new();
    for (id, instance) in &plan.delete {
        let dir = store.instance_dir(id, *instance);
        if !dir.exists() {
            continue; // already gone — nothing to remove, record will be dropped
        }
        match fs::remove_dir_all(&dir) {
            Ok(()) => ctx.removed(&dir.display().to_string()),
            Err(e) => {
                ctx.summary(&format!("skipped {} (in use?): {e}", dir.display()));
                kept.push((id.clone(), *instance));
            }
        }
    }
    // 2. Rewrite the inventory once via the pure core, then persist.
    let mut state = store.load_state()?;
    let counts = rewrite_state(&mut state, plan, &kept);
    store.save_state(&state)?;
    Ok(counts)
}

/// Print the plan for a human (heading + old→new + counts). No-op in JSON and
/// quiet modes — JSON callers get one envelope at the exit point instead.
fn report_plan_human(ctx: &output::Context, plan: &RelocatePlan) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    ctx.heading("vibe self relocate");
    ctx.step(&format!("source: {} → {}", plan.old.display(), plan.new));
    ctx.step(&format!(
        "repoint: {} active instance(s)",
        plan.repoint.len()
    ));
    ctx.step(&format!(
        "remove: {} stale instance(s) built from the old tree",
        plan.delete.len()
    ));
}

pub(super) fn run_relocate_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmRelocateArgs,
) -> Result<()> {
    let store = env.store()?;

    // Validate the new location is a real vibevm checkout (PROP-019 §2.17).
    let new_path = Path::new(&args.target);
    if find_source_root(new_path).is_none() {
        return Err(RelocateError::NotASourceTree {
            path: new_path.to_path_buf(),
        }
        .into());
    }
    let new = external_path(new_path);

    // Determine the old location: explicit `--from`, else inferred from records.
    let state = store.load_state()?;
    let old = match args.from.as_deref() {
        Some(p) => PathBuf::from(p),
        None => infer_old_source(&state).ok_or(RelocateError::NoOldSource)?,
    };

    let active = store.active()?;
    let plan = plan_relocate(&state, &old, &new, active.as_ref());

    // No-op: the recorded source already resolves to the target.
    if same_location(&old, &new) {
        if ctx.is_json() {
            return ctx.emit_json(&serde_json::json!({
                "ok": true,
                "command": "self:relocate",
                "noop": true,
                "old": plan.old.display().to_string(),
                "new": plan.new,
            }));
        }
        ctx.summary(&format!(
            "the recorded source already matches `{}` — nothing to relocate.",
            plan.new
        ));
        return Ok(());
    }

    // No record is sourced from the old location.
    if plan.is_empty() {
        if ctx.is_json() {
            return ctx.emit_json(&serde_json::json!({
                "ok": true,
                "command": "self:relocate",
                "old": plan.old.display().to_string(),
                "new": plan.new,
                "repointed": 0,
                "removed": 0,
                "note": "no instance is sourced from the old location",
            }));
        }
        ctx.summary(&format!(
            "no instance is sourced from `{}` — nothing to relocate.",
            plan.old.display()
        ));
        return Ok(());
    }

    report_plan_human(ctx, &plan);

    if args.dry_run {
        if ctx.is_json() {
            return ctx.emit_json(&serde_json::json!({
                "ok": true,
                "command": "self:relocate",
                "old": plan.old.display().to_string(),
                "new": plan.new,
                "repoint": plan.repoint.len(),
                "remove": plan.delete.len(),
                "untouched": plan.untouched,
                "dry_run": true,
            }));
        }
        ctx.summary("dry-run: no changes made.");
        return Ok(());
    }

    // The removal is irreversible — confirm unless `--yes`/unattended. A
    // repoint-only plan (no deletions) needs no confirm: it is a reversible
    // state edit.
    if !plan.delete.is_empty()
        && !confirm(
            ctx,
            args.yes,
            &format!(
                "Relocate source provenance and remove {} stale instance(s)? This cannot be undone.",
                plan.delete.len()
            ),
        )?
    {
        ctx.summary("aborted.");
        return Ok(());
    }

    let counts = apply_relocate(ctx, &store, &plan)?;

    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:relocate",
            "old": plan.old.display().to_string(),
            "new": plan.new,
            "repointed": plan.repoint.len(),
            "removed": counts.removed,
            "skipped": counts.skipped,
            "dry_run": false,
        }));
    }
    if counts.removed > 0 {
        ctx.summary(&format!("removed {} stale instance(s).", counts.removed));
    }
    if counts.skipped > 0 {
        ctx.summary(&format!(
            "skipped {} locked instance(s) — kept; re-run `vibe self relocate` once they close.",
            counts.skipped
        ));
    }
    if !plan.repoint.is_empty() {
        ctx.summary(&format!(
            "repointed {} active instance's source → {}",
            plan.repoint.len(),
            plan.new
        ));
    }
    ctx.summary("done.");
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
