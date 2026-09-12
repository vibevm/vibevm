//! Argument structs for `vibe doc` — the documentation surface over the
//! `vibe-doc` library (PROP-057 `##PIPE-LIBRARY`). Split from the `cli` hub
//! along command-family lines; the hub re-exports them, so
//! `crate::cli::DocArgs` is the address.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

/// `vibe doc` — work with a documentation package.
#[derive(Debug, clap::Args)]
pub struct DocArgs {
    #[command(subcommand)]
    pub command: DocCommand,
}

/// The `vibe doc` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum DocCommand {
    /// Check a documentation package against the product it documents:
    /// run every documented example and compare its output exactly,
    /// rebuild every `derived` block and compare it with the last build,
    /// resolve every `rule` citation against the current specs, and check
    /// a translation against the documentation it adapts.
    Check(DocCheckArgs),
}

/// `vibe doc check` — the checks that keep a manual from lying.
#[derive(Debug, clap::Args)]
pub struct DocCheckArgs {
    /// Run every `<example>` against the built binary in a sandbox and
    /// compare stdout, stderr and the exit code exactly, after the
    /// normalisation each fixture declares.
    #[arg(long)]
    pub examples: bool,

    /// Rebuild every `<derived>` block from the product and report what
    /// changed since the last build.
    #[arg(long)]
    pub derived: bool,

    /// Resolve every `<rule>` citation against the specifications that
    /// exist now. One question and no other: does the anchor exist.
    #[arg(long)]
    pub citations: bool,

    /// Check a translation against the documentation it adapts: the same
    /// pages, the same anchors, examples borrowed rather than authored.
    /// Structure only — there is no «how far behind» here, because that
    /// needs a history this product does not keep.
    #[arg(long)]
    pub translations: bool,

    /// Require every documentation obligation of the specifications — a
    /// fact marked `actionstage="doc"` naming an audience — to be cited
    /// by a page written for that same audience.
    #[arg(long)]
    pub coverage: bool,

    /// With `--coverage`, the percentage that counts as a pass. The
    /// standing bar is everything told; a lower one is for the
    /// intermediate runs of a campaign that is still writing the pages.
    #[arg(long, default_value_t = vibe_doc::coverage::FULL_COVERAGE, value_name = "PERCENT")]
    pub min: u8,

    /// Record a capture on the page when the example has no golden yet.
    /// Never replaces a golden that already holds text — see `--force`.
    #[arg(long)]
    pub accept: bool,

    /// With `--accept`, also replace a golden that already holds text.
    /// A separate flag on purpose: re-blessing a divergence is a decision
    /// somebody takes, not a side effect of running a check.
    #[arg(long)]
    pub force: bool,

    /// Run only the examples whose `<page>#<id>` contains this text.
    #[arg(long, value_name = "TEXT")]
    pub only: Option<String>,

    /// The documentation package to check. Defaults to the current
    /// directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// The `vibe` binary the examples run. Defaults to the running one,
    /// so a check always exercises the build that is asking.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,

    /// Where sandboxes are built. Keep it short: a deep root plus a
    /// materialised dependency tree overflows the Windows path limit.
    #[arg(long, value_name = "PATH")]
    pub sandbox: Option<PathBuf>,

    /// Seconds one documented command may take before it is killed.
    #[arg(long, default_value_t = 300, value_name = "SECONDS")]
    pub timeout: u64,
}
