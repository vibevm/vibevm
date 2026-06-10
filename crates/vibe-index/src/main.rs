specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");
use std::process::ExitCode;

use vibe_index::cli;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
