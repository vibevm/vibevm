//! Portable export arguments.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
use std::path::PathBuf;
/// `vibe app export`.
#[derive(Debug, Clone, clap::Args)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub struct AppExportArgs {
    /// Write the set to this file instead of standard output.
    #[arg(long, short = 'o', value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Include hidden installations and those a requirement pulled in.
    #[arg(long)]
    pub all: bool,

    /// Preserve original configuration and required dependency state.
    #[arg(long)]
    pub configuration: bool,

    /// Leave the launcher arrangement (collections, pins, overrides) out.
    #[arg(long)]
    pub installations_only: bool,
}
