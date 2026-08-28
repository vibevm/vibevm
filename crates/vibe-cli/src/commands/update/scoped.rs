//! The SCOPED `vibe update <pkgref>...`: re-resolve exactly the named
//! packages and their subtrees, and leave every other slot alone.
//!
//! ## Three regions, and what a failure owes in each
//!
//! ```text
//! 1. read / resolve / fetch / confirm   nothing durable yet
//! 2. stage: in-place fetch, lifecycle   the accumulator alone
//! 3. apply: prune, materialise, boot    the accumulator JOINED with the run
//! ```
//!
//! Region 1 touches nothing the operator can observe: the solve is arithmetic
//! and the fetch lands in the machine-global store. A failure there really did
//! move nothing, so the command's empty fallback draft is the truth.
//!
//! Region 2 is where the project starts changing — `materialise_in_place`
//! advances a slot's own working tree — and it happens BEFORE an
//! [`vibe_install::InstallSlotLifecycle`] exists to record it. That is exactly
//! why [`measured::Measured`] exists: from the first mutation, every failure
//! draft reads it instead of inventing an empty run.
//!
//! Region 3 runs with a live lifecycle, so its failures join the two halves —
//! see [`measured::Measured::joined`]. The prune lives here, after the
//! lifecycle exists, precisely so the run can own its own removals rather than
//! being handed them; the accumulator still records each one, so a removal that
//! fails half-way leaves the earlier ones reported rather than discarded.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

mod apply;
mod measured;
mod stage;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_install::{InstallProgress, InstallSource};
use vibe_registry::CachedPackage;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};
use vibe_workspace::vibedeps;

use crate::commands::compile_trace::{self, CommandExit, RegisteredReportDraft, carry_measured};
use crate::commands::install::{
    build_install_resolver, exact_pinned_pkgref, lane_sizes, resolve_spec_format,
};
use crate::commands::short_name;
use crate::exit_code::InstallError;
use crate::output;

use super::Execution;
use super::draft::{UpdateDraft, UpdateIdentity};
use super::inputs::{install_args_from, load_lockfile};

use measured::Measured;
use stage::{PendingInPlace, Stage, stage};

/// One re-resolved package: what was fetched, what it requires, and — for an
/// in-place node — whether the fetch really advanced the working tree.
type Resolved = (CachedPackage, Vec<PackageRef>, Option<bool>);

pub(super) struct SourceHashes(HashMap<(Group, String), String>);

impl SlotVerifier for SourceHashes {
    fn source_hash<'a>(&'a self, dep: &ResolvedDep) -> Option<&'a str> {
        self.0
            .get(&(dep.group.clone(), dep.name.clone()))
            .map(String::as_str)
    }

    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Unverifiable
    }
}

/// The one boundary: everything after `prepare` and before `finalize`.
pub(super) fn execute_after_open(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    let identity = UpdateIdentity::from_args(execution.selection.root(), &execution.args);
    match run(ctx, execution, trace) {
        Ok(draft) => {
            let parked = draft.delegation.is_some();
            let draft = RegisteredReportDraft::Update(Box::new(draft));
            if parked {
                CommandExit::Parked(draft)
            } else {
                CommandExit::Success(draft)
            }
        }
        // An EMPTY fallback is legal only for region 1, and only because it is
        // also TRUE there: every site past the first mutation carries a draft
        // built from the accumulator.
        Err(error) => compile_trace::classify(error, || {
            RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                &identity,
                0,
                Vec::new(),
                InstallProgress::default(),
                Vec::new(),
            )))
        }),
    }
}

