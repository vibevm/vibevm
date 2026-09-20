use std::path::PathBuf;

/// Arguments for `vibe run <command> [-- <args>...]`.
#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    /// The host-owned `[[command]].id` to invoke.
    pub command: String,

    /// Directory of the selected project/workspace node.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Arguments handed to the selected command unchanged.
    #[arg(last = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
