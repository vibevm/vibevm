//! Top-level command grammar, exported through the existing cli module.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");
use super::*;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new vibevm project in the target directory.
    Init(InitArgs),

    /// List the packages recorded in the project's lockfile.
    List(ListArgs),

    /// List every extension declaration in the selected installed world.
    Extensions(ExtensionsArgs),

    /// Validate the workspace, without network access or materialisation.
    Validate(LifecycleArgs),

    /// Install one or more packages into the current project.
    Install(InstallArgs),

    /// Run the default lifecycle through generated-source production.
    Generate(LifecycleArgs),

    /// Run the default lifecycle through deterministic build work.
    Build(LifecycleArgs),

    /// Run the default lifecycle through deterministic tests.
    Test(LifecycleArgs),

    /// Run one explicit host command declared by `[[command]]` in `vibe.toml`.
    Run(RunArgs),

    /// List and run launchers of an already installed user application.
    App(AppArgs),

    /// Start an interactive shell with installed applications on PATH and the
    /// given per-shell provider choices (`--use SLOT=KEY`).
    #[command(long_about = shell::LONG_ABOUT)]
    Shell(ShellArgs),

    /// Show which remap rows are active and why one fetch resolves where
    /// it does (PROP-066).
    Remap(RemapArgs),

    /// Manage the storage libraries that hold installed applications: list,
    /// add, attach, detach, move an installation between them, and gc.
    Storage(StorageArgs),

    /// Configure the coding agents that run the agent steps of application
    /// installers (PROP-064): list, presets, use, add, remove.
    Agent(AgentArgs),

    /// Run the default lifecycle through agentic creation.
    Create(LifecycleArgs),

    /// Run the default lifecycle through output verification.
    Verify(LifecycleArgs),

    /// Run the default lifecycle through distributable assembly.
    Package(LifecycleArgs),

    /// Run all nine default-lifecycle phases through deployment, then
    /// reconcile the selected deploy profile's targets. `--plan` reports
    /// the planned work read-only instead of running the chain.
    Deploy(DeployArgs),

    /// Reverse the selected deploy profile: remove every receipt-owned
    /// resource, in reverse dependency order. It removes only what a
    /// receipt still owns, and refuses a path changed after deployment.
    Undeploy(UndeployArgs),

    /// List the deployments this machine holds receipts for — project,
    /// profile, target, generation, status, scope and timestamps. Never
    /// secrets. Read-only; `--json` emits the machine form.
    Deployments,

    /// Remove the derived prompt state — dependency slots and generated
    /// boot artifacts — keeping every authored file, the lock, and the
    /// machine cache. Any lifecycle phase may follow the clean prefix.
    Clean(CleanArgs),

    /// Plan or perform terminal removal of the selected VibeVM project layer.
    /// `attach` and `detach` remain reserved for a future live-tool surface.
    Scrape(ScrapeArgs),

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

    /// Work with a documentation package (PROP-057). `vibe doc check
    /// --examples` runs every documented command in a sandbox and compares
    /// its output with what the page promises, exactly.
    Doc(DocArgs),

    /// Inspect and edit the project's consumer-owned adoption-facts registry.
    Facts(crate::commands::facts::FactsArgs),

    /// One bounded, read-only metadata answer about what the selected
    /// project's specs declare and what it recorded about them (PROP-054
    /// `##REF-REQUIREMENTS-SURFACES`): per addressed fact its authoring
    /// status, this project's consumer adoption, and — with `--relations`
    /// — the edges a specmap carries. `--json` is the generated report
    /// exactly; combining the axes into a verdict is never this query's.
    Requirements(RequirementsArgs),

    /// Explain why a package is in (or out of) this project's effective
    /// world: the admitting chain with its rule, or the blocked edges and
    /// what blocked them (PROP-050 ##VIBE-WHY).
    Why(crate::commands::why::WhyArgs),

    /// Meaning-preserving rewrites of authored source trees.
    Refactor(crate::commands::refactor::RefactorArgs),

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

    /// The downloads of this machine: active ones with their last sample,
    /// the history, and a live feed (PROP-068).
    Transfers(TransfersArgs),

    /// Download one file over HTTP(S) through vibe's own network stack —
    /// source policy, `[proxy]`, px, trust — with every phase of the route
    /// described (PROP-068 `#dl`). `--show` / `--headers` print the body or
    /// the response head; `--test-proxy` checks the configured proxy.
    Dl(DlArgs),

    /// The artifact drops of this machine (PROP-066 `#drops`):
    /// `vibe artifacts where [<package>]` prints where vibe releases and
    /// package payloads are expected, by exact file name.
    Artifacts(ArtifactsArgs),

    /// Lend a facility vibe runs for itself to a command you name: today
    /// the vibe-managed px, so `pip`, `npm` or `curl` reach the network
    /// through exactly the proxy vibe uses (`vibe tool px exec -- …`).
    Tool(ToolArgs),

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
