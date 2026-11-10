//! The argument surface of per-project version selection
//! (PROP-075 `#ACTIVATION-ENV-OUTPUT`, `#VERSION-EXEC-SPELLING`,
//! `#ACTIVATION-HOOK`, `#VERSION-FILE-WRITE`): `vibe app env`,
//! `vibe app exec`, `vibe hook`, and the one flag that *writes* a version
//! file, shared by `vibe app use` and `vibe install -g`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT");

/// `--write-version-file[=PATH]` (PROP-075 `#VERSION-FILE-WRITE`), which
/// is nvm's `--save` / `-w`. One declaration, flattened into both commands
/// that carry it, so the two spellings cannot drift.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct WriteVersionFileArgs {
    /// Write the resolved version into this slot's declared version file —
    /// never the requirement or the channel you typed. Without a value the
    /// nearest declared file of this directory is edited, or the package's
    /// first declared name is created in the project root; with a value,
    /// exactly that path, whose format is decided by its file name.
    ///
    /// A `keyed` file (`.sdkmanrc`, `.tool-versions`) is edited one line
    /// at a time, so another tool's line in the same file survives
    /// untouched.
    #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
    pub write_version_file: Option<Option<std::path::PathBuf>>,
}

impl WriteVersionFileArgs {
    /// The path the flag named, `None` for a bare flag and no flag alike;
    /// [`Self::asked`] is what distinguishes them.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.write_version_file
            .as_ref()
            .and_then(Option::as_deref)
            .filter(|path| !path.as_os_str().is_empty())
    }

    /// Whether the flag was given at all.
    pub fn asked(&self) -> bool {
        self.write_version_file.is_some()
    }
}

/// Flags of `vibe app env` (PROP-075 `#ACTIVATION-ENV-OUTPUT`).
#[derive(Debug, clap::Args)]
pub struct AppEnvArgs {
    /// Choose a slot's provider for this shell: `SLOT=VERSION`,
    /// `SLOT=<group>/<name>@<version>`, a channel (`SLOT=latest`), or
    /// `SLOT=none` to switch the slot off. Repeatable; a later entry wins
    /// for its slot.
    #[arg(long = "use", value_name = "SLOT=VER")]
    pub uses: Vec<String>,

    /// Resolve what the project's `vibe.toml` requirements and the declared
    /// version files of this directory select, beside any `--use`.
    #[arg(long)]
    pub project: bool,

    /// Target shell syntax (bash|zsh|fish|powershell|cmd|posix). The
    /// caller's parent shell is detected when this is omitted.
    #[arg(long)]
    pub shell: Option<String>,

    /// Print the lines that undo the selection this shell carries — nvm's
    /// `deactivate`.
    #[arg(long, conflicts_with_all = ["uses", "project"])]
    pub unset: bool,

    /// Record the version this `--use` resolved to in the slot's declared
    /// version file, changing nothing else — nvm's `use --save`. It needs
    /// exactly one `--use`, so which slot's file is written is said and
    /// never guessed, and it moves no slot default.
    #[command(flatten)]
    pub version_file: WriteVersionFileArgs,
}

/// Flags of `vibe app exec` (PROP-075 `#VERSION-EXEC-SPELLING`).
#[derive(Debug, clap::Args)]
pub struct AppExecArgs {
    /// `SLOT` or `SLOT@VER`, where `VER` is a version, a version
    /// requirement or a channel. Omitted, the selection is resolved by the
    /// ordinary precedence.
    pub target: String,

    /// Resolve the project's requirements and version files; implied when
    /// no `@VER` is given.
    #[arg(long)]
    pub project: bool,

    /// The command and its arguments, after a required `--`.
    #[arg(last = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

/// Flags of `vibe app versions` (PROP-075 `#VERSIONS-LIST-OUTPUT`).
#[derive(Debug, clap::Args)]
pub struct AppVersionsArgs {
    /// The package as `<group>/<name>`, or an application slot id. A slot
    /// with providers from several packages lists each under its own
    /// heading.
    pub target: String,

    /// Keep only the versions this channel holds (`lts`, `lts/jod`).
    #[arg(long, value_name = "NAME")]
    pub channel: Option<String>,

    /// Only the versions this home installed; consults no registry and
    /// makes no request.
    #[arg(long)]
    pub installed: bool,

    /// How many rows to show (default 20).
    #[arg(long, value_name = "N", conflicts_with = "all")]
    pub limit: Option<usize>,

    /// Every row, however many the domain holds.
    #[arg(long)]
    pub all: bool,

    /// List from this local registry instead of the configured registries.
    #[arg(long)]
    pub registry: Option<std::path::PathBuf>,
}

/// Flags of `vibe hook` (PROP-075 `#ACTIVATION-HOOK`).
#[derive(Debug, clap::Args)]
pub struct HookArgs {
    /// The shell whose startup file takes the block: bash, zsh, fish or
    /// pwsh. cmd has no prompt hook; `vibe shell` is the way there.
    pub shell: String,

    /// Write the block to stdout instead of the startup file, for a user
    /// who installs it themselves.
    #[arg(long)]
    pub print: bool,

    /// Write the block without asking.
    #[arg(long)]
    pub yes: bool,
}
