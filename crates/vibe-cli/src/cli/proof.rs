//! Arguments for the uncached selected-subject proof front doors.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-EVIDENCE-IDENTITY");

use std::path::PathBuf;

/// A selected subject and its root-relative checked-evidence file.
#[derive(Debug, clap::Args)]
pub struct ProofArgs {
    /// Consumer project root; defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Exact spec address selected by the composition policy.
    #[arg(long)]
    pub subject: String,

    /// Root-relative witness path to write (check) or verify (verify).
    #[arg(long)]
    pub witness: PathBuf,

    /// Trusted Lean toolchain directory; otherwise use engine startup policy.
    #[arg(long)]
    pub lean_toolchain_dir: Option<PathBuf>,

    /// Trusted Rust toolchain directory; otherwise use engine startup policy.
    #[arg(long)]
    pub rust_toolchain_dir: Option<PathBuf>,
}
