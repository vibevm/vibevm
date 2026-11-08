//! The ONE shared install core, and the vocabulary it reports in.
//!
//! This is not a CLI layer: `vibe install`, a phase verb's prerequisite
//! install, `vibe update --all` and a hosted MCP surface all execute the SAME
//! function here, through the ports in [`crate::ports`]. What stays in a
//! surface is argument grammar, rendering, interactive confirmation, credential
//! and provider construction, and registry cell composition.
//!
//! ## Where the compile trace enters
//!
//! [`execute_prepared`] BORROWS `Option<&TraceRun>` and hands it to the traced
//! sibling of every API that compiles: the empty-world regeneration, the fresh
//! fast path's regeneration, and the ready apply. It never opens, finishes or
//! clones a recorder — the owner is the command boundary above, and a second
//! owner of the project's cooperative lock would be a second answer to "is this
//! workspace being traced right now".

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;

mod args;
mod inputs;
mod lease;
mod outcome;
mod ready;
mod resume;
mod selection;
mod world_projection;

pub use world_projection::provisional_world;

use anyhow::{Context, Result};
use vibe_core::PackageRef;
use vibe_core::manifest::Lockfile;
use vibe_install::{InstallRequest, Plan};
use vibe_lifecycle::{AgentBackend, LifecycleLease, RunMetadata};
use vibe_resolver::FeatureRequest;
use vibe_workspace::compile_trace::TraceRun;

use crate::ports::{
    AfterDurableWorld, ConfirmGate, InstallManifestMutation, InstallNarration, InstallObserver,
    PackageSourceBuild, PackageSourceFactory, RegistryEnvironment,
};

pub use args::{InstallInputs, InstallPolicy};
pub use inputs::{generated_by, resolve_project_root, resolve_spec_format, selected_node_manifest};
pub use lease::{acquire_lease, lease_root};
pub use outcome::{
    InstallDisposition, InstallRun, InstallRunContext, NativeInstallContext, WorldCallbackOutcome,
    WorldCallbackSummary,
};
pub use resume::{
    ResumeOutcome, ResumeRequest, ResumedInstall, own_resume, prefixed, resume_slot_continuation,
};
pub use selection::{PreparedSelection, ProvenSelection, SelectedManifest};

use outcome::fresh_run;

/// The run-scoped facts one native-world regeneration reads: the trace, the
/// invocation's identity, the mutation lease and the cancellable progress.
struct NativeRunScope<'a> {
    trace: Option<&'a TraceRun>,
    metadata: &'a RunMetadata,
    lease: &'a LifecycleLease,
    progress: &'a vibe_core::progress::Progress,
}

