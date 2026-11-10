//! The argument surface of `vibe app extras` (PROP-075 `#SLOT-EXTRAS`):
//! the user-facing name of the ecosystem adapter's verbs.
//!
//! The family is named after the user's concept while the declaration is
//! named after the mechanism, and `self-upgrade` sits here rather than in
//! a family of its own because it is the fourth verb of the same adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

/// `vibe app extras <verb>`.
#[derive(Debug, clap::Args)]
pub struct AppExtrasArgs {
    #[command(subcommand)]
    pub command: ExtrasCommand,
}

/// The shared flags of every extras verb: which adapter, and where the
/// version's package tree is resolved from — an adapter verb is the
/// package's own script, so its tree must be reachable.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct ExtrasTargetArgs {
    /// Act on this ecosystem adapter only, by the `id` the package
    /// declares; omitted, every declared adapter is taken.
    #[arg(long, value_name = "ID")]
    pub ecosystem: Option<String>,

    /// Resolve the version's package from this local registry instead of
    /// the configured registries.
    #[arg(long)]
    pub registry: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Subcommand)]
pub enum ExtrasCommand {
    /// List what the slot's current version carries in its own ecosystem:
    /// the author's recommended items with their chosen state, the list
    /// those make, and what the adapter's `list` verb observes in the
    /// installed version.
    List {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Uninstall items from the slot's current version through the
    /// adapter's `remove` verb. The items are opaque: they are handed to
    /// the verb as separate argv elements, exactly as written.
    ///
    /// Removing an item is not the same act as dropping it from a list.
    Remove {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The items to uninstall, verbatim; at least one.
        #[arg(required = true, value_name = "ITEM")]
        items: Vec<String>,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Upgrade the bundled package manager of the slot's current version
    /// through the adapter's `self_upgrade` verb. It takes no items.
    SelfUpgrade {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },
}
