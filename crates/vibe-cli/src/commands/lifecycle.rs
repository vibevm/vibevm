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
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_report::LifecycleStepReport;
use vibe_workspace::Workspace;

use crate::cli::{CleanArgs, CleanChain, InstallArgs, LifecycleArgs};
use crate::output;

use super::install::{InstallDisposition, InstallRunContext, WorldCallbackSummary};

mod agent;
mod dispatch;
mod plan;
mod report;
mod slot;
pub(crate) mod world;

use plan::{provider_and_version, surface_plan, tier_name};
pub(super) use report::emit_failure_outcome;
use report::emit_report;
pub(crate) use report::{check_delegation, render_agent_task_fence};
pub(crate) use slot::{
    emit_transition_outcome as emit_slot_transition_outcome, surface_plan as surface_slot_plan,
};

/// The agent backend the install barrier injects. It reads the selected node's
/// manifest for project `[llm]` and nothing else — no credential, no endpoint,
/// no provider construction happens until an actual agent execution runs.
pub(crate) fn install_agent_backend(project_root: &Path) -> Result<agent::CliAgentBackend> {
    let workspace = Workspace::discover(project_root)
        .context("discovering the workspace for the install-time agent backend")?;
    let llm = vibe_core::manifest::Manifest::read(project_root.join("vibe.toml"))
        .ok()
        .and_then(|manifest| manifest.llm);
    Ok(agent::CliAgentBackend::new(workspace.root.clone(), llm))
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

/// Compose clean with any default-lifecycle phase through the same step list.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
pub fn run_clean(
    ctx: &output::Context,
    args: CleanArgs,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let CleanArgs {
        path,
        assume_yes,
        chain,
    } = args;
    let chain = chain.context("internal: chained clean lost its continuation")?;
    let (requested, mut install_args) = clean_continuation(chain);
    if path != Path::new(".") {
        install_args.path = path;
    }
    execute(
        ctx,
        LifecycleRequest::Clean {
            then: Some(requested),
        },
        requested,
        install_args,
        assume_yes,
        prepare_install,
        root_offline,
    )
}

/// Run the independent clean lifecycle: dispatch once, then terminal wipe.
pub(crate) fn run_clean_only(
    ctx: &output::Context,
    args: CleanArgs,
    root_offline: bool,
) -> Result<()> {
    let chain = vec!["clean".to_string()];
    let plan = world::plan_clean(&args.path)?;
    // The refusal comes FIRST — before a run id is minted, before the plan is
    // narrated, before the wipe is confirmed. Allocating `.vibe/lifecycle/<id>`
    // is itself a mutation, and an invocation this build cannot host must
    // leave the tree byte-identical.
    refuse_untracked_agent_rows(ctx, &plan)?;
    let metadata = RunMetadata {
        requested: "clean".to_string(),
        chain: chain.clone(),
        offline: effective_clean_offline(root_offline)?,
        assume_yes: metadata_assume_yes(ctx, args.assume_yes),
        agent_mode: ctx.agent_mode(),
        force: false,
        trace_compile: false,
        run_id: new_run_id(Path::new(&plan.project.root))?,
        started: crate::commands::init::current_timestamp_utc(),
    };
    let notices = plan.notices.clone();
    surface_plan(ctx, &plan, &metadata, true)?;
    let wipe_plan = super::clean::plan_wipe(&args.path)?;
    super::clean::confirm_wipe(ctx, &wipe_plan, metadata.assume_yes)?;
    let contributions = dispatch::dispatch_plan_untracked(ctx, &plan, metadata)?;
    let wipe_ctx = if ctx.is_json() || ctx.is_quiet() {
        ctx.quiet_child()
    } else {
        ctx.clone()
    };
    super::clean::apply_wipe(&wipe_ctx, wipe_plan)?;
    emit_report(
        ctx,
        "clean",
        chain,
        vec![step_report("clean", StepStatus::Ok)],
        contributions,
        notices,
        // The clean epoch is untracked, so it can never park: an agent row
        // here is refused above, before anything is wiped.
        None,
    )
}

/// The clean epoch runs UNTRACKED: it keeps no `.vibe/lifecycle.toml` record,
/// and its wipe destroys the very tree a parked task would have to live in.
/// There is therefore no honest place in R7.3 to park a pre-wipe `agent` row —
/// and paying the provider for one in resolved agent mode is exactly the
/// accident this refusal exists to prevent. So: refuse explicitly, before the
/// wipe confirmation, spending nothing and destroying nothing.
///
/// Remaining R7 debt, named rather than hidden: a safe pre-wipe park/resume
/// for `phase:clean` agent rows through a tracked seam (a clean-epoch state
/// home that survives its own wipe), tracked with R7.4's bounded outbox GC.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
fn refuse_untracked_agent_rows(ctx: &output::Context, plan: &world::RitualPlan) -> Result<()> {
    if ctx.agent_mode() != RunAgentMode::Agent {
        return Ok(());
    }
    let hosted: Vec<String> = plan
        .executions
        .iter()
        .filter(|execution| execution.row.declaration().handler.kind() == "agent")
        .map(|execution| execution.row.key().to_string())
        .collect();
    if hosted.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "the clean lifecycle cannot host agent contribution(s) {hosted:?} under \
         `--agent-mode agent`: the clean epoch is untracked and its wipe would destroy the \
         outbox a parked task lives in, so this invocation neither paid a provider nor removed \
         anything (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
         fix: run the clean lifecycle with `--agent-mode cli`, or disable the named \
         `phase:clean` agent contribution(s) for hosted runs)"
    )
}

