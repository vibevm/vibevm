//! `vibe reinstall --force`: re-fetch every locked package from source at the
//! lockfile-pinned version, re-materialise `vibedeps/`, regenerate boot.
//!
//! Two branches, one owner. The empty branch has nothing to fetch and only
//! regenerates; the ordinary branch fetches, builds the run, and applies. Both
//! compile under the command's ONE borrowed recorder.
//!
//! ## Where the measured region starts
//!
//! Everything up to the lifecycle construction is arithmetic and machine-store
//! I/O: the resolver is built, and each fetch lands in `~/.vibe/cache/`, not in
//! the operator's project. A failure there really did move nothing. From the
//! lifecycle onward the apply is rewriting `vibedeps/`, so every failure must
//! freeze the run's real `progress()` and one `take_reports()` — an empty
//! record over a half-re-materialised tree is the lie the honest progress model
//! exists to prevent.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{ContentHash, Group};
use vibe_install::{InstallProgress, InstallSlotLifecycle, InstallSource};
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::{
    ResolvedDep, SlotCheck, SlotLifecycleMode, SlotVerifier,
    apply_resolution_with_spec_format_and_slot_lifecycle_traced, run_post_install_slot_lifecycle,
};

use crate::cli::ReinstallArgs;
use crate::commands::compile_trace::{RegisteredReportDraft, carry_measured};
use crate::commands::install::{LifecycleSlotObserver, build_install_resolver};
use crate::exit_code::InstallError;
use crate::output;

use super::continuation;
use super::draft::{ReinstallDraft, ReinstallIdentity, regenerated};
use super::inputs::{confirm, exact_pkgref, reinstall_stream_mode, resolver_args};

struct SourceHashes(HashMap<(Group, String), String>);

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

pub(super) struct Forced<'a> {
    pub(super) args: &'a ReinstallArgs,
    pub(super) identity: &'a ReinstallIdentity,
    pub(super) workspace: &'a Workspace,
    pub(super) lockfile: &'a Lockfile,
    pub(super) metadata: &'a RunMetadata,
    pub(super) spec_format: SpecFormat,
    pub(super) trace: Option<&'a TraceRun>,
    pub(super) embedded_root: Option<&'a Path>,
    /// The owner's ONE resolved offline posture.
    pub(super) offline: bool,
    /// The command's mutation lease: the forced apply's slot run is built on
    /// the ONE acquisition the reinstall boundary made, and the continuation
    /// it may service reuses the same proof.
    pub(super) lease: &'a std::sync::Arc<LifecycleLease>,
}

