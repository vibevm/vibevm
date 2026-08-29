use std::path::PathBuf;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

/// Inspect every retained extension declaration without executing it,
/// and — under `analyze` — compile the selected node's lane in process
/// and report its attribution evidence (R4.3).
#[derive(Debug, Clone, clap::Args)]
pub struct ExtensionsArgs {
    /// Directory of the project/package/workspace node (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    #[command(subcommand)]
    pub command: Option<ExtensionsCommand>,
}

/// The `vibe extensions …` subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ExtensionsCommand {
    /// Compile the selected node's static lane in process and report the
    /// attribution evidence (R4.3): per-provider contribution bytes, frame
    /// overhead, and every transform pass's byte effect.
    Analyze(AnalyzeArgs),
}

/// `vibe extensions analyze` — the lane analyzer (R4.3).
#[derive(Debug, Clone, clap::Args)]
pub struct AnalyzeArgs {
    /// Directory of the project/package/workspace node (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    /// Write the JSON report to this file instead of printing it.
    #[arg(long)]
    pub out: Option<PathBuf>,
}