fn clean_continuation(chain: CleanChain) -> (Phase, InstallArgs) {
    match chain {
        CleanChain::Validate(args) => (Phase::Validate, args.install_args()),
        CleanChain::Install(args) => (Phase::Install, args),
        CleanChain::Generate(args) => (Phase::Generate, args.install_args()),
        CleanChain::Build(args) => (Phase::Build, args.install_args()),
        CleanChain::Test(args) => (Phase::Test, args.install_args()),
        CleanChain::Create(args) => (Phase::Create, args.install_args()),
        CleanChain::Verify(args) => (Phase::Verify, args.install_args()),
        CleanChain::Package(args) => (Phase::Package, args.install_args()),
        CleanChain::Deploy(args) => (Phase::Deploy, args.install_args()),
    }
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
    let mut prepare_install = Some(prepare_install);
    let steps = request.steps();
    let chain = steps.iter().map(step_name).collect::<Vec<_>>();
    let offline = effective_offline(root_offline, &install_args)?;
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
    let identity = run_identity(
        ctx,
        &install_args.path,
        &requested.to_string(),
        &chain,
        install_args.force,
    )?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain: chain.clone(),
        offline,
        assume_yes,
        agent_mode: ctx.agent_mode(),
        force: install_args.force,
        // The EFFECTIVE bit the one selector computed — never a
        // hard-coded false. `current_request` is still absent (the
        // CLI/manifest flag is the next atom), so this is false today
        // for a fresh run; for an ADOPTED run it is the parked run's
        // own sticky bit, and carrying it here is what stops
        // `LifecycleStateStore::begin` from rewriting a traced run's
        // header back to untraced on its own resume.
        trace_compile: identity.compile_trace,
        run_id: identity.run_id,
        started: identity.started,
    };
    let mut reports = Vec::with_capacity(steps.len());
    let mut contribution_reports = Vec::new();
    let mut notices = Vec::new();
    let mut install_lifecycle_run = None;
    let mut install_contribution_reports = Vec::new();

    if let Some(clean_plan) = clean_plan {
        notices.extend(clean_plan.notices.clone());
        surface_plan(ctx, &clean_plan, &metadata, true)?;
        let wipe_plan = super::clean::plan_wipe(&install_args.path)?;
        super::clean::confirm_wipe(ctx, &wipe_plan, assume_yes)?;
        contribution_reports.extend(dispatch::dispatch_plan_untracked(
            ctx,
            &clean_plan,
            metadata.clone(),
        )?);
        let root = super::clean::apply_wipe(&child, wipe_plan)?;
        install_args.path = root;
        reports.push(step_report("clean", StepStatus::Ok));
    }

    let phases = steps
        .iter()
        .filter_map(|step| match step {
            LifecycleStep::Default(phase) => Some(*phase),
            LifecycleStep::Clean => None,
        })
        .collect::<Vec<_>>();
    let mut validate_status = None;
    let mut install_status = None;
    for phase in &phases {
        match phase {
            Phase::Validate => {
                install_args.path = validate(&install_args.path)?;
                validate_status = Some(StepStatus::Ok);
            }
            Phase::Install => {
                let prepare = prepare_install
                    .take()
                    .context("internal: install inputs prepared more than once")?;
                let install_run = super::install::run_with_lifecycle_context(
                    &child,
                    install_args.clone(),
                    prepare(),
                    root_offline,
                    Some(metadata.clone()),
                    Some(ctx),
                    |_, _, run| {
                        install_lifecycle_run = run.lifecycle_run;
                        install_contribution_reports = run
                            .lifecycle_reports
                            .into_iter()
                            .map(slot::contribution_report)
                            .collect();
                        Ok(super::install::WorldCallbackOutcome::default())
                    },
                )?;
                // A parked prerequisite install stops the whole chain — and
                // THIS command renders the one document, because it is the
                // outermost one. The step list is the prefix that really ran:
                // whatever preceded install, then `install: delegated`, and
                // nothing after it.
                if let Some(delegation) = install_run.parked {
                    let mut prefix = reports;
                    if validate_status.is_some() {
                        prefix.push(step_report(
                            Phase::Validate.as_str(),
                            validate_status.unwrap_or(StepStatus::Ok),
                        ));
                    }
                    prefix.push(step_report(Phase::Install.as_str(), StepStatus::Delegated));
                    let mut rows = install_contribution_reports;
                    rows.extend(
                        install_run
                            .slot_reports
                            .into_iter()
                            .map(slot::contribution_report),
                    );
                    return emit_report(
                        ctx,
                        requested.as_str(),
                        chain,
                        prefix,
                        rows,
                        notices,
                        Some(delegation),
                    );
                }
                install_status = Some(match install_run.disposition {
                    InstallDisposition::Fresh => StepStatus::Fresh,
                    InstallDisposition::Applied => StepStatus::Ok,
                    InstallDisposition::Parked => unreachable!("returned above"),
                });
            }
            _ => {}
        }
    }

    let ritual = world::plan_default(&install_args.path, &phases)?;
    notices.extend(ritual.notices.clone());
    surface_plan(ctx, &ritual, &metadata, true)?;
    let state_chain = phases.iter().map(ToString::to_string).collect();
    let outcome = if let Some(shared) = install_lifecycle_run {
        dispatch::dispatch_plan_with_run(ctx, &ritual, &shared, &metadata)?
    } else {
        dispatch::dispatch_plan(ctx, &ritual, metadata, state_chain)?
    };
    let parked = outcome.parked;
    contribution_reports.extend(outcome.reports);
    // The prerequisite install's slot rows belong to THIS report, in every
    // mode. They used to be excluded from JSON because each one was echoed as
    // its own `lifecycle` document; that echo is gone, so excluding them here
    // would drop the machine record of work this command really ran.
    if !install_contribution_reports.is_empty() {
        install_contribution_reports.append(&mut contribution_reports);
        contribution_reports = install_contribution_reports;
    }
    for phase in phases {
        let status = match phase {
            Phase::Validate => validate_status.unwrap_or(StepStatus::Ok),
            Phase::Install => install_status.unwrap_or(StepStatus::Ok),
            _ if ritual.count_for(phase) == 0 => StepStatus::NoOp,
            _ if contribution_reports
                .iter()
                .filter(|row| row.phase == phase.as_str())
                .all(|row| row.status == "fresh") =>
            {
                StepStatus::Fresh
            }
            _ => StepStatus::Ok,
        };
        // Steps end AT the parked phase: later phases did not run, so they
        // are not reported as if they had.
        let parked_here = parked
            .as_ref()
            .is_some_and(|(stopped, _)| stopped == phase.as_str());
        reports.push(step_report(
            phase.as_str(),
            if parked_here {
                StepStatus::Delegated
            } else {
                status
            },
        ));
        if parked_here {
            break;
        }
    }

    emit_report(
        ctx,
        requested.as_str(),
        chain,
        reports,
        contribution_reports,
        notices,
        parked.map(|(_, delegation)| delegation),
    )
}

