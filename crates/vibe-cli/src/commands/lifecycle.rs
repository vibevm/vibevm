//! CLI adapter for the canonical lifecycle engine.
//!
//! Bootstrap validate/install establish a durable world once. The adapter then
//! reloads that world, narrates the complete extension ritual, and dispatches
//! contributions phase by phase. Clean is a separate pre-wipe epoch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::manifest::ExtensionHandler;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{
    ContributionTier, ExtensionProvider, LifecycleRequest, LifecycleStep, Phase, RunMetadata,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};
use vibe_workspace::Workspace;

use crate::cli::{CleanArgs, CleanChain, InstallArgs, LifecycleArgs};
use crate::output;

use super::install::{InstallDisposition, InstallRunContext, WorldCallbackSummary};

mod agent;
mod dispatch;
mod slot;
pub(crate) mod world;

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
    let metadata = RunMetadata {
        requested: "clean".to_string(),
        chain: chain.clone(),
        offline: effective_clean_offline(root_offline)?,
        assume_yes: metadata_assume_yes(ctx, args.assume_yes),
        agent_mode: RunAgentMode::Cli,
        force: false,
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
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain: chain.clone(),
        offline,
        assume_yes,
        agent_mode: RunAgentMode::Cli,
        force: install_args.force,
        run_id: new_run_id(&super::install::resolve_project_root(&install_args.path)?)?,
        started: crate::commands::init::current_timestamp_utc(),
    };
    let mut reports = Vec::with_capacity(steps.len());
    let mut contribution_reports = Vec::new();
    let mut notices = Vec::new();
    let mut install_lifecycle_run = None;
    let mut install_contribution_reports = Vec::new();

    if steps.first() == Some(&LifecycleStep::Clean) {
        let clean_plan = world::plan_clean(&install_args.path)?;
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
                let disposition = super::install::run_with_lifecycle_context(
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
                        Ok(WorldCallbackSummary::default())
                    },
                )?;
                install_status = Some(match disposition {
                    InstallDisposition::Fresh => StepStatus::Fresh,
                    InstallDisposition::Applied => StepStatus::Ok,
                });
            }
            _ => {}
        }
    }

    let ritual = world::plan_default(&install_args.path, &phases)?;
    notices.extend(ritual.notices.clone());
    surface_plan(ctx, &ritual, &metadata, true)?;
    let state_chain = phases.iter().map(ToString::to_string).collect();
    contribution_reports.extend(if let Some(shared) = install_lifecycle_run {
        dispatch::dispatch_plan_with_run(ctx, &ritual, &shared, &metadata)?
    } else {
        dispatch::dispatch_plan(ctx, &ritual, metadata, state_chain)?
    });
    if !ctx.is_json() && !install_contribution_reports.is_empty() {
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
        reports.push(step_report(phase.as_str(), status));
    }

    emit_report(
        ctx,
        requested.as_str(),
        chain,
        reports,
        contribution_reports,
        notices,
    )
}

fn surface_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: &RunMetadata,
    emit_empty: bool,
) -> Result<()> {
    if !emit_empty && plan.executions.is_empty() && plan.notices.is_empty() {
        return Ok(());
    }
    if ctx.is_json() {
        return ctx.emit_json(&LifecyclePlan {
            chain: metadata.chain.clone(),
            command: "lifecycle:plan".to_string(),
            contributions: plan.executions.iter().map(planned_contribution).collect(),
            notices: plan.notices.clone(),
            requested: metadata.requested.clone(),
        });
    }
    render_ritual(ctx, &plan.notices, &plan.executions);
    Ok(())
}

fn planned_contribution(execution: &world::PlannedExecution) -> PlannedContribution {
    let row = &execution.row;
    let (provider, version) = provider_and_version(row.provider());
    PlannedContribution {
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        reference: None,
        slot_target: None,
        tier: tier_name(row.effective_tier()).to_string(),
        version,
    }
}

fn provider_and_version(provider: &ExtensionProvider) -> (String, Option<String>) {
    match provider {
        ExtensionProvider::Dependency(provider) => {
            (provider.id.to_string(), Some(provider.version.clone()))
        }
        ExtensionProvider::Host(provider) => (
            provider.identity.to_string(),
            (!provider.version.is_empty()).then(|| provider.version.clone()),
        ),
    }
}