fn run(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> Result<UpdateDraft> {
    let Execution {
        args,
        embedded_root,
        offline,
        lease,
        user_config,
        selection,
        metadata,
    } = execution;
    let identity = UpdateIdentity::from_args(selection.root(), &args);
    // The bundle PROVEN, in the historical order: the stored manifest result
    // first — a malformed selected manifest is this command's error, in its own
    // words, and no workspace was ever built from it — then the ONE workspace
    // answer, returned as it was. Retrying could succeed against a tree the
    // identity and the trace were never prepared for.
    let proven = selection.prove()?;
    let (project_root, manifest, workspace) = (
        proven.root().to_path_buf(),
        proven.manifest().clone(),
        proven.workspace(),
    );

    let mut lockfile = load_lockfile(&workspace.root)?;
    // PROP-050 ##VERIFY-LOCK-DIFF — the scoped update's own pre-apply
    // snapshot: the lock as loaded (before the entry replacements below
    // mutate it in place) and the boot lanes' byte sizes. Diffed against
    // the post-write state once the apply is durable.
    let old_lock = lockfile.clone();
    let lanes_before = lane_sizes(&workspace.root);
    let spec_format = resolve_spec_format(&manifest, user_config.install.spec_format);

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` configured in `{}/vibe.toml` — `vibe update` re-fetches \
             from the registry.",
            project_root.display()
        );
    }

    let roots = qualify_roots(&args.packages, &manifest, &lockfile)?;

    // PROP-010 §2.5 — the offline posture reaches `vibe update` through the
    // same ladder as install, resolved ONCE by the command owner: root
    // `--offline` / `VIBE_OFFLINE` / user-config `[net].offline`. A scoped
    // offline update with no local source fails in `build_install_resolver`
    // with the same actionable bail `vibe install --offline` gives.
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let resolver = build_install_resolver(
        &install_args_from(&args),
        &manifest,
        embedded_root.as_deref(),
        &project_root,
        &global,
        offline,
        &lockfile.packages,
    )?;

    ctx.heading(&format!(
        "Re-resolving {} package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" },
    ));
    let graph = resolver
        .solve(&roots)
        .context("dependency resolution failed")?;

    // Fetched payload lands in the machine-global store (PROP-010
    // §2.7) — no project cache directory exists to create.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;

    // Fetch every node of the named subtree. A package the lockfile already
    // records as in-place with a present slot is NOT re-fetched: it is updated
    // incrementally on its own `.git` (PROP-022 §2.4) — a version bump on a
    // giant transfers only changed objects rather than re-cloning the tree. We
    // resolve those nodes here but defer the slot mutation past the confirm.
    let mut updated: Vec<Resolved> = Vec::new();
    let mut pending_in_place: Vec<PendingInPlace> = Vec::new();
    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        if let Some(old) = lockfile.find(&node.group, &node.name)
            && old.materialization.is_in_place()
            && vibedeps::is_in_place_slot(&workspace.root, &old.group, &node.name)
        {
            pending_in_place.push(PendingInPlace {
                pkgref,
                group: node.group.clone(),
                name: node.name.clone(),
                version: node.version.clone(),
                registry: old.registry.clone(),
                dependencies: node.dependencies.clone(),
            });
            continue;
        }
        let cached = resolver.resolve_and_fetch(&pkgref, &store_root, None)?;
        updated.push((cached, node.dependencies.clone(), None));
    }
    // Every package this run re-resolved, counted before anything is consumed:
    // a failure draft must be able to say how big the run was even when the
    // set itself was moved into the staging below.
    let resolved = updated.len() + pending_in_place.len();
    if !confirm(ctx, &args, resolved)? {
        return Err(InstallError::UserDeclined.into());
    }

    // ---- region 2: the first durable mutation ----------------------------
    let mut measured = Measured::default();
    // The command's ONE agent backend, shared by the staged apply and any
    // continuation it services.
    let agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend> = std::sync::Arc::new(
        crate::commands::lifecycle::install_agent_backend(&workspace.root, &manifest),
    );
    let staged = stage(
        ctx,
        &mut measured,
        Stage {
            resolver: &resolver,
            manifest: &manifest,
            workspace,
            project_root: &project_root,
            lockfile: &lockfile,
            metadata: &metadata,
            lease: &lease,
            updated,
            pending_in_place,
        },
    );
    let staged = match staged {
        Ok(staged) => staged,
        Err(error) => {
            return Err(carry_measured(error, || {
                RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                    &identity,
                    resolved,
                    measured.bumps().to_vec(),
                    measured.progress(),
                    Vec::new(),
                )))
            }));
        }
    };

    // ---- region 3: the run owns what it does ------------------------------
    let outcome = apply::apply(
        ctx,
        apply::ScopedApply {
            lifecycle: &staged.lifecycle,
            measured: &mut measured,
            identity: &identity,
            project_root: &project_root,
            workspace,
            metadata: &metadata,
            spec_format,
            trace,
            lease: &lease,
            resolved,
            resolution: &staged.resolution,
            source_hashes: &staged.source_hashes,
            updated: &staged.updated,
            lockfile: &mut lockfile,
            old_lock: &old_lock,
            lanes_before: &lanes_before,
            agent: &agent,
        },
    );
    outcome.map_err(|error| {
        carry_measured(error, || {
            RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                &identity,
                resolved,
                measured.bumps().to_vec(),
                // JOINED: the lifecycle owns the prune prefix and whatever the
                // materialise pass reached; the in-place fetches are only known
                // here. One `take_reports`, because a second returns nothing.
                measured.joined(staged.lifecycle.progress()),
                staged.lifecycle.take_reports().unwrap_or_default(),
            )))
        })
    })
}

/// Each named package must already be installed; re-resolve it against its
/// original root constraint so a caret bumps within range.
///
/// `vibe update` refreshes an installed package, so a bare short name resolves
/// against `vibe.lock` alone — no index, no network — and a name not locked
/// fails here with a clear "not installed".
fn qualify_roots(
    packages: &[String],
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> Result<Vec<PackageRef>> {
    let mut roots: Vec<PackageRef> = Vec::with_capacity(packages.len());
    for raw in packages {
        let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
        let pkgref = short_name::qualify_locked(&pkgref, lockfile)?;
        let group = match pkgref.group.as_ref() {
            Some(g) => g.clone(),
            None => bail!(
                "`{pkgref}` resolved without a group — internal: `qualify_locked` should qualify"
            ),
        };
        if lockfile.find(&group, &pkgref.name).is_none() {
            bail!(
                "package `{group}/{}` is not installed — `vibe update` only refreshes installed \
                 packages; use `vibe install {group}/{}` to add it.",
                pkgref.name,
                pkgref.name,
            );
        }
        // The constraint to re-resolve against: the manifest `[requires]`
        // declaration is authoritative — the operator edits it to widen a
        // pin before updating — and the lockfile's `root_dependencies`
        // mirror is only the fallback.
        let constraint = manifest
            .requires
            .packages
            .iter()
            .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            .or_else(|| {
                lockfile
                    .meta
                    .root_dependencies
                    .iter()
                    .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            })
            .map(|r| r.version.clone())
            .unwrap_or(VersionSpec::Latest);
        roots.push(PackageRef::new(
            pkgref.kind,
            Some(group),
            pkgref.name,
            constraint,
        )?);
    }
    Ok(roots)
}

fn confirm(ctx: &output::Context, args: &crate::cli::UpdateArgs, total: usize) -> Result<bool> {
    if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        return Ok(true);
    }
    if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to update \
             non-interactively"
        );
    }
    Confirm::new()
        .with_prompt(format!(
            "Re-materialise {total} package{} into vibedeps/ and regenerate boot?",
            if total == 1 { "" } else { "s" },
        ))
        .default(false)
        .interact()
        .context("reading user confirmation")
}

/// Remove any superseded *versioned* slot so a bump leaves no stale slot (an
/// in-place slot is unversioned — nothing to prune), and record the bumps.
///
/// Both facts are written into `measured` AS THEY HAPPEN, and that is the whole
/// contract: a removal that fails at package N must leave the N−1 slots this
/// run really deleted in the accumulator, and therefore in the failed report.
/// An all-or-nothing helper returning `Result<Vec<String>>` would drop its
/// partial vector on the way out and report a tree it had already changed as
/// untouched.
///
/// A bump is recorded before its removal is attempted: it is a fact about the
/// resolution, and stays true whether or not the stale slot could be deleted.
/// `pruned` conversely names only removals that REALLY removed something — a
/// slot already absent, or an unversioned in-place one, deleted nothing.
fn prune_superseded(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[Resolved],
    measured: &mut Measured,
) -> Result<()> {
    for (cached, _, _) in updated {
        let name = &cached.resolved.name;
        let Some(old_v) = lockfile
            .find(&cached.resolved.group, name)
            .map(|o| o.version.clone())
            .filter(|v| *v != cached.resolved.version)
        else {
            continue;
        };
        measured.record_bump(format!(
            "{}/{} {} -> {}",
            cached.resolved.group, name, old_v, cached.resolved.version
        ));
        if !cached.package_meta().materialization.is_in_place() {
            let group = &cached.package_meta().group;
            let removed = vibedeps::remove_slot(&workspace.root, group, name, &old_v)
                .context("removing the superseded vibedeps/ slot")?;
            if removed {
                measured.record_pruned(vibedeps::slot_rel_path(group, name, &old_v));
            }
        }
    }
    measured.sort_pruned();
    Ok(())
}

/// `Verify` re-materialises every named slot from the fresh fetch — the scoped
/// update wants the fetch reconciled into the slot, not a hash-checked skip.
pub(super) const SCOPED_INTEGRITY: SlotIntegrity = SlotIntegrity::Verify;