/// Direct install callback after durability and before its final document.
pub(crate) fn after_direct_install(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
    run: InstallRunContext,
) -> Result<super::install::WorldCallbackOutcome> {
    let _ = disposition;
    let phases = [Phase::Validate, Phase::Install];
    let ritual = world::plan_default(path, &phases)?;
    let metadata = run.metadata.clone();
    surface_plan(ctx, &ritual, &metadata, false)?;
    let state_chain = metadata.chain.clone();
    let slot_reports = run.lifecycle_reports;
    let outcome = if let Some(shared) = run.lifecycle_run {
        dispatch::dispatch_plan_with_run(ctx, &ritual, &shared, &metadata)?
    } else {
        dispatch::dispatch_plan(ctx, &ritual, metadata, state_chain)?
    };
    let parked = outcome.parked.map(|(_, delegation)| delegation);
    let contributions = outcome.reports;
    // NOTHING is rendered here. `vibe install` is the outermost command on
    // this path, so its one `cli-install-report` carries these rows and any
    // handoff; emitting a lifecycle report beside it was the second document.
    Ok(super::install::WorldCallbackOutcome {
        summary: WorldCallbackSummary {
            selected_contributions: ritual.executions.len() + slot_reports.len(),
            executed_contributions: contributions
                .iter()
                .filter(|row| row.status != "fresh")
                .count()
                + slot_reports.len(),
            successful_contributions: contributions
                .iter()
                .filter(|row| row.status == "ok")
                .count()
                + slot_reports.iter().filter(|row| row.status == "ok").count(),
            fresh_contributions: contributions
                .iter()
                .filter(|row| row.status == "fresh")
                .count(),
            notices: ritual.notices.len(),
        },
        contributions,
        notices: ritual.notices.clone(),
        parked,
    })
}

