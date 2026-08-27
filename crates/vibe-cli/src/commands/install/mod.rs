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

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod closure_diff;
mod document;
mod events;
mod inputs;
mod observer;
mod project_local;
mod report;
mod resolver;
mod resume;

pub(crate) use closure_diff::{emit_closure_diff, lane_sizes};
pub(crate) use project_local::project_packages_root;
pub(crate) use report::{HookReportPresentation, HookReportView, LifecycleHookView};
pub(crate) use resolver::{InstallResolver, build_install_resolver};
pub(crate) use vibe_install::exact_pinned_pkgref;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_install::{InstallRequest, Plan, SlotLifecycleReport};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleRunHandle, RunMetadata};
use vibe_resolver::FeatureRequest;
use vibe_workspace::Workspace;

use crate::cli::InstallArgs;
use crate::commands::short_name;
use crate::exit_code::InstallError;
use crate::output;

pub(crate) use document::emit_command_document;
use document::fresh_run;
use events::CtxObserver;
pub(crate) use inputs::{generated_by, resolve_project_root, resolve_spec_format};
pub(crate) use observer::LifecycleSlotObserver;
use resolver::apply_git_source_flag;
pub(crate) use resume::{ResumeRequest, resume_slot_continuation};

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
    fn new(project_root: PathBuf, disposition: InstallDisposition) -> Self {
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

#[allow(
    dead_code,
    reason = "the bare install facade is a public seam kept for callers that need no \
              post-durability callback; `vibe install` itself uses `run_with_world_callback`"
)]
pub fn run(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<InstallRun> {
    run_with_world_callback(ctx, args, embedded_root, root_offline, |_, _, _| {
        Ok(WorldCallbackOutcome::default())
    })
}

/// The one install implementation with an additive post-durability callback.
///
/// Only the direct top-level install facade supplies a non-empty callback. All
/// install-family callers retain [`run`]'s byte-for-byte rendering behaviour.
pub(crate) fn run_with_world_callback(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    after_durable_world: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
    ) -> Result<WorldCallbackOutcome>,
) -> Result<InstallRun> {
    run_with_lifecycle_context(
        ctx,
        args,
        embedded_root,
        root_offline,
        None,
        None,
        after_durable_world,
    )
}

