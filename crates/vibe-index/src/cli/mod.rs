//! CLI dispatch — clap-derived `Cli` / `Command` enum + per-subcommand
//! modules. The help-text smoke test (`tests/help_smoke.rs`) asserts
//! against the full dispatch shape, so every subcommand renders help
//! and parses its arguments whatever the caller does next.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use clap::{Parser, Subcommand};

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

Specification: spec://vibevm/modules/vibe-index/PROP-005.";

#[derive(Debug, Parser)]
#[command(
    name = "vibe-index",
    version,
    about = ABOUT,
    long_about = LONG_ABOUT,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialise an empty index data directory.
    Init(init::Args),

    /// (Re)build the index from authoritative package state.
    Reindex(reindex::Args),

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

/// Parse `std::env::args` and dispatch the subcommand.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    dispatch(cli.command)
}

/// Dispatcher exposed for in-process integration tests that build a
/// `Command` value directly. Production callers go through [`run`].
pub fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Init(args) => init::run(args),
        Command::Reindex(args) => reindex::run(args),
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
