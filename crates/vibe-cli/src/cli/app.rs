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

/// The edits of one collection (PROP-059 `#launcher-catalog`).
#[derive(Debug, clap::Subcommand)]
pub enum CollectionCommand {
    /// Create an empty collection.
    Create {
        name: String,
        /// An icon file for the collection itself.
        #[arg(long, value_name = "PATH")]
        icon: Option<std::path::PathBuf>,
    },
    /// Rename one; it keeps its keys, and stops counting as seeded.
    Rename { old: String, new: String },
    /// Delete one; the launchers themselves stay installed.
    Delete { name: String },
    /// Append the launcher keys it does not hold yet.
    Add {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// Take launcher keys out and remember that you did, so an install of
    /// the same `menu_group` does not put them back.
    Remove {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// The full order: the collection becomes exactly this sequence.
    Order {
        name: String,
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },
    /// Show every collection, both pinned lists and every override.
    List,
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

    /// List the kept application work directories of the home with their
    /// state, change time and size, and delete them on request
    /// (PROP-065 `#work`); the download cache and the run records are
    /// never touched.
    Work {
        /// The installed package whose kept work directory is meant;
        /// omitted, every kept directory of the home is listed.
        #[arg(group = "kept")]
        package: Option<String>,
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
        /// The installed package; omitted with `--all` or `--revision`.
        #[arg(
            required_unless_present_any = ["all", "revision"],
            conflicts_with_all = ["all", "revision"]
        )]
        package: Option<String>,
        /// List every ready, exposed installation in one answer.
        #[arg(long, conflicts_with = "revision")]
        all: bool,
        /// With `--all`, only the launchers whose author set `menu = true`.
        #[arg(long, requires = "all")]
        menu: bool,
        /// Print only the catalog revision, the one number a client compares
        /// instead of refetching the catalog.
        #[arg(long)]
        revision: bool,
    },

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

    /// Make one installation the global default provider of its slot.
    Use { slot: String, key: String },

    /// List the providers of a slot, its default and the commands each provides.
    Alternatives { slot: String },

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
