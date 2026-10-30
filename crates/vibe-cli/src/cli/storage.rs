//! Grammar for `vibe storage`, the storage libraries of the user settings
//! root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries");

/// Arguments for `vibe storage`.
#[derive(Debug, clap::Args)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub command: StorageCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum StorageCommand {
    /// List every library with its availability and its installations.
    List,

    /// Create a fresh library in a new or empty directory and list it.
    Add {
        path: std::path::PathBuf,
        /// The library id; defaults to the directory name.
        #[arg(long)]
        id: Option<String>,
    },

    /// List an existing library whose marker id is not configured yet.
    Attach { path: std::path::PathBuf },

    /// Unlist a library without deleting it; its installations become unavailable.
    Detach { id: String },

    /// Relocate one portable owned installation into another library.
    Move {
        key: String,
        /// The destination library id.
        #[arg(long)]
        to: String,
    },

    /// List directories under the available libraries that no installation
    /// references; `--delete` removes them.
    Gc {
        #[arg(long)]
        delete: bool,
    },
}
