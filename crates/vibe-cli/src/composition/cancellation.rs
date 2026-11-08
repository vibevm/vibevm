//! Project stop input belongs to the command composition root.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use crate::cli::{Command, RegistrySubcommand};
use std::{path::PathBuf, sync::Arc};
use vibe_core::cancellation::CancellationToken;

pub(crate) fn supported(command: &Command) -> bool {
    match command {
        Command::Install(args) => !args.global,
        Command::Update(args) => !args.global,
        Command::Registry(args) => matches!(&args.command, RegistrySubcommand::Publish(_)),
        _ => false,
    }
}
pub(crate) fn attach(
    ctx: &crate::output::Context,
    stop: Option<PathBuf>,
) -> std::io::Result<(crate::output::Context, CancellationToken)> {
    if stop.is_none() {
        return Ok((ctx.clone(), CancellationToken::default()));
    }
    let token = CancellationToken::new().with_poll(Arc::new(move || {
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
    Ok((ctx.with_progress_scope(progress), token))
}

/// Apply the existing support gate before attaching an operation stop source.
pub(crate) fn for_command(
    ctx: &crate::output::Context,
    command: &Command,
    stop: Option<PathBuf>,
) -> anyhow::Result<(crate::output::Context, CancellationToken)> {
    if supported(command) {
        return Ok(attach(ctx, stop)?);
    }
    if stop.is_some() {
        anyhow::bail!(
            "--stop-file is supported only for project install/update and registry publish"
        );
    }
    Ok((ctx.clone(), CancellationToken::new()))
}
