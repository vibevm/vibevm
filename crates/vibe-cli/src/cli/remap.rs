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
    /// Accept a remap package's offer: record it against the package's
    /// current content hash so its rows apply from now on. A later version
    /// of the package is a new offer.
    Accept(RemapPackagesArgs),
    /// Refuse a remap package's offer, so the question is not asked again.
    Reject(RemapPackagesArgs),
    /// Switch an accepted remap package back on without re-accepting it.
    Enable(RemapPackagesArgs),
    /// Switch an accepted remap package off without forgetting the
    /// acceptance or removing the package.
    Disable(RemapPackagesArgs),
    /// Resolve every artifact this project or home declares through the
    /// active rows and ask each winning target whether it answers —
    /// without downloading anything. Exits 1 when any target is
    /// unreachable.
    Test(RemapTestArgs),
    /// Write a remap package, and optionally the artifacts its rows and
    /// this project's lock name, into a directory that is both a
    /// `file://` registry and an artifact tree — for carrying into a
    /// closed network.
    Export(RemapExportArgs),
    /// Install a remap package from a directory written by `export` as an
    /// offer, and place its artifacts into the machine store and the
    /// application download cache.
    Import(RemapImportArgs),
    /// Verify an explicit source for one held project remap without adopting or activating it.
    BindSource(RemapBindSourceArgs),
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

#[derive(Debug, Args)]
pub struct RemapPackagesArgs {
    /// One or more installed remap packages, as `<group>/<name>`.
    #[arg(value_name = "PACKAGE", required = true)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct RemapTestArgs {
    /// Directory of the project whose lock and embedded sources are
    /// walked. Without a `vibe.lock` there the home's installed
    /// application inputs and the distribution manifest are walked
    /// instead.
    #[arg(long, default_value = ".")]
    pub path: std::path::PathBuf,

    /// Emit the `vibe-remap-test/1` report instead of the human layout.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RemapExportArgs {
    /// The installed remap package to write out, as `<group>/<name>`.
    #[arg(value_name = "PACKAGE")]
    pub package: String,

    /// The directory to write. It becomes a `file://` registry holding the
    /// package, so `vibe install --registry <dir>` reads it unchanged.
    #[arg(long, value_name = "DIR")]
    pub out: std::path::PathBuf,

    /// Also write every artifact the rows and this project's lock name,
    /// laid out as `<dir>/artifacts/<sha256>/<filename>` with a
    /// `MANIFEST.json` listing them. `vibe cache import <dir>` places them.
    #[arg(long)]
    pub with_artifacts: bool,

    /// Directory of the project whose lock names the artifacts to carry.
    #[arg(long, default_value = ".")]
    pub path: std::path::PathBuf,
}

#[derive(Debug, Args)]
pub struct RemapImportArgs {
    /// A directory written by `vibe remap export`.
    #[arg(value_name = "DIR")]
    pub dir: std::path::PathBuf,
}

#[derive(Debug, Args)]
pub struct RemapBindSourceArgs {
    /// Held group/name; its version and content hash remain fixed.
    pub package: String,
    /// TOML source binding with canonical source_url and ref/commit for Git.
    #[arg(long, value_name = "FILE")]
    pub source: std::path::PathBuf,
    /// Project or member whose owning workspace holds the remap lock.
    #[arg(long, default_value = ".")]
    pub path: std::path::PathBuf,
}
