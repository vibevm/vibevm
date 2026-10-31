//! The `vibe app` argument surface (PROP-059 `#launcher-operations`,
//! PROP-065 `#plan`, `#describe`, `#runs`, `#error-codes`) and the
//! structured-installation flags of `install` / `update`.

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

/// `ID=VALUE` of `--set`.
fn parameter_value(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((id, value)) if !id.trim().is_empty() => Ok((id.trim().to_owned(), value.to_owned())),
        _ => Err(format!("`{text}` is not ID=VALUE")),
    }
}

/// The wizard answers shared by `install`, `update` and `app plan`
/// (PROP-065 `#features`, `#profiles`, `#parameters`, `#license`).
#[derive(Debug, Clone, Default, clap::Args)]
pub struct AppChoiceArgs {
    /// Choose an application installer feature by ID; repeatable.
    #[arg(long = "feature", value_name = "ID")]
    pub feature: Vec<String>,

    /// Leave an application installer feature unchosen; repeatable.
    #[arg(long = "no-feature", value_name = "ID")]
    pub no_feature: Vec<String>,

    /// Choose a named feature profile of the installer; `--feature` and
    /// `--no-feature` apply on top of it.
    #[arg(long, value_name = "ID")]
    pub profile: Option<String>,

    /// Give an installer parameter a value; repeatable.
    #[arg(long = "set", value_name = "ID=VALUE", value_parser = parameter_value)]
    pub set: Vec<(String, String)>,

    /// Accept the installer's declared license without being asked.
    #[arg(long)]
    pub accept_license: bool,
}

/// The structured-installation flags of an application install or update
/// (PROP-065 `#features`, `#resume`).
#[derive(Debug, Clone, Default, clap::Args)]
pub struct AppStepArgs {
    #[command(flatten)]
    pub choice: AppChoiceArgs,

    /// Continue the kept application work directory from its first step
    /// that is not done.
    #[arg(long, conflicts_with = "from_step")]
    pub resume: bool,

    /// Restart the kept application work directory at the named step.
    #[arg(long, value_name = "ID")]
    pub from_step: Option<String>,
}

/// The mark of `vibe app mark` (PROP-065 `#uninstall-tree`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MarkArg {
    Manual,
    Auto,
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
        #[command(flatten)]
        choice: AppChoiceArgs,
        /// The agent that would run the installer's agent steps: a
        /// definition of agents.toml or a built-in preset (PROP-064).
        #[arg(long, value_name = "NAME")]
        agent: Option<String>,
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

    /// List the run records of an application package newest first, with
    /// their outcome, directory and per-step logs (PROP-065 `#runs`).
    Runs {
        package: String,
        /// Keep only the newest N records and delete the rest.
        #[arg(long, value_name = "N")]
        prune: Option<usize>,
    },

    /// List the stable error codes of application commands with their
    /// meaning and exit status (PROP-065 `#error-codes`, `#exit-codes`):
    /// 0 success, 1 a failed or interrupted step, 2 a refusal before any
    /// effect, 75 a run parked for a hosting agent.
    Errors,

    /// Change the choice of an installed application in place: another
    /// installer option, feature set, profile or parameter values of the
    /// same version; anything not named keeps its recorded value
    /// (PROP-065 `#modify`).
    Modify {
        package: String,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
        /// Switch to another installer option of the same version by its ID.
        #[arg(long, value_name = "ID")]
        installer: Option<String>,
        #[command(flatten)]
        choice: AppChoiceArgs,
        /// The agent that runs the installer's agent steps (PROP-064).
        #[arg(long, value_name = "NAME")]
        agent: Option<String>,
    },

    /// Report every installed application with its installed version and
    /// the newest version the registries offer, changing nothing
    /// (PROP-065 `#outdated`); `--offline` answers from the cache.
    Outdated {
        /// Look the newest versions up in this local registry instead of the
        /// configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
    },

    /// Expose a hidden installation: it joins its slot's alternatives and
    /// gets its proxies, and becomes the default only of a slot without one
    /// (PROP-059 `#hidden-installations`).
    Expose { package: String },

    /// Hide an installation: no proxies, no place among the alternatives;
    /// the default passes on. It stays installed and reachable with
    /// `vibe shell --with`.
    Hide { package: String },

    /// Mark an installation `manual` (asked for by name) or `auto` (pulled
    /// in by a requirement; autoremove takes it once nothing requires it)
    /// (PROP-065 `#uninstall-tree`).
    Mark {
        #[arg(value_enum)]
        mark: MarkArg,
        package: String,
    },

    /// Remove every auto installation nothing requires any more and every
    /// kept previous version (PROP-065 `#uninstall-tree`).
    Autoremove {
        /// Show what would be removed and change nothing.
        #[arg(long)]
        plan_only: bool,
    },

    /// Hold an installation: `outdated` reports it held and a bulk update
    /// skips it (PROP-065 `#update-and-rollback`).
    Hold { package: String },

    /// Release the hold of an installation.
    Unhold { package: String },

    /// Expose the previous version an update kept, make it the default again
    /// and hide the newer one (PROP-065 `#update-and-rollback`).
    Rollback { package: String },

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
