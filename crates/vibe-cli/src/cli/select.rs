//! Argument struct for `vibe select` — the query-language map search
//! (E-A5B-QUERYLANG), the graph-traversal layer over `vibe query`. Split from
//! the `cli` hub along command-family lines, like `query.rs` beside it; the hub
//! re-exports it, so `crate::cli::SelectArgs` is the address.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::path::PathBuf;

/// `vibe select` — search the code↔spec map by a conjunctive predicate
/// query and walk the bipartite graph. `--where` carries the query string
/// (predicates `uri:`/`symbol:`/`kind:`/`scope:`/`has:`/`lacks:`/`depth:`,
/// whitespace-AND-joined); the grammar, traversal, and rendering live in
/// `vibe-trace`, so this command only hands the string over and prints — it
/// builds no map and parses no query of its own. `--json` (the global flag)
/// emits the machine-readable form; the default is a scannable text view.
#[derive(Debug, clap::Args)]
pub struct SelectArgs {
    /// The query: predicates joined by spaces (AND). Each is `name:value` —
    /// `uri:<exact spec:// address>`, `symbol:<code-symbol substring>`,
    /// `kind:<item_kind or spec kind>`, `scope:<spec:// uri prefix>`,
    /// `has:<verb>` / `lacks:<verb>` (`implements|verifies|documents|deviates|
    /// informs`), `depth:<0..3>`. Required: an empty query is an error, not
    /// "everything" (use `vibe query` for an unfiltered slice).
    #[arg(long = "where", value_name = "QUERY")]
    pub r#where: String,

    /// Maximum number of results. Must be at least 1; clamped to a hard
    /// ceiling of 200. There is no unbounded mode — the ceiling is part of
    /// the design, applied AFTER the graph walk. Defaults to 50.
    #[arg(long, default_value_t = vibe_trace::search::DEFAULT_LIMIT)]
    pub limit: usize,

    /// Project root. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
