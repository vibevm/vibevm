//! `vibe select` — the query-language map search (E-A5B-QUERYLANG). A thin
//! surface over [`vibe_trace::select`]: the capability (parse → build fresh →
//! walk → render) lives in `vibe-trace`, shared one-to-one with the MCP
//! `select` tool, so this command only parses the query string, rejects the
//! one impossible value, and prints — it duplicates no grammar, traversal, or
//! rendering logic.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use anyhow::Result;

use crate::cli::SelectArgs;
use crate::output::Context;

/// Run `vibe select --where "<query>"`: parse the conjunctive predicate query,
/// build the specmap fresh for `path`, walk the bipartite code↔spec graph per
/// `depth`, and return the capped, depth-ordered nodes.
///
/// `--limit 0` is refused outright — the ceiling is hard by design, and `0`
/// must not read as "unbounded" (mirrors `vibe query --limit 0`). The
/// out-of-range upper side is handled by clamping inside the library. A query
/// that fails to parse surfaces the parser's message (it names the offending
/// token and lists the expected) — no silent "match everything".
///
/// The `--json` global flag emits the structured view (printed directly, not
/// wrapped in a vibe envelope, matching `vibe query --json`); the default
/// prints the scannable text view.
pub fn run(ctx: &Context, args: SelectArgs) -> Result<()> {
    if args.limit == 0 {
        anyhow::bail!(
            "`--limit` must be at least 1; there is no unbounded mode. The result ceiling is \
             part of the design (an answer that does not fit an agent's context is worthless) — \
             narrow the query instead. Hard maximum: {}.",
            vibe_trace::search::MAX_LIMIT
        );
    }
    let parsed = vibe_trace::select::parse(&args.r#where)?;
    let out = vibe_trace::select::query(&args.path, &parsed, args.limit)?;
    match vibe_trace::select::render(&out, &parsed, &args.r#where, ctx.is_json()) {
        vibe_trace::select::SelectView::Text(text) => print!("{text}"),
        vibe_trace::select::SelectView::Json(value) => {
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
    fn select_subcommand_parses_where_and_the_defaults() {
        let cli = Cli::try_parse_from(["vibe", "select", "--where", "kind:fn has:implements"])
            .expect("parse `vibe select`");
        let Command::Select(args) = cli.command else {
            panic!("argv did not parse to `select`");
        };
        assert_eq!(args.r#where, "kind:fn has:implements");
        assert_eq!(args.limit, vibe_trace::search::DEFAULT_LIMIT);
        assert_eq!(args.path.to_string_lossy(), ".");
        assert!(!cli.json, "the default is the text view");
    }

    #[test]
    fn limit_and_path_parse_together() {
        let cli = Cli::try_parse_from([
            "vibe",
            "select",
            "--where",
            "symbol:Config depth:1",
            "--limit",
            "5",
            "--path",
            "/tmp/p",
        ])
        .expect("parse `vibe select` with limit and path");
        let Command::Select(args) = cli.command else {
            panic!("argv did not parse to `select`");
        };
        assert_eq!(args.r#where, "symbol:Config depth:1");
        assert_eq!(args.limit, 5);
        assert_eq!(args.path.to_string_lossy(), "/tmp/p");
    }

    #[test]
    fn the_global_json_flag_reaches_select() {
        let cli = Cli::try_parse_from(["vibe", "--json", "select", "--where", "uri:spec://x/Y#z"])
            .expect("parse `vibe --json select --where …`");
        let Command::Select(args) = cli.command else {
            panic!("argv did not parse to `select`");
        };
        assert_eq!(args.r#where, "uri:spec://x/Y#z");
        assert!(cli.json, "--json reaches the select command");
    }
}
