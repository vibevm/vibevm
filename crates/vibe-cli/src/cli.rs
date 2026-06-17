//! Command-line argument schema.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.
//!
//! This file is the hub: the top-level `Cli` / `Command` pair lives
//! here; the per-command-family argument structs live in the `cli/`
//! submodules and are re-exported below, so consumers keep addressing
//! everything as `crate::cli::X`.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use clap::{Parser, Subcommand};

mod agentic;
mod inspect;
mod man;
mod mcp;
mod pkg;
mod registry;
mod skill;
mod vars;
mod workspace;

pub use agentic::*;
pub use inspect::*;
pub use man::*;
pub use mcp::*;
pub use pkg::*;
pub use registry::*;
pub use skill::*;
pub use vars::*;
pub use workspace::*;

#[derive(Debug, Parser)]
#[command(
    name = "vibe",
    version = env!("CARGO_PKG_VERSION"),
    about = "The disciplined runtime for spec-driven vibecoding.",
    long_about = "vibevm: a CLI software project manager for spec-driven AI-assisted development.\n\
                  Manages installable building blocks — flows, feats, stacks, tools — and assembles\n\
                  them into project-level spec content that AI agents read at session boot."
)]
pub struct Cli {
    /// Produce machine-readable JSON output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Reduce output to a single summary line (useful in scripts / CI).
    #[arg(long, global = true, conflicts_with = "json")]
    pub quiet: bool,

    /// Identifier of the agent or harness invoking this command. Free-form
    /// string; conventional values are `claude-code`, `claude-desktop`,
    /// `cursor`, `opencode`, `codex`. When set, the value is stamped onto
    /// every JSON envelope vibe emits (`"invoked_by": "<value>"`) so the
    /// caller's context is recoverable from logs and machine-readable
    /// output. Falls back to the `VIBE_INVOKED_BY` environment variable
    /// when the flag is absent; flag wins on conflict. The `vibevm` skill
    /// installed by `vibe mcp install --with-skill` instructs each agent
    /// to pass this flag automatically.
    #[arg(long = "invoked-by", global = true, value_name = "AGENT")]
    pub invoked_by: Option<String>,

    /// Run unattended — skip every confirmation prompt and refuse to
    /// open any interactive wizard. Equivalent to passing
    /// `--assume-yes` (`vibe install` / `vibe uninstall`) or `--yes`
    /// (`vibe mcp install` / `upgrade` / `uninstall`) to whichever
    /// subcommand needs it. Falls back to the `VIBE_UNATTENDED`
    /// environment variable (truthy values: `1`, `true`, `yes`,
    /// `on` — case-insensitive); flag wins on conflict. Stamps
    /// `"unattended": true` on every JSON envelope so log
    /// aggregators can tell scripted runs from interactive ones.
    /// Designed for first-time-user provisioning, CI, and other
    /// fully scripted environments.
    #[arg(long, global = true)]
    pub unattended: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new vibevm project in the target directory.
    Init(InitArgs),

    /// List the packages recorded in the project's lockfile.
    List(ListArgs),

    /// Install one or more packages into the current project.
    Install(InstallArgs),

    /// Show installed packages whose registry-side latest version is
    /// newer than what the lockfile currently pins. Read-only — does
    /// not touch the lockfile or fetch package content. Per
    /// PROP-003 §M1.10.
    Outdated(OutdatedArgs),

    /// Search the configured `[[registry]]` entries for packages whose
    /// description, name, keywords, or capabilities match a query.
    /// Walks each registry's index server (resolved via
    /// `VIBEVM_INDEX_URL_<R>` per PROP-005); registries without an
    /// index URL or unreachable servers are reported but do not abort
    /// the run. Per ROADMAP §M2.10.
    Search(SearchArgs),

    /// Start the MCP (Model Context Protocol) server over stdio,
    /// exposing the project's lockfile and active subskills to a
    /// connected coding agent (Claude Code, Cursor, etc.). Per
    /// PROP-004 §5.1 / ROADMAP §M1.7. Reads JSON-RPC 2.0 requests
    /// line-by-line from stdin; writes responses to stdout.
    Mcp(McpArgs),

    /// Project package-declared skills into coding agents — vibevm's
    /// standalone mode (PROP-018 §2.6). `vibe skill list` shows what the
    /// installed packages (and the project itself) declare via `[[skill]]`;
    /// `vibe skill install` writes each into the target agents' skill
    /// directories. No LLM required.
    Skill(SkillArgs),

    /// Compose an LLM instruction for the calling agent and park it in the
    /// relay — vibevm's agentic mode (PROP-018 §2.7, §2.10). vibevm has no
    /// inference engine yet, so `vibe agentic explain` does not act: it
    /// queues a project-explanation task that the agent fetches with
    /// `vibe command` and runs on its own LLM.
    Agentic(AgenticArgs),

    /// Drain the agentic relay: print the instruction a `vibe agentic …`
    /// command parked in `.vibe/agentic/command.md` (PROP-018 §2.7) and
    /// clear the slot. Prints "no pending command" when the mailbox is
    /// empty. The calling agent runs this, then carries out the printed
    /// instruction.
    #[command(name = "command")]
    Drain(CommandArgs),

    /// Remove an installed package from the current project.
    Uninstall(UninstallArgs),

    /// Re-fetch and apply changes for one or more installed packages.
    Update(UpdateArgs),

    /// Recompute the materialised dependencies and the boot artifacts
    /// of a workspace without re-resolving (PROP-009 §2.10).
    Reinstall(ReinstallArgs),

    /// Run the spec-consistency linter against the project tree.
    Check(CheckArgs),

    /// Inspect computed project state (effective spec, configuration).
    Show(ShowArgs),

    /// Manage the registry cache (clone, sync).
    Registry(RegistryArgs),

    /// Operate on a multi-package workspace (PROP-007). Today the one
    /// subcommand is `publish` — walk the workspace's self-publishing
    /// members in dependency order and publish each as its own
    /// repository.
    Workspace(WorkspaceArgs),

    /// Manage vibevm's own versions on this machine — the VibeVM Version
    /// Manager (VVM, PROP-019). `vibe man install <selector>` builds and
    /// installs a version from source; `vibe man use` switches the active
    /// one; `vibe man ls` lists what is installed. Self-distribution: the
    /// `vibe` binary manages its own versions.
    #[command(visible_alias = "manager")]
    Man(ManArgs),

    /// Print the runtime variable context — the values vibevm actually uses
    /// (derived from the running binary's location) versus the environment,
    /// so scripts can reconcile a stale `$VIBEVM_HOME` (PROP-019 §2.14).
    /// Modes: `vibe vars`, `vibe vars diff`, `vibe vars full`,
    /// `vibe vars full diff`.
    Vars(VarsArgs),

    /// Print version information.
    Version,
}
