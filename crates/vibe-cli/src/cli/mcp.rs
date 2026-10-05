//! Argument structs for `vibe mcp …` (PROP-004 §5.1 / ROADMAP §M1.7).
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

#[derive(Debug, clap::Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum McpSubcommand {
    /// Run the MCP server over stdio. Blocks until the client
    /// disconnects (EOF on stdin).
    Serve(McpServeArgs),

    /// Detect supported coding agents and write the per-agent MCP
    /// server configuration plus an optional `vibevm` SKILL.md so the
    /// agent picks up vibevm automatically on its next session start.
    /// Six agents supported: Claude Code, Claude Desktop, Cursor,
    /// OpenCode, Codex, Qwen Code. Idempotent — already-correct configs surface
    /// as `unchanged`.
    ///
    /// Without flags, drops into an interactive multi-select picker
    /// (requires a TTY). For CI / scripts use `--auto` (install
    /// everywhere, with skill) or `--agent <name>` (one explicit
    /// target).
    Install(McpInstallArgs),

    /// Same as `install` but printing the planned config diff
    /// without writing any files. Useful for CI / review.
    Status(McpStatusArgs),

    /// Refresh existing vibevm MCP integrations to the version
    /// shipped in this binary. Scans known paths, compares the
    /// on-disk MCP-server entry / SKILL.md to what `install` would
    /// write today, and rewrites only the diverged ones. Does NOT
    /// create new installations — use `mcp install` for that. Useful
    /// after `cargo install --path crates/vibe-cli` (or any vibe
    /// upgrade) to pull the new SKILL.md / wire shape into agents
    /// that already had vibevm wired.
    Upgrade(McpUpgradeArgs),

    /// Remove vibevm MCP integration from one or more agents. Drops
    /// the `vibevm` key from each agent's MCP config (foreign keys
    /// preserved) and deletes the SKILL.md file (and its parent
    /// `vibevm/` skill dir if it becomes empty). Same scope axis as
    /// install / upgrade: project, user, both. Wizard-driven without
    /// flags; fully scriptable with `--scope` / `--what` / `--agent`.
    Uninstall(McpUninstallArgs),
}

#[derive(Debug, clap::Args)]
pub struct McpInstallArgs {
    /// Optional installed MCP package to register. Without this selector,
    /// retain the legacy walk over every package server in the project.
    pub package: Option<String>,

    /// Server names: comma-separated, or a JSON string array for literal commas.
    #[arg(long, requires = "package")]
    pub server: Option<String>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    /// Required only when `--scope` is `project` or `both`. With
    /// `--scope user` (or auto-resolved to `user` because no
    /// `vibe.toml` is present in CWD), the command runs without a
    /// project.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Restrict to a specific agent. One of `all`, `claude`,
    /// `claude-desktop`, `cursor`, `opencode`, `codex`, `qwen-code`. When absent
    /// and `--auto` is also absent, the wizard's agents step asks
    /// (TTY required). Conflicts with `--auto`.
    #[arg(long, conflicts_with = "auto")]
    pub agent: Option<String>,

    /// Detect every supported agent on this machine and install in
    /// all of them. No prompts (except final apply confirm — pass
    /// `--yes` to skip even that). Conflicts with `--agent`.
    #[arg(long)]
    pub auto: bool,

    /// Where to install. One of `project` (per-project files —
    /// `<proj>/.<agent>/...`), `user` (global home / config dirs),
    /// `both` (project AND user). When absent, the wizard asks; with
    /// `--auto` it auto-resolves to `project` if `vibe.toml` is in
    /// `--path`, else `user`.
    #[arg(long)]
    pub scope: Option<String>,

    /// What to install. One of `both` (default — MCP server entry +
    /// SKILL.md), `mcp` (server entry only), `skill` (SKILL.md only).
    /// When absent under `--auto`, defaults to `both`; in
    /// interactive mode the wizard asks.
    #[arg(long)]
    pub what: Option<String>,

    /// Print the planned config without writing files.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the final apply confirm prompt. Implied by `--auto` when
    /// `--scope` is also explicit. The global `--unattended` flag
    /// (or `VIBE_UNATTENDED` env-var) has the same effect; pick
    /// whichever reads better in your context. `--assume-yes` is an
    /// alias for symmetry with `vibe install` / `uninstall` /
    /// `update`.
    #[arg(long, alias = "assume-yes")]
    pub yes: bool,

