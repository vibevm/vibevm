//! CLI dispatch — clap-derived `Cli` / `Command` enum + per-subcommand
//! modules. The help-text smoke test (`tests/help_smoke.rs`) asserts
//! against the full dispatch shape, so every subcommand renders help
//! and parses its arguments whatever the caller does next.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::error::{Error, Result};
use crate::lock::ServerLock;

pub mod add;
pub mod bury;
pub mod capabilities;
pub mod config;
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
pub mod yank;

/// Every CLI-mode verb that WRITES refuses while a server holds the data
/// directory: the server is the single writer in its mode (§2.9), and two
/// writers over one catalog is the state the lock exists to make
/// impossible.
///
/// It lives here rather than beside each verb because it is a property of
/// the dispatch surface, not of any one command — and because the copies
/// had reached three, which is where "one idiom per operation" stops
/// being satisfied by repetition. All four writing verbs call it —
/// `add`, `remove`, `yank`, `bury` — and every future one calls it
/// instead of adding a fifth copy.
pub(crate) fn refuse_if_server_running(data_dir: &std::path::Path) -> Result<()> {
    if let Some(pid) = ServerLock::read_pid(data_dir) {
        return Err(Error::InvalidInput(format!(
            "a vibe-index server is running on this data dir (PID {pid}). \
             Use the HTTP API or stop the server first."
        )));
    }
    Ok(())
}

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

    /// Parse a ladder value for this member — the env rung
    /// (`VIBE_INDEX_LOG`) and the config-file key take the flag's
    /// closed vocabulary. Trimmed, case-insensitive: the same
    /// tolerance the flag has. Returns `None` for anything outside
    /// the set; the ladder's callers turn that into a loud refusal.
    pub fn parse_member(raw: &str) -> Option<LogLevel> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" => Some(LogLevel::Off),
            "error" => Some(LogLevel::Error),
            "warn" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            "trace" => Some(LogLevel::Trace),
            _ => None,
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
    /// Logging level for this run — the top rung of the config ladder
    /// (`--log-level` > `VIBE_INDEX_LOG` / `VIBE_LOG` >
    /// `<data-dir>/state/config.toml` > the built-in `warn`). Passing
    /// the flag beats every other source; `vibe-index config
    /// <data-dir>` names the source behind every effective value.
    #[arg(long, global = true, value_enum)]
    pub log_level: Option<LogLevel>,

    #[command(subcommand)]
    pub command: Command,
}

/// The full clap tree as the derive itself declares it — the very
/// object the binary renders `--help` from. The help smoke test
/// (`tests/help_smoke.rs`) iterates this (`get_subcommands()`)
/// instead of a hand-maintained name list, so a verb added to
/// [`Command`] joins the smoke by itself and a verb that stops
/// appearing in `--help`, or stops rendering, turns the gate red.
/// The hand-list this replaces could only see names someone
/// remembered to re-type: `yank` shipped in `--help` with the list
/// none the wiser (BACKLOG B-094 — "a mark without which a
/// subcommand can be added is a norm with no checker").
pub fn command() -> clap::Command {
    Cli::command()
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

    /// Yank one version of a package: the entry stays, fresh
    /// resolution stops choosing it.
    Yank(yank::Args),

    /// Close a bare name for every group, leaving a tombstone that
    /// names why it is gone and, when there is one, where to move.
    Bury(bury::Args),

    /// Recompute file hashes and check `repomd.json` integrity.
    Verify(verify::Args),

    /// Dump the entire index to stdout.
    Dump(dump::Args),

    /// Run the HTTP server.
    Serve(serve::Args),

    /// Gracefully stop a running server (PID-based).
    Stop(stop::Args),

    /// Print effective configuration values with the source of each
    /// (the config ladder's visible-source verb).
    Config(config::Args),
}

impl Command {
    /// The data directory this invocation addresses — the required
    /// positional every verb carries (`##CLI-SURFACE`). The config
    /// ladder's file rung lives inside it (`<data-dir>/state/config.toml`),
    /// which is why `main` asks for this between parsing and resolving
    /// anything.
    pub fn data_dir(&self) -> Option<&std::path::Path> {
        match self {
            Command::Init(a) => Some(&a.data_dir),
            Command::Reindex(a) => Some(&a.data_dir),
            Command::RescanOrg(a) => Some(&a.data_dir),
            Command::Get(a) => Some(&a.data_dir),
            Command::List(a) => Some(&a.data_dir),
            Command::Search(a) => Some(&a.data_dir),
            Command::Capabilities(a) => Some(&a.data_dir),
            Command::Purls(a) => Some(&a.data_dir),
            Command::Outdated(a) => Some(&a.data_dir),
            Command::Add(a) => Some(&a.data_dir),
            Command::Remove(a) => Some(&a.data_dir),
            Command::Yank(a) => Some(&a.data_dir),
            Command::Bury(a) => Some(&a.data_dir),
            Command::Verify(a) => Some(&a.data_dir),
            Command::Dump(a) => Some(&a.data_dir),
            Command::Serve(a) => Some(&a.data_dir),
            Command::Stop(a) => Some(&a.data_dir),
            Command::Config(a) => Some(&a.data_dir),
        }
    }
}

/// Dispatcher exposed for in-process integration tests that build a
/// `Command` value directly. Production callers arrive from `main`:
/// it parses `Cli`, loads the config ladder, resolves the logging
/// member, installs the tracing subscriber, then hands `cli.command`
/// plus the global `--log-level` flag here — the flag is the ladder's
/// top rung, and the `config` verb shows it as the value's source.
pub fn dispatch(command: Command, log_level: Option<LogLevel>) -> Result<()> {
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
        Command::Yank(args) => yank::run(args),
        Command::Bury(args) => bury::run(args),
        Command::Verify(args) => verify::run(args),
        Command::Dump(args) => dump::run(args),
        Command::Serve(args) => serve::run(args),
        Command::Stop(args) => stop::run(args),
        Command::Config(args) => config::run(args, log_level),
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
