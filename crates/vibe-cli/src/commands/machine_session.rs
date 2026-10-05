//! Early CLI composition of pure planning or an explicit fixed host.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

mod fixed_deploy;
pub(crate) use fixed_deploy::{FixedDeployHostMode, context as fixed_deploy_context};

use crate::cli::{
    AgentModeArg, Cli, FixedCargoMode, FixedDeployMode, FixedPackageMode, FixedValidateMode,
    MachineArgs, MachineSessionArgs, MachineSubcommand,
};
use anyhow::{Result, bail};
use std::{process::ExitCode, sync::Arc};
use vibe_core::manifest::TargetOs;
use vibe_mcp::{
    ServerContext,
    machine_session::{
        cargo::CargoMachineSessionRunner,
        deploy::DeployMachineSessionRunner,
        fixed::FixedMachineSessionRunner,
        package::PackageMachineSessionRunner,
        runner::{MachineSessionRunner, RunnerError},
        state::{MachineSession, TrustedContextKind, TrustedContextSource},
    },
};
use vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute;
use vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute;
use vibe_orchestrator::machine_submission::{
    HostPolicyFacts, TrustedPolicySource, cargo_grant::CargoGrantService,
    cli_grant::CliGrantService, package_grant::PackageGrantService,
};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
    r = 1
)]
pub(crate) fn run(args: &MachineArgs, cli: &Cli) -> ExitCode {
    let MachineSubcommand::Session(args) = &args.command;
    if let Err(error) = validate_output(cli).and_then(|()| validate_mode(args)) {
        eprintln!("vibe machine session: {error}");
        return ExitCode::from(crate::exit_code::USAGE);
    }
    match exchange(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vibe machine session: {error:#}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn validate_mode(args: &MachineSessionArgs) -> Result<()> {
    let valid = match (
        args.fixed_validate,
        args.fixed_package,
        args.fixed_cargo,
        args.fixed_deploy,
    ) {
        (Some(FixedValidateMode::Issue), None, None, None)
        | (None, Some(FixedPackageMode::Issue), None, None)
        | (None, None, Some(FixedCargoMode::Issue), None)
        | (None, None, None, Some(FixedDeployMode::Issue)) => args
            .ttl_ms
            .is_some_and(|ttl| (1..=86_400_000).contains(&ttl)),
        (Some(FixedValidateMode::Reattach), None, None, None)
        | (None, Some(FixedPackageMode::Reattach), None, None)
        | (None, None, Some(FixedCargoMode::Reattach), None)
        | (None, None, None, Some(FixedDeployMode::Reattach)) => args.ttl_ms.is_none(),
        (None, None, None, None) => {
            args.ttl_ms.is_none()
                && !args.approve_captured_script
                && !args.allow_detach
                && !args.allow_cancel_pre_dispatch
                && !args.allow_cancel_deferred_boundary
        }
        _ => false,
    };
    let valid = valid
        && (args.fixed_package.is_none()
            && args.fixed_cargo.is_none()
            && args.fixed_deploy.is_none()
            || (!args.approve_captured_script
                && !args.allow_detach
                && !args.allow_cancel_pre_dispatch
                && !args.allow_cancel_deferred_boundary))
        && (args.fixed_deploy.is_some() == args.deploy_profile.is_some());
    if !valid {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-LOCAL-TRUST-DECISION: invalid fixed mode/permissions/lifetime; fix surface: select one fixed mode; issue requires --ttl-ms <1..=86400000>; fixed Deploy requires --deploy-profile; Validate permissions require --fixed-validate"
        );
    }
    Ok(())
}

/// Clap propagates globals after nested conflict validation. Check the typed
/// root as well, so flags before the subcommand cannot bypass the protocol rule.
pub(crate) fn validate_output(cli: &Cli) -> Result<()> {
    if cli.json
        || cli.quiet
        || cli.verbose
        || cli.invoked_by.is_some()
        || cli.agent_mode != AgentModeArg::Auto
    {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME: global output/agent flags conflict with framed stdout; fix surface: omit --json, --quiet, --verbose, --invoked-by and non-auto --agent-mode for machine session"
        );
    }
    Ok(())
}

