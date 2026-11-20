//! Reference-only AI connection metadata grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
#[derive(Debug, clap::Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: AiCommand,
}
#[derive(Debug, clap::Subcommand)]
pub enum AiCommand {
    Bindings {
        #[command(subcommand)]
        command: AiBindingCommand,
    },
    Connections {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
}
#[derive(Debug, clap::Subcommand)]
pub enum ConnectionCommand {
    /// Select metadata only; does not activate any harness.
    Default {
        #[command(subcommand)]
        command: ConnectionDefaultCommand,
    },
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

#[derive(Debug, clap::Subcommand)]
pub enum ConnectionDefaultCommand {
    Show,
    Set {
        id: String,
        #[arg(long)]
        expect_store: u64,
    },
    Clear {
        #[arg(long)]
        expect_store: u64,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum AiBindingCommand {
    Show {
        harness: String,
    },
    Set {
        harness: String,
        #[arg(long, value_enum)]
        mode: AiBindingMode,
        #[arg(long)]
        connection: Option<String>,
        #[arg(long)]
        expect_store: u64,
    },
}
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum AiBindingMode {
    Native,
    InheritDefault,
    Connection,
}
