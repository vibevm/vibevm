//! Command-line argument schema.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.
//!
//! This file is the hub: the top-level `Cli` / `Command` pair lives
//! here; the per-command-family argument structs live in the `cli/`
//! submodules and are re-exported below, so consumers keep addressing
//! everything as `crate::cli::X`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod agent;
mod agentic;
mod aiui;
mod artifacts;
mod cache;
mod dl;
mod doc;
mod explain;
mod extensions;
mod init;
mod inspect;
mod lifecycle;
mod mcp;
mod pkg;
mod prefs;
mod progress;
mod prompt;
pub use prompt::*;
mod query;
mod registry;
mod remap;
mod requirements;
mod run;
mod scrape;
mod select;
mod shell;
mod skill;
mod source_policy;
mod specmap;
mod storage;
pub use source_policy::SourcePolicyArgs;
mod term;
mod tool;
pub use tool::*;
mod transfers;
mod vars;
mod vvm;
mod workspace;

pub use agentic::*;
pub use aiui::*;
pub use artifacts::*;
pub use cache::*;
pub use dl::*;
pub use doc::*;
pub use explain::*;
pub use extensions::*;
pub use init::{InitAppArchive, InitAppArgs, InitAppWay, InitArgs, InitType};
pub use inspect::*;
pub use lifecycle::*;
pub use mcp::*;
pub use pkg::*;

mod app;
mod app_portable;
pub use agent::*;
pub use app::*;
pub use app_portable::*;
pub use prefs::*;
pub use progress::*;
pub use query::*;
pub use registry::*;
pub use remap::*;
pub use requirements::*;
pub use run::*;
pub use scrape::*;
pub use select::*;
pub use shell::*;
pub use skill::*;
pub use specmap::*;
pub use storage::*;
pub use term::*;
pub use transfers::*;
pub use vars::*;
pub use vvm::*;
pub use workspace::*;

#[derive(Debug, Parser)]
#[command(
    name = "vibe",
    version = env!("CARGO_PKG_VERSION"),
    about = "The disciplined runtime for spec-driven vibecoding.",
    long_about = "vibevm: a CLI software project manager for spec-driven AI-assisted development.\n\
                  Manages installable building blocks — flow, feat, stack, tool, mcp, lang, doc, app, remap — and assembles\n\
                  them into project-level spec content that AI agents read at session boot."
)]
pub struct Cli {
    /// Request cooperative project cancellation by creating this file. Vibe never deletes it.
    #[arg(long, global = true)]
    pub stop_file: Option<PathBuf>,

    /// Produce machine-readable JSON output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Reduce output to a single summary line (useful in scripts / CI).
    #[arg(long, global = true, conflicts_with = "json")]
    pub quiet: bool,

    /// Human output with no colour, no cursor movement and no live fold:
    /// each block is written once, when it ends, as a log would read it.
    #[arg(long, global = true, conflicts_with = "json")]
    pub plain: bool,

    /// Show detailed progress diagnostics and child-operation output.
    /// Human progress is still suppressed by `--quiet` and `--json`.
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Disable progress activity and verbose progress detail without changing
    /// ordinary command output. `VIBE_NO_PROGRESS` provides the same opt-out.
    #[arg(long, global = true)]
    pub no_progress: bool,

    /// `json` writes every progress observation as one
    /// `vibe-progress-event/1` line on stderr, unthrottled and combinable
    /// with `--json` (PROP-060 `#MACHINE-STREAM`); `VIBE_PROGRESS_FORMAT`
    /// provides the same choice.
    #[arg(long, global = true, value_name = "FORMAT", value_enum)]
    pub progress_format: Option<ProgressFormat>,

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