pub(super) fn run(ctx: &output::Context, inputs: Forced<'_>) -> Result<ReinstallDraft> {
    if inputs.lockfile.packages.is_empty() {
        return regenerate_only(ctx, &inputs);
    }
    let Forced {
        args,
        identity,
        workspace,
        lockfile,
        metadata,
        spec_format,
        trace,
        embedded_root,
        offline,
        lease,
    } = inputs;

    ctx.heading(&format!(
        "\nReinstall --force — re-fetch {} package{} from source:",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    for p in &lockfile.packages {
        ctx.step(&format!("{}:{}@{}", p.kind, p.name, p.version));
    }

    if !confirm(
        ctx,
        args,
        &format!(
            "Re-fetch {} package{} from source and re-materialise vibedeps/?",
            lockfile.packages.len(),
            if lockfile.packages.len() == 1 {
                ""
            } else {
                "s"
            },
        ),
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    // The resolver is built from the workspace root manifest — registries,
    // mirrors, overrides and git-source declarations are root-level — taken
    // from the tree this command already loaded rather than re-read. The
    // offline posture arrives already resolved by the owner.
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let resolver = build_install_resolver(
        &resolver_args(),
        &workspace.root_manifest,
        embedded_root,
        &workspace.root,
        &global,
        offline,
        &lockfile.packages,
    )
    .context("building the install resolver")?;

    // Re-fetch every locked package at its exact pinned version — no
    // re-resolution, the lockfile decides the version. The recorded
    // `content_hash` is forwarded so a source serving disagreeing bytes
    // is rejected: `vibe reinstall` reproduces the lock, never drifts it.
    //
    // The old "wipe the project cache so every fetch re-downloads" step
    // (PROP-009 §2.10) retired with the project cache itself: payload now lands
    // in the machine-global store (`~/.vibe/cache/`, PROP-010 §2.7), which our
    // code never rewrites — every fetch still walks the sources, and the pin
    // gate plus the read-time entry check make the re-fetched bytes prove
    // themselves against the lockfile regardless of what the store already
    // holds.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;

    let mut resolution: Vec<ResolvedDep> = Vec::with_capacity(lockfile.packages.len());
    let mut source_hashes = HashMap::new();
    for locked in &lockfile.packages {
        let pkgref = exact_pkgref(locked.kind, &locked.group, &locked.name, &locked.version)?;
        let cached = resolver
            .resolve_and_fetch(&pkgref, &store_root, Some(&locked.content_hash))
            .with_context(|| {
                format!(
                    "re-fetching `{}/{}@{}` from source",
                    locked.group, locked.name, locked.version
                )
            })?;
        source_hashes.insert(
            (cached.resolved.group.clone(), cached.resolved.name.clone()),
            cached.content_hash.clone(),
        );
        resolution.push(ResolvedDep {
            kind: cached.package_meta().kind,
            group: cached.resolved.group.clone(),
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            content_dir: cached.cache_dir.clone(),
            source_hash: Some(ContentHash::from_validated(cached.content_hash.clone())),
            manifest: cached.manifest.clone(),
            // The recorded resolution edges — `apply_resolution` walks
            // them to compose each node's dependency boot. A lockfile
            // dependency pkgref is group-qualified (PROP-008 §2.6).
            requires: locked
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            admitted_by: None,
            via_override: None,
            // `--force` materialises with `Verify` (below), so this flag does
            // not change reinstall's behaviour; set from the source for
            // consistency with `vibe install` (PROP-011 §2.6).
            source_mutable: vibe_workspace::freshness::is_in_workspace_file_source(
                &cached.source_uri,
                &workspace.root,
            ),
            in_place_changed: None,
        });
    }

    // The PREPARED constructor over the tree this command already owns: the
    // wrapper's own `Workspace::discover` would be a second read of the very
    // tree the apply below is about to rewrite, and the manifest it needs is
    // already on that tree.
    // The command's ONE agent backend, built from the values it already holds
    // — no discovery, no second manifest read — and shared by the apply and any
    // continuation it services.
    let agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend> =
        std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
            &workspace.root,
            &workspace.root_manifest,
        ));
    let lifecycle = InstallSlotLifecycle::from_projection_observed_prepared(
        &workspace.root,
        &workspace.root_manifest,
        &resolution,
        &resolution,
        workspace,
        metadata.clone(),
        reinstall_stream_mode(ctx),
        vibe_install::SlotLifecycleSeams {
            observer: std::sync::Arc::new(LifecycleSlotObserver::new(ctx, metadata.clone())),
            agent: agent.clone(),
        },
        lease.clone(),
    )?;

    // From here on the run is rewriting the tree. Every failure freezes it.
    let outcome = apply(
        ctx,
        Apply {
            lifecycle: &lifecycle,
            identity,
            workspace,
            metadata,
            spec_format,
            trace,
            resolution: &resolution,
            source_hashes: SourceHashes(source_hashes),
            lease,
            agent: &agent,
        },
    );
    outcome.map_err(|error| {
        carry_measured(error, || {
            RegisteredReportDraft::Reinstall(Box::new(ReinstallDraft::failed(
                identity,
                // The run's progress, PROMOTED to the completed boundary as
                // soon as the apply succeeded — see `apply`. Reading it before
                // that promotion reported `complete: false` with no regenerated
                // nodes over a tree whose boot had already been rewritten.
                lifecycle.progress(),
                lifecycle.take_reports().unwrap_or_default(),
            )))
        })
    })
}

struct Apply<'a> {
    lifecycle: &'a InstallSlotLifecycle,
    identity: &'a ReinstallIdentity,
    workspace: &'a Workspace,
    metadata: &'a RunMetadata,
    spec_format: SpecFormat,
    trace: Option<&'a TraceRun>,
    resolution: &'a [ResolvedDep],
    source_hashes: SourceHashes,
    lease: &'a std::sync::Arc<LifecycleLease>,
    /// The command's ONE agent backend, shared with the continuation this
    /// apply may service.
    agent: &'a std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
}

