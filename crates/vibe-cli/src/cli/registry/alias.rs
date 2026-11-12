//! Arguments for `vibe registry alias …` — the short names a
//! directory-layout registry ISSUES for its own packages
//! (PROP-008 `#short-name`).
//!
//! A bare `vibe install <name>` resolves through these rows and
//! through nothing else (`##ISSUED-NAMES-ONLY`), so issuing a
//! package's own name is the ordinary edit rather than a forbidden
//! one.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

/// `vibe registry alias <registry> add|remove|list`.
#[derive(Debug, clap::Args)]
pub struct RegistryAliasArgs {
    /// The `[[registry]].name` whose table to read or edit. Only a
    /// `monorepo` or `localdir` registry carries one — the table lives
    /// in the registry's own `_index/`.
    pub registry: String,

    #[command(subcommand)]
    pub action: RegistryAliasAction,

    /// Project directory (default: current).
    #[arg(long, default_value = ".", value_name = "DIR")]
    pub path: std::path::PathBuf,

    /// Decide and report, write nothing.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, clap::Subcommand)]
pub enum RegistryAliasAction {
    /// Issue `<alias>` as a short name for `<group>/<name>`.
    Add {
        /// The short name. It obeys the package-name grammar, because
        /// it is typed exactly where a package name is typed. It MAY be
        /// the target's own name — `node` for `org.vibevm.node/node` —
        /// and that row is what lets the package be typed bare.
        alias: String,
        /// The package it stands for — fully qualified, never versioned,
        /// and carried by this very registry.
        target: String,
        /// Re-point a spelling that already means something else.
        #[arg(long)]
        replace: bool,
    },
    /// Withdraw the short name `<alias>`. A bare `<alias>` then
    /// resolves to nothing here, including when a package bears it.
    Remove { alias: String },
    /// Print the table, one `alias -> group/name` per line.
    List,
}
