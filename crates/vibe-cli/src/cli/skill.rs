//! Argument structs for `vibe skill …` (PROP-018 §2.6).
//!
//! Standalone-mode skill projection: read the `[[skill]]` declarations of
//! installed packages (and the project itself) and write each into coding
//! agents' skill directories. Split from the `cli` hub along command-family
//! lines; the hub re-exports everything.

specmark::scope!("spec://vibevm/common/PROP-018#vibe-skill");

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillSubcommand {
    /// List the skills declared by installed packages (and the project
    /// itself) that `vibe skill install` can project into agents.
    /// Read-only.
    List(SkillListArgs),

    /// Project declared skills into coding agents' skill directories
    /// (`.<agent>/skills/<name>/…`). Default: every declared skill into
    /// every skill-supporting agent (Claude Code, OpenCode, Codex).
    /// Idempotent — an identical projection surfaces as `unchanged`.
    Install(SkillInstallArgs),

    /// Remove vibevm-projected skills from agents. Strips only the named
    /// skills' own directories; foreign skill dirs are left untouched.
    Uninstall(SkillUninstallArgs),
}

#[derive(Debug, clap::Args)]
pub struct SkillListArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct SkillInstallArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Restrict to specific skills by name (repeatable). Default: all
    /// declared skills.
    #[arg(long = "skill")]
    pub skills: Vec<String>,

    /// Restrict to an agent. One of `all` (default), `claude`,
    /// `opencode`, `codex`. Skill-unsupported agents are reported
    /// `skipped`.
    #[arg(long)]
    pub agent: Option<String>,

    /// Where to project. One of `project` (default — `<proj>/.<agent>/…`),
    /// `user` (global home dirs), `both`.
    #[arg(long)]
    pub scope: Option<String>,

    /// Print the projection plan without writing files.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the apply confirm prompt. `--assume-yes` is an alias for
    /// symmetry with `vibe install` / `vibe mcp install`.
    #[arg(long, alias = "assume-yes")]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct SkillUninstallArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Restrict to specific skills by name (repeatable). Default: all
    /// declared skills.
    #[arg(long = "skill")]
    pub skills: Vec<String>,

    /// Restrict to an agent. One of `all` (default), `claude`,
    /// `opencode`, `codex`.
    #[arg(long)]
    pub agent: Option<String>,

    /// Where to remove from. One of `project` (default), `user`, `both`.
    #[arg(long)]
    pub scope: Option<String>,

    /// Print the removal plan without writing.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the apply confirm prompt. `--assume-yes` is an alias.
    #[arg(long, alias = "assume-yes")]
    pub yes: bool,
}
