//! CLI adapter for the default lifecycle's phase line.
//!
//! R2.2 deliberately has no contribution runtime. This module only expands a
//! typed [`LifecycleRequest`], invokes the two existing built-ins (`validate`
//! and `install`), and records every other phase as an honest no-op.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
use vibe_wire::generated::lifecycle_report::{LifecycleReport, LifecycleStepReport};
use vibe_workspace::Workspace;

use crate::cli::{CleanArgs, CleanChain, InstallArgs, LifecycleArgs};
use crate::output;

use super::install::InstallDisposition;

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

    let reports = traverse(request, |step| {
        let (phase, status) = match step {
            LifecycleStep::Clean => {
                // R2.2 has no clean contributions. Preserve PROP-053's human
                // deletion account, while quiet/JSON keep one final report.
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
        Ok((phase, status))
    })?;

    emit_report(ctx, requested, reports)
}

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
) -> Result<()> {
    let chain = steps.iter().map(|step| step.phase.clone()).collect();
    let report = LifecycleReport {
        chain,
        command: "lifecycle".to_string(),
        ok: true,
        requested: requested.to_string(),
        steps,
    };

    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe lifecycle: {} completed ({} phases)",
            requested,
            report.steps.len(),
        ));
        return Ok(());
    }

    ctx.heading(&format!("lifecycle `{requested}`:"));
    for step in &report.steps {
        ctx.step(&format!("{}: {}", step.phase, status_name(&step.status)));
    }
    ctx.summary(&format!(
        "vibe lifecycle: {} completed ({} phases)",
        requested,
        report.steps.len(),
    ));
    Ok(())
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
