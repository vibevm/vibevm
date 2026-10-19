//! Grammar for `vibe app alternatives`.

#[derive(Debug, clap::Args)]
pub struct AlternativesArgs {
    /// Slot to list when no operation follows.
    pub slot: Option<String>,
    #[command(subcommand)]
    pub command: Option<AlternativesCommand>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum AlternativeShell {
    Posix,
    Powershell,
    Cmd,
}

impl AlternativeShell {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Posix => "posix",
            Self::Powershell => "powershell",
            Self::Cmd => "cmd",
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum AlternativesCommand {
    Select {
        slot: String,
        installation: String,
    },
    Choose {
        slot: String,
    },
    Use {
        slot: String,
        installation: String,
        #[arg(long, value_enum)]
        shell: Option<AlternativeShell>,
    },
    Clear {
        slot: Option<String>,
        #[arg(long, conflicts_with = "slot")]
        all: bool,
        #[arg(long, value_enum)]
        shell: Option<AlternativeShell>,
    },
    Shell {
        slot: String,
        installation: String,
    },
    Run {
        slot: String,
        command: String,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Start the selected installation's default or named authored launcher.
    Start {
        slot: String,
        #[arg(long)]
        launcher: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
