//! Argument struct for `vibe query` — the simple-level map search
//! (A5A-MAPSEARCH). Split from the `cli` hub along command-family lines;
//! the hub re-exports it, so `crate::cli::QueryArgs` is the address.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::path::PathBuf;

/// `vibe query` — search the code↔spec map by independent filters, joined
/// with AND, over a hard result ceiling. None of the filters is required;
/// the ones set narrow together. `--json` (the global flag) emits the
/// machine-readable form; the default is a scannable text view. The
/// capability (build fresh → search → render) lives in `vibe-trace`; this
/// command only selects filters and prints — it builds no map of its own.
#[derive(Debug, clap::Args)]
pub struct QueryArgs {
    /// Exact `spec://…#anchor` URI to match. Only spec units carry an
    /// address, so this excludes every code item.
    #[arg(long)]
    pub uri: Option<String>,

    /// Substring of a code item's symbol to match (case-sensitive, like
    /// `grep`). Only code items carry a symbol, so this excludes every spec
    /// unit.
    #[arg(long)]
    pub symbol: Option<String>,

    /// Element kind to match exactly: a code item's `item_kind` (`fn`,
    /// `struct`, `mod`, …) or a spec unit's own kind (`req`, `prop`, …).
    /// The two vocabularies never overlap, so one filter serves both
    /// families.
    #[arg(long)]
    pub kind: Option<String>,

    /// Maximum number of results. Must be at least 1; clamped to a hard
    /// ceiling of 200. There is no unbounded mode — the ceiling is part of
    /// the design, because an answer that does not fit an agent's context
    /// is worthless. Defaults to 50.
    #[arg(long, default_value_t = vibe_trace::search::DEFAULT_LIMIT)]
    pub limit: usize,

    /// Project root. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
