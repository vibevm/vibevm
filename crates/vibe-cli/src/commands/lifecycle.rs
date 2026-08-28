//! CLI adapter for the canonical lifecycle engine.
//!
//! Bootstrap validate/install establish a durable world once. The adapter then
//! reloads that world, narrates the complete extension ritual, and dispatches
//! contributions phase by phase. Clean is a separate pre-wipe epoch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase, RunMetadata};

use crate::cli::{InstallArgs, LifecycleArgs};
use vibe_orchestrator::ports::RunObserver;

use crate::commands::compile_trace;
use crate::output;

use super::install::{
    CliInstallObserver, CliPackageSourceFactory, CliRegistryEnvironment, PreparedSelection,
};

mod agent;
mod callback;
mod clean;
mod draft;
mod observer;
mod plan;
mod report;
mod slot;

pub(crate) use callback::DirectInstallWorld;
use clean::refuse_untracked_agent_rows;
pub use clean::run_clean;
pub(crate) use clean::run_clean_only;
pub(crate) use draft::render_lifecycle;
pub(crate) use observer::CliRunObserver;
pub(crate) use report::render_agent_task_fence;
pub(crate) use slot::{
    emit_transition_outcome as emit_slot_transition_outcome, surface_plan as surface_slot_plan,
};
pub(crate) use vibe_orchestrator::values::{StepStatus, check_delegation, step_report};

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

