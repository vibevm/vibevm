//! Argument structs for `vibe progress` — the Progress Control adapter
//! over the standalone `progress-core` (PROP-043 §5). Split from the
//! `cli` hub along command-family lines; the hub re-exports everything.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#tool");

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

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

    /// Validate the markup (read-only: writes nothing by default) and
    /// exit non-zero on errors — closed vocabularies (with nearest-value
    /// hints), placement law, well-formedness. Unlike `scan`, `check`
    /// leaves the campaign zone untouched — neither the cache nor the
    /// `state/` projections — so a validation run cannot rewrite a frozen
    /// zone the way its name says it would not. Pass `--write-state` to
    /// also warm the cache and projections, exactly as `scan` would.
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

    /// Write `baseline.json` from the campaign's own verdicts — the
    /// artifact the next campaign's `rescan` consumes. Reads the cache;
    /// verifies nothing and invents no verdict.
    Baseline(ProgressBaselineArgs),

    /// Regenerate `RESUME.md` from the campaign journal and print it —
    /// the first read of every campaign session.
    Resume(ProgressCommonArgs),

    /// Record a gate's verdict into the campaign's gate panel. The
    /// automation seam: whoever ran the real gate reports the result here,
    /// and the dashboard reads it out of `campaign.json`.
    Gate(ProgressGateArgs),

    /// Record that a file's verdicts were re-derived against its current
    /// text, so the staleness warning stops firing on it. Verifies
    /// nothing — the caller did the re-derivation and reports it here —
    /// and refuses any file whose markers are not all judged.
    Seal(ProgressSealArgs),
}

#[derive(Debug, Args)]
pub struct ProgressCommonArgs {
    /// Root of the observed tree (default: current directory).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// The campaign zone — the `campaigns/<id>/` whose `run/` holds the
    /// cache, the `state/` projections, the baseline and the journal.
    /// This selects a **state zone**, never a perimeter to read: the tree
    /// a verb parses is always `--path` (default: `.`), and `--campaign`
    /// only says which campaign's records and projections a run writes
    /// into — or, for the read verbs, reads from. Default: the single
    /// `campaigns/<id>/` under the root, when exactly one exists; ad-hoc
    /// mode (no zone, no state) when zero or several do.
    #[arg(long)]
    pub campaign: Option<PathBuf>,

    /// Distrust the cache: parse every observed file even where its
    /// content hash says nothing changed. A verification run that must
    /// not inherit a cached parse says so on the command line rather than
    /// hoping (PROP-043 §7.5 — the cache is erasable acceleration). The
    /// cache is still *written*, so the run leaves the campaign's records
    /// and state projections exactly as a warm run would — for the verbs
    /// that write; `check` writes only under `--write-state`.
    ///
    /// `gate` parses nothing, so the flag does nothing there.
    #[arg(long)]
    pub no_cache: bool,
}

#[derive(Debug, Args)]
pub struct ProgressCheckArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Campaign gate: additionally require zero unmarked facts —
    /// paragraphs, list items, table body cells — in scope (PROP-043 §3.9).
    #[arg(long)]
    pub exhaustive: bool,

    /// Persist the run: write the campaign cache and the `state/`
    /// projections exactly as `scan` would. Off by default — `check` is
    /// read-only, so a validation run leaves the zone's records and state
    /// projections untouched. Reach for it when a check is also the run
    /// that should warm the cache; otherwise prefer `scan`, the verb named
    /// for writing.
    #[arg(long)]
    pub write_state: bool,
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

    /// Share of the carried-forward units re-verified anyway — the §7.3
    /// control sample, because code-side invalidation is deliberately
    /// coarse. `0` disables it; the draw is seeded from the baseline's own
    /// content, so a rescan is reproducible and reviewable.
    #[arg(
        long,
        value_name = "0.0..=1.0",
        default_value_t = progress_core::baseline::DEFAULT_CONTROL_RATE,
        value_parser = parse_control_rate,
    )]
    pub control_rate: f64,
}

#[derive(Debug, Args)]
pub struct ProgressBaselineArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// Where to write the baseline (default: `<campaign zone>/baseline.json`
    /// — the path §7.4's zone layout fixes and the next campaign's
    /// `rescan --baseline` reads).
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// `--control-rate` takes a fraction in `0.0..=1.0`; anything else is a
/// clap error at the boundary, never a silently clamped sample size.
fn parse_control_rate(s: &str) -> Result<f64, String> {
    let rate: f64 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a number (expected a fraction in 0.0..=1.0)"))?;
    if (0.0..=1.0).contains(&rate) {
        Ok(rate)
    } else {
        Err(format!("`{s}` is outside 0.0..=1.0"))
    }
}

#[derive(Debug, Args)]
pub struct ProgressGateArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// The gate's name — `floor`, `check`, `conform`, … One record per
    /// name: recording again replaces that gate's previous verdict.
    pub name: String,

    /// The verdict the run produced.
    #[arg(long, value_enum)]
    pub status: GateStatusArg,

    /// Free text pinned to the verdict: the failing test, the exit code,
    /// the reason it is stale.
    #[arg(long)]
    pub detail: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProgressSealArgs {
    #[command(flatten)]
    pub common: ProgressCommonArgs,

    /// The files to seal. Each is judged on its own: one refusal does not
    /// abort the rest, and the run exits non-zero if any refused.
    ///
    /// Sealing a file asserts that **every** verdict in it is valid for
    /// the text on disk right now — so a file where only some anchors
    /// were re-derived belongs in no list here; it is refused, and stays
    /// flagged until the whole of it is re-verified.
    #[arg(required = true, value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// The verdicts `vibe progress gate` accepts (PROP-043 §7.2). Declared
/// here rather than in `progress-core`: the core carries no clap
/// dependency (separability law §2). An unlisted value is a clap error,
/// never a silently stored string.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GateStatusArg {
    /// The gate ran and passed.
    Green,
    /// The gate ran and failed.
    Red,
    /// The gate has not been re-run since the corpus changed.
    Stale,
}
