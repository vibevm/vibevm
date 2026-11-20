//! The SCOPED `vibe update <pkgref>...`: re-resolve exactly the named
//! packages and their subtrees, and leave every other slot alone.
//!
//! ## Three regions, and what a failure owes in each
//!
//! ```text
//! 1. read / resolve / prepare / confirm   no project semantic effects yet
//! 2. stage: bound checkout, lifecycle   the accumulator alone
//! 3. apply: prune, materialise, boot    the accumulator JOINED with the run
//! ```
//!
//! Region 1 may acquire cache/Git objects and FETCH_HEAD in the original slot
//! context. It does not advance HEAD/index/refs/config or the worktree, publish
//! authored inputs, or run hooks/boot. Its empty semantic progress is truthful;
//! it is not a claim that acquisition performed no I/O.
//!
//! Region 2 is where project semantic state starts changing — `materialise_in_place`
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
mod guarded_leaf;
mod measured;
mod noop;
mod roots;
mod stage;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallProgress, InstallSource};
use vibe_registry::CachedPackage;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::{InstallRollback, ResolvedDep, SlotCheck, SlotVerifier};
use vibe_workspace::vibedeps;

use crate::commands::compile_trace::{self, CommandExit, RegisteredReportDraft, carry_measured};
use crate::commands::install::{exact_pinned_pkgref, lane_sizes, resolve_spec_format};
use crate::exit_code::InstallError;
use crate::output;

use super::Execution;
use super::all::Roots;
use super::draft::{UpdateDraft, UpdateIdentity};
use super::inputs::install_args_from;

use measured::Measured;
use roots::{qualify_roots, source_only_roots};
use stage::{PendingInPlace, Stage, stage};

