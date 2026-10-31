//! The `vibe app` argument surface (PROP-059 `#launcher-operations`,
//! PROP-065 `#plan`, `#describe`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

/// User application operations. Inside a project (the working directory or
/// `--path` holds `vibe.toml`) they act on the project home; `-g` or no
/// project selects the user settings root.
#[derive(Debug, clap::Args)]
pub struct AppArgs {
    #[command(subcommand)]
    pub command: AppCommand,

    /// Selected project directory: its project home `vibevm/vibeopt` is the
    /// application home, and launchers resolve `$project_root` in it.
    #[arg(long, global = true)]
    pub path: Option<std::path::PathBuf>,

    /// Act on the user settings root even inside a project.
    #[arg(short = 'g', long, global = true)]
    pub global: bool,
}

#[derive(Debug, clap::Subcommand)]
pub enum AppCommand {
    /// List every installed application with its status and slot default.
    List,

    /// Show every declared installer and platform option without selecting one.
    Installers {
        package: String,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
    },

    /// Resolve the option, features, conditions and needs of an application
    /// installation and print its steps as run or skip, without downloading
    /// or running anything (PROP-065 `#plan`).
    Plan {
        package: String,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
        /// Select an application platform or distribution ID.
        #[arg(long, value_name = "PLATFORM")]
        platform: Option<String>,
        /// Select an application architecture ID within the selected platform.
        #[arg(long, value_name = "ARCH")]
        arch: Option<String>,
        /// Select one application installer option by its stable ID.
        #[arg(long, value_name = "ID")]
        installer: Option<String>,
        /// Choose a feature by ID; repeatable.
        #[arg(long = "feature", value_name = "ID")]
        feature: Vec<String>,
        /// Leave a feature unchosen; repeatable.
        #[arg(long = "no-feature", value_name = "ID")]
        no_feature: Vec<String>,
    },

    /// Print the declared features and steps of every installer option of a
    /// package as written (PROP-065 `#record`); `--json` prints the
    /// `describe` document.
    Steps {
        package: String,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
    },

    /// Describe a package's whole installation choice: identity, options,
    /// results, launchers, features and steps (PROP-065 `#describe`).
    Describe {
        package: String,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
    },

    /// List launchers of an installed application.
    Launchers { package: String },

    /// Run one launcher by its exact stable ID.
    Run {
        package: String,
        launcher_id: String,
        /// Arguments passed literally after the launcher's declared arguments.
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Make one installation the global default provider of its slot.
    Use { slot: String, key: String },

    /// List the providers of a slot, its default and the commands each provides.
    Alternatives { slot: String },

    /// Run the declared default launcher, or ask in an interactive terminal.
    Start {
        package: String,
        #[arg(long)]
        launcher: Option<String>,
        /// Arguments passed literally after the launcher's declared arguments.
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
