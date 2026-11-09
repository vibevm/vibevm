//! The argument surface of per-project version selection
//! (PROP-075 `#ACTIVATION-ENV-OUTPUT`, `#VERSION-EXEC-SPELLING`,
//! `#ACTIVATION-HOOK`): `vibe app env`, `vibe app exec` and `vibe hook`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT");

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
