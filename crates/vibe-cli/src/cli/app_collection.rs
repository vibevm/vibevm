//! The collection edits of `vibe app` (PROP-059 `#launcher-catalog`).

/// The edits of one collection (PROP-059 `#launcher-catalog`).
#[derive(Debug, clap::Subcommand)]
pub enum CollectionCommand {
    /// Create an empty collection.
    Create {
        name: String,
        /// An icon file for the collection itself.
        #[arg(long, value_name = "PATH")]
        icon: Option<std::path::PathBuf>,
    },
    /// Rename one; it keeps its keys, and stops counting as seeded.
    Rename { old: String, new: String },
    /// Delete one; the launchers themselves stay installed.
    Delete { name: String },
    /// Append the launcher keys it does not hold yet.
    Add {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// Take launcher keys out and remember that you did, so an install of
    /// the same `menu_group` does not put them back.
    Remove {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// The full order: the collection becomes exactly this sequence.
    Order {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// Show every collection, both pinned lists and every override.
    List,
}
