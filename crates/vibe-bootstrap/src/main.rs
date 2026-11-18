use clap::Parser;
fn main() -> std::process::ExitCode {
    match vibe_bootstrap::kit::run(vibe_bootstrap::kit::Args::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vibe-installer: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
