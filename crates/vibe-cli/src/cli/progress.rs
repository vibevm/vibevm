//! Argument structs for `vibe progress` — the Progress Control adapter
//! over the standalone `progress-core` (PROP-043 §5). Split from the
//! `cli` hub along command-family lines; the hub re-exports everything.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#tool");

use std::path::PathBuf;

use clap::{Args, Subcommand};

/// `vibe progress` — inline `<status>` markup: scan, validate, report,
/// and drive the actualization campaign (PROP-043).
#[derive(Debug, Args)]
pub struct ProgressArgs {
    #[command(subcommand)]
    pub command: ProgressSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProgressSubcommand {
    /// Parse the observed tree, refresh the campaign cache and the
    /// dashboard state projections (when a campaign zone is present).
    Scan(ProgressCommonArgs),

    /// Validate the markup: closed vocabularies (with nearest-value
    /// hints), placement law, well-formedness. Non-zero exit on errors.
    Check(ProgressCheckArgs),

    /// Render the tree status: XML natively, `--md` table, `--json`.
    Report(ProgressReportArgs),

    /// Materialize the per-file cache view under the campaign zone
    /// (`run/mirror/`), for LLM batch work.
    Mirror(ProgressCommonArgs),

    /// Stitch the observed corpus into whole-context LLM input:
    /// `--digest` map form, or full form sharded by `--max-tokens`.
    Weave(ProgressWeaveArgs),

    /// Three-way compare against a previous campaign's baseline:
    /// new / changed (suspect) / carried-forward units.
    Rescan(ProgressRescanArgs),

    /// Regenerate `RESUME.md` from the campaign journal and print it —
    /// the first read of every campaign session.
    Resume(ProgressCommonArgs),
}

#[derive(Debug, Args)]
pub struct ProgressCommonArgs {
    /// Root of the observed tree (default: current directory).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Campaign zone directory (default: the single `campaigns/<id>/`
    /// under the root, when exactly one exists).
    #[arg(long)]
    pub campaign: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ProgressCheckArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Campaign gate: additionally require zero unmarked paragraphs in
    /// scope (PROP-043 §3.9).
    #[arg(long)]
    pub exhaustive: bool,
}

#[derive(Debug, Args)]
pub struct ProgressReportArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Render the Markdown table instead of the native XML.
    #[arg(long)]
    pub md: bool,

    /// One of the five resolution views: done | todo | qa | remove | doc.
    #[arg(long)]
    pub view: Option<String>,

    /// Filter markers by audience: user | author | dev.
    #[arg(long)]
    pub audience: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProgressWeaveArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Emit the digest map (headings + markers + counters) instead of
    /// the full corpus.
    #[arg(long)]
    pub digest: bool,

    /// Shard the full weave at roughly this many tokens per shard.
    #[arg(long, value_name = "N")]
    pub max_tokens: Option<usize>,

    /// Write shards into this directory (default: stdout, single shard).
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ProgressRescanArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Path to the previous campaign's `baseline.json`.
    #[arg(long)]
    pub baseline: PathBuf,
}