/// The ONE agent backend a command injects, built from values it ALREADY holds
/// — the STORED snapshot before it has been consumed, and a root the caller
/// carried. An unreadable manifest carries no `[llm]`, and the stored parse
/// error is still owed to the boundary that proves the bundle.
///
/// `workspace_root` is the caller's own — `lease.root()`, or the bundle's loaded
/// root. It is deliberately not a path this function locates: the previous
/// shape called `lease_root(project_root)`, which ran a whole extra
/// `Workspace::discover` and, when that discovery failed, silently swallowed
/// the error and fell back to the selected root. So a command that had already
/// leased a workspace root could hand its agent a DIFFERENT root, and nothing
/// said so. The snapshot contributes exactly one thing — a clone of `[llm]` —
/// and nothing else about it crosses into the backend.
pub(crate) fn install_agent_backend_from(
    workspace_root: &Path,
    llm: Option<&vibe_core::manifest::Manifest>,
) -> agent::CliAgentBackend {
    agent::CliAgentBackend::new(
        workspace_root.to_path_buf(),
        llm.and_then(|parsed| parsed.llm.clone()),
    )
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
    // The PRE-WIPE epoch owns its own snapshot: its own manifest read, its own
    // tree built from that read, and its own backend derived from it. It is
    // deliberately NOT the command snapshot taken below — that one describes
    // the tree AFTER the wipe, and reusing it here would plan the clean point
    // against a world this epoch has not seen.
    // It is prepared over the root this command ALREADY resolved and leased —
    // never over the raw `--path` again, so the tree it wipes is the tree the
    // lease pins.
    let clean_epoch = (steps.first() == Some(&LifecycleStep::Clean))
        .then(|| clean::prepare_epoch(&project_root, &lease))
        .transpose()?;
    let clean_plan = clean_epoch.as_ref().map(|epoch| &epoch.plan);
    if let Some(clean_plan) = clean_plan {
        refuse_untracked_agent_rows(ctx, clean_plan)?;
    }
    // The command's ONE selected-manifest snapshot: it answers compile-trace
    // activation here, and the prerequisite install consumes it at the
    // boundary that has always read `vibe.toml`. Two reads would be two
    // answers, and the second would race the first.
    // The command's ONE selected-world bundle, bound to the root this command
    // already resolved and leased: the snapshot answers compile-trace activation
    // here, and the prerequisite install proves the pair at the boundary that
    // has always read `vibe.toml`. Two reads would be two answers, and the
    // second would race the first.
    let selection = super::install::SelectedManifest::read(&project_root).prepare();
    // The ONE agent backend this command injects. Built from the bundle the
    // command already owns and the root the lease already pinned — no locator
    // call — it serves the prerequisite install's slot barrier, every resume,
    // and the phase dispatch: one seam, one configuration, one answer.
    let agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend> = std::sync::Arc::new(
        install_agent_backend_from(lease.root(), selection.parsed_ref()),
    );
    let trace_request = selection.request(install_args.trace_compile);
    let prelude = run_prelude(
        ctx,
        selection,
        lease.clone(),
        &requested.to_string(),
        &chain,
        install_args.force,
        trace_request,
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
        mut selection,
        lease,
        selected,
    } = prelude;

    if let Some(epoch) = clean_epoch {
        let clean::CleanEpoch {
            plan: clean_plan,
            agent: clean_agent,
            selection: clean_selection,
        } = epoch;
        notices.extend(clean_plan.notices().to_vec());
        let observer = CliRunObserver::new(ctx);
        observer.observe_plan(&clean_plan, &metadata, true)?;
        // From the epoch's own carried root and tree: the wipe removes exactly
        // what the pre-wipe epoch planned over, with no second discovery
        // between the plan and the removal.
        let wipe_plan =
            super::clean::plan_wipe_prepared(clean_selection.root(), clean_selection.workspace())?;
        super::clean::confirm_wipe(ctx, &wipe_plan, assume_yes)?;
        contribution_reports.extend(vibe_orchestrator::dispatch_plan_untracked(
            &observer,
            &clean_plan,
            &lease,
            &clean_agent,
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
        // The ONE strict post-wipe reload, rebuilding at the SAME carried
        // root. A failure returns here — before any trace opens —
        // because a tree that will not load right after a clean is a real
        // fault; an invalid snapshot is unchanged by a wipe and keeps its own
        // stored error for the funnel.
        selection = selection.reload_after_clean()?;
        // The post-wipe reload is a SECOND workspace load under the SAME
        // lease, so it owes the same root agreement: a tree that rediscovered
        // under a different root would run the remaining phases against a
        // workspace this command never leased. The gate is the lease's one
        // `ensure_root` — which is also why this refusal can never again
        // carry the mangled spacing a hand-rolled string continuation did.
        if let Some(loaded) = selection.loaded_root() {
            lease.ensure_root(loaded, "after the clean epoch's rediscovery")?;
        }
        // …and the selected-node twin of that gate. `selected` was derived
        // ONCE, pre-wipe, and rides across this boundary (it is never
        // re-derived); the rediscovered topology must still map this root to
        // that node. This boundary precedes `begin` (no state header is
        // written yet), so what is at stake is the carried identity itself —
        // and, at any future or reused reload boundary that sits after
        // writes, state/outbox bytes minted under it.
        if let Some(loaded) = selection.loaded_workspace() {
            let observed = loaded
                .node_rel_of(selection.root())
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
        selection,
        lease,
        selected,
    };
    // The boundary's OWNER share. The executed region below consumes its own
    // clones into the run and the callback; this one lives to the end of the
    // command — through the final report and the trace finalisation — so the
    // workspace stays owned until the last byte this invocation owes is
    // written.
    let lease_owner = prelude.lease.clone();
    // The same root the execution used, cloned rather than re-resolved: two
    // canonicalisations are two answers to "which node did this command act on".
    let failed_root = prelude.selection.root().to_path_buf();
    let preparation = prelude.prepare_trace(&now);
    let phases = steps
        .iter()
        .filter_map(|step| match step {
            LifecycleStep::Default(phase) => Some(*phase),
            LifecycleStep::Clean => None,
        })
        .collect::<Vec<_>>();
    let observer = CliRunObserver::new(ctx);
    let install_observer = CliInstallObserver::new(&child, Some(ctx));
    let confirm_gate = super::install::CliConfirmGate::new(&child, install_args.assume_yes);
    let sources = CliPackageSourceFactory {
        args: &install_args,
    };
    let environment = CliRegistryEnvironment::new(prepare_install);
    let exit = execute_after_open(
        &failed_root,
        vibe_orchestrator::PhaseRun {
            requested,
            phases,
            chain,
            metadata,
            install_args: install_args.inputs(),
            policy: install_args.policy(root_offline, &user_config),
            lease: prelude.lease,
            selection: prelude.selection,
            steps: reports,
            contributions: contribution_reports,
            notices,
            observer: &observer,
            install_observer: &install_observer,
            confirm_gate: &confirm_gate,
            sources: &sources,
            environment: &environment,
            // A phase verb admits no manifest-mutating flag: `--git` and its
            // siblings live only on the explicit `vibe install` verb.
            manifest_mutation: &super::install::NoManifestMutation,
            agent,
            trace: preparation.recorder(),
        },
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

/// Classify the shared phase run into this command's registered report family.
///
/// The shared service names no family — the same core is also `vibe install`'s
/// body and `vibe update --all`'s delegate — so it hands back the measurement,
/// the caller's exact error object and the emission bit its own site froze, and
/// THIS boundary chooses `cli-lifecycle-report`. Nothing is reformatted: the
/// error is moved into the carrier the trace funnel already unwraps.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn execute_after_open(
    project_root: &Path,
    inputs: vibe_orchestrator::PhaseRun<'_>,
) -> compile_trace::CommandExit<compile_trace::RegisteredReportDraft> {
    match vibe_orchestrator::run_phases(inputs) {
        vibe_orchestrator::PhaseOutcome::Completed(values) => compile_trace::CommandExit::Success(
            compile_trace::RegisteredReportDraft::Lifecycle(Box::new(values)),
        ),
        vibe_orchestrator::PhaseOutcome::Parked(values) => compile_trace::CommandExit::Parked(
            compile_trace::RegisteredReportDraft::Lifecycle(Box::new(values)),
        ),
        vibe_orchestrator::PhaseOutcome::Failed {
            measurement,
            original,
            emit_machine_failure,
        } => compile_trace::CommandExit::Failed {
            report: registered_family(project_root, measurement),
            original_error: original,
            emit_when_trace_disabled: emit_machine_failure,
        },
    }
}

/// Project a measurement into the registered family its measuring site chose.
///
/// The substrate's own barrier failure is install-shaped and stays so, even
/// inside a phase verb: a prerequisite install's slot failure has always
/// emitted a `cli-install-report` and no lifecycle root beside it, and a
/// hosting agent parses exactly that. Everything else is this command's own
/// lifecycle family.
pub(crate) fn registered_family(
    project_root: &Path,
    measurement: vibe_orchestrator::failure::Measurement,
) -> compile_trace::RegisteredReportDraft {
    match measurement {
        vibe_orchestrator::failure::Measurement::InstallBarrier {
            progress, reports, ..
        } => compile_trace::RegisteredReportDraft::Install(Box::new(
            super::install::InstallDraft::failed(project_root, *progress, reports),
        )),
        other => lifecycle_family(other),
    }
}

/// Project ANY measurement into this command's own registered family.
///
/// A slot measurement reaches here when the prerequisite install failed inside
/// a phase verb; its rows become lifecycle rows exactly as they always did,
/// because this command's document is the one a hosting agent parses.
pub(crate) fn lifecycle_family(
    measurement: vibe_orchestrator::failure::Measurement,
) -> compile_trace::RegisteredReportDraft {
    let values = match measurement {
        vibe_orchestrator::failure::Measurement::Lifecycle {
            rows,
            stopped_phase,
            requested,
            chain,
        } => vibe_orchestrator::values::LifecycleValues::failed(
            &requested,
            chain,
            &stopped_phase,
            rows,
        ),
        vibe_orchestrator::failure::Measurement::Slot { reports, .. }
        | vibe_orchestrator::failure::Measurement::InstallBarrier { reports, .. } => {
            vibe_orchestrator::values::LifecycleValues::failed(
                Phase::Install.as_str(),
                Vec::new(),
                Phase::Install.as_str(),
                reports
                    .into_iter()
                    .map(vibe_orchestrator::values::contribution_report)
                    .collect(),
            )
        }
    };
    compile_trace::RegisteredReportDraft::Lifecycle(Box::new(values))
}

/// The prelude epoch, whose OWN inherent `prepare_trace` opens this command's
/// trace owner.
///
/// The join it performs — "a loaded tree names the one canonical trace home,
/// and nothing else does" — used to be a surface trait here. It is not a
/// surface fact: both halves are the epoch's (`selection.loaded_root()`) and
/// the funnel's, and a second copy of the pairing is a second answer to which
/// root a member's install may lock. The surface still injects the clock.
pub(crate) use vibe_orchestrator::RunPrelude;

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
    selection: PreparedSelection,
    lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    requested: &str,
    chain: &[String],
    force: bool,
    compile_trace: bool,
) -> Result<RunPrelude> {
    // The whole epoch — the post-acquisition root law, the selected-node
    // derivation and the one identity selection — is the shared service's.
    // This surface contributes exactly one fact it owns: its resolved agent
    // mode.
    vibe_orchestrator::run_prelude(
        selection,
        lease,
        requested,
        chain,
        ctx.agent_mode(),
        force,
        compile_trace,
    )
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
