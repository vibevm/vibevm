//! The `vibe app` argument surface (PROP-059 `#launcher-operations`,
//! PROP-065 `#plan`, `#describe`, `#runs`, `#work`, `#error-codes`) and
//! the structured-installation flags of `install` / `update`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

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
    /// Complete common configuration input; - reads standard input once.
    #[arg(long, value_name = "FILE")]
    pub configuration_input: Option<std::path::PathBuf>,
    #[arg(skip)]
    pub parsed_configuration: Option<vibe_app::configuration::ConfigurationSubmission>,
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
/// (PROP-065 `#features`, `#resume`, `#work`).
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

    /// Keep the application work directory after a successful run, so a
    /// later `--from-step ID` restarts the build inside it.
    #[arg(long)]
    pub keep_work: bool,

    /// How the step panel shows child output (PROP-065 `#display`):
    /// `auto` folds the last lines under the running step on an
    /// interactive terminal and streams live elsewhere, `live` starts with
    /// the fold open, `tail` keeps it closed, `quiet` shows only each
    /// step's log path. Falls back to `VIBE_APP_LOG`. Press `l` to fold
    /// and unfold while a step runs.
    #[arg(long, value_name = "MODE", value_enum)]
    pub log: Option<AppLogArg>,

    /// How many child-output lines stay folded under the running step
    /// (default 5, `0` draws the step row alone). Falls back to
    /// `VIBE_APP_LOG_TAIL`.
    #[arg(long, value_name = "N")]
    pub log_tail: Option<usize>,
}

/// How a step run shows the output of its children (PROP-065 `#display`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#display")]
pub enum AppLogArg {
    /// Fold the output under the running step on an interactive terminal,
    /// stream it live anywhere else.
    Auto,
    /// Start with the fold open: every line above the step panel.
    Live,
    /// Keep the fold closed wherever a terminal can draw one.
    Tail,
    /// Show no child output at all, only each step's log path.
    Quiet,
}

/// The mark of `vibe app mark` (PROP-065 `#uninstall-tree`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MarkArg {
    Manual,
    Auto,
}

/// The icon slot `vibe app icon --prefer` takes when it has a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum IconPreference {
    Png,
    Svg,
    Symbolic,
    Ico,
    Icns,
}

impl IconPreference {
    /// The slot name the icon documents use.
    pub fn slot(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Symbolic => "symbolic",
            Self::Ico => "ico",
            Self::Icns => "icns",
        }
    }
}

