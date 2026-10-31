//! `vibe remap` — the read-only views over the active rows
//! (`PROP-066#commands`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#commands");

use clap::{Args, Subcommand};

/// Show which remap rows are active and why one fetch resolves where it
/// does (PROP-066).
///
/// A remap row replaces **where** an artifact is looked for and nothing
/// else: every digest is still checked exactly as before, so a row can
/// make a fetch fail or succeed and can never change what gets installed.
/// Rows come from `~/.vibe/settings.toml`, `.vibe/settings.local.toml`,
/// the PROP-002 `[[mirror]]` / `[[override]]` tables, and the global
/// `--remap <file>` flag or `VIBE_REMAP`.
#[derive(Debug, Args)]
pub struct RemapArgs {
    #[command(subcommand)]
    pub command: RemapCommand,
}

#[derive(Debug, Subcommand)]
pub enum RemapCommand {
    /// List the rows active for this invocation, by source, with the
    /// locally computed diagnostics of every target.
    List(RemapListArgs),
    /// Print the whole chain for one fetch: the declared origin, every
    /// matching row in winning order, the chosen target and the digest
    /// that will be checked.
    Explain(RemapExplainArgs),
}

#[derive(Debug, Args)]
pub struct RemapListArgs {
    /// Emit the `vibe-remap/1` listing document instead of the human
    /// layout.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RemapExplainArgs {
    /// What is being fetched: a URL, or `sha256:<hex>` / a bare 64-hex
    /// digest for an artifact matched by its digest alone.
    #[arg(value_name = "URL|SHA256")]
    pub what: String,

    /// Narrow the question to one application package's installation,
    /// as `<group>/<name>[@<version>]`, so `scope` rows apply.
    #[arg(long, value_name = "PKGREF")]
    pub package: Option<String>,

    /// Narrow it further to one step of that package, so a
    /// `scope = { package = …, step = … }` row applies.
    #[arg(long, value_name = "ID")]
    pub step: Option<String>,

    /// Emit the `vibe-remap/1` explanation document instead of the human
    /// layout.
    #[arg(long)]
    pub json: bool,
}
