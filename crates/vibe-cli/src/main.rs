//! The `vibe` CLI entry point. Keeps this file thin: parse args, dispatch.
//!
//! Spec: `VIBEVM-SPEC.md` §9.

#![deny(unsafe_code)]

use std::collections::BTreeSet;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::Parser;
use vibe_core::user_config::UserConfig;

mod cli;
mod commands;
mod exit_code;
mod output;
mod registry;

use cli::{Cli, Command};
use exit_code::as_exit_code;

/// Names of environment variables that were promoted from the
/// user-level config at startup (i.e. the live env was unset and
/// the user-config carried a default that we wrote into the
/// process env). `vibe show config` reads this set so it can
/// distinguish "operator-set live env" from "promoted from user-
/// config" without re-loading the file mid-run. Empty when no
/// promotions happened.
static PROMOTED_FROM_USER_CONFIG: OnceLock<BTreeSet<String>> = OnceLock::new();

/// Public read-only accessor consumed by `vibe show config`. Returns
/// an empty set if `promote_user_config_env` has not yet run (e.g.
/// embedded test harnesses).
pub(crate) fn promoted_env_names() -> &'static BTreeSet<String> {
    PROMOTED_FROM_USER_CONFIG.get_or_init(BTreeSet::new)
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // clap handles its own formatting and picks the right category.
            e.exit();
        }
    };

    promote_user_config_env();
    init_tracing();

    let ctx = output::Context::from_flags(
        cli.quiet,
        cli.json,
        cli.invoked_by.as_deref(),
        cli.unattended,
    );

    let result = match cli.command {
        Command::Init(args) => commands::init::run(&ctx, args),
        Command::List(args) => commands::list::run(&ctx, args),
        Command::Install(args) => commands::install::run(&ctx, args),
        Command::Outdated(args) => commands::outdated::run(&ctx, args),
        Command::Search(args) => commands::search::run(&ctx, args),
        Command::Mcp(args) => commands::mcp::run(&ctx, args),
        Command::Uninstall(args) => commands::uninstall::run(&ctx, args),
        Command::Update(args) => commands::update::run(&ctx, args),
        Command::Reinstall(args) => commands::reinstall::run(&ctx, args),
        Command::Check(args) => commands::check::run(&ctx, args),
        Command::Show(args) => commands::show::run(&ctx, args),
        Command::Registry(args) => commands::registry::run(&ctx, args),
        Command::Workspace(args) => commands::workspace::run(&ctx, args),
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

/// Read `~/.config/vibe/config.toml` (per `vibe-core::user_config`)
/// and promote any `[env]` entries that aren't already set in the
/// live environment. This makes the user-config layer actually
/// load-bearing per `VIBEVM-SPEC.md` §9.5: subsequent consumers
/// (`vibe-registry::default_cache_root`, the tracing init, future
/// LLM-key paths) read whatever is in the process env without
/// caring who put it there.
///
/// Live env-vars set by the operator at invocation time always
/// win — they were already in the process env by the time we
/// observe them via `std::env::var_os`, so the `if !is_set` guard
/// is sufficient.
///
/// A malformed user-config file is reported via `eprintln!` and the
/// promotion silently continues with whatever fields parsed —
/// failing the entire CLI invocation because of an inert config
/// layer would be the wrong UX. `vibe show config` is the
/// authoritative path for surfacing that the layer is broken;
/// every other command just runs.
fn promote_user_config_env() {
    let cfg = match UserConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: user-level config could not be read: {e}");
            let _ = PROMOTED_FROM_USER_CONFIG.set(BTreeSet::new());
            return;
        }
    };
    let mut promoted: BTreeSet<String> = BTreeSet::new();
    for (name, value) in &cfg.env {
        if std::env::var_os(name).is_some() {
            // Live env wins — leave it alone.
            continue;
        }
        // SAFETY: vibe is a single-threaded CLI binary. Promotion
        // happens at the very top of `main`, before the dispatcher
        // selects a subcommand and well before any thread is
        // spawned (rayon, reqwest's tokio internals when present,
        // etc.). The Rust 1.85+ `unsafe` marker on `set_var` exists
        // to flag mid-execution multi-threaded mutation, which we
        // are not doing here. No other thread can be observing the
        // environment variables at this point.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var(name, value);
        }
        promoted.insert(name.clone());
    }
    let _ = PROMOTED_FROM_USER_CONFIG.set(promoted);
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
