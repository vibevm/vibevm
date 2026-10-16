//! Early CLI composition of pure planning or an explicit fixed host.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

mod client_deploy;
mod fixed_deploy;
pub(crate) use fixed_deploy::{FixedDeployHostMode, context as fixed_deploy_context};
mod exact_local_install;
pub(crate) use exact_local_install::{
    ExactLocalInstallHostMode, context as exact_local_install_context,
};

use crate::cli::{
    AgentModeArg, Cli, FixedCargoMode, FixedDeployMode, FixedInstallMode, FixedPackageMode,
    FixedValidateMode, MachineArgs, MachineSessionArgs, MachineSubcommand,
};
use anyhow::{Result, bail};
use std::{process::ExitCode, sync::Arc};
use vibe_core::manifest::TargetOs;
use vibe_mcp::{
    ServerContext,
    machine_session::{
        cargo::CargoMachineSessionRunner,
        deploy::DeployMachineSessionRunner,
        exact_local_install::ExactLocalInstallMachineSessionRunner,
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
    dispatch(
        args,
        cli,
        None,
        &mut std::io::stdin(),
        &mut std::io::stdout(),
    )
}

/// CLI composition entry: only the embedding host can supply this capability.
/// Parsed arguments never create client deployment authority.
pub(crate) fn dispatch<R: std::io::Read, W: std::io::Write>(
    args: &MachineArgs,
    cli: &Cli,
    client_host: Option<
        Arc<vibe_orchestrator::machine_operation::client_deploy::ClientDeployRoute>,
    >,
    reader: &mut R,
    writer: &mut W,
) -> ExitCode {
    let MachineSubcommand::Session(args) = &args.command;
    if let Err(error) = validate_output(cli).and_then(|()| validate_mode(args)) {
        eprintln!("vibe machine session: {error}");
        return ExitCode::from(crate::exit_code::USAGE);
    }
    let result = if args.client_deploy {
        client_deploy::exchange(args, client_host, reader, writer)
    } else {
        exchange(args)
    };
    match result {
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
        args.fixed_install,
    ) {
        (Some(FixedValidateMode::Issue), None, None, None, None)
        | (None, Some(FixedPackageMode::Issue), None, None, None)
        | (None, None, Some(FixedCargoMode::Issue), None, None)
        | (None, None, None, Some(FixedDeployMode::Issue), None)
        | (None, None, None, None, Some(FixedInstallMode::Issue)) => args
            .ttl_ms
            .is_some_and(|ttl| (1..=86_400_000).contains(&ttl)),
        (Some(FixedValidateMode::Reattach), None, None, None, None)
        | (None, Some(FixedPackageMode::Reattach), None, None, None)
        | (None, None, Some(FixedCargoMode::Reattach), None, None)
        | (None, None, None, Some(FixedDeployMode::Reattach), None)
        | (None, None, None, None, Some(FixedInstallMode::Reattach)) => args.ttl_ms.is_none(),
        (None, None, None, None, None) => {
            args.ttl_ms.is_none()
                && !args.approve_captured_script
                && !args.approve_exact_local_install
                && !args.allow_detach
                && !args.allow_cancel_pre_dispatch
                && !args.allow_cancel_deferred_boundary
        }
        _ => false,
    };
    let install_source = args.registry_root.is_some()
        && args.package_pin.as_ref().is_some_and(|pin| !pin.is_empty())
        && (1..=8).contains(&args.source_files.len())
        && args.source_files.iter().any(|file| file == "vibe.toml");
    let valid = valid
        && (args.fixed_package.is_none()
            && args.fixed_cargo.is_none()
            && args.fixed_deploy.is_none()
            && args.fixed_install.is_none()
            || (!args.approve_captured_script
                && !args.allow_detach
                && !args.allow_cancel_pre_dispatch
                && !args.allow_cancel_deferred_boundary))
        && (args.fixed_deploy.is_some() == args.deploy_profile.is_some())
        && (args.fixed_install.is_some() == install_source)
        && (args.fixed_install.is_some()
            || (args.registry_root.is_none()
                && args.package_pin.is_none()
                && args.source_files.is_empty()
                && !args.approve_exact_local_install));
    if !valid {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-LOCAL-TRUST-DECISION: invalid fixed mode/permissions/lifetime/source; fix surface: select one fixed mode; issue requires --ttl-ms <1..=86400000>; fixed Deploy requires --deploy-profile; fixed Install requires --registry-root, --package-pin and --source-file vibe.toml; approvals require their matching mode"
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
            &args.deploy_allow_profiles,
            mode,
        )?;
        let mut runner = DeployMachineSessionRunner::new(context)?;
        runner.run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())?;
        return Ok(());
    }
    if let Some(mode) = args.fixed_install {
        let mode = match mode {
            FixedInstallMode::Issue => {
                ExactLocalInstallHostMode::Issue(args.ttl_ms.ok_or_else(|| {
                    anyhow::anyhow!("fixed Install issue requires --ttl-ms <1..=86400000>")
                })?)
            }
            FixedInstallMode::Reattach => ExactLocalInstallHostMode::Reattach,
        };
        let context = exact_local_install_context(
            ServerContext::with_store_root(&root, root.join(".vibe/cache")),
            &root,
            args.registry_root
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Install requires --registry-root"))?,
            args.package_pin
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Install requires --package-pin"))?,
            &args.source_files,
            args.approve_exact_local_install,
            mode,
        )?;
        let mut runner = ExactLocalInstallMachineSessionRunner::new(context)?;
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

#[cfg(test)]
mod exact_local_install_cli_tests {
    use super::*;
    use crate::cli::Command;
    use clap::Parser;

    fn parsed(extra: &[&str]) -> std::result::Result<MachineSessionArgs, clap::Error> {
        let mut argv = vec![
            "vibe",
            "machine",
            "session",
            "--root",
            "/tmp/vibe-machine-install-test",
        ];
        argv.extend_from_slice(extra);
        let cli = Cli::try_parse_from(argv)?;
        let Command::Machine(machine) = cli.command else {
            panic!("machine command expected")
        };
        let MachineSubcommand::Session(args) = machine.command;
        Ok(args)
    }

    #[test]
    fn fixed_install_cli_requires_explicit_source_approval_and_issue_lifetime() {
        let issue = parsed(&[
            "--fixed-install",
            "issue",
            "--registry-root",
            "/tmp/vibe-registry-test",
            "--package-pin",
            "flow:org.example/fixture@=1.1.0",
            "--source-file",
            "vibe.toml",
            "--source-file",
            "README.md",
            "--ttl-ms",
            "60000",
        ])
        .unwrap();
        assert!(validate_mode(&issue).is_ok());
        assert!(!issue.approve_exact_local_install);
        let mut missing_ttl = parsed(&[
            "--fixed-install",
            "issue",
            "--registry-root",
            "/tmp/vibe-registry-test",
            "--package-pin",
            "flow:org.example/fixture@=1.1.0",
            "--source-file",
            "vibe.toml",
        ])
        .unwrap();
        assert!(validate_mode(&missing_ttl).is_err());
        missing_ttl.fixed_install = Some(FixedInstallMode::Reattach);
        assert!(validate_mode(&missing_ttl).is_ok());
        assert!(
            parsed(&[
                "--fixed-install",
                "issue",
                "--registry-root",
                "relative",
                "--package-pin",
                "flow:org.example/fixture@=1.1.0",
                "--source-file",
                "vibe.toml",
                "--ttl-ms",
                "60000",
            ])
            .is_err()
        );
        assert!(parsed(&["--approve-exact-local-install"]).is_err());
        assert!(parsed(&["--fixed-install", "issue", "--fixed-validate", "issue"]).is_err());
    }
}
