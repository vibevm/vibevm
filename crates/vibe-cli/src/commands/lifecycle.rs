//! CLI adapter for the default lifecycle's phase line.
//!
//! This adapter owns the two-epoch ritual: validate and bootstrap install run
//! first, then the durable world is reloaded and every effective contribution
//! is planned and narrated. R2.3 never dispatches a handler.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};
use vibe_workspace::Workspace;

use crate::cli::{CleanArgs, CleanChain, InstallArgs, LifecycleArgs};
use crate::output;

use super::install::{InstallDisposition, WorldCallbackSummary};

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
    if steps.first() == Some(&LifecycleStep::Clean) {
        let clean_plan = world::plan_clean(&install_args.path)?;
        render_ritual(ctx, &clean_plan.notices, &clean_plan.contributions);
        refuse_undispatchable_clean(&clean_plan.contributions)?;
    }

    let mut reports = Vec::with_capacity(steps.len());
    for step in &steps {
        let (phase, status) = match step {
            LifecycleStep::Clean => {
                let clean_ctx = if ctx.is_json() || ctx.is_quiet() {
                    &child
                } else {
                    ctx
                };
                let root = super::clean::wipe(
                    clean_ctx,
                    &install_args.path,
                    clean_assume_yes || install_args.assume_yes,
                )?;
                install_args.path = root;
                ("clean".to_string(), StepStatus::Ok)
            }
            LifecycleStep::Default(Phase::Validate) => {
                install_args.path = validate(&install_args.path)?;
                (Phase::Validate.to_string(), StepStatus::Ok)
            }
            LifecycleStep::Default(Phase::Install) => {
                let prepare = prepare_install
                    .take()
                    .context("internal: install inputs prepared more than once")?;
                let embedded_root = prepare();
                let disposition =
                    super::install::run(&child, install_args.clone(), embedded_root, root_offline)?;
                let status = match disposition {
                    InstallDisposition::Fresh => StepStatus::Fresh,
                    InstallDisposition::Applied => StepStatus::Ok,
                };
                (Phase::Install.to_string(), status)
            }
            LifecycleStep::Default(phase) => (phase.to_string(), StepStatus::NoOp),
        };
        reports.push(LifecycleStepReport {
            phase,
            status: status.as_str().to_string(),
        });
    }

    let phases: Vec<Phase> = steps
        .iter()
        .filter_map(|step| match step {
            LifecycleStep::Default(phase) => Some(*phase),
            LifecycleStep::Clean => None,
        })
        .collect();
    let ritual = world::plan_default(&install_args.path, &phases)?;
    for report in &mut reports {
        let Ok(phase) = report.phase.parse::<Phase>() else {
            continue;
        };
        if !matches!(phase, Phase::Validate | Phase::Install) && ritual.count_for(phase) > 0 {
            report.status = StepStatus::Planned.as_str().to_string();
        }
    }
    render_ritual(ctx, &ritual.notices, &ritual.contributions);
    emit_report(
        ctx,
        requested,
        reports,
        ritual.contributions,
        ritual.notices,
    )
}

#[cfg(test)]
fn traverse(
    request: LifecycleRequest,
    mut run_step: impl FnMut(LifecycleStep) -> Result<(String, StepStatus)>,
) -> Result<Vec<LifecycleStepReport>> {
    request
        .steps()
        .into_iter()
        .map(|step| {
            let (phase, status) = run_step(step)?;
            Ok(LifecycleStepReport {
                phase,
                status: status.as_str().to_string(),
            })
        })
        .collect()
}

/// The validation phase is intentionally offline: loading the workspace parses
/// and validates the root and every member manifest without constructing a
/// resolver or touching the network.
fn validate(path: &Path) -> Result<std::path::PathBuf> {
    let project_root = super::install::resolve_project_root(path)?;
    Workspace::discover(&project_root).context("validating the workspace and its manifests")?;
    Ok(project_root)
}