/// Choose this invocation's durable run identity through the one selector,
/// before anything is allocated. `state_root` is the workspace root (where
/// `.vibe/lifecycle.toml` lives); the fresh candidate is allocated under the
/// selected project root.
pub(crate) fn run_identity(
    ctx: &output::Context,
    path: &Path,
    requested: &str,
    chain: &[String],
    force: bool,
) -> Result<vibe_lifecycle::RunIdentity> {
    let project_root = super::install::resolve_project_root(path)?;
    let workspace_root = Workspace::discover(&project_root)
        .map(|workspace| workspace.root)
        .unwrap_or_else(|_| project_root.clone());
    vibe_lifecycle::select_run_identity(
        &workspace_root,
        &project_root,
        requested,
        chain,
        ctx.agent_mode(),
        force,
        // The CLI/manifest trace request lands in the next command atom;
        // every current call site selects with the request absent.
        false,
        crate::commands::init::current_timestamp_utc(),
    )
    .map_err(Into::into)
}

fn new_run_id(project_root: &Path) -> Result<String> {
    vibe_lifecycle::process::allocate_run_id(project_root).map_err(Into::into)
}

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

fn validate(path: &Path) -> Result<std::path::PathBuf> {
    let project_root = super::install::resolve_project_root(path)?;
    Workspace::discover(&project_root).context("validating the workspace and its manifests")?;
    Ok(project_root)
}

fn effective_offline(root_offline: bool, install_args: &InstallArgs) -> Result<bool> {
    let user = UserConfig::load().context("loading user config for lifecycle envelope")?;
    Ok(output::resolve_offline(
        root_offline || install_args.offline,
        user.net.offline,
    ))
}

fn effective_clean_offline(root_offline: bool) -> Result<bool> {
    let user = UserConfig::load().context("loading user config for clean lifecycle envelope")?;
    Ok(output::resolve_offline(root_offline, user.net.offline))
}

fn metadata_assume_yes(ctx: &output::Context, explicit: bool) -> bool {
    explicit || ctx.is_unattended() || ctx.is_json()
}
