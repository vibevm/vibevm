//! CLI adapter for the canonical lifecycle engine.
//!
//! Bootstrap validate/install establish a durable world once. The adapter then
//! reloads that world, narrates the complete extension ritual, and dispatches
//! contributions phase by phase. Clean is a separate pre-wipe epoch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase, RunMetadata};
use vibe_wire::generated::lifecycle_report::LifecycleStepReport;

use crate::cli::{InstallArgs, LifecycleArgs};
use crate::commands::compile_trace;
use crate::output;

use super::install::{InstallDisposition, PreparedWorkspace};

mod agent;
mod callback;
mod clean;
mod dispatch;
mod draft;
mod phase;
mod plan;
mod report;
mod slot;
pub(crate) mod world;

pub(crate) use callback::after_direct_install;
use clean::refuse_untracked_agent_rows;
pub use clean::run_clean;
pub(crate) use clean::run_clean_only;
pub(crate) use draft::LifecycleDraft;
use plan::{provider_and_version, surface_plan, tier_name};
pub(crate) use report::{check_delegation, render_agent_task_fence};
pub(crate) use slot::{
    emit_transition_outcome as emit_slot_transition_outcome, surface_plan as surface_slot_plan,
};

/// The agent backend the install barrier injects.
///
/// Built from values the caller ALREADY has: the workspace root it discovered
/// and the selected node's `[llm]` table from the manifest that discovery
/// produced. It reads nothing — no credential, no endpoint, and no provider
/// construction happens until an actual agent execution runs.
///
/// It used to discover the workspace and re-read the manifest itself, which
/// made it a second (and third) read of a tree the install is mutating: a
/// backend built from a different snapshot than the run it serves.
pub(crate) fn install_agent_backend(
    workspace_root: &Path,
    manifest: &vibe_core::manifest::Manifest,
) -> agent::CliAgentBackend {
    agent::CliAgentBackend::new(workspace_root.to_path_buf(), manifest.llm.clone())
}

