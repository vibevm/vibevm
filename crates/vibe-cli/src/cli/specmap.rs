//! Argument struct for `vibe specmap` — the package-carried traceability-map
//! generator (V5-PACKAGE-MAP §2.2). Split from the `cli` hub along
//! command-family lines; the hub re-exports it, so `crate::cli::SpecmapArgs`
//! is the address.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

/// `vibe specmap` — build the package's carried traceability map and write it
/// into the package directory. The carried map is minted under the package's
/// coordinate `spec://<group>/<name>/…` (globally unique), not the local
/// `specmap.toml` nickname (which is not). A package without a `specmap.toml`
/// does not participate in traceability and is left untouched — a clear no-op.
#[derive(Debug, clap::Args)]
pub struct SpecmapArgs {
    /// Package directory carrying the `vibe.toml` to read. Defaults to the
    /// current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