fn regenerate_native_world(
    project_root: &std::path::Path,
    workspace: &vibe_workspace::Workspace,
    resolution: &[vibe_workspace::install::ResolvedDep],
    spec_format: vibe_core::manifest::SpecFormat,
    scope: NativeRunScope<'_>,
) -> Result<(Vec<String>, outcome::NativeInstallContext)> {
    let NativeRunScope {
        trace,
        metadata,
        lease,
        progress,
    } = scope;
    progress.checkpoint()?;
    vibe_install::configuration_admission::resolved_packages(resolution)?;
    let (world, lowering, sidecar) =
        crate::world::prepare_owner_runtime_inputs(project_root, workspace, resolution)?;
    let platform = vibe_lifecycle::native::NativePlatform::current()?;
    let run = vibe_workspace::extension_world::OwnerRuntimeRunFacts {
        run_id: metadata.run_id.clone(),
        state_root: lease.root().join(".vibe"),
        platform: platform.key().to_owned(),
        offline: metadata.offline,
        created_at: metadata.started.clone(),
    };
    let mut make_provider = |policies| {
        Ok(
            vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(platform, policies)
                .with_cancellation(progress.cancellation().clone()),
        )
    };
    let (nodes, carriage) = vibe_workspace::install::regenerate_boot_from_traced_native(
        workspace,
        resolution,
        world,
        spec_format,
        trace,
        lowering,
        run,
        &mut make_provider,
    )?;
    Ok((nodes, outcome::NativeInstallContext::new(carriage, sidecar)))
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
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallExecution<'a> {
    /// The surface-neutral install inputs.
    pub args: InstallInputs,
    /// Where this run's registry environment is seeded and loaded — once.
    pub environment: &'a dyn RegistryEnvironment,
    /// The narrow execution policy: the already-resolved offline posture, the
    /// slot-integrity strategy and the operator's spec-format default.
    pub policy: InstallPolicy,
    /// The command's mutation lease — the outermost lock, acquired by the
    /// owning boundary (direct install, a phase verb, update, reinstall)
    /// before anything execution-shaped was read. The install substrate
    /// never acquires; it consumes this proof and shares it onward by Arc.
    pub lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The command's ONE selected-world provenance bundle: the canonical root,
    /// the manifest snapshot taken at it, and the tree built from THAT
    /// snapshot — see [`PreparedSelection`]. One value rather than three
    /// fields, so a caller cannot hand this boundary a manifest from one moment
    /// and a tree from another. Proven below, at the boundary that historically
    /// performed the read.
    pub selection: PreparedSelection,
    /// The invocation's durable identity and effective posture.
    pub metadata: RunMetadata,
    /// The surface's own manifest mutation, applied at its one position: the
    /// grammar and the exit codes stay in the surface, the position stays here.
    pub manifest_mutation: &'a dyn InstallManifestMutation,
    /// The caller-selected package-source composition root. Borrowed, not
    /// recreated inside the install kernel, so a hosted surface can supply a
    /// credential-free policy without a surface back-edge.
    pub sources: &'a dyn PackageSourceFactory,
    /// The surface's one confirmation policy. The core invokes it only for a
    /// solved Ready plan, immediately before materialisation.
    pub confirm_gate: &'a dyn ConfirmGate,
    /// The CHILD install narration policy — deliberately not the phase
    /// observer's.
    pub observer: &'a dyn InstallObserver,
    /// The agent backend every agent row of this run is served by: the slot
    /// barrier, the phase dispatch and any resume share this ONE injection.
    pub agent: std::sync::Arc<dyn AgentBackend>,
    /// The command owner's recorder, borrowed. `None` is not "off" here — it
    /// is "this caller's command is not tracing", which is the same thing to
    /// every layer below.
    pub trace: Option<&'a TraceRun>,
}