/// Execute a top-level default-lifecycle phase verb.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
pub fn run(
    ctx: &output::Context,
    requested: Phase,
    args: LifecycleArgs,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    execute(
        ctx,
        LifecycleRequest::Default(requested),
        requested,
        args.install_args(),
        false,
        prepare_install,
        root_offline,
    )
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn execute(
    ctx: &output::Context,
    request: LifecycleRequest,
    requested: Phase,
    mut install_args: InstallArgs,
    clean_assume_yes: bool,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let child = ctx.quiet_child();
    let steps = request.steps();
    let chain = steps.iter().map(step_name).collect::<Vec<_>>();
    // The OUTERMOST lock. The read-only locator epoch resolves the selected
    // node and discovers the workspace root; the lease pins that root; and
    // everything execution consumes below — config, manifest, workspace,
    // state, run id — is loaded AFTER the acquisition, so no pre-lease
    // snapshot can go stale under a concurrent mutator this lease just
    // refused. A contended workspace refuses here, typed, before any run id,
    // state row or outbox byte exists.
    let project_root = super::install::resolve_project_root(&install_args.path)?;
    let lease = super::install::acquire_lease(&project_root)?;
    // The ONE user-config load of this command. It decides the offline
    // posture below AND rides into the prerequisite install, which used to
    // load its own — two loads of a file an operator can edit mid-run being
    // two answers to the same question.
    let user_config = UserConfig::load().context("loading user config for lifecycle envelope")?;
    let offline = output::resolve_offline(
        root_offline || install_args.offline,
        user_config.net.offline,
    );
    let assume_yes =
        clean_assume_yes || install_args.assume_yes || ctx.is_unattended() || ctx.is_json();
    // The durable identity is chosen ONCE, here, after the project/state root
    // is known and before anything is allocated: a resume of a parked run
    // adopts its id and original start, every other invocation gets one fresh
    // allocation. The resulting metadata then travels unchanged through the
    // prerequisite install and every later phase.
    // A clean-composed invocation is planned and judged BEFORE the identity is
    // selected: `select_run_identity` allocates a scratch run directory on the
    // fresh branch, and a hosted clean this build cannot serve must not leave
    // one behind.
    let clean_plan = (steps.first() == Some(&LifecycleStep::Clean))
        .then(|| world::plan_clean(&install_args.path))
        .transpose()?;
    if let Some(clean_plan) = clean_plan.as_ref() {
        refuse_untracked_agent_rows(ctx, clean_plan)?;
    }
    // The command's ONE selected-manifest snapshot: it answers compile-trace
    // activation here, and the prerequisite install consumes it at the
    // boundary that has always read `vibe.toml`. Two reads would be two
    // answers, and the second would race the first.
    let manifest = super::install::SelectedManifest::read(&project_root);
    // Built FROM that snapshot — one read, one tree. `None` when the snapshot
    // itself did not parse: there is no sound workspace to build then, and the
    // stored error speaks at its own boundary inside the executed region.
    let workspace = manifest.prepare_workspace(&project_root);
    let prelude = run_prelude(
        ctx,
        project_root,
        workspace,
        lease.clone(),
        &requested.to_string(),
        &chain,
        install_args.force,
        manifest.request(install_args.trace_compile),
    )?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain: chain.clone(),
        offline,
        assume_yes,
        agent_mode: ctx.agent_mode(),
        force: install_args.force,
        // The EFFECTIVE bit the one selector computed — never the raw
        // request. For an ADOPTED run it is the parked run's own sticky
        // bit, and carrying it here is what stops
        // `LifecycleStateStore::begin` from rewriting a traced run's
        // header back to untraced on its own resume.
        trace_compile: prelude.identity.compile_trace,
        run_id: prelude.identity.run_id.clone(),
        started: prelude.identity.started.clone(),
        selected: prelude.selected.clone(),
    };
    let mut reports = Vec::with_capacity(steps.len());
    let mut contribution_reports = Vec::new();
    let mut notices = Vec::new();
    let RunPrelude {
        identity,
        mut project_root,
        mut workspace,
        lease,
        selected,
    } = prelude;

    if let Some(clean_plan) = clean_plan {
        notices.extend(clean_plan.notices.clone());
        surface_plan(ctx, &clean_plan, &metadata, true)?;
        let wipe_plan = super::clean::plan_wipe(&install_args.path)?;
        super::clean::confirm_wipe(ctx, &wipe_plan, assume_yes)?;
        contribution_reports.extend(dispatch::dispatch_plan_untracked(
            ctx,
            &clean_plan,
            &lease,
            metadata.clone(),
        )?);
        let root = super::clean::apply_wipe(&child, wipe_plan)?;
        install_args.path = root;
        reports.push(step_report("clean", StepStatus::Ok));
        // The identity was chosen before the wipe, as it always was — but the
        // SESSION may only open now: a recorder opened first would have had
        // its own lock file and index deleted underneath it by the very clean
        // it belongs to. Every failure up to this line is therefore untraced,
        // which is exactly right — nothing had compiled yet.
        //
        // ONE strict rediscovery, from the same snapshot, and a failure here is
        // this command's error. It is deliberately not `.ok()`: the wipe just
        // rewrote the tree, so a workspace that will not load afterwards is a
        // real fault, and quietly continuing would install into a world nobody
        // could describe.
        project_root = super::install::resolve_project_root(&install_args.path)?;
        workspace = match manifest.parsed_ref() {
            // The ONE strict post-wipe load, from the SAME snapshot. A failure
            // returns here — before any trace opens — because a tree that will
            // not load right after a clean is a real fault.
            Some(_) => PreparedWorkspace::Loaded(Box::new(
                manifest
                    .rediscover(&project_root)
                    .context("re-reading the workspace after the clean epoch")?,
            )),
            // An invalid snapshot is unchanged by a wipe, and its stored error
            // is still owed to the command funnel — replacing it with a
            // generic rediscovery error would report the wrong thing.
            None => PreparedWorkspace::SelectedManifestInvalid,
        };
        // The post-wipe reload is a SECOND workspace load under the SAME
        // lease, so it owes the same root agreement: a tree that rediscovered
        // under a different root would run the remaining phases against a
        // workspace this command never leased. The gate is the lease's one
        // `ensure_root` — which is also why this refusal can never again
        // carry the mangled spacing a hand-rolled string continuation did.
        if let Some(loaded) = workspace.loaded_root() {
            lease.ensure_root(loaded, "after the clean epoch's rediscovery")?;
        }
        // …and the selected-node twin of that gate. `selected` was derived
        // ONCE, pre-wipe, and rides across this boundary (it is never
        // re-derived); the rediscovered topology must still map this root to
        // that node. This boundary precedes `begin` (no state header is
        // written yet), so what is at stake is the carried identity itself —
        // and, at any future or reused reload boundary that sits after
        // writes, state/outbox bytes minted under it.
        if let Some(loaded) = workspace.loaded_workspace() {
            let observed = loaded
                .node_rel_of(&project_root)
                .map(|rel| rel.as_str().to_string());
            lease.ensure_selected(
                &metadata.selected,
                observed.as_deref(),
                "after the clean epoch's rediscovery",
            )?;
        }
    }

    let prelude = RunPrelude {
        identity,
        project_root,
        workspace,
        lease,
        selected,
    };
    // The boundary's OWNER share. The executed region below consumes its own
    // clones into the run and the callback; this one lives to the end of the
    // command — through the final report and the trace finalisation — so the
    // workspace stays owned until the last byte this invocation owes is
    // written.
    let lease_owner = prelude.lease.clone();
    let preparation = prelude.prepare_trace(&now);
    let phases = steps
        .iter()
        .filter_map(|step| match step {
            LifecycleStep::Default(phase) => Some(*phase),
            LifecycleStep::Clean => None,
        })
        .collect::<Vec<_>>();
    let exit = phase::execute_after_open(
        ctx,
        &child,
        phase::PhaseInputs {
            requested,
            phases,
            chain,
            metadata,
            install_args,
            root_offline,
            lease: prelude.lease,
            project_root: prelude.project_root,
            user_config,
            manifest,
            workspace: prelude.workspace,
            steps: reports,
            contributions: contribution_reports,
            notices,
        },
        prepare_install,
        preparation.recorder(),
    );
    // Consumes the owner: finishes the index against the real outcome, drops
    // the last handle (and with it the cooperative lock), and returns the
    // member the one root attaches.
    let finalized = compile_trace::finalize(preparation, exit, &now);
    let rendered = compile_trace::render_finalized(ctx, finalized);
    drop(lease_owner);
    rendered
}

