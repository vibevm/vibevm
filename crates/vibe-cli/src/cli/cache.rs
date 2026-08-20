//! Argument structs for `vibe cache …` (PROP-010 §2.8).
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CacheSubcommand {
    /// Print the machine store root (`~/.vibe/cache/`) — where fetched
    /// package content lives. Works anywhere, project or not: the store
    /// is machine-global and moves with `$VIBE_SETTINGS`.
    Path,

    /// List the packages and versions present in the machine store —
    /// the offline-resolvable inventory. Works anywhere.
    List,

    /// Deliberately pre-warm the store: resolve the named package(s)
    /// and their whole dependency closure and fetch every node into
    /// the machine store — nothing is materialised into any project
    /// (no `vibe.lock`, no `vibedeps/`, `vibe.toml` untouched). Inside
    /// a project, the project's `[[registry]]` entries are the source;
    /// outside one, the user-level `~/.vibe/registry.toml` registries.
    /// The "I am about to go offline, pull down what I will need"
    /// workflow (PROP-010 §2.8).
    Add(CacheAddArgs),

    /// Reclaim store space — an explicit operator action, never a
    /// surprise and never automatic (PROP-010 §2.1). Requires exactly
    /// one target: `--all`, `--package`, or `--older-than`.
    Clean(CacheCleanArgs),

    /// The integrity sweep (PROP-010 §2.8 CMD-CHECK): walk every
    /// store entry, recompute its content hash, and compare against
    /// the recorded `v<version>.sha256` sidecar. **The only place the
    /// store is fully re-hashed** — verification is a command an
    /// operator runs, not a tax every install pays. Reports `ok` /
    /// `mismatch` (identity, path, both hashes) / `unrecorded`
    /// (no sidecar); exits non-zero when anything is not ok.
    /// `--repair` then fixes what the sweep found, cheapest first:
    /// unrecorded entries get a sidecar recorded from their current
    /// bytes; mismatched entries are re-fetched at the SAME version
    /// (`REPAIR-DOES-NOT-PULL` — never advanced, that is `vibe
    /// update`'s job).
    Check(CacheCheckArgs),
}

#[derive(Debug, clap::Args)]
pub struct CacheAddArgs {
    /// Package references, each `<group>/<name>[@<version>]` (or the
    /// `<kind>:<name>` short form, qualified against the configured
    /// registries like `vibe install` does).
    #[arg(required = true)]
    pub packages: Vec<String>,

    /// Where to look for a project (`vibe.toml`). Inside a project,
    /// its registries are the source; without one, the user-level
    /// registries serve (PROP-010 §2.4).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct CacheCleanArgs {
    /// Remove EVERY entry in the store. Confirm-gated (the established
    /// confirmation contract: `--assume-yes` / `--unattended` /
    /// `--json` imply yes; a non-TTY without them is a hard error).
    #[arg(long, conflicts_with_all = ["package", "older_than"])]
    pub all: bool,

    /// Remove one package: the whole name (`org.vibevm/wal`) or a
    /// single version (`org.vibevm/wal@0.2.0`). Removing the last
    /// version prunes the name's directory — a deleted package leaves
    /// no residue naming it.
    #[arg(
        long = "package",
        value_name = "GROUP/NAME[@VERSION]",
        conflicts_with = "older_than"
    )]
    pub package: Option<String>,

    /// Remove entries whose store directory is older than this many
    /// days (by the entry directory's mtime).
    #[arg(long = "older-than", value_name = "DAYS")]
    pub older_than: Option<u64>,

    /// Skip the `--all` confirmation prompt (non-interactive envs).
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct CacheCheckArgs {
    /// Fix what the sweep finds, cheapest first: record a sidecar for
    /// unrecorded entries (from their current bytes); re-fetch
    /// mismatched entries at the SAME version from the configured
    /// registries (inside a project — the project's; outside — the
    /// user-level ones).
    #[arg(long)]
    pub repair: bool,

    /// Where to look for a project (`vibe.toml`) — its registries
    /// serve `--repair` re-fetches; without one, the user-level
    /// registries serve (PROP-010 §2.4).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
