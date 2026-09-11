//! Argument structs for `vibe self …` — the VibeVM Version Manager
//! (PROP-019 §2.2). Carries the full verb set: `install`, activation
//! (`use`/`env`), introspection (`ls`/`current`/`which`/`doctor`), `remove`/`gc`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct VvmArgs {
    #[command(subcommand)]
    pub command: VvmSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum VvmSubcommand {
    /// Install a version: verified release bundle from a binary execution,
    /// or a two-binary build from a source execution / explicit mirror.
    Install(VvmInstallArgs),

    /// Import a ready-built local vibe executable without network access.
    Import(VvmImportArgs),

    /// Bootstrap from an aggregate manifest and its release download base.
    Bootstrap(VvmBootstrapArgs),

    /// Refresh and activate the current logical version. Binary executions
    /// refetch their mutable release; source executions rebuild `latest`.
    Update(VvmUpdateArgs),

    /// Switch the active version; `--eval` creates a shell-local override.
    Use(VvmUseArgs),

    /// Switch back to the previously active immutable local instance.
    /// Repeating the command toggles between the two instances.
    Rollback,

    /// List installed versions, marking the active one (`*`).
    #[command(visible_alias = "list")]
    Ls,

    /// Print the running/active payload's exact selector and provenance.
    Current,

    /// Print the active instance's `vibe`, `vibe-index`, or source path.
    Which(VvmWhichArgs),

    /// Print exactly the source tree used by the running/active instance.
    Source,

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

#[derive(Debug, clap::Args)]
pub struct VvmImportArgs {
    /// Path to a ready-built vibe executable.
    #[arg(value_name = "PATH-TO-EXE")]
    pub path: PathBuf,

    /// Release tag to inventory the executable under (for example 1.0.0).
    #[arg(long, value_name = "X.Y.Z")]
    pub tag: String,

    /// Source commit that produced the executable, when known.
    #[arg(long, value_name = "SHA")]
    pub commit: Option<String>,

    /// Build profile metadata (`debug` | `release`). Defaults to `release`.
    #[arg(long, value_name = "PROFILE", default_value = "release")]
    pub profile: String,

    /// Activate the imported instance and write the stable shims.
    #[arg(long = "use")]
    pub activate: bool,

    /// Compatibility flag; distinct payloads now always become a fresh
    /// immutable local #N while the mutable tag keeps the same version label.
    #[arg(long)]
    pub replace_candidate: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmBootstrapArgs {
    /// Already-downloaded aggregate `DISTRIBUTIONS.json`.
    #[arg(long, value_name = "PATH")]
    pub manifest: PathBuf,

    /// Expected semantic version, without a `v` prefix.
    #[arg(long, value_name = "X.Y.Z")]
    pub version: String,

    /// Release asset directory, normally ending in `/download/vX.Y.Z`.
    #[arg(long, value_name = "URL")]
    pub release_base: String,

    /// Install a fresh immutable #N even when bundle bytes are unchanged.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmWhichArgs {
    /// Instance member to locate: vibe | vibe-index | source.
    #[arg(default_value = "vibe")]
    pub component: String,
}

/// Flags for `self update`: force controls both binary refresh and source
/// rebuild; profile knobs apply to the source path only.
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
    /// Version selector, including an exact local `<kind>:<id>#N`.
    pub selector: String,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Print a shell-local override line instead of changing durable state.
    /// Clear it with `unset VIBEVM_SHELL_HOME` (sh/fish) or
    /// `Remove-Item Env:VIBEVM_SHELL_HOME` (PowerShell).
    #[arg(long)]
    pub eval: bool,
}

#[derive(Debug, clap::Args)]
pub struct VvmEnvArgs {
    /// Version (including exact `<kind>:<id>#N`) to emit an activation line
    /// for. Defaults to the active one.
    pub selector: Option<String>,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Target shell syntax (bash|zsh|fish|powershell|posix). The emitted
    /// VIBEVM_SHELL_HOME override wins over durable `current`; clear it to
    /// return to the durable selection.
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
    /// Version to remove. A terminal `#N` removes only that immutable local
    /// instance. Omit to pick interactively; never wipes all without `--all`.
    #[arg(conflicts_with = "all")]
    pub selector: Option<String>,

    #[command(flatten)]
    pub kind: ForcedKind,

    /// Remove every installed version (asks for confirmation).
    #[arg(long, conflicts_with_all = ["selector", "tag", "branch", "commit"])]
    pub all: bool,

    /// Remove only instance binaries, retaining source and provenance.
    #[arg(long, conflicts_with = "src")]
    pub bin: bool,

    /// Remove only managed/instance-owned source, retaining binaries.
    #[arg(long, conflicts_with = "bin")]
    pub src: bool,

    /// Remove an active version when it is not the currently executing instance.
    /// A running instance is never self-removable.
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

    /// Remove old instances except the current and its immediate rollback.
    #[arg(long)]
    pub prune_others: bool,

    /// Skip the confirmation prompt for `--prune-others`.
    #[arg(short = 'y', long, alias = "assume-yes")]
    pub yes: bool,
}