/// The injected instant. Both the supersession pass and the finish read time
/// through this one closure — there is no clock inside the owner, and no
/// `Drop` that invents a timestamp.
fn now() -> vibe_wire::generated::shared::Timestamp {
    chrono::Utc::now()
}

/// This invocation's durable run identity, plus the ONE root its trace may be
/// stored under.
///
/// The two answers come from the same discovery and must not be confused:
///
/// * **identity/state** may fall back to the selected project root. That
///   fallback is old, load-bearing and compatible — a project outside any
///   discoverable workspace still gets a run id and a `.vibe/lifecycle.toml`.
/// * **trace storage** may NOT. A trace's lock and index belong to the
///   canonical workspace root, because one install regenerates shared package
///   units plus every node — so an invocation entered through a member that
///   silently traced into the member's own directory would let two members
///   hold independent locks over the same work. When discovery genuinely
///   fails there is no canonical root to name, so no trace opens at all and
///   the command's own validation error stays authoritative.
///
/// Hence `workspace` is the typed [`PreparedWorkspace`] state rather than an
/// `Option`: `Loaded` alone names a trace home, and the two unavailable arms
/// say WHICH way the one attempt failed so that nothing downstream retries it.
/// The whole workspace VALUE is retained, not just its root — the command is
/// about to validate against it, install through it and lock a trace to it,
/// and re-reading it for each of those would be three snapshots of a tree the
/// command is itself changing.
///
/// The canonical `project_root` is carried for the same reason: it is selected
/// once per prelude epoch, and nothing downstream re-resolves it.
pub(crate) struct RunPrelude {
    pub(crate) identity: vibe_lifecycle::RunIdentity,
    pub(crate) project_root: PathBuf,
    pub(crate) workspace: PreparedWorkspace,
    /// The workspace mutation lease this command holds — see
    /// [`run_prelude`], which both receives and returns it.
    pub(crate) lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The canonical workspace-relative identity of the selected node this
    /// command runs from — derived ONCE in [`run_prelude`] from the prepared
    /// workspace (never re-derived, not even across the post-clean reload,
    /// which instead PROVES it still holds). It is selector input (which
    /// park this invocation may adopt) and the value the metadata carries
    /// into the state header, deliberately NOT a member of `RunIdentity`:
    /// the selector decides identity, it does not echo its inputs.
    pub(crate) selected: String,
}