    /// PROP-054 `##AGENT-HANDSHAKE`: how this invocation executes `agent`
    /// lifecycle contributions. `cli` calls the configured provider and pays
    /// for it (the R7.2 behaviour); `agent` never constructs a provider —
    /// each selected agent row is PARKED as a Markdown task under
    /// `.vibe/agentic/outbox/<run-id>/` for the hosting agent to perform,
    /// and the same command resumes the run once the declared outputs exist.
    /// The default, `auto`, resolves to `agent` exactly when the resolved
    /// `--invoked-by` / `VIBE_INVOKED_BY` value is present (something is
    /// hosting this process) and to `cli` otherwise. An explicit
    /// `cli`/`agent` always wins over `auto`'s inference.
    #[arg(
        long = "agent-mode",
        global = true,
        value_name = "MODE",
        default_value = "auto"
    )]
    pub agent_mode: AgentModeArg,

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

    /// PROP-010 §2.5: forbid network access for the invocation. Under
    /// `--offline`, resolution and fetch must be satisfiable entirely
    /// from local sources (the cache, `file://` mirrors, the project's
    /// own `vibe.lock` + `vibedeps/`); anything not available locally
    /// is a hard error with an actionable message — never a silent
    /// degrade to a partial result. Falls back to the `VIBE_OFFLINE`
    /// environment variable (truthy values: `1`, `true`, `yes`, `on`
    /// — case-insensitive), then the user-config `[net].offline` key;
    /// the flag wins on conflict. Online remains the default and is
    /// unchanged. `vibe install --offline` (PROP-030 §3.1) stays and
    /// ORs into the same posture as one more input.
    #[arg(long, global = true)]
    pub offline: bool,

    /// PROP-075 `#TEMPLATE-CACHE`: refetch a template package's version
    /// list for this invocation regardless of its age, instead of
    /// reading the copy cached under
    /// `~/.vibe/cache/templates/<group>/<name>/versions.json` while it
    /// is inside `[template] version_list_ttl` (fifteen minutes by
    /// default). A **digest** document is immutable and is never
    /// revalidated, by this flag or anything else: a released version's
    /// checksums do not change, and a change upstream is precisely the
    /// `template_render_changed` alarm. `--offline` wins on conflict,
    /// because a posture that forbids the network outranks a request to
    /// use it.
    #[arg(long, global = true)]
    pub refresh: bool,

    /// PROP-077 `#NO-RECOMMENDS-FLAG`: do not install what vibe was not
    /// asked for. One flag for every package kind, because it answers one
    /// question — may vibe install things I did not name? On a project
    /// `vibe install` / `vibe update` the solver's best-effort
    /// `[recommends]` expansion is not run at all, so the solve is the hard
    /// solve and nothing else; on `vibe install -g` / `vibe update -g` and
    /// an application install into a project home the recommendation walk
    /// is not entered; `vibe reinstall` of either plane follows its own
    /// plane; and an author's recommended **ecosystem** items (PROP-075
    /// `#ECOSYSTEM-RECOMMENDS`) are treated as empty, because an author's
    /// recommendation is one idea with two spellings. `[suggests]` is
    /// unaffected: it installs nothing and is only ever reported.
    ///
    /// It changes a solve, never a replay. A project's lock already records
    /// an accepted recommendation as a non-root graph member, so `vibe
    /// install` with no arguments materialises exactly what the lock says
    /// whichever way this flag is passed; a later deliberate re-solve
    /// (`vibe update`) re-applies whichever way it is passed then. Nothing
    /// is persisted, and there is deliberately no `--with-recommends`:
    /// recommendations are on by default, and a flag that re-asserts a
    /// default is surface with no question behind it.
    ///
    /// On a command that neither solves nor installs it is a usage refusal
    /// naming this flag: a flag silently ignored is worse than a flag
    /// refused.
    #[arg(long = "no-recommends", global = true)]
    pub no_recommends: bool,

    /// PROP-066 `#layers`: read `[[remap]]` rows from this file for this
    /// invocation. Repeatable; the last file wins among them, and every
    /// one of them outranks a row in `~/.vibe/settings.toml`,
    /// `.vibe/settings.local.toml` or a PROP-002 `[[mirror]]` /
    /// `[[override]]` table. `VIBE_REMAP=<file>[,<file>]` names the same
    /// files through the environment; the flags come later and therefore
    /// win. A row changes only WHERE an artifact is looked for — every
    /// digest is still checked exactly as before, so a remap can make a
    /// fetch fail or succeed and can never change what is installed.
    /// `vibe remap list` shows the active rows and `vibe remap explain`
    /// the chain for one fetch.
    #[arg(long = "remap", global = true, value_name = "FILE")]
    pub remap: Vec<PathBuf>,

    /// PROP-066 `#consent`: accept the remap packages this run meets.
    /// Bare (`--accept-remap`) accepts every offer the run would apply;
    /// with a value (`--accept-remap org.corp/intranet`, repeatable)
    /// only the named ones. An acceptance is recorded against the
    /// package's current content hash, so a later version of it is a new
    /// offer. Without this flag an attended terminal is asked and an
    /// unattended run refuses with `consent_required`.
    #[arg(
        long = "accept-remap",
        global = true,
        value_name = "PACKAGE",
        num_args = 0..=1,
        default_missing_value = "",
        action = clap::ArgAction::Append,
    )]
    pub accept_remap: Vec<String>,

    /// PROP-066 `#consent`: refuse every unanswered remap offer for THIS
    /// run and record nothing — the rows do not apply and the next run
    /// asks again. `vibe remap reject <package>` is the permanent answer.
    #[arg(long = "reject-remap", global = true)]
    pub reject_remap: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// The renderer family `--progress-format` selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ProgressFormat {
    /// The terminal renderer of PROP-060 `#DISPLAY`.
    Human,
    /// One JSON line per observation on stderr.
    Json,
}

mod command;
pub use command::Command;

/// `vibe bin` subcommands (PROP-025 §4).
#[derive(clap::Subcommand, Debug)]
pub enum BinCmd {
    /// Every `[[binary]]` declared by the project's installed packages,
    /// with build state and description.
    List,
    /// Build the named tools (default: all declared) release-mode in
    /// their slots. Consent-gated: a non-`org.vibevm` group requires
    /// `--assume-yes` (the build runs the package's build scripts).
    Build {
        /// Binary names; empty builds everything declared.
        names: Vec<String>,
        /// Consent to build non-allow-listed groups' code.
        #[arg(long)]
        assume_yes: bool,
    },
    /// Print the artifact path (non-zero exit when not built).
    Path {
        /// The declared binary name.
        name: String,
    },
    /// Resolve through THIS project's lockfile and run the tool,
    /// building it first if absent. The exit code passes through.
    #[command(trailing_var_arg = true, allow_hyphen_values = true)]
    Exec {
        /// The declared binary name.
        name: String,
        /// Consent to build non-allow-listed groups' code.
        #[arg(long)]
        assume_yes: bool,
        /// Arguments handed to the tool unchanged.
        args: Vec<String>,
    },
}

// The grammar's reds live in their own cell: this file is the argument
// surface, and the surface plus its proofs is two responsibilities.
#[cfg(test)]
#[path = "cli/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cli/app_tests.rs"]
mod app_tests;
