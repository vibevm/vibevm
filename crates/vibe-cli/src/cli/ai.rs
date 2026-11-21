//! Reference-only AI connection metadata grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
#[derive(Debug, clap::Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: AiCommand,
}
#[derive(Debug, clap::Subcommand)]
pub enum AiCommand {
    Setup {
        #[arg(long = "agent")]
        agents: Vec<String>,
        /// Configure metadata for this user launcher alias; never install or retry it.
        #[arg(long, conflicts_with = "agents")]
        from_agent_alias: Option<String>,
        /// Explicitly preview/apply persistent native settings, not metadata bindings.
        #[arg(long, conflicts_with_all = ["mode", "set_default", "scope", "from_agent_alias"])]
        native_settings: bool,
        #[arg(long, value_enum)]
        mode: Option<AiBindingMode>,
        #[arg(long)]
        connection: Option<String>,
        #[arg(long, requires = "scope", conflicts_with_all = ["agents", "from_agent_alias", "mode", "connection"])]
        set_default: Option<String>,
        #[arg(long, value_enum, requires = "set_default")]
        scope: Option<AiDefaultScope>,
        #[arg(long,value_parser=setup_preview_id)]
        apply_preview: Option<String>,
    },
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

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum AiDefaultScope {
    DefaultOnly,
    ManagedOverrides,
}

fn setup_preview_id(raw: &str) -> Result<String, String> {
    if raw
        .strip_prefix("ai-setup/1:sha256:")
        .or_else(|| raw.strip_prefix("ai-default-scope/1:sha256:"))
        .or_else(|| raw.strip_prefix("pi-native-settings/1:sha256:"))
        .or_else(|| raw.strip_prefix("qwen-native-settings/1:sha256:"))
        .or_else(|| raw.strip_prefix("codex-native-settings/1:sha256:"))
        .or_else(|| raw.strip_prefix("opencode-native-settings/1:sha256:"))
        .is_some_and(|h| {
            h.len() == 64
                && h.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        Ok(raw.into())
    } else {
        Err("expected a version1 AI setup preview identifier".into())
    }
}
