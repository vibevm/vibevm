#![deny(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use std::process::ExitCode;

use clap::Parser;
use vibe_index::cli;

fn main() -> ExitCode {
    // 1. Parse — `--help` and parse errors answer before any log,
    //    exactly as before this flag existed.
    let cli = cli::Cli::parse();
    // 2. Fold the flag into the ONE lever.
    apply_log_level(cli.log_level);
    // 3. The subscriber reads exactly one place.
    init_tracing();
    // 4. Work.
    match cli::dispatch(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Fold `--log-level` into the one lever. The flag WRITES `VIBE_LOG`,
/// so `init_tracing` still reads exactly one place and the process
/// environment stays the full truth about what an operator observes
/// (Р51). Absent flag: nothing is written and `VIBE_LOG` (or the
/// `warn` default) governs, exactly as before this flag existed.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: the flag folds into the one lever by writing it, \
              and the write runs at the very top of main — after the parse, \
              before the subscriber, before the dispatcher and before any \
              thread exists (`serve` boots its runtime far later); set_var's \
              race is with concurrent readers, and none can be observing \
              yet. The alternative composition — the flag OVERRIDING the \
              variable in code — was rejected because it leaves an operator \
              looking at a set VIBE_LOG that no longer explains the output"
)]
fn apply_log_level(level: Option<cli::LogLevel>) {
    let Some(level) = level else { return };
    // SAFETY: vibe-index is a single-threaded CLI binary at this
    // point. The write happens at the very top of `main`, before the
    // dispatcher selects a subcommand and well before any thread is
    // spawned (`serve` boots its tokio/axum runtime far later). The
    // Rust 1.85+ `unsafe` marker on `set_var` exists to flag
    // mid-execution multi-threaded mutation, which we are not doing
    // here. No other thread can be observing the environment at this
    // point.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("VIBE_LOG", level.as_filter());
    }
}

/// Install the tracing subscriber unconditionally — a binary's job, not
/// the library's. One lever, `VIBE_LOG` (default `warn`); there is no
/// `RUST_LOG` fallback and no second lever — the global `--log-level`
/// flag is not one either: it folds INTO `VIBE_LOG` (in `main`, before
/// this runs), so the filter below reads exactly one place and cannot
/// diverge from what the process environment says. WARN-level
/// observability (quarantine refusals on load, auto-commit-push
/// outcomes) must be on for every subcommand, not only the flag-gated
/// ones.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
