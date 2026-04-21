//! The `vibe` CLI entry point. Keeps this file thin: parse args, dispatch.
//!
//! Spec: `VIBEVM-SPEC.md` §9.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod exit_code;
mod output;

use cli::{Cli, Command};
use exit_code::as_exit_code;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // clap handles its own formatting and picks the right category.
            e.exit();
        }
    };

    init_tracing();

    let ctx = output::Context::from_flags(cli.quiet, cli.json);

    let result = match cli.command {
        Command::Init(args) => commands::init::run(&ctx, args),
        Command::List(args) => commands::list::run(&ctx, args),
        Command::Install(args) => commands::install::run(&ctx, args),
        Command::Uninstall(args) => commands::uninstall::run(&ctx, args),
        Command::Registry(args) => commands::registry::run(&ctx, args),
        Command::Version => {
            println!("vibe {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            ctx.error(&err);
            as_exit_code(&err)
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
