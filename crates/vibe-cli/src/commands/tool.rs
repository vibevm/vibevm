//! `vibe tool <tool>` — a facility vibe runs for itself, lent to a
//! command the user names. Today there is one: the vibe-managed px
//! (`PROP-068#proxy-tool`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool");

#[path = "tool/px.rs"]
pub(crate) mod px;

#[path = "tool/which.rs"]
pub(crate) mod which;

use anyhow::Result;
use vibe_core::cancellation::CancellationToken;

use crate::cli::{
    ToolArgs, ToolCommand, ToolPxCommand, VvmArgs, VvmPxArgs, VvmPxCommand, VvmSubcommand,
};
use crate::output;

/// Where the child of `vibe tool px exec` is looked for: the ambient
/// `PATH` and, on Windows, `PATHEXT`, read once at the composition root
/// and threaded in rather than reached for here.
#[derive(Debug, Clone, Default)]
pub struct ToolEnv {
    pub path: Option<String>,
    pub pathext: Option<String>,
    /// `--verbose`: `exec` is transparent, so what vibe has to say about
    /// the route it lent is said only when the invocation asked for
    /// detail — and then on stderr, never on the child's stdout.
    pub verbose: bool,
}

/// Where the child is looked for, read through the composition root's
/// own reader so no cell below reaches for the environment itself.
pub fn env(read: fn(&str) -> Option<String>, verbose: bool) -> ToolEnv {
    ToolEnv {
        path: read("PATH"),
        pathext: read("PATHEXT"),
        verbose,
    }
}

/// `vibe tool <tool> …`. The exit code is the child's for `exec`, so the
/// answer is a code rather than `()`.
pub fn run(
    ctx: &output::Context,
    args: &ToolArgs,
    env: &ToolEnv,
    installation: impl FnOnce() -> crate::commands::vvm::VvmEnv,
    cancellation: &CancellationToken,
) -> Result<Option<i32>> {
    let ToolCommand::Px(command) = &args.command;
    match &command.command {
        ToolPxCommand::Exec(exec) => px::run_exec(ctx, exec, env, cancellation).map(Some),
        ToolPxCommand::Serve(serve) => px::run_serve(ctx, serve, cancellation).map(|()| None),
        // The companion report of `vibe self px status`, under the name a
        // user of `vibe tool px` already has in hand.
        ToolPxCommand::Status => crate::commands::vvm::run(
            ctx,
            VvmArgs {
                command: VvmSubcommand::Px(VvmPxArgs {
                    command: VvmPxCommand::Status,
                }),
            },
            installation(),
        )
        .map(|()| None),
    }
}
