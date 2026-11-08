//! `vibe artifacts` (`PROP-066#drops`): where hand-placed artifacts go.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::PathBuf;

/// Arguments of `vibe artifacts`.
#[derive(Debug, clap::Args)]
pub struct ArtifactsArgs {
    #[command(subcommand)]
    pub command: ArtifactsCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum ArtifactsCommand {
    /// Print where vibe releases and package payloads are expected in the
    /// configured artifact drops — with a package, every input it
    /// downloads, the exact file name, the paths searched and whether a
    /// verified copy is already there.
    Where(ArtifactsWhereArgs),
}

#[derive(Debug, clap::Args)]
pub struct ArtifactsWhereArgs {
    /// An application package, `group/name[@version]`.
    pub package: Option<String>,

    /// Resolve the package from this local registry directory instead of
    /// the configured registries.
    #[arg(long, value_name = "DIR")]
    pub registry: Option<PathBuf>,
}
