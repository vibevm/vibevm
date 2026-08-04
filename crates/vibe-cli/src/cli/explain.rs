//! Argument struct for `vibe explain` — the host's traceability-explain
//! command (V3-EXPLAIN-SURFACE). Split from the `cli` hub along
//! command-family lines; the hub re-exports it, so `crate::cli::ExplainArgs`
//! is the address.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

/// `vibe explain <target>` — answer "what implements / verifies this spec
/// rule (or this code symbol)?" over THIS tree, by building the specmap
/// fresh in memory. `--json` (the global flag) emits the raw one-hop
/// subgraph; the default is the deterministic text view.
#[derive(Debug, clap::Args)]
pub struct ExplainArgs {
    /// A `spec://…#anchor` URI or a code symbol to explain — the subgraph
    /// of what implements, verifies, documents, or deviates from it.
    pub target: String,

    /// Project root. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