pub(crate) fn run_with_lifecycle_context(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    metadata: Option<RunMetadata>,
    lifecycle_output: Option<&output::Context>,
    after_durable_world: impl FnOnce(
        &Path,
        InstallDisposition,
        InstallRunContext,
    ) -> Result<WorldCallbackOutcome>,
) -> Result<InstallRun> {
    let mut after_durable_world = Some(after_durable_world);
    let project_root = resolve_project_root(&args.path)?;
    // PROP-011 §2.3 — the materialise-diff strategy, read once from the
    // user config so a malformed config fails before any resolution. The
    // same load supplies the `[net].offline` rung of the offline ladder.
    let user_config = UserConfig::load().context("loading the user config")?;
    let slot_integrity = user_config.install.slot_integrity;
    // PROP-010 §2.5 — the resolved offline posture: CLI flag (root
    // `--offline` OR this command's PROP-030 §3.1 `--offline`) >
    // `VIBE_OFFLINE` > user-config `[net].offline`. Resolved here, once,
    // so the resolver below receives a single boolean.
    let offline = output::resolve_offline(root_offline || args.offline, user_config.net.offline);
    let metadata = metadata.unwrap_or_else(|| RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: ctx.agent_mode(),
        force: args.force,
        run_id: String::new(),
        started: crate::commands::init::current_timestamp_utc(),
    });
    // An explicit `vibe install` selects its own durable identity through the
    // ONE selector, exactly as a phase verb does; a chained install inherits
    // the caller's already-selected metadata unchanged.
    let metadata = if metadata.run_id.is_empty() {
        let identity = super::lifecycle::run_identity(
            ctx,
            &args.path,
            &metadata.requested,
            &metadata.chain,
            metadata.force,
        )
        .context("selecting the install lifecycle run identity")?;
        RunMetadata {
            run_id: identity.run_id,
            started: identity.started,
            ..metadata
        }
    } else {
        metadata
    };
    // The resolved run metadata is the invocation's identity; the callback
    // context owns a clone so a later seam can still read it.
    let run_metadata = metadata.clone();
    let mut lifecycle_run = InstallRunContext {
        metadata,
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
    };

    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;
    let spec_format = resolve_spec_format(&manifest, &user_config);

    // M1.15: `vibe install <pkgref> --git <url> --tag/branch/rev <ref>`
    // adds a git-source declaration to `[requires.packages]` before
    // resolving. The added declaration is picked up by the resolver
    // built immediately below; subsequent installs of the same project
    // reproduce the install via the now-recorded git-source entry.
    if args.git.is_some() {
        apply_git_source_flag(&args, &mut manifest, &project_root)
            .context("recording --git declaration to vibe.toml")?;
    }

    let global = vibe_core::GlobalRegistryConfig::load()?;
    // The workspace and its lockfile are read BEFORE the resolver is
    // built: the lock entries are the provenance channel for
    // store-backed resolutions (PROP-010 §2.6) and ride into the
    // resolver as a builder input. The same snapshot serves short-name
    // qualification below — one read, two consumers.
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
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
        let nodes =
            vibe_workspace::install::regenerate_boot_with_spec_format(&workspace, spec_format)
                .context("regenerating boot artifacts for the empty world")?;
        let after = after_durable_world
            .take()
            .context("internal: install durable-world callback already consumed")?;
        let world = after(&project_root, InstallDisposition::Fresh, lifecycle_run)?;
        return Ok(fresh_run(&project_root, nodes, world));
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

    let plan = vibe_install::plan_with_spec_format(
        &resolver,
        &project_root,
        request,
        spec_format,
        &CtxObserver(ctx),
    )?;
    match plan {
        Plan::Fresh => {
            // PROP-011 §2.2 — application is just a whole-tree boot
            // regeneration (cheap, self-healing — §2.4).
            ctx.heading("vibe.lock is fresh — skipping resolution");
            let ws = Workspace::discover(&project_root)
                .context("re-discovering the workspace for boot regeneration")?;
            let nodes = vibe_workspace::install::regenerate_boot_with_spec_format(&ws, spec_format)
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
            if let Some(resumed) = resume_slot_continuation(
                ctx,
                resume::ResumeRequest {
                    project_root: &project_root,
                    workspace: &workspace,
                    manifest: &manifest,
                    metadata: &run_metadata,
                    spec_format,
                    disposition: InstallDisposition::Fresh,
                    progress: vibe_install::InstallProgress::fresh(nodes.clone()),
                    // The fresh fast path skips resolution entirely.
                    packages_resolved: 0,
                },
            )? {
                return Ok(resumed);
            }
            let world = after(&project_root, InstallDisposition::Fresh, lifecycle_run)?;
            Ok(fresh_run(&project_root, nodes, world))
        }
        Plan::Ready(planned) => {
            // Show the plan: the packages to materialise.
            report::present_resolution(ctx, &planned.resolution);
            // Counted here, from the solved graph itself, before the plan is
            // consumed by the apply.
            let packages_resolved = planned.resolution.len();

            // Confirm (unless --assume-yes or --json or not a TTY).
            let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
                true
            } else if !console::user_attended() {
                // No TTY → refuse to apply without explicit --assume-yes.
                // This matches the book's "ask a human" discipline for any
                // destructive action.
                bail!(
                    "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
                );
            } else {
                Confirm::new()
                    .with_prompt(format!(
                        "Materialise {} package{} into vibedeps/ and regenerate boot artifacts?",
                        planned.resolution.len(),
                        if planned.resolution.len() == 1 {
                            ""
                        } else {
                            "s"
                        },
                    ))
                    .default(false)
                    .interact()
                    .context("reading user confirmation")?
            };
            if !approved {
                return Err(InstallError::UserDeclined.into());
            }

            // PROP-054 ##INSTALL-IS-CONSENT: `[hooks]` is translated to
            // `slot:` contributions and runs through the lifecycle handler
            // engine. The install confirmation above is the sole trust
            // decision; there is no hook-specific prompt or allow flag.
            let observer = LifecycleSlotObserver::new(
                lifecycle_output.unwrap_or(ctx),
                lifecycle_run.metadata.clone(),
            );
            let applied = vibe_install::apply_with_spec_format_and_lifecycle_observed(
                &resolver,
                *planned,
                slot_integrity,
                spec_format,
                lifecycle_run.metadata.clone(),
                if ctx.is_json() {
                    StreamMode::Capture
                } else if ctx.suppresses_output() {
                    StreamMode::Null
                } else {
                    StreamMode::Inherit
                },
                // `agent` is legal at `slot:` points too, so the same
                // `vibe-llm` adapter the create phase uses is injected here;
                // an install-time agent contribution must not silently degrade
                // to the refusing default just because it ran at the barrier.
                vibe_install::SlotLifecycleSeams {
                    observer: std::sync::Arc::new(observer),
                    agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                        &project_root,
                    )?),
                },
            );
            // A parked slot row is a durable handoff, not an install failure:
            // the chain stopped at that row's point, whatever preceded it is
            // already durable and measured in `progress`, and nothing was paid
            // for. It travels OUT as a value — this layer renders nothing, so
            // the outermost command owns the single document.
            let applied = match applied {
                Ok(applied) => applied,
                Err(vibe_install::Error::Delegated {
                    delegation,
                    reports,
                    progress,
                }) => {
                    crate::commands::lifecycle::check_delegation(&delegation)?;
                    let mut parked =
                        InstallRun::new(project_root.clone(), InstallDisposition::Parked);
                    parked.packages_resolved = packages_resolved;
                    parked.progress = *progress;
                    parked.slot_reports = reports;
                    parked.parked = Some(*delegation);
                    return Ok(parked);
                }
                // A slot row FAILED. `vibe install` is still the outermost
                // command, so its one document reports the failure —
                // `ok: false` with the executed rows — before the error
                // reaches the exit code. Without this, removing the per-row
                // echo would have taken the machine record of a failed
                // install with it.
                Err(vibe_install::Error::SlotFailed {
                    source,
                    reports,
                    progress,
                }) => {
                    report::emit_failed_document(ctx, &project_root, &progress, &reports)?;
                    return Err(anyhow::Error::new(*source));
                }
                Err(error) => return Err(error.into()),
            };
            lifecycle_run.lifecycle_run = applied.lifecycle_run.clone();
            lifecycle_run.lifecycle_reports = applied.lifecycle_reports.clone();
            let after = after_durable_world
                .take()
                .context("internal: install durable-world callback already consumed")?;
            // An apply can finish without visiting a live slot-scoped park:
            // an unchanged slot produces no payload event, so the post-install
            // plan is empty and the delegated row is never revisited. The
            // persisted continuation is exactly the mechanism for that case —
            // consume it before anything reports a completed run.
            if let Some(resumed) = resume_slot_continuation(
                ctx,
                resume::ResumeRequest {
                    project_root: &project_root,
                    workspace: &workspace,
                    manifest: &manifest,
                    metadata: &run_metadata,
                    spec_format,
                    disposition: InstallDisposition::Applied,
                    progress: applied.progress.clone(),
                    packages_resolved,
                },
            )? {
                return Ok(resumed);
            }
            let world = after(&project_root, InstallDisposition::Applied, lifecycle_run)?;
            // PROP-050 ##VERIFY-LOCK-DIFF — after a successful apply, print
            // the closure diff (the pre-apply lock snapshot vs the freshly
            // written one, lane bytes before/after): a mid-graph re-export
            // widening is a reviewed event, not a silent seep. Emitted ahead
            // of the final report so the `--json` stream keeps the report as
            // its last document. A read failure of the just-written lock
            // skips the diff rather than failing the completed install.
            if let Ok(new_lock) = Lockfile::read(&lockfile_path) {
                emit_closure_diff(
                    ctx,
                    "install",
                    &lockfile_snapshot,
                    &new_lock,
                    &lanes_before,
                    &lane_sizes(&workspace.root),
                );
            }
            let mut run = InstallRun::new(
                project_root.clone(),
                if world.parked.is_some() {
                    InstallDisposition::Parked
                } else {
                    InstallDisposition::Applied
                },
            );
            run.packages_resolved = packages_resolved;
            run.progress = applied.progress.clone();
            run.hooks = applied
                .outcome
                .hook_reports
                .iter()
                .chain(&applied.post_install_reports)
                .cloned()
                .collect();
            run.slot_reports = applied.lifecycle_reports.clone();
            // ONLY the phase-ritual rows: the slot rows live on
            // `slot_reports`, and the document joins the two exactly once.
            // Carrying them in both places double-counted every slot row.
            run.contributions = world.contributions;
            run.notices = world.notices;
            run.parked = world.parked;
            run.world_summary = world.summary;
            Ok(run)
        }
    }
}
