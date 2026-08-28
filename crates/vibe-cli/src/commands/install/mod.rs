//! `vibe install <kind>:<name>[@version] …` — the thin CLI layer over
//! the `vibe-install` orchestrator (VIBEVM-SPEC §5.6, §9.1, §11.1).
//!
//! This module owns exactly the CLI's share of the transaction: input
//! normalisation (path canonicalisation, pkgref parsing, PROP-008 §2.6
//! short-name qualification), the `--git` declaration recording, cell
//! construction behind the [`vibe_install::InstallSource`] seam
//! (R-001 — the registry module builds the cells), the interactive
//! confirmation between plan and apply, and rendering. The pipeline
//! itself lives in `vibe-install`.
//!
//! ## Where the compile trace enters
//!
//! [`execute_prepared`] BORROWS `Option<&TraceRun>` and hands it to the traced
//! sibling of every API that compiles: the empty-world regeneration, the fresh
//! fast path's regeneration, and the ready apply. It never opens, finishes or
//! clones a recorder — the owner is the command boundary above
//! ([`direct::run`] for `vibe install`, `lifecycle::execute` for a phase
//! verb), and a second owner of the project's cooperative lock would be a
//! second answer to "is this workspace being traced right now".

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod closure_diff;
mod direct;
mod draft;
mod events;
mod inputs;
mod lease;
mod observer;
mod project_local;
mod ready;
mod report;
mod resolver;
mod resume;

pub(crate) use closure_diff::{emit_closure_diff, lane_sizes};
pub(crate) use direct::run as run_direct;
pub(crate) use draft::InstallDraft;
pub(crate) use project_local::project_packages_root;
pub(crate) use report::{HookReportPresentation, LifecycleHookView};
pub(crate) use resolver::{InstallResolver, build_install_resolver};
pub(crate) use vibe_install::exact_pinned_pkgref;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::PackageRef;
use vibe_core::manifest::Lockfile;
use vibe_core::user_config::UserConfig;
use vibe_install::{InstallRequest, Plan, SlotLifecycleReport};
use vibe_lifecycle::{LifecycleRunHandle, RunMetadata};
use vibe_resolver::FeatureRequest;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;

use crate::cli::InstallArgs;
use crate::commands::short_name;
use crate::output;

use events::CtxObserver;
pub(crate) use inputs::{
    PreparedWorkspace, SelectedManifest, generated_by, resolve_project_root, resolve_spec_format,
    selected_node_manifest,
};
pub(crate) use lease::acquire_lease;
/// The retained-process lease for hand-built unit-test fixtures.
#[cfg(test)]
pub(crate) use lease::test_lease;
pub(crate) use observer::LifecycleSlotObserver;
use resolver::apply_git_source_flag;
/// The serviced-continuation value, for the cross-module reds that build one.
#[cfg(test)]
pub(crate) use resume::ResumedInstall;
/// The neutral transport, for the cross-module reds that build one by hand.
/// Production constructs it only inside the resume seam.
#[cfg(test)]
pub(crate) use resume::carry_resume_failure;
pub(crate) use resume::{
    ResumeFailure, ResumeOutcome, ResumeRequest, own_resume, prefixed, resume_slot_continuation,
    take_resume_failure,
};

/// Whether the existing install implementation applied a plan or proved the
/// materialised world fresh. Lifecycle callers consume this instead of
/// inferring machine state from rendered text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallDisposition {
    Fresh,
    Applied,
    /// A hosted `agent` row parked for the hosting agent. The install stopped
    /// AT THAT ROW's point and did NOT render: a park travels outward as a
    /// value so the outermost command, and only it, emits the one document.
    ///
    /// How much is durable when that happens is point-dependent, and nothing
    /// here assumes. A `slot:pre-install` park precedes the remaining
    /// materialisation, the lockfile barrier and every post-barrier row; a
    /// `slot:post-install` or `phase:install` park follows a COMPLETE, durable
    /// apply and stops only what came after it. The accompanying
    /// [`vibe_install::InstallProgress`] is the boundary-measured record of
    /// which of those it was.
    Parked,
}

