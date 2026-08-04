//! `vibe explain` — the host traceability-explain command
//! (V3-EXPLAIN-SURFACE). A thin surface over [`vibe_trace::explain`] (and its
//! fragment sibling [`vibe_trace::fragment`]): the capability (config-load →
//! build fresh → render) lives in `vibe-trace`, shared one-to-one with the
//! MCP `explain` tool, so this command only selects the form and prints — it
//! duplicates no build-or-render logic.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use anyhow::Result;

use crate::cli::ExplainArgs;
use crate::output::Context;

/// Run `vibe explain <target>`: build the specmap fresh for `path` and
/// render the subgraph around `target`.
///
/// The `--json` global flag selects the raw subgraph (matching the stack's
/// `trace explain --json` byte-for-byte — the value IS the structured
/// answer an agent consumes, so it is printed directly, not wrapped in a
/// vibe envelope). The default prints the deterministic text view.
///
/// `--fragment` switches to the code-text view ([`vibe_trace::fragment`]):
/// the source of the element behind `target` plus a drift verdict. Drift is
/// information, not a refusal, so the exit code stays zero — the fragment is
/// printed either way.
pub fn run(ctx: &Context, args: ExplainArgs) -> Result<()> {
    if args.fragment {
        let out = vibe_trace::fragment(&args.path, &args.target, ctx.is_json())?;
        match out {
            vibe_trace::Fragment::Text(text) => print!("{text}"),
            vibe_trace::Fragment::Json(value) => {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
        return Ok(());
    }
    let out = vibe_trace::explain(&args.path, &args.target, ctx.is_json())?;
    match out {
        vibe_trace::Explain::Text(text) => print!("{text}"),
        vibe_trace::Explain::Json(value) => {
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
    fn explain_subcommand_parses_target_and_default_path() {
        let cli = Cli::try_parse_from(["vibe", "explain", "spec://demo/D#x"])
            .expect("parse `vibe explain <target>`");
        let Command::Explain(args) = cli.command else {
            panic!("argv did not parse to `explain`");
        };
        assert_eq!(args.target, "spec://demo/D#x");
        assert_eq!(args.path.to_string_lossy(), ".");
        assert!(!cli.json, "the default is the text view");
    }

    #[test]
    fn the_global_json_flag_selects_the_json_form() {
        let cli = Cli::try_parse_from(["vibe", "--json", "explain", "x::f"])
            .expect("parse `vibe --json explain <target>`");
        let Command::Explain(args) = cli.command else {
            panic!("argv did not parse to `explain`");
        };
        assert_eq!(args.target, "x::f");
        assert!(cli.json, "--json reaches the explain command");
    }

    /// `--path` overrides the default root.
    #[test]
    fn path_flag_overrides_the_default_root() {
        let cli = Cli::try_parse_from(["vibe", "explain", "t", "--path", "/tmp/p"])
            .expect("parse `vibe explain <target> --path <root>`");
        let Command::Explain(args) = cli.command else {
            panic!("argv did not parse to `explain`");
        };
        assert_eq!(args.target, "t");
        assert_eq!(args.path.to_string_lossy(), "/tmp/p");
    }

    /// `--fragment` selects the code-text view over the subgraph (V7).
    #[test]
    fn the_fragment_flag_selects_the_fragment_view() {
        let cli = Cli::try_parse_from(["vibe", "explain", "x::f", "--fragment"])
            .expect("parse `vibe explain <target> --fragment`");
        let Command::Explain(args) = cli.command else {
            panic!("argv did not parse to `explain`");
        };
        assert!(args.fragment, "--fragment reaches the explain command");
    }
}
