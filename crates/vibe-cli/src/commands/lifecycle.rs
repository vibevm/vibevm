//! CLI adapter for the canonical lifecycle engine.
//!
//! Bootstrap validate/install establish a durable world once. The adapter then
//! reloads that world, narrates the complete extension ritual, and dispatches
//! contributions phase by phase. Clean is a separate pre-wipe epoch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::manifest::ExtensionHandler;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{
    ContributionTier, DispatchBatch, ExecutionSession, ExtensionProvider, LifecycleRequest,
    LifecycleStep, Phase, RunMetadata,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle::e1::reply::ReplyStatus;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};
use vibe_workspace::Workspace;

use crate::cli::{CleanArgs, CleanChain, InstallArgs, LifecycleArgs};
use crate::output;

use super::install::{InstallDisposition, InstallRunContext, WorldCallbackSummary};

mod world;

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
        run_id: new_run_id(),
    };
    let notices = plan.notices.clone();
    surface_plan(ctx, &plan, &metadata, true)?;
    let wipe_plan = super::clean::plan_wipe(&args.path)?;
    super::clean::confirm_wipe(ctx, &wipe_plan, metadata.assume_yes)?;
    let contributions = dispatch_plan(ctx, &plan, metadata)?;
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
        force: false,
        run_id: new_run_id(),
    };
    let mut reports = Vec::with_capacity(steps.len());
    let mut contribution_reports = Vec::new();
    let mut notices = Vec::new();

    if steps.first() == Some(&LifecycleStep::Clean) {
        let clean_plan = world::plan_clean(&install_args.path)?;
        notices.extend(clean_plan.notices.clone());
        surface_plan(ctx, &clean_plan, &metadata, true)?;
        let wipe_plan = super::clean::plan_wipe(&install_args.path)?;
        super::clean::confirm_wipe(ctx, &wipe_plan, assume_yes)?;
        contribution_reports.extend(dispatch_plan(ctx, &clean_plan, metadata.clone())?);
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
                let disposition =
                    super::install::run(&child, install_args.clone(), prepare(), root_offline)?;
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
    contribution_reports.extend(dispatch_plan(ctx, &ritual, metadata)?);
    for phase in phases {
        let status = match phase {
            Phase::Validate => validate_status.unwrap_or(StepStatus::Ok),
            Phase::Install => install_status.unwrap_or(StepStatus::Ok),
            _ if ritual.count_for(phase) == 0 => StepStatus::NoOp,
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

fn dispatch_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    let mut session = ExecutionSession::new(plan.project.clone(), plan.world.clone(), metadata);
    let mut reports = Vec::with_capacity(plan.executions.len());
    let mut cursor = 0;
    while cursor < plan.executions.len() {
        let phase = plan.executions[cursor].phase.clone();
        let end = plan.executions[cursor..]
            .iter()
            .position(|execution| execution.phase != phase)
            .map_or(plan.executions.len(), |offset| cursor + offset);
        let rows = plan.executions[cursor..end]
            .iter()
            .map(|execution| execution.row.clone())
            .collect::<Vec<_>>();
        let DispatchBatch { outcomes, failure } = session.dispatch_phase(&phase, &rows);
        for (execution, outcome) in plan.executions[cursor..end].iter().zip(outcomes) {
            let report = contribution_report(execution, &outcome.reply);
            render_outcome(ctx, &report);
            reports.push(report);
        }
        if let Some(failure) = failure {
            return Err(anyhow::Error::new(failure).context(format!(
                "phase `{phase}` stopped before any later lifecycle contribution"
            )));
        }
        cursor = end;
    }
    Ok(reports)
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
        tier: tier_name(row.effective_tier()).to_string(),
        version,
    }
}

fn contribution_report(
    execution: &world::PlannedExecution,
    reply: &vibe_wire::generated::lifecycle::e1::reply::Reply,
) -> LifecycleContributionReport {
    let row = &execution.row;
    let (provider, version) = provider_and_version(row.provider());
    LifecycleContributionReport {
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        message: reply.message.clone(),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        status: reply_status(&reply.status),
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

fn render_outcome(ctx: &output::Context, report: &LifecycleContributionReport) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    if let Some(message) = &report.message {
        ctx.step(&format!("log [{}]: {message}", report.provider));
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
    let contribution_summary = if report.contributions.is_empty() {
        "0 contribution(s) selected, 0 executed, 0 ok".to_string()
    } else {
        format!(
            "{} contribution(s) selected, {} executed, {} ok",
            report.contributions.len(),
            report.contributions.len(),
            report
                .contributions
                .iter()
                .filter(|row| row.status == "ok")
                .count(),
        )
    };
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

/// Direct install callback after durability and before its final document.
pub(crate) fn after_direct_install(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
    run: InstallRunContext,
) -> Result<WorldCallbackSummary> {
    let phases = [Phase::Validate, Phase::Install];
    let ritual = world::plan_default(path, &phases)?;
    let metadata = RunMetadata {
        requested: Phase::Install.to_string(),
        chain: phases.iter().map(ToString::to_string).collect(),
        offline: run.offline,
        assume_yes: run.assume_yes,
        agent_mode: RunAgentMode::Cli,
        force: false,
        run_id: new_run_id(),
    };
    surface_plan(ctx, &ritual, &metadata, false)?;
    let contributions = dispatch_plan(ctx, &ritual, metadata)?;
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
        selected_contributions: ritual.executions.len(),
        executed_contributions: contributions.len(),
        successful_contributions: contributions
            .iter()
            .filter(|row| row.status == "ok")
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

fn new_run_id() -> String {
    static NEXT_RUN: AtomicU64 = AtomicU64::new(1);
    format!(
        "{}-{}",
        std::process::id(),
        NEXT_RUN.fetch_add(1, Ordering::Relaxed),
    )
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

fn reply_status(status: &ReplyStatus) -> String {
    match status {
        ReplyStatus::Ok => "ok",
        ReplyStatus::Fail => "fail",
        ReplyStatus::Skip => "skip",
    }
    .to_string()
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
