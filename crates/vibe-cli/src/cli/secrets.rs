//! Generic keyring grammar; values never occur in arguments.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct SecretsArgs {
    #[command(subcommand)]
    pub command: Option<SecretsCommand>,
}
#[derive(Debug, Subcommand)]
pub enum SecretsCommand {
    /// List metadata and optimistic tokens, never values.
    List,
    /// Observe one item without reading its value.
    Details { id: String },
    /// Create a new opaque item; stdin bytes are preserved exactly.
    Create {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        label: String,
        #[arg(long, conflicts_with = "value_file")]
        value_stdin: bool,
        /// Read exact bytes from an owned 0600 file in an owned 0700 directory.
        #[arg(long, conflicts_with = "value_stdin")]
        value_file: Option<std::path::PathBuf>,
    },
    /// Change only a label using the observed snapshot tokens.
    Edit {
        id: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        expect_store: u64,
        #[arg(long)]
        expect_current: String,
    },
    /// Replace a value without changing the logical ID.
    Rotate {
        id: String,
        #[arg(long)]
        expect_current: String,
        #[arg(long, conflicts_with = "value_file")]
        value_stdin: bool,
        /// Read exact bytes from an owned 0600 file in an owned 0700 directory.
        #[arg(long, conflicts_with = "value_stdin")]
        value_file: Option<std::path::PathBuf>,
    },
    /// Logical removal; unknown references refuse, even with --yes.
    Remove {
        id: String,
        #[arg(long)]
        expect_store: u64,
        #[arg(long)]
        expect_current: String,
        #[arg(long)]
        yes: bool,
    },
}
