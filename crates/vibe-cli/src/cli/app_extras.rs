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

/// `--no-extras`, on the two verbs that compute a list (PROP-075
/// `#SLOT-EXTRAS`). It is declared per verb rather than shared, because a
/// flag silently accepted where it means nothing is worse than one that is
/// not offered: `remove` and `self-upgrade` compute no list.
///
/// Its companion `--no-recommends` is the one global flag of
/// `PROP-077#NO-RECOMMENDS-FLAG` and is deliberately **not** redefined
/// here — an author's recommendation is one idea with two spellings, and a
/// user who declines it declines both.
#[derive(Debug, Clone, Copy, Default, clap::Args)]
pub struct NoExtrasArgs {
    /// Leave out the `add` list of your own
    /// `[slot.<slot>.extras.<ecosystem>]`, keeping only what the package's
    /// author recommends.
    ///
    /// Your `drop` list is still honoured: declining your extras must
    /// never install *more* than you asked for.
    #[arg(long)]
    pub no_extras: bool,
}

#[derive(Debug, clap::Subcommand)]
pub enum ExtrasCommand {
    /// List what the slot's current version carries in its own ecosystem:
    /// the author's recommended items with their chosen state, your own
    /// `add` and `drop`, the final list those make, and what the adapter's
    /// `list` verb observes in the installed version.
    ///
    /// The final list answers this invocation's own `--no-extras` and
    /// `--no-recommends`, so a listing is an honest preview of the apply.
    List {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        #[command(flatten)]
        decline: NoExtrasArgs,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Install the final list into the slot's current version now: the
    /// author's recommended items that are in the installation's chosen
    /// feature set, minus the ones you dropped, plus your own `add` list.
    ///
    /// This is what runs by itself after `vibe install -g` places a new
    /// version of a slot, and it is the command a failure there names:
    /// nothing was rolled back, so running it again is the whole retry.
    Apply {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        #[command(flatten)]
        decline: NoExtrasArgs,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Add items to your own `[slot.<slot>.extras.<ecosystem>]` list in
    /// `~/.vibe/config.toml`, so every new version of this slot gets them.
    ///
    /// This edits your settings and installs nothing; `vibe app extras
    /// apply <slot>` puts the list into the version you have now. The
    /// items are opaque and are written exactly as you type them.
    Add {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The items to add, verbatim; at least one.
        #[arg(required = true, value_name = "ITEM")]
        items: Vec<String>,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Decline one of the package author's recommended items by its id, in
    /// your own settings.
    ///
    /// A drop names a **recommendation id**, not an item: naming the
    /// author's row rather than re-typing their string is what keeps the
    /// two sides from drifting. `vibe app extras list <slot>` prints the
    /// ids. A drop is honoured in every case, `--no-extras` included.
    ///
    /// Dropping an item from the list is not the same act as uninstalling
    /// it from the version you have — that is `vibe app extras remove`.
    Drop {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The recommendation ids to decline; at least one.
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Append the items of a file to your own `add` list: one item per
    /// line, blank lines and `#` comments ignored.
    ///
    /// That is the format of `$NVM_DIR/default-packages`, and the format an
    /// adapter's own `list` verb writes, so an existing list moves over in
    /// one command.
    Import {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The file to read, one item per line.
        #[arg(value_name = "FILE")]
        file: std::path::PathBuf,
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