fn emit_report(
    ctx: &output::Context,
    requested: Phase,
    steps: Vec<LifecycleStepReport>,
    contributions: Vec<LifecycleContributionReport>,
    notices: Vec<String>,
) -> Result<()> {
    let chain = steps.iter().map(|step| step.phase.clone()).collect();
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
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe lifecycle: {} completed ({} phases, {} contribution(s) planned, {} notice(s))",
            requested,
            report.steps.len(),
            report.contributions.len(),
            report.notices.len(),
        ));
        return Ok(());
    }

    ctx.heading(&format!("lifecycle `{requested}`:"));
    for step in &report.steps {
        ctx.step(&format!("{}: {}", step.phase, status_name(&step.status)));
    }
    ctx.summary(&format!(
        "vibe lifecycle: {} completed ({} phases, {} contribution(s) planned, {} notice(s))",
        requested,
        report.steps.len(),
        report.contributions.len(),
        report.notices.len(),
    ));
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Ok,
    Fresh,
    NoOp,
    Planned,
}

impl StepStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fresh => "fresh",
            Self::NoOp => "no-op",
            Self::Planned => "planned",
        }
    }
}

fn render_ritual(
    ctx: &output::Context,
    notices: &[String],
    contributions: &[LifecycleContributionReport],
) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    for notice in notices {
        ctx.step(&format!("lifecycle notice: {notice}"));
    }
    for row in contributions {
        let version = row
            .version
            .as_deref()
            .map(|version| format!("@{version}"))
            .unwrap_or_default();
        ctx.step(&format!(
            "would run `{}` — point={}, handler={}, provider={}{} tier={} (planned; not run)",
            row.key, row.point, row.handler, row.provider, version, row.tier,
        ));
    }
}

/// Callback used only by the top-level direct install facade after the durable
/// world exists and before install's established final report is rendered.
pub(crate) fn after_direct_install(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
) -> Result<WorldCallbackSummary> {
    let ritual = world::plan_default(path, &[Phase::Validate, Phase::Install])?;
    render_ritual(ctx, &ritual.notices, &ritual.contributions);
    if ctx.is_json() && (!ritual.contributions.is_empty() || !ritual.notices.is_empty()) {
        let reports = vec![
            LifecycleStepReport {
                phase: Phase::Validate.to_string(),
                status: StepStatus::Ok.as_str().to_string(),
            },
            LifecycleStepReport {
                phase: Phase::Install.to_string(),
                status: match disposition {
                    InstallDisposition::Fresh => StepStatus::Fresh,
                    InstallDisposition::Applied => StepStatus::Ok,
                }
                .as_str()
                .to_string(),
            },
        ];
        emit_report(
            ctx,
            Phase::Install,
            reports,
            ritual.contributions.clone(),
            ritual.notices.clone(),
        )?;
    }
    Ok(WorldCallbackSummary {
        planned_contributions: ritual.contributions.len(),
        notices: ritual.notices.len(),
    })
}

/// Pre-wipe clean gate shared by bare clean and the composed clean prefix.
pub(crate) fn guard_clean(ctx: &output::Context, path: &Path) -> Result<()> {
    let ritual = world::plan_clean(path)?;
    render_ritual(ctx, &ritual.notices, &ritual.contributions);
    refuse_undispatchable_clean(&ritual.contributions)
}

fn refuse_undispatchable_clean(contributions: &[LifecycleContributionReport]) -> Result<()> {
    if contributions.is_empty() {
        return Ok(());
    }
    let rows = contributions
        .iter()
        .map(|row| {
            format!(
                "`{}` (handler={}, provider={})",
                row.key, row.handler, row.provider,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "phase:clean has {} planned contribution(s) that R2.3 cannot dispatch handlers yet: {rows}; disable the exact key(s) before wiping or wait for handler dispatch",
        contributions.len(),
    )
}

fn status_name(status: &str) -> &str {
    status
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use specmark::verifies;

    use super::*;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
    fn traversal_stops_at_the_first_failed_phase() {
        let mut seen = Vec::new();
        let error = traverse(LifecycleRequest::Default(Phase::Build), |step| {
            seen.push(step);
            match step {
                LifecycleStep::Default(Phase::Install) => bail!("install refused"),
                LifecycleStep::Default(phase) => Ok((phase.to_string(), StepStatus::Ok)),
                LifecycleStep::Clean => unreachable!(),
            }
        })
        .unwrap_err();

        assert_eq!(
            seen,
            [
                LifecycleStep::Default(Phase::Validate),
                LifecycleStep::Default(Phase::Install),
            ]
        );
        assert_eq!(error.to_string(), "install refused");
    }
}
