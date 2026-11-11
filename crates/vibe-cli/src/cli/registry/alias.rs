//! Arguments for `vibe registry alias …` — the short spellings a
//! directory-layout registry offers for its own packages
//! (PROP-008 `#short-name`).

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
    /// Offer `<alias>` as a short spelling of `<group>/<name>`.
    Add {
        /// The short spelling. It obeys the package-name grammar,
        /// because it is typed exactly where a package name is typed,
        /// and it may not be a name this registry publishes.
        alias: String,
        /// The package it stands for — fully qualified, never versioned,
        /// and carried by this very registry.
        target: String,
        /// Re-point a spelling that already means something else.
        #[arg(long)]
        replace: bool,
    },
    /// Withdraw the spelling `<alias>`.
    Remove { alias: String },
    /// Print the table, one `alias -> group/name` per line.
    List,
}