fn render_ritual(
    ctx: &output::Context,
    notices: &[String],
    executions: &[world::PlannedExecution],
) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    for notice in notices {
        ctx.step(&format!("lifecycle notice: {notice}"));
    }
    for execution in executions {
        let row = &execution.row;
        let handler = match &row.declaration().handler {
            ExtensionHandler::Builtin { name } => format!("builtin:{name}"),
            other => other.kind().to_string(),
        };
        ctx.step(&format!(
            "will run `{}` — point={}, handler={}, provider={} tier={}",
            row.key(),
            row.declaration().point,
            handler,
            row.provider(),
            tier_name(row.effective_tier()),
        ));
    }
}

fn emit_report(
    ctx: &output::Context,
    requested: &str,
    chain: Vec<String>,
    steps: Vec<LifecycleStepReport>,
    contributions: Vec<LifecycleContributionReport>,
    notices: Vec<String>,
) -> Result<()> {
    let report = LifecycleReport {
        chain,
        command: "lifecycle".to_string(),
        contributions,
        notices,
        ok: true,
        requested: requested.to_string(),
        steps,
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    let fresh = report
        .contributions
        .iter()
        .filter(|row| row.status == "fresh")
        .count();
    let executed = report.contributions.len() - fresh;
    let ok = report
        .contributions
        .iter()
        .filter(|row| row.status == "ok")
        .count();
    let contribution_summary = format!(
        "{} contribution(s) selected, {executed} executed, {ok} ok, {fresh} fresh",
        report.contributions.len(),
    );
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe lifecycle: {requested} completed ({} phases, {contribution_summary}, {} notice(s))",
            report.steps.len(),
            report.notices.len(),
        ));
        return Ok(());
    }
    ctx.heading(&format!("lifecycle `{requested}`:"));
    for step in &report.steps {
        ctx.step(&format!("{}: {}", step.phase, step.status));
    }
    ctx.summary(&format!(
        "vibe lifecycle: {requested} completed ({} phases, {contribution_summary}, {} notice(s))",
        report.steps.len(),
        report.notices.len(),
    ));
    Ok(())
}

pub(super) fn emit_failure_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    phase: &str,
    contributions: &[LifecycleContributionReport],
) -> Result<()> {
    if !ctx.is_json() {
        return Ok(());
    }
    ctx.emit_json(&LifecycleReport {
        chain: metadata.chain.clone(),
        command: "lifecycle".into(),
        contributions: contributions.to_vec(),
        notices: Vec::new(),
        ok: false,
        requested: metadata.requested.clone(),
        steps: vec![LifecycleStepReport {
            phase: phase.into(),
            status: "fail".into(),
        }],
    })
}

/// Direct install callback after durability and before its final document.
pub(crate) fn after_direct_install(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
    run: InstallRunContext,
) -> Result<WorldCallbackSummary> {
    let phases = [Phase::Validate, Phase::Install];
    let ritual = world::plan_default(path, &phases)?;
    let metadata = run.metadata.clone();
    surface_plan(ctx, &ritual, &metadata, false)?;
    let state_chain = metadata.chain.clone();
    let slot_reports = run.lifecycle_reports;
    let contributions = if let Some(shared) = run.lifecycle_run {
        dispatch::dispatch_plan_with_run(ctx, &ritual, &shared, &metadata)?
    } else {
        dispatch::dispatch_plan(ctx, &ritual, metadata, state_chain)?
    };
    if ctx.is_json() && (!contributions.is_empty() || !ritual.notices.is_empty()) {
        emit_report(
            ctx,
            Phase::Install.as_str(),
            phases.iter().map(ToString::to_string).collect(),
            vec![
                step_report(Phase::Validate.as_str(), StepStatus::Ok),
                step_report(
                    Phase::Install.as_str(),
                    match disposition {
                        InstallDisposition::Fresh => StepStatus::Fresh,
                        InstallDisposition::Applied => StepStatus::Ok,
                    },
                ),
            ],
            contributions.clone(),
            ritual.notices.clone(),
        )?;
    }
    Ok(WorldCallbackSummary {
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
    })
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

const fn tier_name(tier: ContributionTier) -> &'static str {
    match tier {
        ContributionTier::Preset => "preset",
        ContributionTier::Dependency => "dependency",
        ContributionTier::HostDeclaration => "host-declaration",
        ContributionTier::HostActivation => "host-activation",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Ok,
    Fresh,
    NoOp,
}

impl StepStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fresh => "fresh",
            Self::NoOp => "no-op",
        }
    }
}