fn apply(ctx: &output::Context, inputs: Apply<'_>) -> Result<ReinstallDraft> {
    let Apply {
        lifecycle,
        identity,
        workspace,
        metadata,
        spec_format,
        trace,
        resolution,
        source_hashes,
        lease,
        agent,
    } = inputs;
    let applied = apply_resolution_with_spec_format_and_slot_lifecycle_traced(
        workspace,
        resolution,
        SlotIntegrity::Verify,
        spec_format,
        Some(&source_hashes),
        SlotLifecycleMode::Callback(lifecycle),
        trace,
    );
    // A hosted row parked: a durable handoff for THIS command, reported as
    // `reinstall` with `resume: vibe reinstall`, exit 0, nothing paid for and
    // every post-barrier row skipped.
    if let Some(delegation) = lifecycle.parked() {
        crate::commands::lifecycle::check_delegation(&delegation)?;
        return Ok(parked_draft(identity, lifecycle, &delegation));
    }
    let mut outcome = applied.context("re-materialising the workspace")?;
    // The apply is DONE: slots are materialised, stale ones pruned, boot
    // regenerated. Promote the run's progress from the materialisation
    // boundary to that completed record immediately, before anything can park,
    // fail or service a continuation — every one of those reads
    // `lifecycle.progress()`, and the boundary snapshot they used to read says
    // `complete: false` with no regenerated nodes and no pruned tail, over a
    // tree that has already been rewritten.
    lifecycle.record_complete(InstallProgress::complete(&outcome));
    if let Some(plan) = outcome.take_post_install_plan() {
        let ran = run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(lifecycle));
        if let Some(delegation) = lifecycle.parked() {
            crate::commands::lifecycle::check_delegation(&delegation)?;
            return Ok(parked_draft(identity, lifecycle, &delegation));
        }
        ran.context("running post-install lifecycle")?;
    }
    // An apply can finish without revisiting a live slot-scoped park: an
    // unchanged slot raises no payload event, so the post-install plan is empty
    // and the delegated row is never reached again. Service the persisted
    // continuation BEFORE clearing it — clearing an unserviced one forgets a
    // target this run promised to finish.
    if let Some(serviced) = continuation::service_if_owed(
        ctx,
        lifecycle,
        continuation::Request {
            identity,
            workspace,
            metadata,
            // The COMPLETED record promoted above, so a resumed park reports
            // the boot this apply really regenerated.
            progress: lifecycle.progress(),
            lease,
            agent: agent.clone(),
        },
    )? {
        return Ok(ReinstallDraft::completed(
            identity,
            serviced.progress,
            serviced.rows,
            serviced.parked.as_ref(),
        ));
    }
    lifecycle.clear_continuation().map_err(anyhow::Error::msg)?;
    let rows = lifecycle.take_reports()?;
    // ORDINARY SUCCESS keeps its historical projection.
    //
    // The run's completed record above is what every park, failure and serviced
    // continuation reports, and it is the truthful one for them: they had
    // nothing to say before. A completed `vibe reinstall --force`, by contrast,
    // has ALWAYS reported the regenerated nodes and the pruned slots and
    // nothing else — `materialised` and `skipped` are empty in every
    // trace-disabled document an existing consumer has ever parsed. Widening
    // them here would change those bytes for a run that did not change.
    Ok(ReinstallDraft::completed(
        identity,
        regenerated(outcome.nodes_regenerated, outcome.pruned),
        rows,
        None,
    ))
}

/// No locked packages — `--force` has nothing to re-fetch. Still regenerate
/// boot so a stale artifact is recomputed, under the same borrowed recorder.
fn regenerate_only(ctx: &output::Context, inputs: &Forced<'_>) -> Result<ReinstallDraft> {
    ctx.heading("\nReinstall --force — no packages locked; regenerate boot only.");
    if !confirm(
        ctx,
        inputs.args,
        "No packages are locked — regenerate boot artifacts only?",
    )? {
        return Err(InstallError::UserDeclined.into());
    }
    let outcome = apply_resolution_with_spec_format_and_slot_lifecycle_traced(
        inputs.workspace,
        &[],
        SlotIntegrity::Verify,
        inputs.spec_format,
        None,
        SlotLifecycleMode::None,
        inputs.trace,
    )
    .context("regenerating the workspace")?;
    // No resolution means no materialised or skipped slots, so the completed
    // record and the regenerated shape are the same value here; the explicit
    // one says which fields this branch can ever populate.
    Ok(ReinstallDraft::completed(
        inputs.identity,
        regenerated(outcome.nodes_regenerated, outcome.pruned),
        Vec::new(),
        None,
    ))
}

fn parked_draft(
    identity: &ReinstallIdentity,
    lifecycle: &InstallSlotLifecycle,
    delegation: &vibe_lifecycle::Delegation,
) -> ReinstallDraft {
    ReinstallDraft::completed(
        identity,
        lifecycle.progress(),
        // A park is a SUCCESSFUL outcome the operator must be told about;
        // losing the whole handoff because a row list could not be taken would
        // strand the run.
        lifecycle.take_reports().unwrap_or_default(),
        Some(delegation),
    )
}