/// What one install invocation did, in the shape its caller renders.
///
/// Nothing in the install substrate prints a report any more: `vibe install`
/// renders a `cli-install-report`, a phase verb renders its own
/// `cli-lifecycle-report`, and update/reinstall render theirs. Returning the
/// outcome instead of printing it is what makes "exactly one document" a
/// property of the call graph rather than a hope at each call site.
pub(crate) struct InstallRun {
    pub(crate) disposition: InstallDisposition,
    pub(crate) progress: vibe_install::InstallProgress,
    /// How many packages this invocation RESOLVED — the solved graph's size,
    /// counted where the plan is produced. Not `materialised.len()`: a slot
    /// that was already present is resolved and skipped, and reading the
    /// count off the materialised list would silently under-report exactly
    /// the runs that changed the least. Zero on the fresh fast path, which
    /// resolves nothing at all.
    pub(crate) packages_resolved: usize,
    pub(crate) hooks: Vec<vibe_workspace::hooks::HookReport>,
    pub(crate) slot_reports: Vec<SlotLifecycleReport>,
    pub(crate) contributions:
        Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport>,
    pub(crate) notices: Vec<String>,
    pub(crate) parked: Option<vibe_lifecycle::Delegation>,
    pub(crate) world_summary: WorldCallbackSummary,
    pub(crate) project_root: PathBuf,
}

impl InstallRun {
    pub(crate) fn new(project_root: PathBuf, disposition: InstallDisposition) -> Self {
        Self {
            disposition,
            progress: vibe_install::InstallProgress::default(),
            packages_resolved: 0,
            hooks: Vec::new(),
            slot_reports: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            parked: None,
            world_summary: WorldCallbackSummary::default(),
            project_root,
        }
    }
}

/// Effective invocation facts the durable-world lifecycle callback needs in
/// the canonical handler envelope.
#[derive(Debug, Clone)]
pub(crate) struct InstallRunContext {
    pub(crate) metadata: RunMetadata,
    /// The command's mutation lease, shared by Arc into the callback: the
    /// post-durability world dispatch reuses this proof and never
    /// reacquires. Present on every path — the callback may run when no slot
    /// lifecycle exists at all (the empty-world no-op, the fresh fast path),
    /// and its `dispatch_plan` still needs the one owner.
    pub(crate) lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    pub(crate) lifecycle_run: Option<LifecycleRunHandle>,
    pub(crate) lifecycle_reports: Vec<SlotLifecycleReport>,
}

/// Counts produced by an additive post-durability observer. Keeping them
/// typed prevents quiet rendering from dropping a class of ritual output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WorldCallbackSummary {
    pub(crate) selected_contributions: usize,
    pub(crate) executed_contributions: usize,
    pub(crate) successful_contributions: usize,
    pub(crate) fresh_contributions: usize,
    pub(crate) notices: usize,
}

/// What the post-durability observer produced. The counts are the narration;
/// the rows and the handoff are machine facts the OUTERMOST command folds into
/// its single document — the observer itself renders nothing.
#[derive(Debug, Default)]
pub(crate) struct WorldCallbackOutcome {
    pub(crate) summary: WorldCallbackSummary,
    pub(crate) contributions:
        Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport>,
    pub(crate) notices: Vec<String>,
    pub(crate) parked: Option<vibe_lifecycle::Delegation>,
}

/// Everything one install execution needs that its caller already decided.
///
/// The identity is NOT selected here any more. A caller either owns the
/// command (and selected one identity together with its trace request, before
/// anything was allocated) or is chained inside one (and carries that outer
/// metadata unchanged). A fallback selection at this depth was a second
/// selector: it ran after the config load, could allocate a second run
/// directory, and had no way to know the effective trace bit its caller had
/// already committed to.
pub(crate) struct InstallExecution<'a> {
    pub(crate) args: InstallArgs,
    pub(crate) embedded_root: Option<PathBuf>,
    pub(crate) root_offline: bool,
    /// The command's mutation lease — the outermost lock, acquired by the
    /// owning boundary (direct install, a phase verb, update, reinstall)
    /// before anything execution-shaped was read. The install substrate
    /// never acquires; it consumes this proof and shares it onward by Arc.
    pub(crate) lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The ONE canonical selection of this command's project root. Resolved by
    /// the prelude epoch and carried; nothing below re-canonicalises a path.
    pub(crate) project_root: PathBuf,
    /// The command's ONE `UserConfig` load. Prepared by the owner, because it
    /// has to be: the config decides the offline posture that goes into the
    /// metadata, and the metadata is fixed before this function runs.
    pub(crate) user_config: UserConfig,
    /// The command's ONE selected-manifest snapshot — see
    /// [`SelectedManifest`]. Consumed below at the boundary that historically
    /// performed the read.
    pub(crate) manifest: SelectedManifest,
    /// What the owner's ONE attempt to build the workspace produced — see
    /// [`PreparedWorkspace`]. Consumed below; never retried.
    pub(crate) workspace: PreparedWorkspace,
    pub(crate) metadata: RunMetadata,
    /// Where slot-lifecycle narration goes when the install is a phase verb's
    /// prerequisite: the OUTER context, so its rows are still visible while
    /// the install's own summary stays suppressed.
    pub(crate) lifecycle_output: Option<&'a output::Context>,
    /// The command owner's recorder, borrowed. `None` is not "off" here — it
    /// is "this caller's command is not tracing", which is the same thing to
    /// every layer below.
    pub(crate) trace: Option<&'a TraceRun>,
}

