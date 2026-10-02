//! Explicit on-demand machine frame grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct MachineArgs {
    #[command(subcommand)]
    pub command: MachineSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum MachineSubcommand {
    /// Exchange generated frames with the pure planner until stdin EOF.
    Session(MachineSessionArgs),
}

#[derive(Debug, Args)]
pub struct MachineSessionArgs {
    /// Explicit absolute project directory containing vibe.toml.
    #[arg(long, value_parser = absolute_root, conflicts_with_all = ["json", "quiet", "verbose", "invoked_by"])]
    pub root: PathBuf,
}

fn absolute_root(value: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(value);
    if !root.is_absolute() {
        return Err("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: root must be absolute; fix surface: --root <absolute-project-root>".to_owned());
    }
    Ok(root)
}
