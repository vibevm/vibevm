//! Argument structs for `vibe mcp …` (PROP-004 §5.1 / ROADMAP §M1.7).
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::Subcommand;

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
    /// Five agents supported: Claude Code, Claude Desktop, Cursor,
    /// OpenCode, Codex. Idempotent — already-correct configs surface
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
    /// Project root with `vibe.toml`. Defaults to current directory.
    /// Required only when `--scope` is `project` or `both`. With
    /// `--scope user` (or auto-resolved to `user` because no
    /// `vibe.toml` is present in CWD), the command runs without a
    /// project.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Restrict to a specific agent. One of `all`, `claude`,
    /// `claude-desktop`, `cursor`, `opencode`, `codex`. When absent
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
    /// `codex`. Default: all five.
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
    /// `opencode`, `codex`. Default: scan all five.
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
pub struct McpServeArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    /// The server reloads the lockfile fresh on every tool call so a
    /// concurrent `vibe install` run becomes visible without a
    /// restart.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
