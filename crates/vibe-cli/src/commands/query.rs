//! `vibe query` — the simple-level map search (A5A-MAPSEARCH). A thin
//! surface over [`vibe_trace::search`]: the capability (config-load → build
//! fresh → search → render) lives in `vibe-trace`, shared one-to-one with
//! the MCP `query` tool, so this command only selects the filters, rejects
//! the one impossible value, and prints — it duplicates no build-or-search
//! logic.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use anyhow::{Result, bail};

use crate::cli::QueryArgs;
use crate::output::Context;

/// Run `vibe query`: build the specmap fresh for `path` and return the nodes
/// matching the AND-joined filters, capped at `limit`.
///
/// `--limit 0` is refused outright — the ceiling is hard by design, and `0`
/// must not read as "unbounded" (УТОЧНИ-2). The out-of-range upper side is
/// handled by clamping inside the library.
///
/// The `--json` global flag emits the structured view (the value an agent
/// consumes, printed directly — not wrapped in a vibe envelope, matching
/// `vibe explain --json`); the default prints the scannable text view.
pub fn run(ctx: &Context, args: QueryArgs) -> Result<()> {
    if args.limit == 0 {
        bail!(
            "`--limit` must be at least 1; there is no unbounded mode. The result ceiling is \
             part of the design (an answer that does not fit an agent's context is worthless) — \
             narrow with a filter instead. Hard maximum: {}.",
            vibe_trace::search::MAX_LIMIT
        );
    }
    let filters = vibe_trace::search::Filters {
        uri: args.uri,
        symbol: args.symbol,
        kind: args.kind,
        limit: args.limit,
    };
    let out = vibe_trace::search::query(&args.path, &filters)?;
    match vibe_trace::search::render(&out, &filters, ctx.is_json()) {
        vibe_trace::search::SearchView::Text(text) => print!("{text}"),
        vibe_trace::search::SearchView::Json(value) => {
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Command};
    use clap::Parser;

    #[test]
    fn query_subcommand_parses_a_filter_and_the_defaults() {
        let cli =
            Cli::try_parse_from(["vibe", "query", "--kind", "fn"]).expect("parse `vibe query`");
        let Command::Query(args) = cli.command else {
            panic!("argv did not parse to `query`");
        };
        assert_eq!(args.kind.as_deref(), Some("fn"));
        assert!(args.uri.is_none() && args.symbol.is_none());
        assert_eq!(args.limit, vibe_trace::search::DEFAULT_LIMIT);
        assert_eq!(args.path.to_string_lossy(), ".");
        assert!(!cli.json, "the default is the text view");
    }

    #[test]
    fn multiple_filters_and_limit_parse_together() {
        let cli = Cli::try_parse_from([
            "vibe", "query", "--kind", "struct", "--symbol", "Config", "--limit", "5",
        ])
        .expect("parse `vibe query` with several filters");
        let Command::Query(args) = cli.command else {
            panic!("argv did not parse to `query`");
        };
        assert_eq!(args.kind.as_deref(), Some("struct"));
        assert_eq!(args.symbol.as_deref(), Some("Config"));
        assert_eq!(args.limit, 5);
    }

    #[test]
    fn the_global_json_flag_and_uri_reach_query() {
        let cli = Cli::try_parse_from(["vibe", "--json", "query", "--uri", "spec://x/Y#z"])
            .expect("parse `vibe --json query --uri …`");
        let Command::Query(args) = cli.command else {
            panic!("argv did not parse to `query`");
        };
        assert_eq!(args.uri.as_deref(), Some("spec://x/Y#z"));
        assert!(cli.json, "--json reaches the query command");
    }
}
