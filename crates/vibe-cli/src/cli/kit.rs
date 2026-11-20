//! Local kit production, separate from installation and user settings.
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct KitArgs {
    #[command(subcommand)]
    pub command: KitCommand,
}

#[derive(Debug, Subcommand)]
pub enum KitCommand {
    /// Write a commented local recipe; never replace an existing entry.
    Init {
        #[arg(long, default_value = "vibe-kit.toml")]
        file: PathBuf,
    },
    /// Validate recipe structure and, by default, actual pinned local inputs.
    Validate {
        #[arg(long)]
        file: PathBuf,
        /// Check structure only; does not claim artifacts exist or are ready.
        #[arg(long)]
        structure_only: bool,
    },
}