    /// Force-write even when no agent is detected in the project
    /// tree / on this machine (useful when the agent's marker dir
    /// is not yet present but the operator wants the config
    /// provisioned).
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct McpStatusArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct McpUninstallArgs {
    /// Remove registrations owned by this installed MCP package only.
    pub package: Option<String>,

    /// Server names to remove: comma-separated, or a JSON string array for literal commas.
    #[arg(long, requires = "package")]
    pub server: Option<String>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    /// Project-scope walks require it; user-scope works anywhere.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Where to remove from. `project` (only project files), `user`
    /// (only user-level), `both` (default — wipe project AND user).
    /// In wizard mode this is the first prompt.
    #[arg(long)]
    pub scope: Option<String>,

    /// Restrict to one or more agents. Same vocabulary as install:
    /// `all`, `claude`, `claude-desktop`, `cursor`, `opencode`,
    /// `codex`, `qwen-code`. Default: all six.
    #[arg(long)]
    pub agent: Option<String>,

    /// Restrict to MCP-config files only (keep SKILL.md). Default:
    /// remove both.
    #[arg(long = "config-only", conflicts_with = "skill_only")]
    pub config_only: bool,

    /// Restrict to SKILL.md files only (keep MCP server entry).
    /// Default: remove both.
    #[arg(long = "skill-only")]
    pub skill_only: bool,

    /// Print the removal plan and exit without writing.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the apply confirm prompt. Useful in CI / cron.
    /// `--assume-yes` is an alias for symmetry with `vibe install`
    /// / `uninstall` / `update`.
    #[arg(long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct McpUpgradeArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    /// When `vibe.toml` is absent, project-scope upgrades are silently
    /// skipped (only user-scope is scanned).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Restrict the scan to one scope. `project` (only project files,
    /// requires `vibe.toml`), `user` (only user-level), `both`
    /// (default — scan everything that exists).
    #[arg(long)]
    pub scope: Option<String>,

    /// Restrict the scan to one or more agents. Same vocabulary as
    /// `mcp install`: `all`, `claude`, `claude-desktop`, `cursor`,
    /// `opencode`, `codex`, `qwen-code`. Default: scan all six.
    #[arg(long)]
    pub agent: Option<String>,

    /// Restrict to MCP-config files only (skip SKILL.md). Default:
    /// scan both.
    #[arg(long = "config-only", conflicts_with = "skill_only")]
    pub config_only: bool,

    /// Restrict to SKILL.md files only (skip MCP configs). Default:
    /// scan both.
    #[arg(long = "skill-only")]
    pub skill_only: bool,

    /// Print the refresh plan and exit without writing.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the apply confirm prompt. Useful in CI / cron.
    /// `--assume-yes` is an alias for symmetry with `vibe install`
    /// / `uninstall` / `update`.
    #[arg(long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("fixed_host")
        .args(["fixed_validate", "fixed_package", "fixed_cargo", "fixed_deploy", "fixed_install"])
        .multiple(false)
))]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
pub struct McpServeArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    /// The server reloads the lockfile fresh on every tool call so a
    /// concurrent `vibe install` run becomes visible without a
    /// restart.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Explicit cooperative fixed Validate host; omitted means no lifecycle grant.
    #[arg(long, value_enum, conflicts_with_all = ["fixed_package", "fixed_cargo"])]
    pub fixed_validate: Option<McpFixedValidateMode>,

    /// Explicit cooperative fixed package host; omitted means no package grant.
    #[arg(long, value_enum, conflicts_with_all = ["fixed_validate", "fixed_cargo"])]
    pub fixed_package: Option<McpFixedPackageMode>,

    /// Explicit cooperative fixed offline Cargo host; omitted means no build grant.
    #[arg(long, value_enum, conflicts_with_all = ["fixed_validate", "fixed_package"])]
    pub fixed_cargo: Option<McpFixedCargoMode>,

    /// Explicit project-local fixed Deploy host; omitted means no Deploy grant.
    #[arg(long, value_enum, conflicts_with_all = ["fixed_validate", "fixed_package", "fixed_cargo"])]
    pub fixed_deploy: Option<McpFixedDeployMode>,

    /// Explicit exact one-package local Install host.
    #[arg(long, value_enum, conflicts_with_all = ["fixed_validate", "fixed_package", "fixed_cargo", "fixed_deploy"])]
    pub fixed_install: Option<McpFixedInstallMode>,

    /// Absolute local registry root for the selected Install package.
    #[arg(long, value_parser = absolute_registry_root, requires = "fixed_install")]
    pub registry_root: Option<PathBuf>,

    /// Fully qualified exact package pin selected by this host.
    #[arg(long, requires = "fixed_install")]
    pub package_pin: Option<String>,

    /// Every file in the exact package source tree; repeat per file.
    #[arg(long = "source-file", requires = "fixed_install")]
    pub source_files: Vec<String>,

    /// Permit a separate typed approval of this exact Install plan.
    #[arg(long, requires = "fixed_install")]
    pub approve_exact_local_install: bool,

    /// Authored Deploy profile; required for fixed Deploy and never inferred.
    #[arg(long, requires = "fixed_deploy")]
    pub deploy_profile: Option<String>,

    /// First issue lifetime in milliseconds; forbidden on reattach.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=86_400_000), requires = "fixed_host")]
    pub ttl_ms: Option<u64>,

    /// Permit typed approval of the exact captured Validate script.
    #[arg(long, requires = "fixed_validate")]
    pub approve_captured_script: bool,

    /// Permit typed detach selection for the fixed Validate owner.
    #[arg(long, requires = "fixed_validate")]
    pub allow_detach: bool,

    /// Permit typed cancellation before Validate dispatch.
    #[arg(long, requires = "fixed_validate")]
    pub allow_cancel_pre_dispatch: bool,

    /// Permit typed cancellation at a deferred Validate boundary.
    #[arg(long, requires = "fixed_validate")]
    pub allow_cancel_deferred_boundary: bool,

    /// Explicit absolute recovery store for manifest migration authority.
    /// Omission leaves migration tools ungranted; no store is inferred.
    #[arg(long)]
    pub migration_recovery_store: Option<PathBuf>,

    /// Grant one portable relative authored-file operand; repeat for members.
    /// With an explicit store and no file flags, only root vibe.toml is granted.
    #[arg(long = "migration-file", requires = "migration_recovery_store")]
    pub migration_file: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpFixedPackageMode {
    Issue,
    Reattach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpFixedCargoMode {
    Issue,
    Reattach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpFixedDeployMode {
    Issue,
    Reattach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpFixedValidateMode {
    Issue,
    Reattach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpFixedInstallMode {
    Issue,
    Reattach,
}

fn absolute_registry_root(value: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(value);
    if !root.is_absolute() {
        return Err("fixed Install --registry-root must be absolute".into());
    }
    Ok(root)
}