fn exchange(args: &MachineSessionArgs) -> Result<()> {
    validate_mode(args)?;
    if !args.root.is_absolute() {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: root must be absolute; fix surface: --root <absolute-project-root>"
        );
    }
    let root = super::resolve_project_root(&args.root)?;
    debug_assert!(root.is_absolute());
    if let Some(mode) = args.fixed_deploy {
        let mode = match mode {
            FixedDeployMode::Issue => FixedDeployHostMode::Issue(args.ttl_ms.ok_or_else(|| {
                anyhow::anyhow!("fixed Deploy issue requires --ttl-ms <1..=86400000>")
            })?),
            FixedDeployMode::Reattach => FixedDeployHostMode::Reattach,
        };
        let context = fixed_deploy_context(
            ServerContext::with_store_root(&root, root.join(".vibe/cache")),
            &root,
            args.deploy_profile
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Deploy requires --deploy-profile"))?,
            mode,
        )?;
        let mut runner = DeployMachineSessionRunner::new(context)?;
        runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())?;
        return Ok(());
    }
    if let Some(mode) = args.fixed_cargo {
        let target_os = TargetOs::current().ok_or_else(|| anyhow::anyhow!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-CARGO-BUILD: unsupported target platform; fix surface: run fixed Cargo on a supported platform"
        ))?;
        let project = Arc::new(vibe_safefs::Project::open(&root)?);
        let service = CargoGrantService::from_trusted_builtin_host(
            project,
            "vibe-cli-project-local/e1".into(),
            target_os,
        )?;
        let host = match mode {
            FixedCargoMode::Issue => service.issue(args.ttl_ms.ok_or_else(|| {
                anyhow::anyhow!("fixed Cargo issue requires --ttl-ms <1..=86400000>")
            })?)?,
            FixedCargoMode::Reattach => service.reattach()?,
        };
        let context = ServerContext::with_store_root(&root, root.join(".vibe/cache"))
            .with_fixed_cargo_route(Arc::new(FixedCargoRoute::new(Arc::new(host))))?;
        let mut runner = CargoMachineSessionRunner::new(context)?;
        runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())?;
        return Ok(());
    }
    if let Some(mode) = args.fixed_package {
        let target_os = TargetOs::current().ok_or_else(|| anyhow::anyhow!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-PACKAGE-GRANT-DECISION: unsupported target platform; fix surface: run fixed package on a supported platform"
        ))?;
        let project = Arc::new(vibe_safefs::Project::open(&root)?);
        let service = PackageGrantService::from_trusted_host(
            project,
            "vibe-cli-project-local/e1".into(),
            target_os,
        )?;
        let host = match mode {
            FixedPackageMode::Issue => service.issue(args.ttl_ms.ok_or_else(|| {
                anyhow::anyhow!("fixed package issue requires --ttl-ms <1..=86400000>")
            })?)?,
            FixedPackageMode::Reattach => service.reattach()?,
        };
        let context = ServerContext::with_store_root(&root, root.join(".vibe/cache"))
            .with_fixed_package_route(Arc::new(FixedPackageRoute::new(Arc::new(host))))?;
        let mut runner = PackageMachineSessionRunner::new(context)?;
        runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())?;
        return Ok(());
    }
    if let Some(mode) = args.fixed_validate {
        let target_os = TargetOs::current().ok_or_else(|| anyhow::anyhow!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-LOCAL-TRUST-DECISION: unsupported target platform; fix surface: run fixed Validate on a supported platform"
        ))?;
        let project = Arc::new(vibe_safefs::Project::open(&root)?);
        let target = format!("physical-root:{}", project.root_correlation_token()?);
        let service = CliGrantService::from_trusted_host(
            project,
            target,
            "vibe-cli-project-local/e1".into(),
            target_os,
            TrustedPolicySource::from_host_identifier("vibe-cli/fixed-validate/e1".into())?,
            HostPolicyFacts {
                approve_captured_script: args.approve_captured_script,
                approve_exact_local_install: false,
                allow_detach: args.allow_detach,
                allow_cancel_pre_dispatch: args.allow_cancel_pre_dispatch,
                allow_cancel_deferred_boundary: args.allow_cancel_deferred_boundary,
            },
        )?;
        // This branch is host authority for cooperating project writers; it
        // neither authenticates an OS user nor restores process-local approval.
        let host = match mode {
            FixedValidateMode::Issue => service.issue(
                args.ttl_ms
                    .ok_or_else(|| anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-LOCAL-TRUST-DECISION: issue requires a lifetime; fix surface: --ttl-ms <1..=86400000>"))?,
            )?,
            FixedValidateMode::Reattach => service.reattach()?,
        };
        let context = ServerContext::with_store_root(&root, root.join(".vibe/cache"))
            .with_fixed_lifecycle_host(Arc::new(host))?;
        let mut runner = FixedMachineSessionRunner::new(context)?;
        runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())?;
        return Ok(());
    }
    // The pure runner never reads this explicit inert store or grants execution.
    let context = ServerContext::with_store_root(&root, root.join(".vibe/cache"));
    let session = MachineSession::new(vec![TrustedContextSource {
        kind: TrustedContextKind::Workspace,
        context,
        target_roots: vec![root],
    }])
    .map_err(|error| anyhow::anyhow!("trusted session unavailable: {error:?}"))?;
    let mut runner = MachineSessionRunner::new(session)
        .map_err(|_| anyhow::anyhow!("pure planner unavailable"))?;
    runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())
        .map_err(|error| {
            let reason = match error {
                RunnerError::Frame(error) => format!("invalid frame: {error:?}"),
                RunnerError::Output { .. } => "frame output failed".to_owned(),
                RunnerError::Service(_) => "pure planner unavailable".to_owned(),
                RunnerError::EnvelopeSerialization(_) => "reply serialization failed".to_owned(),
                RunnerError::Diagnostic(_) => "pure planner diagnostic refused publication".to_owned(),
            };
            anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME: {reason}; fix surface: machine session frame input/output")
        })?;
    Ok(())
}
