//! Reference-only AI connection metadata grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
#[derive(Debug, clap::Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: AiCommand,
}
#[derive(Debug, clap::Subcommand)]
pub enum AiCommand {
    Connections {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
}
#[derive(Debug, clap::Subcommand)]
pub enum ConnectionCommand {
    List,
    Details {
        id: String,
    },
    Add {
        #[arg(long)]
        label: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        api_kind: String,
        #[arg(long)]
        default_model: String,
        #[arg(long)]
        secret_ref: String,
    },
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