impl RunPrelude {
    /// Open the owner against the canonical trace home — or stand down
    /// honestly, without a lock and without a tree.
    pub(crate) fn prepare_trace(
        &self,
        clock: &dyn Fn() -> vibe_wire::generated::shared::Timestamp,
    ) -> compile_trace::TracePreparation {
        match self.workspace.loaded_root() {
            Some(root) => compile_trace::prepare(root, &self.identity, &clock),
            None => compile_trace::without_workspace(&self.identity),
        }
    }
}

/// Choose this invocation's durable run identity through the one selector,
/// before anything is allocated.
///
/// It RESOLVES and DISCOVERS nothing: the caller has already canonicalised the
/// project root once and already built (or failed to build) the workspace from
/// its own manifest snapshot. A second resolution here would be a second
/// answer to "which node is this", and a second discovery a second answer to
/// "what does its tree look like".
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_prelude(
    ctx: &output::Context,
    project_root: PathBuf,
    workspace: PreparedWorkspace,
    lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    requested: &str,
    chain: &[String],
    force: bool,
    compile_trace: bool,
) -> Result<RunPrelude> {
    // The post-acquisition root law: a workspace loaded under a DIFFERENT
    // root than the one this command leased would read and write state
    // beside another process's lock, on a pre-lease snapshot this lease
    // never authorised. The one gate — and the one refusal spelling — is
    // the lease's own `ensure_root`; the locator's discovery-failed fallback
    // is the one exception (the lease already pins the selected root under
    // the same fallback law identity selection has always applied, and the
    // execution boundary then surfaces the stored discovery error itself).
    if let Some(loaded) = workspace.loaded_root() {
        lease.ensure_root(loaded, "at the run prelude")?;
    }
    // The selected-node identity, derived from the ONE prepared snapshot: a
    // `Loaded` tree maps the canonical selected root through the workspace's
    // own authored rels — and a Loaded tree that cannot map it is an
    // internal refusal, never a fallback guess. Every unavailable arm falls
    // back to `"."` under the same fallback law the state root itself
    // applies there: when discovery failed, the selected node IS the root.
    let selected = match workspace.loaded_workspace() {
        Some(workspace) => workspace
            .node_rel_of(&project_root)
            .with_context(|| {
                format!(
                    "internal: the canonical selected root `{}` is not a node of the \
                     workspace loaded for this run",
                    project_root.display()
                )
            })?
            .as_str()
            .to_string(),
        None => ".".to_string(),
    };
    // The identity borrows the lease BEFORE its state read, so the prior
    // state it decides adoption against is a post-acquisition snapshot.
    let identity = vibe_lifecycle::select_run_identity(
        &lease,
        &project_root,
        requested,
        chain,
        &selected,
        ctx.agent_mode(),
        force,
        compile_trace,
        crate::commands::init::current_timestamp_utc(),
    )?;
    Ok(RunPrelude {
        identity,
        project_root,
        workspace,
        lease,
        selected,
    })
}

// The identity-only selector that `vibe update` and `vibe reinstall` used to
// call is gone. It existed because those two commands owned no trace session:
// they read the manifest themselves, loaded what could be loaded, and requested
// nothing — a shape that could not carry an effective trace bit and, worse,
// could run a SECOND time inside the same invocation (update selected once for
// its metadata and again for its slot lifecycle; reinstall selected a third
// time inside its continuation helper). Both now own a prelude epoch and call
// `run_prelude` exactly once, like every other command in the binary.

fn step_name(step: &LifecycleStep) -> String {
    match step {
        LifecycleStep::Clean => "clean".to_string(),
        LifecycleStep::Default(phase) => phase.to_string(),
    }
}

fn step_report(phase: &str, status: StepStatus) -> LifecycleStepReport {
    LifecycleStepReport {
        phase: phase.to_string(),
        status: status.as_str().to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Ok,
    Fresh,
    NoOp,
    /// The chain parked at this phase for the hosting agent; no later phase
    /// ran, and none is reported.
    Delegated,
}

impl StepStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fresh => "fresh",
            Self::NoOp => "no-op",
            Self::Delegated => "delegated",
        }
    }
}
