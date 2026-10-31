//! Grammar for `vibe agent`: the agent hooks of the user settings root
//! (PROP-064 `#agent-definition`, `#presets`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

/// `KEY=VALUE` of `--env`.
fn env_pair(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((key, value)) if !key.trim().is_empty() => {
            Ok((key.trim().to_owned(), value.to_owned()))
        }
        _ => Err(format!("`{text}` is not KEY=VALUE")),
    }
}

/// Arguments for `vibe agent`.
#[derive(Debug, clap::Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum AgentCommand {
    /// List the agent definitions of agents.toml, the default, the trusted
    /// groups and the external registration of this process.
    List,

    /// List the built-in presets with their command and argument templates.
    Presets,

    /// Make a definition the default; a preset name is first written as a
    /// definition of that name.
    Use { name: String },

    /// Add an agent definition to agents.toml.
    Add {
        name: String,
        /// The built-in preset whose fields the definition overrides.
        #[arg(long)]
        preset: Option<String>,
        /// The executable name or path.
        #[arg(long)]
        command: Option<String>,
        /// One argument of the interactive template; repeatable, in order.
        #[arg(
            long = "interactive-arg",
            value_name = "ARG",
            allow_hyphen_values = true
        )]
        interactive_args: Vec<String>,
        /// One argument of the unattended template; repeatable, in order.
        #[arg(
            long = "unattended-arg",
            value_name = "ARG",
            allow_hyphen_values = true
        )]
        unattended_args: Vec<String>,
        /// The working-directory template (default `{work}`).
        #[arg(long)]
        cwd: Option<String>,
        /// An environment addition; repeatable.
        #[arg(long = "env", value_name = "KEY=VALUE", value_parser = env_pair)]
        env: Vec<(String, String)>,
        /// An argument appended in unattended runs; repeatable.
        #[arg(long = "extra-arg", value_name = "ARG", allow_hyphen_values = true)]
        extra_args: Vec<String>,
        /// Also make it the default.
        #[arg(long)]
        default: bool,
    },

    /// Remove an agent definition; a default naming it is cleared.
    Remove { name: String },
}
