//! CLI dispatch — clap-derived `Cli` / `Command` enum + per-subcommand
//! modules. The help-text smoke test (`tests/help_smoke.rs`) asserts
//! against the full dispatch shape, so every subcommand renders help
//! and parses its arguments whatever the caller does next.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use clap::{Parser, Subcommand, ValueEnum};

use crate::error::Result;

pub mod add;
pub mod capabilities;
pub mod dump;
pub mod get;
pub mod init;
pub mod kinds;
pub mod list;
pub mod outdated;
pub mod purls;
pub mod reindex;
pub mod remove;
pub mod rescan_org;
pub mod search;
pub mod serve;
pub mod stop;
pub mod verify;

const ABOUT: &str = "Standalone package index utility for vibevm-shaped registries.";

const LONG_ABOUT: &str = "Standalone package index utility for vibevm-shaped registries.

vibe-index maintains an opt-in per-org metadata index alongside (or
near) the package repos that make up a vibevm registry. It runs in two
modes:

  * CLI mode (default — every subcommand except `serve`) operates
    directly on a data directory of index files. Reads on-disk state,
    mutates, writes back atomically. Suited for scripted `reindex`
    invocations, post-publish hooks, CI pipelines.

  * Server mode (`vibe-index serve`) boots an axum HTTP server. The
    index is held in RAM and persisted to disk on every mutation.
    Single-writer; reads open, writes guarded by bearer-token auth.

Specification: spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005.";

/// The operator's coarse dial for `vibe-index`'s one logging lever.
///
/// A closed set, not an `EnvFilter` directive string: the directive
/// language belongs to `VIBE_LOG`, which stays the full-power lever, and a
/// flag that also spoke it would be a second legal spelling of one thing.
/// `off` is a member because `VIBE_LOG=off` is legal and a flag that cannot
/// say what the variable can is an asymmetry nobody could explain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// The `EnvFilter` directive this dial stands for — the exact string
    /// written into `VIBE_LOG` before the subscriber reads it.
    pub fn as_filter(self) -> &'static str {
        match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "vibe-index",
    version,
    about = ABOUT,
    long_about = LONG_ABOUT,
)]
pub struct Cli {
    /// Logging level for this run. Folds into the one lever `VIBE_LOG`,
    /// which the subscriber reads exactly once at start-up; passing the
    /// flag SETS that variable, so the process environment always
    /// explains the output an operator sees.
    #[arg(long, global = true, value_enum)]
    pub log_level: Option<LogLevel>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialise an empty index data directory.
    Init(init::Args),

    /// (Re)build the index from authoritative package state.
    Reindex(reindex::Args),

    /// Re-enumerate the org unconditionally and refresh the org-image
    /// cache (ignores the cache + validator).
    RescanOrg(rescan_org::Args),

    /// Read one package entry from the index.
    Get(get::Args),

    /// List packages in the index.
    List(list::Args),

    /// Full-text search across the index.
    Search(search::Args),

    /// List packages providing a given capability.
    Capabilities(capabilities::Args),

    /// List packages whose `describes` matches a given PURL.
    Purls(purls::Args),

    /// Compare a `vibe.lock` against the index and report outdated entries.
    Outdated(outdated::Args),

    /// Insert/upsert a single index entry from a `vibe.toml` manifest.
    Add(add::Args),

    /// Remove one or all versions of a package from the index.
    Remove(remove::Args),

    /// Recompute file hashes and check `repomd.json` integrity.
    Verify(verify::Args),

    /// Dump the entire index to stdout.
    Dump(dump::Args),

    /// Run the HTTP server.
    Serve(serve::Args),

    /// Gracefully stop a running server (PID-based).
    Stop(stop::Args),
}

/// Dispatcher exposed for in-process integration tests that build a
/// `Command` value directly. Production callers arrive from `main`:
/// it parses `Cli`, folds `--log-level` into `VIBE_LOG`, installs the
/// tracing subscriber, then hands `cli.command` here.
pub fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Init(args) => init::run(args),
        Command::Reindex(args) => reindex::run(args),
        Command::RescanOrg(args) => rescan_org::run(args),
        Command::Get(args) => get::run(args),
        Command::List(args) => list::run(args),
        Command::Search(args) => search::run(args),
        Command::Capabilities(args) => capabilities::run(args),
        Command::Purls(args) => purls::run(args),
        Command::Outdated(args) => outdated::run(args),
        Command::Add(args) => add::run(args),
        Command::Remove(args) => remove::run(args),
        Command::Verify(args) => verify::run(args),
        Command::Dump(args) => dump::run(args),
        Command::Serve(args) => serve::run(args),
        Command::Stop(args) => stop::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dial_maps_to_its_own_filter_string() {
        let all = [
            (LogLevel::Off, "off"),
            (LogLevel::Error, "error"),
            (LogLevel::Warn, "warn"),
            (LogLevel::Info, "info"),
            (LogLevel::Debug, "debug"),
            (LogLevel::Trace, "trace"),
        ];
        for (level, directive) in all {
            assert_eq!(level.as_filter(), directive);
        }
        // Six dials, six distinct directives — the fold must not
        // collapse two knobs onto one filter.
        let mut directives: Vec<&str> = all.iter().map(|(l, _)| l.as_filter()).collect();
        directives.sort_unstable();
        directives.dedup();
        assert_eq!(directives.len(), all.len());
    }

    #[test]
    fn log_level_parses_before_the_subcommand() {
        let cli = Cli::try_parse_from(["vibe-index", "--log-level", "debug", "dump", "/tmp/idx"])
            .expect("the global flag must parse before the subcommand");
        assert_eq!(cli.log_level, Some(LogLevel::Debug));
    }

    #[test]
    fn log_level_parses_after_the_subcommand() {
        // `global = true` is load-bearing: the operator's form
        // `vibe-index <sub> … --log-level debug` must parse, and would
        // be a clap parse error without it.
        let cli = Cli::try_parse_from(["vibe-index", "dump", "/tmp/idx", "--log-level", "debug"])
            .expect("the global flag must parse after the subcommand");
        assert_eq!(cli.log_level, Some(LogLevel::Debug));
    }

    #[test]
    fn absent_flag_leaves_the_lever_alone() {
        let cli = Cli::try_parse_from(["vibe-index", "dump", "/tmp/idx"])
            .expect("a plain invocation must parse");
        assert_eq!(cli.log_level, None);
    }
}
