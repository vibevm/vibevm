//! Command-line argument schema.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.
//!
//! This file is the hub: the top-level `Cli` / `Command` pair lives
//! here; the per-command-family argument structs live in the `cli/`
//! submodules and are re-exported below, so consumers keep addressing
//! everything as `crate::cli::X`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use clap::{Parser, Subcommand};

mod agentic;
mod aiui;
mod cache;
mod explain;
mod inspect;
mod mcp;
mod pkg;
mod prefs;
mod progress;
mod query;
mod registry;
mod select;
mod skill;
mod specmap;
mod term;
mod vars;
mod vvm;
mod workspace;

pub use agentic::*;
pub use aiui::*;
pub use cache::*;
pub use explain::*;
pub use inspect::*;
pub use mcp::*;
pub use pkg::*;
pub use prefs::*;
pub use progress::*;
pub use query::*;
pub use registry::*;
pub use select::*;
pub use skill::*;
pub use specmap::*;
pub use term::*;
pub use vars::*;
pub use vvm::*;
pub use workspace::*;

#[derive(Debug, Parser)]
#[command(
    name = "vibe",
    version = env!("CARGO_PKG_VERSION"),
    about = "The disciplined runtime for spec-driven vibecoding.",
    long_about = "vibevm: a CLI software project manager for spec-driven AI-assisted development.\n\
                  Manages installable building blocks — flow, feat, stack, tool, mcp, lang — and assembles\n\
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

    /// The agent-facing observation surface (PROP-042). `vibe aiui render`
    /// renders the `vibe tree` TUI headlessly to a symbolic snapshot (text or
    /// cells) — no terminal, deterministic — so an agent can *see* the
    /// interface and golden tests can diff it. Read-only.
    Aiui(AiuiArgs),

    /// Launch the vibeterm terminal app hosting a detected interactive shell
    /// (Windows prefers PowerShell 7 `pwsh`; unix uses `$SHELL`). The terminal an
    /// agent or human can observe; `--exec` overrides the shell (PROP-042 §5).
    Term(TermArgs),

    /// Launch the vibeframe terminal — the simple terminal frame VibeTree runs
    /// in (a copy of vibeterm's minimal single-window terminal). Same flags as
    /// `term`; hosts the detected shell or `--exec`.
    Frame(TermArgs),

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

    /// Inspect and edit the project's consumer-owned adoption-facts registry.
    Facts(crate::commands::facts::FactsArgs),

    /// Explain why a package is in (or out of) this project's effective
    /// world: the admitting chain with its rule, or the blocked edges and
    /// what blocked them (PROP-050 ##VIBE-WHY).
    Why(crate::commands::why::WhyArgs),

    /// The sealed-circle report for one provider: open / sealed / the
    /// named circle, who actually befriends it, which grants its
    /// allow-friends rejects, and whether it is in the root's friend
    /// closure (PROP-050 ##ALLOW-FRIENDS-EXHAUSTIVE).
    Friends(crate::commands::friends::FriendsArgs),

    /// Inspect computed project state (effective spec, configuration).
    Show(ShowArgs),

    /// Inspect and edit application/user preferences — the three-level
    /// app-prefs store (`vibe-settings`, PROP-040 §8). `vibe prefs get/set/
    /// list/check/migrate` plus `vibe prefs show-origins`. Distinct from
    /// `vibe show config` (the project-config view).
    Prefs(PrefsArgs),

    /// Analyze the resolved spec/dependency tree (PROP-036): the effective
    /// boot load type per package (`static` / `dynamic` / `none`), the
    /// transitive / condition / static-lane flags, the two boot lanes, and the
    /// in-place `@spec` markers. Read-only. `--json` emits the machine model
    /// (validated against the shipped `package-tree.schema.v1.json`); a
    /// non-tty or `--plain` renders a static ASCII tree.
    Tree(TreeArgs),

    /// Manage the registry cache (clone, sync).
    Registry(RegistryArgs),

    /// Operate on the machine-global package store `~/.vibe/cache/`
    /// (PROP-010 §2.8): `path` prints its root, `list` the
    /// offline-resolvable inventory, `add` pre-warms packages (and
    /// their dependency closure) into it without touching any project,
    /// `clean` reclaims space — all, by age, or by package, always as
    /// an explicit operator action. Top-level on purpose: the store is
    /// machine-global and its headline case is work that has no
    /// project yet.
    Cache(CacheArgs),

    /// Operate on a multi-package workspace (PROP-007). Today the one
    /// subcommand is `publish` — walk the workspace's self-publishing
    /// members in dependency order and publish each as its own
    /// repository.
    Workspace(WorkspaceArgs),

    /// Manage vibevm's own versions on this machine — the VibeVM Version
    /// Manager (VVM, PROP-019). `vibe self install <selector>` builds and
    /// installs a version from source; `vibe self use` switches the active
    /// one; `vibe self ls` lists what is installed. Self-distribution: the
    /// `vibe` binary manages its own versions.
    #[command(name = "self")]
    Vvm(VvmArgs),

    /// The registry of what this project can invoke: every `[[binary]]` and
    /// every `[[mcp_server]]` the installed packages declare, in one table.
    /// The boot lane already names which language disciplines are installed;
    /// this names what they brought that can be RUN. `--json` for agents.
    Tools {
        /// Emit the registry as JSON rather than a table.
        #[arg(long)]
        json: bool,
    },

    /// Build and dispatch the tools installed packages declare via
    /// `[[binary]]` (PROP-025): `list` the table, `build` (consent-gated)
    /// into the slot, `path` an artifact, `exec` through the project's
    /// lockfile — the rustup dispatch model.
    Bin {
        #[command(subcommand)]
        cmd: BinCmd,
    },

    /// Traceability explain over THIS tree (PROP-014 §2.6): build the
    /// specmap fresh in memory and render what implements, verifies,
    /// documents, or deviates from a spec unit or code symbol — the host's
    /// built-in answer to the canonical "which test verifies this rule?"
    /// (`vibe explain "spec://…#anchor"`). `--json` emits the raw one-hop
    /// subgraph; the default is the deterministic text view. Contrast
    /// `vibe trace`, a delegating alias to the installed stack's `trace`.
    Explain(ExplainArgs),

    /// Search the code↔spec map by independent filters (A5A-MAPSEARCH):
    /// `--uri` (exact spec address), `--symbol` (substring of a code
    /// symbol), and `--kind` (a code `item_kind` or a spec unit kind),
    /// AND-joined, over a hard result ceiling (`--limit`, default 50, max
    /// 200; no unbounded mode). None is required — bare `vibe query` shows a
    /// bounded slice of the whole map, with truncation named. The grep-like
    /// counterpart to `vibe explain` (a point lookup): `explain` looks at
    /// ONE target's subgraph; `query` FINDS the many nodes that fit.
    /// `--json` emits the machine-readable form. Read-only.
    Query(QueryArgs),

    /// Search the code↔spec map by a conjunctive predicate query and walk the
    /// bipartite graph (E-A5B-QUERYLANG) — the traversal layer over `vibe
    /// query`. `--where` carries the query: `uri:`/`symbol:`/`kind:` (the
    /// same filters as `query`), `scope:` (a `spec://` prefix), `has:`/`lacks:`
    /// (a verb an edge does/does not touch), and `depth:<0..3>` (an undirected
    /// walk; seeds stay at `d0`). Predicates are whitespace-AND-joined. Reach
    /// for `select` over `query` when the answer is relational — "every spec
    /// rule with NO verifier" (`lacks:verifies`), "the implementers of this
    /// rule and one hop around them" (`uri:… depth:1`); reach for `query` for
    /// a flat filter, and `explain` for one target's subgraph. `--json` emits
    /// the machine-readable form. Read-only.
    Select(SelectArgs),

    /// Generate the package's carried traceability map (V5-PACKAGE-MAP §2.2).
    /// Reads the package's `vibe.toml` (for its `(group, name)` coordinate) and
    /// `specmap.toml` (its scan policy; presence is the opt-in), and writes the
    /// map — built fresh with the same engine `vibe explain` uses — minted under
    /// the coordinate `spec://<group>/<name>/…` (globally unique, where the
    /// local `specmap.toml` nickname is not), so a consumer can query an
    /// installed package without rebuilding. A package without a `specmap.toml`
    /// is left untouched. Read-only to the tree outside the one map file.
    Specmap(SpecmapArgs),

    /// Traceability queries over the project's specmap (PROP-014 §2.6) —
    /// a delegating alias: arguments pass through verbatim to the
    /// installed `rust-ai-native trace` (the engine ships with the
    /// discipline stack and versions with the project, not with vibe).
    /// Example: `vibe trace explain "spec://<ns>/<doc>#<anchor>"`.
    #[command(trailing_var_arg = true, allow_hyphen_values = true)]
    Trace {
        /// Arguments handed to `rust-ai-native trace` unchanged.
        args: Vec<String>,
    },

    /// Print the runtime variable context — the values vibevm actually uses
    /// (derived from the running binary's location) versus the environment,
    /// so scripts can reconcile a stale `$VIBEVM_HOME` (PROP-019 §2.14).
    /// Modes: `vibe vars`, `vibe vars diff`, `vibe vars full`,
    /// `vibe vars full diff`.
    Vars(VarsArgs),

    /// Progress Control (PROP-043): scan/validate the inline `<status>`
    /// markup, render reports, and drive the actualization campaign
    /// (mirror, weave, rescan, resume).
    Progress(ProgressArgs),

    /// Print version information.
    Version,
}

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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    /// The root `--offline` parses before any subcommand (PROP-010
    /// §2.5): the posture is a property of the invocation, not of one
    /// subcommand.
    #[test]
    fn offline_flag_parses_on_the_root() {
        let cli = Cli::try_parse_from(["vibe", "--offline", "list"])
            .expect("parse `vibe --offline list`");
        assert!(cli.offline, "--offline reaches the root Cli");
        let Command::List(_) = cli.command else {
            panic!("argv did not parse to `list`");
        };
    }

    /// Absent the flag, the root posture is online — the default.
    #[test]
    fn offline_defaults_to_false_on_the_root() {
        let cli = Cli::try_parse_from(["vibe", "list"]).expect("parse `vibe list`");
        assert!(!cli.offline);
    }

    /// `vibe install --offline` (PROP-030 §3.1) keeps parsing to the
    /// subcommand's own flag — the posture absorbs it as one more
    /// input, it does not replace it. Note clap's actual mechanics for
    /// a global root arg that shares its id with a subcommand arg:
    /// they are one argument, so both matches carry the value. That is
    /// harmless here — `install::run` resolves the posture as
    /// `root_offline || args.offline`.
    #[test]
    fn install_local_offline_flag_still_parses() {
        let cli = Cli::try_parse_from(["vibe", "install", "--offline"])
            .expect("parse `vibe install --offline`");
        let Command::Install(args) = cli.command else {
            panic!("argv did not parse to `install`");
        };
        assert!(args.offline, "--offline reaches InstallArgs");
        assert!(cli.offline, "the shared id also sets the root field");
    }

    /// `vibe --offline install` sets the root posture — and, because
    /// clap unifies the global root arg with the same-id subcommand
    /// arg, `InstallArgs.offline` sees it too. Either way the OR in
    /// `install::run` resolves the same posture.
    #[test]
    fn root_offline_reaches_the_install_command() {
        let cli = Cli::try_parse_from(["vibe", "--offline", "install"])
            .expect("parse `vibe --offline install`");
        assert!(cli.offline);
        let Command::Install(args) = cli.command else {
            panic!("argv did not parse to `install`");
        };
        assert!(args.offline, "the shared id carries the root flag down");
    }
}
