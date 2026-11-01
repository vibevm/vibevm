//! `vibe transfers` (PROP-068 `#verbs`): the downloads of this machine, as
//! the journal under the settings root records them.

/// Arguments of `vibe transfers`.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct TransfersArgs {
    #[command(subcommand)]
    pub command: Option<TransfersCommand>,

    #[command(flatten)]
    pub filter: TransfersFilter,
}

/// What a listing keeps.
#[derive(Debug, Default, Clone, clap::Args)]
pub struct TransfersFilter {
    /// Only history since a duration ago (`90s`, `30m`, `2h`, `7d`) or an
    /// RFC 3339 instant; without it the listing keeps the last 50 entries.
    #[arg(long, value_name = "D")]
    pub since: Option<String>,

    /// Only the transfers of one package (`group/name` or a full key).
    #[arg(long, value_name = "KEY")]
    pub package: Option<String>,

    /// Only one purpose: package-source, archive, native-input,
    /// package-clone, release-bundle, index-file or other.
    #[arg(long, value_name = "P")]
    pub purpose: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
pub enum TransfersCommand {
    /// One line per transfer already active, then one per sample and per
    /// start and end, until standard input closes. It never starts or
    /// stops a transfer.
    Watch,

    /// The history alone.
    History {
        #[command(flatten)]
        filter: TransfersFilter,
    },

    /// Empty the history.
    ClearHistory,
}
