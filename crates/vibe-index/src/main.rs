#![deny(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use std::process::ExitCode;

use clap::Parser;
use vibe_index::cli;
use vibe_index::config::{self, Ladder};

fn main() -> ExitCode {
    // 1. Parse — `--help` and parse errors answer before any log,
    //    exactly as before the ladder existed.
    let cli = cli::Cli::parse();
    // 2. Read the file rung. It lives at
    //    `<data-dir>/state/config.toml` and the data directory is the
    //    required positional, so the ladder is loadable only now,
    //    after the parse — the natural order `config.rs` documents. A
    //    present-but-broken file refuses here, before any work: every
    //    verb runs on a config layer it understands, or not at all.
    let ladder = match cli.command.data_dir() {
        Some(dir) => match Ladder::load(dir) {
            Ok(ladder) => ladder,
            Err(e) => return fail(e),
        },
        None => Ladder::absent(),
    };
    // 3. Resolve the logging member through the ladder — flag >
    //    env > file > default — and install the subscriber on the
    //    result. The flag no longer WRITES `VIBE_LOG`: with a file
    //    rung in the ladder the process environment can no longer be
    //    the full explanation of the output (a value may come from
    //    inside the data directory), so the explanation the ruling
    //    requires is the visible source — `vibe-index config
    //    <data-dir>` names the rung behind every effective value
    //    (Р51's fold is superseded by B-086's ruling; `VIBE_LOG`
    //    itself keeps working unchanged, as the env rung).
    let log = match config::resolve_log_filter(&ladder, cli.log_level, &config::live_env) {
        Ok(log) => log,
        Err(e) => return fail(e),
    };
    init_tracing(&log.value);
    // 4. Work. The global flag rides along: it is the ladder's top
    //    rung, and the `config` verb shows it as the value's source.
    match cli::dispatch(cli.command, cli.log_level) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

fn fail(e: vibe_index::Error) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::FAILURE
}

/// Install the tracing subscriber unconditionally — a binary's job, not
/// the library's. One filter string, resolved by the config ladder
/// (flag > env > file > the built-in `warn`); an unparseable
/// `VIBE_LOG` directive still falls back to `warn`, exactly as before
/// the ladder existed. WARN-level observability (quarantine refusals
/// on load, auto-commit-push outcomes, scanner skips) must be on for
/// every subcommand, not only the flag-gated ones.
fn init_tracing(filter: &str) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