/// The one install implementation, with an additive post-durability callback.
///
/// Renders nothing: every path returns [`InstallRun`] or an error, and the
/// outermost command owns the single document.
pub(crate) fn execute_prepared(
    ctx: &output::Context,
    execution: InstallExecution<'_>,
    // The callback receives the CURRENT workspace by borrow — the one this
    // execution loaded and, on a `--git` run, mutated in place. Post-durability
    // world planning must see that exact value; rediscovering would be a second
    // byte snapshot of a tree this command just changed.
    after_durable_world: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
        &Workspace,
    ) -> Result<WorldCallbackOutcome>,
) -> Result<InstallRun> {
    let InstallExecution {
        args,
        embedded_root,
        root_offline,
        lease,
        project_root,
        user_config,
        manifest,
        workspace,
        metadata,
        lifecycle_output,
        trace,
    } = execution;
    let mut after_durable_world = Some(after_durable_world);
    // PROP-011 §2.3 — the materialise-diff strategy, read from the ONE user
    // config this command loaded before anything was allocated. The same load
    // supplied the `[net].offline` rung of the offline ladder.
    let slot_integrity = user_config.install.slot_integrity;
    // PROP-010 §2.5 — the resolved offline posture: CLI flag (root
    // `--offline` OR this command's PROP-030 §3.1 `--offline`) >
    // `VIBE_OFFLINE` > user-config `[net].offline`. Resolved here, once,
    // so the resolver below receives a single boolean.
    let offline = output::resolve_offline(root_offline || args.offline, user_config.net.offline);
    // The resolved run metadata is the invocation's identity; the callback
    // context owns a clone so a later seam can still read it.
    let run_metadata = metadata.clone();
    let lifecycle_run = InstallRunContext {
        metadata,
        // Shared, not moved: the resume seams below rebuild their slot runs
        // on this same one acquisition.
        lease: lease.clone(),
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
    };

    // The command's ONE authoritative read of its world, consumed at exactly
    // the point this function has always read the tree: after the config load
    // and after the identity was selected, so a malformed manifest still fails
    // here, with the same error, after the same side effects.
    //
    // The lock entries beside it are the provenance channel for store-backed
    // resolutions (PROP-010 §2.6) and ride into the resolver as a builder
    // input; the same snapshot serves short-name qualification below.
    // The stored manifest result FIRST: a malformed selected manifest is this
    // command's error, in its own words, at the point it has always been
    // raised — after the config load and after the identity was selected.
    let mut manifest = manifest.into_manifest()?;
    let mut workspace = match workspace {
        PreparedWorkspace::Loaded(workspace) => *workspace,
        // The FIRST answer, returned as it was. Retrying here could succeed
        // against a tree the identity and the trace were never prepared for.
        PreparedWorkspace::DiscoveryFailed(error) => {
            return Err(anyhow::Error::new(*error)
                .context("discovering the workspace enclosing the project"));
        }
        // A caller with no prelude builds the tree here — from the snapshot
        // just consumed, so this is still one read of the selected node.
        PreparedWorkspace::DiscoverHere => {
            Workspace::discover_with_selected_manifest(&project_root, &manifest)
                .context("discovering the workspace enclosing the project")?
        }
        // Unreachable in practice: the line above returns the stored manifest
        // error first. Named rather than merged so that a future caller which
        // rewraps a parsed manifest beside this arm is a compile-time question
        // instead of a silent success.
        PreparedWorkspace::SelectedManifestInvalid => {
            anyhow::bail!(
                "internal: the selected manifest was reported invalid but its error was                  already consumed"
            );
        }
    };
    let spec_format = resolve_spec_format(&manifest, &user_config);

    // M1.15: `vibe install <pkgref> --git <url> --tag/branch/rev <ref>`
    // adds a git-source declaration to `[requires.packages]` before
    // resolving. The added declaration is picked up by the resolver
    // built immediately below; subsequent installs of the same project
    // reproduce the install via the now-recorded git-source entry.
    if args.git.is_some() {
        // Built once, applied to the STORED RAW snapshot, and persisted from
        // that same value — no second read of a file this command is rewriting.
        let dep = apply_git_source_flag(&args, &mut manifest, &project_root)
            .context("recording --git declaration to vibe.toml")?;
        // Then the SAME delta is replayed onto the finalised node inside the
        // loaded tree — never an assignment, which would restore
        // `var_packages` and erase the concrete versions the loader resolved.
        if let Some(selected) = inputs::selected_node_manifest_mut(&mut workspace, &project_root) {
            vibe_install::record_git_source(selected, dep);
        }
    }

    let global = vibe_core::GlobalRegistryConfig::load()?;
    let lockfile_path = workspace.root.join(Lockfile::FILENAME);
    // An unsupported-schema lock reads as EMPTY on the install path: install
    // is the regeneration verb the schema policy names, so it must never
    // refuse to run because the artifact it is about to rewrite is outdated.
    let lockfile_snapshot = if lockfile_path.exists() {
        match Lockfile::read(&lockfile_path) {
            Ok(lock) => lock,
            Err(vibe_core::Error::UnsupportedLockfile { .. }) => Lockfile::empty(
                generated_by(),
                crate::commands::init::current_timestamp_utc(),
            ),
            Err(other) => return Err(other.into()),
        }
    } else {
        Lockfile::empty(
            generated_by(),
            crate::commands::init::current_timestamp_utc(),
        )
    };
    // ##EMPTY-REQUIRES-IS-A-NO-OP (PROP-011, 2026-08-24): a bare install
    // over a workspace whose `[requires]` union is empty is a fresh
    // project, not an error — and it must not demand a registry either
    // (`vibe init && vibe install` works out of the box). Regenerate the
    // boot artifacts of the empty world and report the fresh shape.
    if args.packages.is_empty()
        && workspace.iter_nodes().all(|(_, node)| {
            node.requires.packages.is_empty() && node.requires.git_packages.is_empty()
        })
        && lockfile_snapshot.meta.root_dependencies.is_empty()
    {
        ctx.heading("nothing declared — regenerating boot artifacts for the empty world");
        let nodes = vibe_workspace::install::regenerate_boot_traced(&workspace, spec_format, trace)
            .context("regenerating boot artifacts for the empty world")?;
        let after = after_durable_world
            .take()
            .context("internal: install durable-world callback already consumed")?;
        let world = after(
            &project_root,
            InstallDisposition::Fresh,
            lifecycle_run,
            &workspace,
        )?;
        return Ok(InstallDraft::fresh_run(&project_root, nodes, world));
    }

    // PROP-050 ##VERIFY-LOCK-DIFF — the lane-size half of the pre-apply
    // snapshot, taken beside the lock snapshot so one read point feeds
    // the whole diff. Sampled again after a successful apply below.
    let lanes_before = lane_sizes(&workspace.root);
    let resolver = build_install_resolver(
        &args,
        &manifest,
        embedded_root.as_deref(),
        &project_root,
        &global,
        offline,
        &lockfile_snapshot.packages,
    )?;

    // Parse the CLI pkgrefs and qualify short names at the input
    // boundary (PROP-008 §2.6) — manifests only ever store the
    // qualified form, and the orchestrator requires it.
    let cli_roots: Vec<PackageRef> = args
        .packages
        .iter()
        .map(|raw| PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`")))
        .collect::<Result<_>>()?;
    let cli_roots: Vec<PackageRef> = cli_roots
        .iter()
        .map(|r| short_name::qualify(&resolver, r, &lockfile_snapshot))
        .collect::<Result<_>>()?;

    let request = InstallRequest {
        roots: cli_roots,
        features: FeatureRequest {
            explicit: args.features.clone(),
            no_defaults: args.no_default_features,
            all: args.all_features,
        },
        language: args.language.clone(),
        exact: args.exact,
        generated_by: generated_by(),
    };

    // The PREPARED planner, over the two values this command already owns.
    // The compatibility wrapper would re-read the selected `vibe.toml` — the
    // very file a `--git` run rewrote a few lines above — and re-discover the
    // tree that rewrite was replayed into.
    let plan = vibe_install::plan_prepared_with_spec_format(
        &resolver,
        &project_root,
        &mut manifest,
        &mut workspace,
        request,
        spec_format,
        &CtxObserver(ctx),
    )?;
    match plan {
        Plan::Fresh => {
            // PROP-011 §2.2 — application is just a whole-tree boot
            // regeneration (cheap, self-healing — §2.4).
            ctx.heading("vibe.lock is fresh — skipping resolution");
            // The one workspace snapshot this command owns. Nothing between
            // its read and here mutated the tree — the fresh fast path
            // resolves nothing and copies nothing — so a re-read could only
            // ever differ by racing another process, which is exactly the
            // difference a single snapshot exists to refuse.
            let nodes =
                vibe_workspace::install::regenerate_boot_traced(&workspace, spec_format, trace)
                    .context("regenerating boot artifacts from the materialised state")?;
            let after = after_durable_world
                .take()
                .context("internal: install durable-world callback already consumed")?;
            // A slot run that parked at a POST-install row wrote the lock
            // first, so its resume lands here — on the fresh fast path, which
            // would otherwise never rebuild that run at all and would report a
            // clean completion over a live delegated row. Rebuild it from the
            // exact target set the original pass recorded, and finish it
            // before anything reports fresh.
            match resume_slot_continuation(
                ctx,
                resume::ResumeRequest {
                    project_root: &project_root,
                    workspace: &workspace,
                    manifest: &manifest,
                    metadata: &run_metadata,
                    lease: &lease,
                    spec_format,
                    disposition: InstallDisposition::Fresh,
                    progress: vibe_install::InstallProgress::fresh(nodes.clone()),
                    // The fresh fast path skips resolution entirely.
                    packages_resolved: 0,
                },
            )? {
                // A satisfied resume still owes the post-durability world: the
                // hosting output arrived, so an authored `phase:install` row
                // must run, in the run the resume just finished.
                ResumeOutcome::Completed(resumed) => {
                    return resume::finish_resumed(*resumed, &project_root, &workspace, after);
                }
                // TRANSPORTED, not reduced. The family is not knowable here —
                // this same function is `vibe install`'s body, a phase verb's
                // prerequisite and `vibe update --all`'s delegate — so the
                // measurement travels outward neutrally and the one outer
                // command that owns the report consumes it.
                ResumeOutcome::Failed(failure) => {
                    return Err(resume::carry_resume_failure(failure));
                }
                ResumeOutcome::Nothing => {}
            }
            let world = after(
                &project_root,
                InstallDisposition::Fresh,
                lifecycle_run,
                &workspace,
            )?;
            Ok(InstallDraft::fresh_run(&project_root, nodes, world))
        }
        // The ready apply is its own cell — the confirmation, the traced
        // apply, the slot-failure carrier and the closure diff — so this
        // function stays the shape of the DECISION (empty / fresh / ready)
        // rather than of the largest branch.
        Plan::Ready(planned) => ready::apply(
            ctx,
            ready::ReadyApply {
                args: &args,
                project_root: &project_root,
                manifest: &manifest,
                workspace: &workspace,
                resolver: &resolver,
                planned: *planned,
                slot_integrity,
                spec_format,
                lockfile_path: &lockfile_path,
                lockfile_snapshot: &lockfile_snapshot,
                lanes_before: &lanes_before,
                run_metadata: &run_metadata,
                lifecycle_output,
                trace,
            },
            lifecycle_run,
            after_durable_world
                .take()
                .context("internal: install durable-world callback already consumed")?,
        ),
    }
}