/// The one install implementation, with an additive post-durability callback.
///
/// Renders nothing: every path returns [`InstallRun`] or an error, and the
/// outermost command owns the single document.
/// ```no_run
/// use vibe_orchestrator::InstallExecution;
/// use vibe_orchestrator::ports::NoAfterDurableWorld;
/// # fn call(execution: InstallExecution<'_>) -> anyhow::Result<()> {
/// // The post-durability stage is a NAMED port, never a closure.
/// let mut stage = NoAfterDurableWorld;
/// let run = vibe_orchestrator::execute_prepared(execution, &mut stage)?;
/// let _ = run.disposition;
/// # Ok(())
/// # }
/// ```
pub fn execute_prepared(
    execution: InstallExecution<'_>,
    // The post-durability stage sees the CURRENT workspace by borrow — the one
    // this execution loaded and, on a `--git` run, mutated in place. Named and
    // typed rather than a closure: a `FnOnce` could capture a whole rendering
    // context, which is exactly what this boundary exists to refuse.
    after_durable_world: &mut dyn AfterDurableWorld,
) -> Result<InstallRun> {
    let InstallExecution {
        args,
        environment,
        policy,
        lease,
        manifest_mutation,
        selection,
        metadata,
        sources,
        confirm_gate,
        observer,
        agent,
        trace,
    } = execution;
    let mut after_durable_world = Some(after_durable_world);
    // PROP-011 §2.3 — the materialise-diff strategy, and PROP-010 §2.5's
    // whole offline ladder, both decided by the surface from the ONE config it
    // loaded before anything was allocated. They arrive as answers, not as the
    // config that also carries provider settings this crate may not see.
    let InstallPolicy {
        offline,
        slot_integrity,
        spec_format_default,
    } = policy;
    // The resolved run metadata is the invocation's identity; the callback
    // context owns a clone so a later seam can still read it.
    let run_metadata = metadata.clone();
    let mut lifecycle_run = InstallRunContext {
        metadata,
        // Shared, not moved: the resume seams below rebuild their slot runs
        // on this same one acquisition.
        lease: lease.clone(),
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
        native: None,
    };

    // The command's ONE authoritative world, PROVEN at exactly the point this
    // function has always read the tree: after the identity was selected, so a
    // malformed manifest still fails here, with the same error, after the same
    // side effects. The stored manifest result speaks first and the ONE
    // workspace answer second — both inside the bundle, which is why they
    // cannot have come from different moments.
    let (project_root, mut manifest, mut workspace) = selection.prove()?.into_parts();
    // ---- the lease/root/selected gate ------------------------------------
    //
    // This is a PUBLIC entry point: the lease, the bundle and `metadata.selected`
    // still arrive as independent values, and nothing before this line proves
    // they describe the same tree. Everything after it mutates — the surface's
    // manifest write, the registry reads, the materialisation, the state store
    // rooted at the lease. So the agreement is proven HERE, once, after the tree
    // is loaded and before the first durable action, through the lease's own two
    // typed gates rather than a hand-rolled spelling of either refusal.
    //
    // The root gate first: a workspace loaded under a DIFFERENT root than the
    // one this command leased would write state beside another process's lock.
    lease.ensure_root(&workspace.root, "at install execution")?;
    // …then the selected-node gate, derived from THIS workspace's own authored
    // rels and the canonical root the BUNDLE carried. `node_rel_of` maps only
    // an already-canonical node and never normalises a spelling, so a root that
    // this tree does not contain is `None` — an honest mismatch, not a guess.
    let observed_selected = workspace
        .node_rel_of(&project_root)
        .map(|rel| rel.as_str().to_string());
    lease.ensure_selected(
        &run_metadata.selected,
        observed_selected.as_deref(),
        "at install execution",
    )?;

    let spec_format = resolve_spec_format(&manifest, spec_format_default);
    let progress = observer.progress();
    progress.checkpoint()?;

    // The SURFACE's own source-mutation epoch, at exactly the position it has
    // always occupied: after the manifest and the tree are consumed, before the
    // global registry config is loaded, so a refusal here still precedes every
    // registry read. M1.15's `--git`/`--tag`/`--branch`/`--rev` grammar and its
    // exit codes belong to the surface; what the core knows is that a surface
    // may record a declaration here and that a failure is the surface's own
    // error, unchanged.
    vibe_install::configuration_admission::workspace_inputs_for_install(
        &workspace,
        &project_root,
        &manifest,
    )?;
    let authored_before = manifest.clone();
    manifest_mutation.apply(&mut manifest, &mut workspace, &project_root)?;
    workspace.advance_authored_selected(&project_root, &authored_before, &manifest)?;
    vibe_install::configuration_admission::workspace_inputs_for_install(
        &workspace,
        &project_root,
        &manifest,
    )?;

    // ---- the registry epoch: SEED, then LOAD, exactly once ---------------
    //
    // One call, one snapshot, and the surface owes the order. The core used to
    // load `GlobalRegistryConfig` right here and ask for the embedded root far
    // below, past the empty-world fast path — but on a source install it is the
    // surface's own embedded lookup that SEEDS the machine-global defaults this
    // load reads, so on a fresh machine the first run failed and the second
    // succeeded. The order is no longer expressible here, because neither half
    // is performed here.
    //
    // It runs on the empty-world path too, exactly as the global load always
    // did; what stays skipped there is the package SOURCE, which is the
    // expensive half (it inspects registry state and credentials).
    let environment = environment.prepare()?;
    let lockfile_path = workspace.root.join(Lockfile::FILENAME);
    // Only recognized legacy schemas1–6 may regenerate. Rich/current/future
    // lock state must never be replaced with an empty world on read failure.
    let (lockfile_snapshot, legacy_schema) = if lockfile_path.exists() {
        match Lockfile::read(&lockfile_path) {
            Ok(lock) => (lock, None),
            Err(vibe_core::Error::UnsupportedLockfile {
                found: found @ 1..=6,
                ..
            }) => (
                Lockfile::empty(generated_by(), vibe_core::timestamp::now_utc()),
                Some(found),
            ),
            Err(other) => return Err(other.into()),
        }
    } else {
        (
            Lockfile::empty(generated_by(), vibe_core::timestamp::now_utc()),
            None,
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
        && (lockfile_snapshot.meta.schema_version < vibe_core::manifest::CURRENT_SCHEMA_VERSION
            || lockfile_snapshot.packages.is_empty())
    {
        progress.checkpoint()?;
        observer.narrate(InstallNarration::EmptyWorld);
        progress.cancellation().publishing();
        if manifest != authored_before {
            vibe_install::publish_admitted_manifest(&project_root, &authored_before, &manifest)?;
        }
        if let Some(schema) = legacy_schema {
            vibe_install::publish_empty_legacy_lock(&workspace.root, schema, &lockfile_snapshot)?;
        }
        let boot_task = progress.task("Generating boot artifacts for the empty world");
        let (nodes, native) = regenerate_native_world(
            &project_root,
            &workspace,
            &[],
            spec_format,
            NativeRunScope {
                trace,
                metadata: &run_metadata,
                lease: &lease,
                progress: &progress,
            },
        )
        .context("regenerating boot artifacts for the empty world")?;
        progress.cancellation().committed();
        progress.checkpoint()?;
        boot_task.set_progress(nodes.len() as u64, Some(nodes.len() as u64), "nodes");
        boot_task.finish();
        lifecycle_run.native = Some(native);
        let after = after_durable_world
            .take()
            .context("internal: install durable-world callback already consumed")?;
        let world = after.after(&project_root, lifecycle_run, &workspace)?;
        return Ok(fresh_run(&project_root, nodes, world));
    }

    // PROP-050 ##VERIFY-LOCK-DIFF — the lane-size half of the pre-apply
    // snapshot, taken beside the lock snapshot so one read point feeds
    // the whole diff. Sampled again after a successful apply below.
    let lanes_before = observer.lane_sizes(&workspace.root);
    let request = InstallRequest {
        roots: Vec::new(),
        features: FeatureRequest {
            explicit: args.features.clone(),
            no_defaults: args.no_default_features,
            all: args.all_features,
        },
        language: args.language.clone(),
        exact: args.exact,
        generated_by: generated_by(),
        offline,
    };

    // Source-free Fresh admission precedes expensive/possibly unavailable
    // registry construction. Both results enter the SAME Fresh/Ready owner below.
    let source_free_fresh = args.packages.is_empty()
        && vibe_install::freshness_prepared(
            &project_root,
            &manifest,
            &workspace,
            &request,
            &lockfile_snapshot,
            spec_format,
        )?
        .is_fresh();
    let (plan, resolver) = if source_free_fresh {
        (Plan::Fresh, None)
    } else {
        // The source is built from the ONE environment snapshot, and only here: a
        // chain that never reaches this point never pays for the composition.
        let resolver = sources.build(PackageSourceBuild {
            progress: &progress,
            manifest: &manifest,
            embedded_root: environment.embedded_root.as_deref(),
            project_root: &project_root,
            workspace_root: &workspace.root,
            global: &environment.global,
            offline,
            locked: &lockfile_snapshot.packages,
        })?;

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
            .map(|r| resolver.qualify(r, &lockfile_snapshot))
            .collect::<Result<_>>()?;

        // The PREPARED planner, over the two values this command already owns.
        // The compatibility wrapper would re-read the selected `vibe.toml` — the
        // value a `--git` delta just changed in memory — and re-discover a tree
        // different from the one whose source delta is being admitted.
        let plan = vibe_install::plan_prepared_without_publication(
            resolver.as_ref(),
            &project_root,
            &mut manifest,
            &mut workspace,
            InstallRequest {
                roots: cli_roots,
                ..request
            },
            spec_format,
            observer.plan_events(),
        )?;
        (plan, Some(resolver))
    };
    progress.checkpoint()?;
    match plan {
        Plan::Fresh => {
            // PROP-011 §2.2 — application is just a whole-tree boot
            // regeneration (cheap, self-healing — §2.4).
            progress.checkpoint()?;
            observer.narrate(InstallNarration::FreshLock);
            progress.cancellation().publishing();
            let boot_task = progress.task("Regenerating boot artifacts from the fresh lock");
            // The one workspace snapshot this command owns. Nothing between
            // its read and here mutated the tree — the fresh fast path
            // resolves nothing and copies nothing — so a re-read could only
            // ever differ by racing another process, which is exactly the
            // difference a single snapshot exists to refuse.
            let resolution = provisional_world(&workspace, &lockfile_snapshot, &[])
                .context("projecting the exact fresh install world")?;
            vibe_install::configuration_admission::resolved_packages(&resolution)?;
            if manifest != authored_before {
                vibe_install::publish_admitted_manifest(
                    &project_root,
                    &authored_before,
                    &manifest,
                )?;
            }
            let (nodes, native) = regenerate_native_world(
                &project_root,
                &workspace,
                &resolution,
                spec_format,
                NativeRunScope {
                    trace,
                    metadata: &run_metadata,
                    lease: &lease,
                    progress: &progress,
                },
            )
            .context("regenerating boot artifacts from the materialised state")?;
            progress.cancellation().committed();
            progress.checkpoint()?;
            boot_task.set_progress(nodes.len() as u64, Some(nodes.len() as u64), "nodes");
            boot_task.finish();
            lifecycle_run.native = Some(native);
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
                observer,
                &agent,
                resume::ResumeRequest {
                    project_root: &project_root,
                    workspace: &workspace,
                    metadata: &run_metadata,
                    lease: &lease,
                    disposition: InstallDisposition::Fresh,
                    progress: vibe_install::InstallProgress::fresh(nodes.clone()),
                    // The fresh fast path skips resolution entirely.
                    packages_resolved: 0,
                },
            )? {
                // A satisfied resume still owes the post-durability world: the
                // hosting output arrived, so an authored `phase:install` row
                // must run, in the run the resume just finished.
                ResumeOutcome::Completed(mut resumed) => {
                    resumed.context.native = lifecycle_run.native.take();
                    return resume::finish_resumed(*resumed, &project_root, &workspace, after);
                }
                // TRANSPORTED, not reduced. The family is not knowable here —
                // this same function is `vibe install`'s body, a phase verb's
                // prerequisite and `vibe update --all`'s delegate — so the
                // measurement travels outward neutrally and the one outer
                // command that owns the report consumes it.
                ResumeOutcome::Failed(failure) => {
                    return Err(crate::failure::carry(failure));
                }
                ResumeOutcome::Nothing => {}
            }
            let world = after.after(&project_root, lifecycle_run, &workspace)?;
            Ok(fresh_run(&project_root, nodes, world))
        }
        // The ready apply is its own cell — the confirmation, the traced
        // apply, the slot-failure carrier and the closure diff — so this
        // function stays the shape of the DECISION (empty / fresh / ready)
        // rather than of the largest branch.
        Plan::Ready(planned) => {
            let resolver = resolver
                .as_ref()
                .context("a Ready install has no prepared source")?;
            if manifest != authored_before {
                vibe_install::publish_admitted_manifest(
                    &project_root,
                    &authored_before,
                    &manifest,
                )?;
            }
            ready::apply(
                ready::ReadyApply {
                    project_root: &project_root,
                    workspace: &workspace,
                    resolver: resolver.as_ref(),
                    planned: *planned,
                    slot_integrity,
                    spec_format,
                    lockfile_path: &lockfile_path,
                    lockfile_snapshot: &lockfile_snapshot,
                    lanes_before: &lanes_before,
                    run_metadata: &run_metadata,
                    confirm_gate,
                    observer,
                    agent: &agent,
                    trace,
                },
                lifecycle_run,
                after_durable_world
                    .take()
                    .context("internal: install durable-world callback already consumed")?,
            )
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
