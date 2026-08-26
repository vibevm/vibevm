use std::path::PathBuf;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

/// Inspect every retained extension declaration without executing it.
#[derive(Debug, Clone, clap::Args)]
pub struct ExtensionsArgs {
    /// Directory of the project/package/workspace node (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
