//! Early CLI composition of the incumbent pure planner frame runner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use crate::cli::{AgentModeArg, Cli, MachineArgs, MachineSessionArgs, MachineSubcommand};
use anyhow::{Result, bail};
use std::process::ExitCode;
use vibe_mcp::{
    ServerContext,
    machine_session::{
        runner::{MachineSessionRunner, RunnerError},
        state::{MachineSession, TrustedContextKind, TrustedContextSource},
    },
};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
    r = 1
)]
pub(crate) fn run(args: &MachineArgs, cli: &Cli) -> ExitCode {
    if let Err(error) = validate_output(cli) {
        eprintln!("vibe machine session: {error}");
        return ExitCode::from(crate::exit_code::USAGE);
    }
    let MachineSubcommand::Session(args) = &args.command;
    match exchange(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vibe machine session: {error:#}");
            ExitCode::FAILURE
        }
    }
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
    if !args.root.is_absolute() {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: root must be absolute; fix surface: --root <absolute-project-root>"
        );
    }
    let root = super::resolve_project_root(&args.root)?;
    debug_assert!(root.is_absolute());
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
