specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use std::process::ExitCode;

use vibe_index::cli;

fn main() -> ExitCode {
    init_tracing();
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Install the tracing subscriber unconditionally — a binary's job, not
/// the library's. One lever, `VIBE_LOG` (default `warn`); there is no
/// `RUST_LOG` fallback and no second lever. WARN-level observability
/// (quarantine refusals on load, auto-commit-push outcomes) must be on
/// for every subcommand, not only the flag-gated ones.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
