//! Project stop input belongs to the command composition root.
//!
//! Two inputs reach the operation's one cancellation token: the caller-owned
//! `--stop-file`, honoured at once, and the platform's stop signals. An
//! attended terminal asks before a Ctrl-C stops (`question`); `SIGTERM`,
//! Ctrl-Break and a Ctrl-C nobody can answer stop at once.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use crate::cli::{Command, RegistrySubcommand};
use std::io::IsTerminal;
use std::{path::PathBuf, sync::Arc};
use vibe_core::cancellation::CancellationToken;

mod question;
mod signals;
mod watch;

pub(crate) fn supported(command: &Command) -> bool {
    match command {
        Command::Install(args) => !args.global,
        Command::Update(args) => !args.global,
        Command::Registry(args) => matches!(&args.command, RegistrySubcommand::Publish(_)),
        _ => false,
    }
}

/// The stop input of one invocation, kept until the operation returned.
pub(crate) struct StopInput {
    watch: Option<watch::Watch>,
}

impl StopInput {
    fn none() -> Self {
        Self { watch: None }
    }

    /// End the stop input before the outcome is reported: an open question
    /// is answered first, and later signals take their default action. A
    /// stop the person confirmed after the last checkpoint is said so,
    /// rather than left looking honoured.
    pub(crate) fn settle(self, ctx: &crate::output::Context, token: &CancellationToken) {
        let Some(watch) = self.watch else {
            return;
        };
        if watch.settle(token) {
            ctx.diagnostic(
                "the operation ended before the requested stop took effect; nothing was rolled back for it",
            );
        }
    }
}

pub(crate) fn attach(
    ctx: &crate::output::Context,
    stop: Option<PathBuf>,
) -> (crate::output::Context, CancellationToken, StopInput) {
    let posture = question::posture(question::Terminal {
        stdin: std::io::stdin().is_terminal(),
        stderr: std::io::stderr().is_terminal(),
        unattended: ctx.is_unattended(),
        machine: ctx.is_json(),
    });
    let shared = Arc::new(watch::Shared::new());
    let base = CancellationToken::new();
    let held = Arc::clone(&shared);
    let token = base.clone().with_poll(Arc::new(move || {
        // A checkpoint reached while the question is shown waits for the
        // answer; the stop file is read after it.
        held.hold();
        stop.as_ref()
            .is_some_and(|path| match std::fs::symlink_metadata(path) {
                Ok(_) => true,
                Err(error) => error.kind() != std::io::ErrorKind::NotFound,
            })
    }));
    let mut ctx = ctx.clone();
    if let Some(transfers) = ctx.transfers() {
        ctx = ctx.with_transfers(transfers.with_cancellation(token.clone()));
    }
    let progress = ctx.progress().with_cancellation(token.clone());
    vibe_registry::git_backend::transfers::observe_by_default(progress.clone());
    let ctx = ctx.with_progress_scope(progress);
    let watch = watch::Watch::start(posture, shared, base, ctx.clone());
    (ctx, token, StopInput { watch })
}

/// Apply the existing support gate before attaching an operation stop source.
pub(crate) fn for_command(
    ctx: &crate::output::Context,
    command: &Command,
    stop: Option<PathBuf>,
) -> anyhow::Result<(crate::output::Context, CancellationToken, StopInput)> {
    if supported(command) {
        return Ok(attach(ctx, stop));
    }
    if stop.is_some() {
        anyhow::bail!(
            "--stop-file is supported only for project install/update and registry publish"
        );
    }
    Ok((ctx.clone(), CancellationToken::new(), StopInput::none()))
}