/// One re-resolved package: what was fetched, what it requires, and — for an
/// in-place node — whether the fetch really advanced the working tree.
pub(super) type Resolved = (
    CachedPackage,
    Vec<PackageRef>,
    Option<bool>,
    Vec<vibe_core::manifest::LockedEmbeddedSource>,
    Option<vibe_core::manifest::PackageConfiguration>,
);

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
    roots: Roots,
    trace: Option<&TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    let identity = UpdateIdentity::from_args(execution.selection.root(), &execution.args);
    let result = run(ctx, execution, roots, trace).and_then(|mut draft| {
        if draft.delegation.is_none()
            && let Err(error) =
                super::super::authority_refresh::refresh(ctx, &identity.project_root)
        {
            draft.ok = false;
            return Err(compile_trace::carry_measured(error, || {
                RegisteredReportDraft::Update(Box::new(draft))
            }));
        }
        Ok(draft)
    });
    match result {
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

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn run(
    ctx: &output::Context,
    execution: Execution,
    roots: Roots,
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
        adopted: _,
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

    let configured_all = matches!(&roots, Roots::All(_))
        && workspace
            .iter_nodes()
            .any(|(_, node)| !node.requires.configurations.is_empty());
    if configured_all {
        vibe_install::configuration_admission::workspace_inputs_for_install(
            workspace,
            &project_root,
            &manifest,
        )?;
        if workspace.iter_nodes().any(|(_, node)| {
            !node.requires.git_packages.is_empty()
                || !node.requires.path_packages.is_empty()
                || !node.requires.var_packages.is_empty()
        }) {
            bail!(
                "configured whole-workspace update requires ordinary local-registry declarations"
            );
        }
    } else {
        vibe_install::configuration_admission::workspace_inputs(
            workspace,
            &project_root,
            &manifest,
        )?;
    }
    if args.package_configuration_review.is_some() && !configured_all {
        bail!("package configuration review response is inapplicable to this update route");
    }
    let prior_lock_bytes = std::fs::read(workspace.root.join(Lockfile::FILENAME))?;
    let mut lockfile = super::inputs::captured_lockfile(&prior_lock_bytes)?;
    if lockfile
        .configurations
        .values()
        .any(|record| !record.forwarded_inputs.is_empty())
    {
        bail!(
            "forwarded package update requires separately admitted source/target renewal before effects"
        );
    }
    // PROP-050 ##VERIFY-LOCK-DIFF — the scoped update's own pre-apply
    // snapshot: the lock as loaded (before the entry replacements below
    // mutate it in place) and the boot lanes' byte sizes. Diffed against
    // the post-write state once the apply is durable.
    let old_lock = lockfile.clone();
    let lanes_before = lane_sizes(&workspace.root);
    let spec_format = resolve_spec_format(&manifest, user_config.install.spec_format);

    // PROP-010 §2.5 — the offline posture reaches `vibe update` through the
    // same ladder as install, resolved ONCE by the command owner: root
    // `--offline` / `VIBE_OFFLINE` / user-config `[net].offline`. A scoped
    // offline update with no local source fails in `build_install_resolver`
    // with the same actionable bail `vibe install --offline` gives.
    let install_args = install_args_from(&args);
    // Availability follows retained source selection or demanded catalog, not an
    // eager snapshot of unrelated current registry entries. A plain update
    // replays held sources and needs no configured registry; an explicit source
    // reselection does, and refuses with the places an operator can fix it.
    // The offline posture keeps the resolver's own, more specific bail.
    if (args.reselect_source || args.sources_only) && !offline {
        let global = vibe_core::GlobalRegistryConfig::load()?;
        if !crate::commands::install::any_package_source(
            &install_args,
            &manifest,
            embedded_root.as_deref(),
            &project_root,
            &global,
            offline,
        ) {
            bail!(
                "no `[[registry]]` configured in `{}` or in `~/.vibe/registry.toml` — \
                 `vibe update` cannot choose a source again without one.",
                project_root.join(Manifest::FILENAME).display(),
            );
        }
    }

    let every = matches!(roots, Roots::All(_));
    let roots = match roots {
        Roots::All(roots) => roots,
        Roots::Named if args.sources_only => source_only_roots(&args.packages, &lockfile)?,
        Roots::Named => qualify_roots(&args.packages, &manifest, &lockfile)?,
    };
    if configured_all
        && lockfile.packages.iter().any(|row| {
            row.materialization.is_in_place() || !row.source_url.as_str().starts_with("file://")
        })
    {
        bail!("configured whole-workspace update refuses mixed or in-place sources");
    }
    let original_inputs = if configured_all {
        Some(vibe_install::configured_workspace_inputs(
            workspace,
            &project_root,
        )?)
    } else {
        None
    };
    let record_capture =
        vibe_install::ProjectRecordPublication::capture_before(&workspace.root, &old_lock)?;

    let resolver = crate::commands::install::build_install_resolver_deferred(
        &install_args,
        &manifest,
        embedded_root.as_deref(),
        &project_root,
        offline,
        &lockfile.packages,
    )?
    .with_locked_checkouts(vibe_install::locked_checkout_paths(
        &workspace.root,
        &lockfile.packages,
    ))
    .with_locked_updates();
    let resolver = if args.reselect_source || args.sources_only {
        // `--all` chooses again for every locked coordinate, the closure
        // included — "every package in the lockfile".
        let keys: Vec<String> = if every {
            lockfile
                .packages
                .iter()
                .map(|row| format!("{}/{}", row.group, row.name))
                .collect()
        } else {
            roots.iter().map(PackageRef::qualified_name).collect()
        };
        resolver.with_reselected_sources(&keys)
    } else {
        resolver
    };

    let resolver = vibe_install::PlanningSource {
        source: &resolver,
        configuration: original_inputs.clone().map(|inputs| {
            vibe_resolver::configuration::ConfigurationSnapshot::with_history(
                inputs,
                lockfile.configurations.clone(),
            )
            .for_review_preview()
        }),
    };

    ctx.heading(&format!(
        "Re-resolving {} package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" },
    ));
    let solving = ctx.progress().task("Resolving selected package closure");
    solving.set_progress(0, Some(roots.len() as u64), "roots");
    // PROP-050 `#EFFECTIVE-SET`: the same effective set install computes —
    // a non-root package's private requirement is not resolved or fetched.
    let graph =
        match vibe_install::solve_visible_graph(&resolver, &manifest, &roots, !ctx.no_recommends())
            .context("dependency resolution failed")
        {
            Ok(graph) => {
                solving.set_progress(roots.len() as u64, Some(roots.len() as u64), "roots");
                solving.finish();
                graph
            }
            Err(error) => {
                solving.fail("dependency resolution failed");
                return Err(error);
            }
        };

    // Fetched payload lands in the machine-global store (PROP-010
    // §2.7) — no project cache directory exists to create.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;

    // Fetch every node of the named subtree. A package the lockfile already
    // records as in-place with a present slot is NOT re-fetched: it is updated
    // incrementally on its own `.git` (PROP-022 §2.4) — a version bump on a
    // giant transfers only changed objects rather than re-cloning the tree. We
    // resolve those nodes here but defer the slot mutation past the confirm.
    let graph_len = graph.iter().count();
    let fetching = ctx.progress().task("Fetching selected package closure");
    fetching.set_progress(0, Some(graph_len as u64), "packages");
    let mut updated: Vec<Resolved> = Vec::new();
    let mut pending_in_place: Vec<PendingInPlace> = Vec::new();
    for (index, node) in graph.iter().enumerate() {
        let pkgref = exact_pinned_pkgref(node);
        let package = fetching
            .progress()
            .task(format!("Selecting {}/{}", node.group, node.name));
        if let Some(old) = lockfile.find(&node.group, &node.name)
            && old.materialization.is_in_place()
            && vibedeps::is_in_place_slot(&workspace.root, &old.group, &node.name)
        {
            let slot = vibedeps::in_place_slot_abs_path(&workspace.root, &old.group, &node.name);
            let prepared = resolver.prepare_in_place(&pkgref, &slot)?;
            if prepared.slot() != slot {
                bail!("prepared in-place source belongs to another destination");
            }

            vibe_install::configuration_admission::package_manifest(&pkgref, prepared.manifest())?;
            pending_in_place.push(PendingInPlace {
                embedded_sources: vibe_registry::lock_embedded_sources_with(
                    &prepared.manifest().embedded_sources,
                    offline,
                )?,
                prepared,
                pkgref,
                group: node.group.clone(),
                name: node.name.clone(),
                version: node.version.clone(),
                dependencies: node.dependencies.clone(),
            });
            package.skip("deferred to in-place refresh");
            fetching.set_progress((index + 1) as u64, Some(graph_len as u64), "packages");
            continue;
        }
        let cached = match resolver.resolve_and_fetch(&pkgref, &store_root, None) {
            Ok(cached) => cached,
            Err(error) => {
                package.fail("package fetch failed");
                fetching.fail("package closure fetch failed");
                return Err(error.into());
            }
        };
        let configuration = resolver.configuration_for_node(node)?;
        if let Some(record) = &configuration {
            if !record.selected_subskill_paths().is_empty() {
                bail!(
                    "configured subskill update requires fresh source-admission evidence before effects; automatic witness renewal is not connected"
                );
            }
            record.validate(&cached.manifest)?;
        } else {
            vibe_install::configuration_admission::package_manifest(&pkgref, &cached.manifest)?;
        }
        let embedded_sources =
            vibe_registry::lock_embedded_sources_with(&cached.manifest.embedded_sources, offline)?;
        updated.push((
            cached,
            node.dependencies.clone(),
            None,
            embedded_sources,
            configuration,
        ));
        package.finish();
        fetching.set_progress((index + 1) as u64, Some(graph_len as u64), "packages");
    }
    fetching.finish();
    if configured_all {
        super::review::gate(
            original_inputs
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing governed input snapshot"))?,
            super::review::Subject {
                workspace: &workspace.root,
                selected: &project_root,
                manifest: &manifest,
            },
            &prior_lock_bytes,
            &old_lock,
            &updated,
            args.package_configuration_review.as_deref(),
        )?;
    }
    let mut reconciliation = if configured_all {
        Some(guarded_leaf::prepare(
            workspace,
            &old_lock,
            &updated,
            resolver.source,
            configured_all,
            spec_format,
        )?)
    } else {
        None
    };
    let planning_base = reconciliation.as_ref().map_or(&lockfile, |plan| &plan.base);

    // Every package this run re-resolved, counted before anything is consumed:
    // a failure draft must be able to say how big the run was even when the
    // set itself was moved into the staging below.
    let resolved = updated.len() + pending_in_place.len();
    let full_world = stage::preflight_world(workspace, planning_base, &updated, &pending_in_place)?;
    let (planned_lock, records) =
        stage::prepare_records(record_capture, planning_base, &updated, &pending_in_place)?;
    // A no-op `--all` stops here, before the confirmation and before region 2:
    // no slot or lock byte would move, so only boot is regenerated (change-only)
    // and the report is `unchanged`.
    if every
        && let Some(draft) = noop::unchanged_draft(
            ctx,
            &noop::NoopProbe {
                workspace,
                lease: &lease,
                old_lock: &old_lock,
                planned_lock: &planned_lock,
                updated: &updated,
                pending_in_place: &pending_in_place,
                spec_format,
            },
            noop::NoopBoot {
                full_world: &full_world,
                trace,
            },
            &identity,
            resolved,
        )?
    {
        return Ok(draft);
    }
    if !confirm(ctx, &args, resolved)? {
        return Err(InstallError::UserDeclined.into());
    }

    records.validate_before_publication()?;
    // The set response is not a retained source capability. Re-observe the exact
    // live and held payloads at the last pre-effect boundary after confirmation.
    if configured_all && super::review::set::applies(&old_lock, &updated) {
        super::review::gate(
            original_inputs
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing governed input snapshot"))?,
            super::review::Subject {
                workspace: &workspace.root,
                selected: &project_root,
                manifest: &manifest,
            },
            &prior_lock_bytes,
            &old_lock,
            &updated,
            args.package_configuration_review.as_deref(),
        )?;
    }

    // ---- region 2: the first project semantic mutation ----------------------------
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
            manifest: &manifest,
            workspace,
            project_root: &project_root,
            metadata: &metadata,
            lease: &lease,
            updated,
            pending_in_place,
            full_world,
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
            checked_removal: reconciliation.as_mut().and_then(|plan| plan.removal.take()),
            measured: &mut measured,
            identity: &identity,
            project_root: &project_root,
            manifest: &manifest,
            workspace,
            metadata: &metadata,
            spec_format,
            trace,
            lease: &lease,
            resolved,
            full_world: &staged.full_world,
            resolution: &staged.resolution,
            source_hashes: &staged.source_hashes,
            updated: &staged.updated,
            lockfile: &mut lockfile,
            old_lock: &old_lock,
            planned_lock,
            records,
            lanes_before: &lanes_before,
            agent: &agent,
            original_inputs: original_inputs.as_ref(),
        },
    );
    outcome.map_err(|error| {
        let build = || {
            let mut draft = UpdateDraft::failed(
                &identity,
                resolved,
                measured.bumps().to_vec(),
                measured.joined(staged.lifecycle.progress()),
                staged.lifecycle.take_reports().unwrap_or_default(),
            );
            draft.retained_backups = measured.retained_backups.clone();
            RegisteredReportDraft::Update(Box::new(draft))
        };
        if measured.retained_backups.is_empty() {
            carry_measured(error, build)
        } else {
            compile_trace::carry(build(), error, true)
        }
    })
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
    ctx.suspend_progress(|| {
        crate::output::prompts::confirm(
            &format!(
                "Re-materialise {total} package{} into vibedeps/ and regenerate boot?",
                if total == 1 { "" } else { "s" },
            ),
            false,
        )
    })
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
///
/// Each removal moves the slot aside into `rollback` (same volume, a rename)
/// rather than deleting it, so a later failure or acknowledged stop before the
/// lock is published can put the superseded slot back.
fn prune_superseded(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[Resolved],
    measured: &mut Measured,
    rollback: &mut InstallRollback,
) -> Result<()> {
    for (cached, _, _, _, _) in updated {
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
            let slot = vibedeps::slot_abs_path(&workspace.root, group, name, &old_v);
            if slot.exists() {
                // A slot is a directory. Moving aside whatever else sits at
                // its path would succeed where the delete it replaces refused.
                if !slot.is_dir() {
                    return Err(vibe_workspace::WorkspaceError::Io {
                        path: slot,
                        reason: "the superseded slot path is not a directory".into(),
                    })
                    .context("removing the superseded vibedeps/ slot");
                }
                rollback
                    .remove(&slot)
                    .context("removing the superseded vibedeps/ slot")?;
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
