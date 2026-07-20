//! Argument structs for `vibe self …` — the VibeVM Version Manager
//! (PROP-019 §2.2). Carries the full verb set: `install`, activation
//! (`use`/`env`), introspection (`ls`/`current`/`which`/`doctor`), `remove`/`gc`.

specmark::scope!("spec://vibevm/common/PROP-019#surface");

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct VvmArgs {
    #[command(subcommand)]
    pub command: VvmSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum VvmSubcommand {
    /// Build and install a version of vibevm from source.
    Install(VvmInstallArgs),

    /// Rebuild and activate the latest in-tree version — a shorthand for
    /// `self install latest`.
    Update(VvmUpdateArgs),

    /// Switch the active version (repoints `VIBEVM_HOME`).
    Use(VvmUseArgs),

    /// List installed versions, marking the active one (`*`).
    #[command(visible_alias = "list")]
    Ls,

    /// Print the active version's canonical id (`<kind>:<id>`).
    Current,

    /// Print the absolute path of the active `vibe` binary.
    Which,

    /// Verify the install and environment; `--fix` repairs PATH and shims.
    Doctor(VvmDoctorArgs),

    /// Remove installed version(s) — safe by default (no wipe without
    /// `--all`; no selector opens an interactive picker).
    #[command(visible_aliases = ["rm", "del", "uninstall"])]
    Remove(VvmRemoveArgs),

    /// Reclaim disk: clean the Rust build cache, or prune old versions.
    Gc(VvmGcArgs),

    /// Print the shell line that activates a version in the current shell.
    Env(VvmEnvArgs),

    /// Repoint source provenance to a moved checkout and remove the instances
    /// built from the abandoned tree (PROP-019 §2.17).
    Relocate(VvmRelocateArgs),
}

/// The `--tag`/`--branch`/`--commit` triplet shared by the selector-taking
/// verbs (install / use / env / remove): force how the selector is read.
/// Mutually exclusive; absent means "infer by shape" (PROP-019 §2.3).
/// Flattened into each verb's args so the four call sites stay identical.
#[derive(Debug, clap::Args)]
pub struct ForcedKind {
    /// Interpret the selector as a git tag.
    #[arg(long, conflicts_with_all = ["branch", "commit"])]
    pub tag: bool,

    /// Interpret the selector as a git branch.
    #[arg(long, conflicts_with_all = ["tag", "commit"])]
    pub branch: bool,

    /// Interpret the selector as a git commit.
    #[arg(long, conflicts_with_all = ["tag", "branch"])]
    pub commit: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmInstallArgs {
    /// Version selector: latest | stable | <X.Y.Z> | <commit> | <branch>.
    /// Defaults to `latest` (in-tree: the current checkout).
    #[arg(default_value = "latest")]
    pub selector: String,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Build profile (`debug` | `release`). Defaults to `debug`.
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<String>,

    /// Shorthand for `--profile release`.
    #[arg(long, conflicts_with = "profile")]
    pub release: bool,

    /// Source mirror to clone when not building from a source tree
    /// (gitverse | github). Defaults to an interactive choice, else gitverse.
    #[arg(long, value_name = "MIRROR")]
    pub mirror: Option<String>,

    /// Rebuild even if this version is already installed.
    #[arg(long)]
    pub force: bool,
}

/// Flags for `self update` — `self install latest` with only the build
/// knobs (the selector is fixed to `latest`, no mirror: an in-tree rebuild).
#[derive(Debug, clap::Args)]
pub struct VvmUpdateArgs {
    /// Build profile (`debug` | `release`). Defaults to `debug`.
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<String>,

    /// Shorthand for `--profile release`.
    #[arg(long, conflicts_with = "profile")]
    pub release: bool,

    /// Rebuild even if this version is already installed.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmUseArgs {
    /// Version selector: latest | stable | <X.Y.Z> | <commit> | <branch>.
    pub selector: String,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Print the shell line to `eval` in the current shell instead of
    /// writing the durable environment.
    #[arg(long)]
    pub eval: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmEnvArgs {
    /// Version to emit the activation line for. Defaults to the active one.
    pub selector: Option<String>,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Target shell syntax (bash|zsh|fish|powershell|posix). Defaults to the
    /// detected shell.
    #[arg(long)]
    pub shell: Option<String>,
}

/// Flags for `self relocate` — repoint source provenance after a checkout move
/// (PROP-019 §2.17).
#[derive(Debug, clap::Args)]
pub struct VvmRelocateArgs {
    /// The new vibevm source-tree path — where the checkout moved TO. Must
    /// resolve to a real vibevm checkout.
    pub target: String,

    /// The old source-tree path to move FROM. Inferred from the recorded
    /// `source_path` of the installed external instances when omitted.
    #[arg(long, value_name = "PATH")]
    pub from: Option<String>,

    /// Show what would be repointed and removed; change nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the confirmation prompt (non-interactive runs / scripts).
    #[arg(short = 'y', long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmDoctorArgs {
    /// Apply fixes: write the shims and put the shim dir on PATH (with
    /// consent).
    #[arg(long)]
    pub fix: bool,

    /// Skip the confirmation prompt for `--fix`.
    #[arg(short = 'y', long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmRemoveArgs {
    /// Version to remove. Omit to pick interactively; never wipes all
    /// without `--all`.
    pub selector: Option<String>,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Remove every installed version (asks for confirmation).
    #[arg(long)]
    pub all: bool,

    /// Remove only the built binary, keeping the source tree.
    #[arg(long, conflicts_with = "src")]
    pub bin: bool,

    /// Remove only the source tree, keeping the built binary.
    #[arg(long, conflicts_with = "bin")]
    pub src: bool,

    /// Remove even the active version.
    #[arg(long)]
    pub force: bool,

    /// Skip confirmation prompts.
    #[arg(short = 'y', long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmGcArgs {
    /// Clean the Rust build cache (the shared `--target-dir`).
    #[arg(long, conflicts_with = "prune_others")]
    pub build: bool,

    /// Remove all versions except the current, including their sources.
    #[arg(long)]
    pub prune_others: bool,

    /// Skip the confirmation prompt for `--prune-others`.
    #[arg(short = 'y', long, alias = "assume-yes")]
    pub yes: bool,
}