/// Which ordered pinned list `vibe app pin` and `unpin` mean
/// (PROP-059 `#launcher-catalog`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PinArg {
    Taskbar,
    Favorites,
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
        /// Emit common evaluation and execution only when fully resolved.
        #[arg(long)]
        configuration: bool,
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
        /// Describe separate package and installer common configurations.
        #[arg(long)]
        configuration: bool,
        /// Resolve from this local registry instead of the configured registries.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
    },

    /// Continue an exact retained operation; --set supplies ephemeral secrets only.
    Resume {
        package: String,
        #[arg(long, value_name = "ID")]
        operation: String,
        #[arg(long="set",value_name="ID=VALUE",value_parser=parameter_value)]
        parameters: Vec<(String, String)>,
    },

    /// List the run records of an application package newest first, with
    /// their outcome, directory and per-step logs (PROP-065 `#runs`).
    Runs {
        package: String,
        /// Keep only the newest N records and delete the rest.
        #[arg(long, value_name = "N")]
        prune: Option<usize>,
    },

    /// List the kept application work directories of the home with their
    /// state, change time and size, and delete them on request
    /// (PROP-065 `#work`); the download cache and the run records are
    /// never touched.
    Work {
        /// The installed package whose kept work directory is meant;
        /// omitted, every kept directory of the home is listed.
        #[arg(group = "kept")]
        package: Option<String>,
        /// Inspect or clean one exact finite import cohort, including its owned attempts.
        #[arg(long, value_name = "OPERATION_ID", group = "kept")]
        import: Option<String>,
        /// Delete the selected work directory; needs a package or `--all`.
        #[arg(long, requires = "kept")]
        clean: bool,
        /// With `--clean`, every kept work directory of the home.
        #[arg(long, group = "kept", requires = "clean")]
        all: bool,
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
        /// Keep the application work directory after a successful run, as
        /// `install --keep-work` does (PROP-065 `#work`).
        #[arg(long)]
        keep_work: bool,
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

    /// List launchers of an installed application, or with `--all` the whole
    /// launcher catalog of the home — every ready, exposed installation with
    /// its keys, collections, pins, overrides and usage facts
    /// (PROP-059 `#menu`, `#launcher-catalog`).
    Launchers {
        /// The installed package; omitted with `--all`, `--revision`,
        /// `--export-system` or `--remove-system`.
        #[arg(
            required_unless_present_any = ["all", "revision", "export_system", "remove_system"],
            conflicts_with_all = ["all", "revision", "export_system", "remove_system"]
        )]
        package: Option<String>,
        /// List every ready, exposed installation in one answer.
        #[arg(long, conflicts_with_all = ["revision", "export_system", "remove_system"])]
        all: bool,
        /// With `--all`, only the launchers whose author set `menu = true`.
        #[arg(long, requires = "all")]
        menu: bool,
        /// Print only the catalog revision, the one number a client compares
        /// instead of refetching the catalog.
        #[arg(long, conflicts_with_all = ["export_system", "remove_system"])]
        revision: bool,
        /// Write the platform's own menu entries (`.desktop` files and icon
        /// copies, Start Menu shortcuts, `.app` wrappers) for the launchers
        /// with `menu = true` you have not hidden; a later export rewrites
        /// and prunes them (PROP-059 `#launcher-runtime`).
        #[arg(long, conflicts_with = "remove_system")]
        export_system: bool,
        /// With `--export-system`, only the launchers of this collection.
        #[arg(long, value_name = "NAME", requires = "export_system")]
        collection: Option<String>,
        /// Remove every system entry an export wrote.
        #[arg(long)]
        remove_system: bool,
        /// With `--export-system` or `--remove-system`, list the paths and
        /// change nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// List the launchers started detached through vibe that are still
    /// running, with their process ids and start times (PROP-059
    /// `#launcher-runtime`).
    Running,

    /// End a launcher started through vibe: ask it to end, wait, then end
    /// it without asking (PROP-059 `#launcher-runtime`).
    Stop {
        /// A launcher key, a package key, or a process id from `running`.
        target: String,
    },

    /// Print one line per change of the home — installed, updated, removed,
    /// launchers-changed, collections-changed, started, exited — until
    /// standard input closes or you interrupt it (PROP-059
    /// `#launcher-runtime`).
    Watch,

    /// Arrange the launcher catalog into named, ordered collections of
    /// launcher keys — a game library's shelves (PROP-059
    /// `#launcher-catalog`).
    Collection {
        #[command(subcommand)]
        command: CollectionCommand,
    },

    /// Pin one launcher key to an ordered list a client draws; a repeated
    /// pin moves it to the end (PROP-059 `#launcher-catalog`).
    Pin {
        /// `<group>/<name>@<version>#<launcher id>`, as the catalog spells it.
        key: String,
        #[arg(long, value_enum, default_value_t = PinArg::Taskbar)]
        to: PinArg,
    },

    /// Take one launcher key off a pinned list.
    Unpin {
        key: String,
        #[arg(long, value_enum, default_value_t = PinArg::Taskbar)]
        from: PinArg,
    },

    /// Override one launcher for this user: hide it from a client's menus
    /// while it stays in the catalog, give it a name or an icon of your own
    /// that survives update and reinstall, or reset it to the package's
    /// declaration (PROP-059 `#launcher-catalog`).
    #[command(group = clap::ArgGroup::new("override").required(true).multiple(false))]
    Launcher {
        key: String,
        /// Keep it out of a client's menus; it stays in the catalog marked
        /// `hidden_by: "user"`.
        #[arg(long, group = "override")]
        hide: bool,
        /// Undo a hide.
        #[arg(long, group = "override")]
        show: bool,
        /// A name of your own.
        #[arg(long, value_name = "NAME", group = "override")]
        name: Option<String>,
        /// An icon file of your own.
        #[arg(long, value_name = "PATH", group = "override")]
        icon: Option<std::path::PathBuf>,
        /// Forget the whole override row.
        #[arg(long, group = "override")]
        reset: bool,
    },

    /// Print the path of the best existing icon file of an installed
    /// application for a size, never rasterising (PROP-059 `#menu`).
    Icon {
        package: String,
        /// The launcher whose icon set is asked for; falls back to the
        /// application's.
        #[arg(long, value_name = "ID")]
        launcher: Option<String>,
        /// The pixel size the caller needs.
        #[arg(long, value_name = "N")]
        size: u32,
        /// Take a file of this slot when one exists.
        #[arg(long, value_enum)]
        prefer: Option<IconPreference>,
    },

    /// Print the launcher rows desktop entries derive, as manifest TOML
    /// (PROP-059 `#desktop-entries`).
    ImportDesktop {
        /// The `.desktop` files to read.
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,
        /// An unpacked payload to resolve `Exec` and theme icons against.
        #[arg(long, value_name = "DIR")]
        app_root: Option<std::path::PathBuf>,
    },

    /// Run one launcher by its exact stable ID.
    Run {
        package: String,
        launcher_id: String,
        /// Arguments passed literally after the launcher's declared arguments.
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Write the portable set of the user home's applications — each
    /// installation's installer option, features, parameters, licence,
    /// mark and hold — and your launcher arrangement, as TOML a later
    /// `vibe app import` reads on another machine (PROP-059 `#portable-set`).
    Export(super::AppExportArgs),

    /// Install the applications of a set `vibe app export` wrote, through
    /// the ordinary registry resolution and this host's platform rows, then
    /// restore its launcher arrangement (PROP-059 `#portable-set`).
    Import(super::AppImportArgs),

    /// Make one installation the global default provider of its slot, or
    /// switch the whole slot off and back on (PROP-059 `#slot-off`).
    ///
    /// `--none` removes the slot's PATH proxies and its link directories'
    /// links, so every one of its commands falls through to the system
    /// program; the installations, their index records and the remembered
    /// default all stay. `--default` puts that remembered default back, and
    /// naming a KEY both switches the slot on and makes KEY the default.
    Use {
        slot: String,
        /// The installation key to make the default; omitted with `--none`
        /// or `--default`.
        key: Option<String>,
        /// Switch the slot off: no proxy, no link, system program again.
        #[arg(long, conflicts_with = "default")]
        none: bool,
        /// Switch the slot back on at its remembered default.
        #[arg(long)]
        default: bool,
        /// Also write the resolved version into the slot's version file
        /// (PROP-075 `#VERSION-FILE-WRITE`).
        #[command(flatten)]
        version_file: WriteVersionFileArgs,
    },

    /// List the providers of a slot, its default and the commands each provides.
    Alternatives { slot: String },

    /// Carry an application's user state from one installed version of a
    /// slot to another (PROP-075 `#MIGRATION-FAILURE`): the old version
    /// runs each declared entry's `export` step, the new one its `import`
    /// step, over a shared work directory that is always kept.
    ///
    /// Nothing is ever rolled back. A failed export stops its own entry
    /// before the import runs and changes nothing; a failed import leaves
    /// the new version installed and the slot's default exactly where you
    /// put it, and the report names what was not carried and the command
    /// that retries it. An unattended run that names neither `--only` nor
    /// `--all` refuses rather than migrating unasked.
    Migrate(AppMigrateArgs),

    /// Print the lines that switch the current shell to a selection
    /// (PROP-075 `#ACTIVATION-ENV-OUTPUT`): `PATH` with the chosen
    /// installations' payload directories in front, and `VIBE_APP_USE`.
    ///
    /// Stdout carries the lines and nothing else, so
    /// `eval "$(vibe app env --use node=22.23.4)"` puts a shell in the same
    /// state `vibe shell --use node=22.23.4` does. With `--shell cmd` the
    /// lines are bare `NAME=VALUE` pairs, because cmd has no `eval`; the one
    /// incantation that consumes them is
    /// `for /f "usebackq tokens=1,* delims==" %a in (`vibe app env --shell cmd`) do @set "%a=%b"`.
    Env(AppEnvArgs),

    /// List every version of a package or slot — the template's and the
    /// registries' — newest first, with each version's origin, channels
    /// and local state (PROP-075 `#VERSIONS-LIST-OUTPUT`). Nothing is
    /// rendered and nothing is installed.
    Versions(AppVersionsArgs),

    /// Run one command with a slot's chosen version first on PATH, without
    /// changing any default, state or file (PROP-075
    /// `#VERSION-EXEC-SPELLING`). `--` is required before the command, so a
    /// flag of the command is never read as vibe's.
    Exec(AppExecArgs),

    /// Run the declared default launcher, or ask in an interactive terminal.
    /// With `--json` the launcher is started detached and the answer names
    /// its process id, command line and log instead of waiting for it
    /// (PROP-059 `#launcher-catalog`).
    Start {
        /// The installed package, or a launcher key whose `#<launcher id>`
        /// selects the launch point.
        package: String,
        #[arg(long)]
        launcher: Option<String>,
        /// Arguments passed literally after the launcher's declared arguments.
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[path = "app_activation.rs"]
mod activation;
#[path = "app_migration.rs"]
mod migration;
pub use activation::{AppEnvArgs, AppExecArgs, AppVersionsArgs, HookArgs, WriteVersionFileArgs};
pub use migration::AppMigrateArgs;
#[path = "app_collection.rs"]
mod collection;
pub use collection::CollectionCommand;
