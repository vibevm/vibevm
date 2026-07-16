//! Argument structs for `vibe prefs` — application/user preferences
//! (PROP-040 §8 `#prefs-command`). Split from the `cli` hub along
//! command-family lines; the hub re-exports everything, so
//! `crate::cli::X` paths stay stable.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use std::path::PathBuf;

use clap::{Args, Subcommand};

/// `vibe prefs` — inspect and edit application/user preferences (PROP-040 §8).
///
/// Distinct from `vibe show config`, which remains the project-config view
/// (`vibe.toml`); `vibe prefs` operates on the three-level app-prefs store
/// (`~/.vibe/`, `.vibe/settings.toml`, `.vibe/settings.local.toml`).
#[derive(Debug, Args)]
pub struct PrefsArgs {
    #[command(subcommand)]
    pub command: PrefsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PrefsSubcommand {
    /// Print the resolved value of one key and which layer set it.
    Get(PrefsGetArgs),

    /// Set one key in a file layer (basic write; phase 2.7 enriches with
    /// diff-from-default + comment-preserve).
    Set(PrefsSetArgs),

    /// List every resolved key with its value and origin.
    List(PrefsPathArgs),

    /// Validate every layer against the schema (unknown + deprecated keys).
    Check(PrefsPathArgs),

    /// Rewrite deprecated keys to their `replaced_by` targets in each layer.
    Migrate(PrefsPathArgs),

    /// Print the full per-layer breakdown — the resolved value and every
    /// layer's contribution — for one key or, with no key, for every resolved
    /// key (PROP-040 §8 `#show-origins`).
    #[command(name = "show-origins")]
    ShowOrigins(PrefsOriginsArgs),

    /// Open the interactive settings TUI (PROP-041) — a surface over the
    /// three-level store: browse pages, see where each value comes from, and
    /// (S2) edit per-type fields. Launches when the session is attended.
    Ui(PrefsPathArgs),
}

#[derive(Debug, Args)]
pub struct PrefsGetArgs {
    /// The dotted key to read, e.g. `tree.palette`.
    pub key: String,

    /// Project root used to locate the repo-shared (L2) and user-project (L3)
    /// files. L1 is always the user's `~/.vibe/settings.toml`. Defaults to the
    /// current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct PrefsSetArgs {
    /// The dotted key to write, e.g. `tree.palette`.
    pub key: String,

    /// The value, coerced to a TOML bool/int/float/string/array as parseable;
    /// an unquoted bareword becomes a string.
    pub value: String,

    /// Which file layer to write: `L1` (user-machine), `L2` (repo-shared,
    /// committed), or `L3` (user-project, gitignored). Defaults to `L3` — the
    /// safe, personal layer — so a personal tweak is never committed by
    /// accident. Pass `--layer L1` for a global default.
    #[arg(long, value_name = "L1|L2|L3")]
    pub layer: Option<String>,

    /// Project root used to locate L2/L3. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// Shared `--path` for the no-key subcommands.
#[derive(Debug, Args)]
pub struct PrefsPathArgs {
    /// Project root used to locate L2/L3. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct PrefsOriginsArgs {
    /// One key to break down. Omit for every resolved key.
    pub key: Option<String>,

    /// Project root used to locate L2/L3. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
