//! Argument structs for the project-inspection commands — `vibe show`
//! and `vibe check`.
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    #[command(subcommand)]
    pub command: ShowSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ShowSubcommand {
    /// Print the effective spec — every spec/boot file plus every
    /// installed package's `files_written`, concatenated with
    /// `spec://` provenance headers in stable order.
    Effective(ShowEffectiveArgs),

    /// Print the effective configuration with per-value provenance
    /// (default / vibe.toml / env-var).
    Config(ShowConfigArgs),

    /// Print every active feature recorded in the lockfile, grouped
    /// by package. Per PROP-003 §2.10 / `vibe show features`.
    Features(ShowFeaturesArgs),

    /// Print every active subskill recorded in the lockfile, grouped
    /// by package, with delivery mode and any `describes` PURL.
    Subskills(ShowSubskillsArgs),

    /// Print every PURL the project's lockfile binds to (the union of
    /// per-package `describes` declarations). Useful as a sanity
    /// check for upstream-version drift.
    Purls(ShowPurlsArgs),
}

#[derive(Debug, clap::Args)]
pub struct ShowEffectiveArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowConfigArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowFeaturesArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowSubskillsArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowPurlsArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct TreeArgs {
    /// Project root. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Force the plain ASCII tree instead of the interactive TUI. The TUI
    /// is Phase 2 (PROP-036 §2.11); today output is plain regardless, so
    /// this flag is currently a no-op on a tty.
    #[arg(long)]
    pub plain: bool,

    /// Open the in-terminal console TUI (today's default). Mutually exclusive
    /// with `-t` (TERMINAL-AIUI §6.2).
    #[arg(short = 'c', long, conflicts_with = "terminal")]
    pub console: bool,

    /// Open in the vibeterm desktop terminal instead of the current terminal.
    /// Mutually exclusive with `-c` (TERMINAL-AIUI §6.2).
    #[arg(short = 't', long, conflicts_with = "console")]
    pub terminal: bool,
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// WAL is "stale" past this age. Default 24h matches the boot
    /// snippet's freshness rule.
    #[arg(long = "wal-max-age-hours", default_value_t = 24)]
    pub wal_max_age_hours: u64,

    /// REVIEW marker age threshold in days (`<!-- REVIEW: YYYY-MM-DD ... -->`).
    /// Default 14d per `VIBEVM-SPEC.md` §12.
    #[arg(long = "review-max-age-days", default_value_t = 14)]
    pub review_max_age_days: u64,
}
