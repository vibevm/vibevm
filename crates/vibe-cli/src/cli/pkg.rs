//! Argument structs for the package-lifecycle commands — `vibe init`,
//! `list`, `install`, `outdated`, `search`, `update`, `reinstall`,
//! `uninstall`.
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use super::AppStepArgs;

mod discovery;
pub use discovery::{OutdatedArgs, SearchArgs};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Filter by package kind (flow, feat, stack, tool, mcp, lang, doc, app, remap).
    #[arg(long)]
    pub kind: Option<String>,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Append per-package active features and subskill paths to the
    /// text-mode output. JSON output already carries these fields
    /// regardless. Off by default to preserve table width.
    #[arg(long)]
    pub verbose: bool,
}

impl InstallArgs {
    /// Project this grammar onto the shared service's neutral inputs.
    ///
    /// Only what the shared core itself reads crosses. Registry, solver and
    /// source-preference flags do not: their one consumer is the package-source
    /// factory, which is this surface's own composition root and closes over
    /// these args directly. Neither does any source-mutation grammar —
    /// `--git`/`--tag`/`--branch`/`--rev`/`--git-auth`/`--git-token-env` are
    /// this surface's vocabulary with this surface's exit codes, and they reach
    /// the core only through the manifest-mutation port.
    pub(crate) fn inputs(&self) -> vibe_orchestrator::InstallInputs {
        vibe_orchestrator::InstallInputs {
            packages: self.packages.clone(),
            features: self.features.clone(),
            no_default_features: self.no_default_features,
            all_features: self.all_features,
            language: self.language.clone(),
            exact: self.exact,
        }
    }

    /// The narrow execution policy, decided from the ONE user config this
    /// command loaded and the offline ladder it already resolved.
    ///
    /// `root_offline` is the root command's own `--offline`; the ladder
    /// (surface flags > `VIBE_OFFLINE` > user config) collapses HERE, at the
    /// surface that owns every rung, so no rung is re-derived below.
    pub(crate) fn policy(
        &self,
        root_offline: bool,
        root_no_recommends: bool,
        user_config: &vibe_core::user_config::UserConfig,
    ) -> vibe_orchestrator::InstallPolicy {
        vibe_orchestrator::InstallPolicy {
            offline: crate::output::resolve_offline(
                root_offline || self.offline,
                user_config.net.offline,
            ),
            slot_integrity: user_config.install.slot_integrity,
            spec_format_default: user_config.install.spec_format,
            no_recommends: root_no_recommends,
        }
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct InstallArgs {
    #[command(flatten)]
    pub source_policy: super::SourcePolicyArgs,
    /// Zero or more package references, each `<group>/<name>[@<version>]`.
    /// A bare `<name>` works only where a registry has ISSUED that short
    /// name in its own alias table (PROP-008 §2.6) — a package is not
    /// reachable by its own name; `vibe search` marks the names that are
    /// issued.
    /// When empty, `vibe install` reads the project's `vibe.toml`
    /// `[requires].packages` and installs every entry — same shape as
    /// `cargo build` / `npm install` against an existing manifest.
    /// When non-empty, each pkgref is added to (or updates the
    /// constraint on) `vibe.toml` `[requires].packages`.
    pub packages: Vec<String>,

    /// Install a user application or an `mcp:` package under the Vibe settings root.
    #[arg(short = 'g', long = "global")]
    pub global: bool,

    /// Target coding agent for a global MCP package. Without this flag an
    /// interactive terminal offers all detected agents by default. For a
    /// global application, the agent that runs the installer's agent steps:
    /// a definition of agents.toml or a built-in preset (PROP-064).
    #[arg(long, requires = "global")]
    pub agent: Option<String>,

    /// Global MCP server names: comma-separated, or a JSON string array for literal commas.
    /// Without the flag, multiple servers offer checkboxes (default: all).
    #[arg(long, requires = "global")]
    pub server: Option<String>,

    /// Select an application platform or distribution ID (for example `ubuntu`).
    #[arg(long, value_name = "PLATFORM")]
    pub platform: Option<String>,

    /// Select an application architecture ID within the selected platform.
    #[arg(long, value_name = "ARCH")]
    pub arch: Option<String>,

    /// Select one application installer option by its stable ID.
    #[arg(long, value_name = "ID")]
    pub installer: Option<String>,

    /// Place an owned (archive or source-built) application payload in this
    /// storage library of the user home instead of the default
    /// `<settings>/opt`; refused for a project home.
    #[arg(long, value_name = "ID")]
    pub library: Option<String>,

    /// Application feature, resume and step flags (PROP-065).
    #[command(flatten)]
    pub steps: Box<AppStepArgs>,

    /// `install -g` of an application: leave out the `add` list of your own
    /// `[slot.<slot>.extras.<ecosystem>]` when the new version's ecosystem
    /// adapters are run (PROP-075 `#SLOT-EXTRAS`).
    ///
    /// Your `drop` list is still honoured, and the author's own recommended
    /// items still arrive — `--no-recommends` is the flag that declines
    /// those. Declining your extras must never install *more* than you
    /// asked for.
    #[arg(long, requires = "global")]
    pub no_extras: bool,

    /// `install -g` of an application: a file of extra HTTP request
    /// headers to send with this install's own input downloads — one
    /// `Name: value` per line, `#` comments and blank lines skipped
    /// (PROP-075 `#DOWNLOAD-HEADERS`).
    ///
    /// A header value is a credential, so it is a **file** and never an
    /// argument: every local user can read `/proc/<pid>/cmdline`. The
    /// values are never logged, journaled or put in an error, they reach
    /// only `http://` and `https://` inputs of this install — not registry,
    /// index or template reads — and they are not sent to any origin other
    /// than the one the input's URL names.
    #[arg(long, value_name = "PATH", requires = "global")]
    pub download_header_file: Option<PathBuf>,

    /// `install -g` of an application: also write the resolved version into
    /// the slot's declared version file (PROP-075 `#VERSION-FILE-WRITE`).
    #[command(flatten)]
    pub version_file: super::WriteVersionFileArgs,

    /// Resolve and print the complete project closure without publication,
    /// or print the selected global application's installation plan.
    #[arg(long)]
    pub plan_only: bool,

    /// Install an application without exposing it: no proxy, no place among
    /// its slot's alternatives, never the default (PROP-059
    /// `#hidden-installations`); `vibe shell --with` or `vibe app expose`
    /// reach it.
    #[arg(long)]
    pub hidden: bool,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Path to a local-directory registry (M0 only; M1 adds git registry).
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Skip the interactive confirmation prompt (non-interactive envs).
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// Override the project's resolved language preference for this
    /// install (PROP-003 §2.7). BCP-47 tag (`ru`, `ja`, `pt-BR`).
    /// When set, supplies the head of the language fallback chain;
    /// canonical (no-suffix) content is always the last fallback.
    /// Without this flag, the project-level `[i18n]` block in
    /// `vibe.toml` decides; absent any declaration, English-only.
    #[arg(long)]
    pub language: Option<String>,

    /// Activate one or more features on every root package
    /// (PROP-003 §2.4). Repeatable, and accepts comma-separated lists:
    /// `--features a,b --features c` is equivalent to passing `a`,
    /// `b`, `c` together. Underscore-prefixed implementation-detail
    /// features cannot be activated this way. Default features are
    /// also included unless `--no-default-features` is set.
    #[arg(long, value_delimiter = ',')]
    pub features: Vec<String>,

    /// Skip activation of `[features].default` entries on every root
    /// package. Combined with `--features X,Y`, only the explicitly
    /// listed features are active.
    #[arg(long)]
    pub no_default_features: bool,

    /// Activate every non-private (no `_`-prefixed) feature on every
    /// root package. Mutually exclusive with `--features`; when both
    /// are set, `--all-features` wins.
    #[arg(long)]
    pub all_features: bool,

    /// Pin the resolved version exactly (`=x.y.z`) instead of the
    /// default caret constraint (`^x.y.z`) when writing pkgrefs to
    /// `vibe.toml` `[requires].packages`. Same semantics as npm's
    /// `--save-exact`. Overrides whatever constraint shape the user
    /// supplied on the CLI, including explicit caret / tilde / range.
    #[arg(long)]
    pub exact: bool,

    /// Strict authentication gate — when set, a 401 / 403 against an
    /// `auth = "none"` (public) registry halts the install instead of
    /// walking to the next registry. Default behaviour (without this
    /// flag) follows PROP-002 §2.3.1: public-401 means "no public
    /// answer here", walk past, useful when the host returns 401 for
    /// missing public repos (GitVerse). Strict mode is for CI / cron
    /// where an authenticated registry is supposed to answer; if its
    /// 401 leaks through to a public fallback, you want to know
    /// rather than silently install a different package. Per-registry
    /// `auth = "token-env"` / `"credential-helper"` halt on 401
    /// regardless of this flag.
    #[arg(long)]
    pub auth_required: bool,

    /// Select the dependency solver cell (PROP-017). Defaults to
    /// `resolvo` (CDCL SAT); `naive` (DFS fast path) and `sat`
    /// (backtracking) are selectable fallbacks.
    #[arg(long, value_name = "naive|sat|resolvo")]
    pub solver: Option<String>,

    /// PROP-030: prefer the embedded registry (a source install's in-tree
    /// `packages/`) over the declared `[[registry]]` walk on a coordinate
    /// clash — already the default for a source-installed developer; this flag
    /// forces it. Mutually exclusive with `--no-prefer-embedded`.
    #[arg(long)]
    pub prefer_embedded: bool,

    /// PROP-030: consult the declared `[[registry]]` walk before the embedded
    /// registry, so a published package wins a coordinate clash and the
    /// embedded copy only fills gaps. Mutually exclusive with
    /// `--prefer-embedded`.
    #[arg(long)]
    pub no_prefer_embedded: bool,

    /// PROP-030: ignore the ambient embedded registry entirely for this
    /// command — resolve only from the declared `[[registry]]` walk. Also set
    /// by `VIBE_NO_DEFAULT_REGISTRY=1`.
    #[arg(long)]
    pub no_default_registry: bool,

    /// PROP-030 §3.1: resolve strictly offline — consult only the embedded
    /// registry and explicit local / path sources, never a network
    /// `[[registry]]`. No git host is contacted; a package not available
    /// locally fails without a single network request (and thus without any
    /// credential prompt). Contrast `--embedded-short-circuit`, which still
    /// reaches the network for coordinates the embedded registry lacks.
    #[arg(long)]
    pub offline: bool,

    /// PROP-030 §3.1: prefer the embedded registry and short-circuit the
    /// network for every coordinate it already provides — the declared
    /// `[[registry]]` walk is consulted ONLY for packages the embedded
    /// registry does not carry. A fully-embedded dependency graph resolves
    /// with zero network access (no version-enumeration round-trip, no
    /// credential prompt), while a genuinely missing package is still
    /// fetched from the network. Implies embedded-first precedence;
    /// mutually exclusive with `--no-prefer-embedded`.
    #[arg(long)]
    pub embedded_short_circuit: bool,

    /// PROP-030 §3.3: explicitly opt in to project-local packages (the
    /// `<project_root>/packages/` directory) winning a clash inside the
    /// local-registry family. This is already the default when
    /// project-packages are discovered; the flag is an explicit affirmation
    /// (mirrors `--prefer-embedded`'s relationship to its default). Mutually
    /// exclusive with `--no-prefer-local`.
    #[arg(long)]
    pub prefer_local: bool,

    /// PROP-030 §3.3: ignore the project-local `packages/` directory for
    /// this command — resolve only from vibe-embedded (when available) and
    /// the declared `[[registry]]` walk. Use when a project's `packages/`
    /// is stale, broken, or being deliberately bypassed. Mutually exclusive
    /// with `--prefer-local`. Does NOT suppress vibe-embedded — for that,
    /// use `--no-default-registry`.
    #[arg(long)]
    pub no_prefer_local: bool,

    /// Add a git-source declaration for the single positional pkgref
    /// — fetches the package directly from this git URL rather than
    /// resolving it through `[[registry]]`. PROP-002 §2.4.1.
    /// Requires exactly one of `--tag`, `--branch`, or `--rev`.
    /// Cannot be combined with `--exact` (constraint shape is
    /// orthogonal to git-source) or with `--registry` (git-source
    /// bypasses the registry layer).
    #[arg(long, value_name = "URL", group = "source")]
    pub git: Option<String>,

    /// Git tag to pin against when `--git <url>` is set. Mutually
    /// exclusive with `--branch` / `--rev`. Immutable; force-pushed
    /// tag rewrite caught as `IntegrityError` on next install via
    /// content-hash. PROP-002 §2.4.1.
    #[arg(long, value_name = "TAG", group = "git_ref", requires = "git")]
    pub tag: Option<String>,

    /// Git branch to track when `--git <url>` is set. Mutually
    /// exclusive with `--tag` / `--rev`. Mutable: `vibe install`
    /// (no `update`) sticks to the lockfile-pinned commit; `vibe
    /// update` re-walks branch HEAD. PROP-002 §2.4.1.
    #[arg(long, value_name = "BRANCH", group = "git_ref", requires = "git")]
    pub branch: Option<String>,

    /// Git commit SHA to pin against when `--git <url>` is set.
    /// Mutually exclusive with `--tag` / `--branch`. Most strict;
    /// the lockfile records the same SHA. PROP-002 §2.4.1.
    #[arg(long, value_name = "REV", group = "git_ref", requires = "git")]
    pub rev: Option<String>,

    /// Auth regime for the `--git <url>` target — same enum as
    /// `[[registry]] auth`: `none` / `token-env` / `credential-helper`
    /// / `ssh`. Default `none`. PROP-002 §2.4.1.
    #[arg(long, value_name = "AUTH", requires = "git")]
    pub git_auth: Option<String>,

    /// Env-var name when `--git-auth token-env`. Default derived
    /// from URL host (e.g. `https://gitlab.acme.example/...` →
    /// `VIBEVM_REGISTRY_TOKEN_GITLAB_ACME_EXAMPLE`).
    #[arg(long, value_name = "ENV_VAR", requires = "git")]
    pub git_token_env: Option<String>,

    /// Project install: ignore reusable lifecycle contribution fingerprints for
    /// the post-durability callback. Application install (global or into the
    /// project home): ask the selected installer to force its install operation.
    #[arg(long)]
    pub force: bool,

    /// PROP-054 `##OBS-TRACE`: record every compiler pass this invocation runs
    /// into `.vibe/trace/<run-id>/`. The flag is one half of the activation —
    /// the selected project's own `[compile] trace = true` is the other, and
    /// either alone enables it. Neither can switch the other off.
    #[arg(long)]
    pub trace_compile: bool,
}

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    #[command(flatten)]
    pub source_policy: super::SourcePolicyArgs,
    /// Package references `<group>/<name>` to update. Each must be
    /// currently installed, which is also why a bare `<name>` still
    /// works here: it is resolved against `vibe.lock` alone, never
    /// against a registry's issued short names (PROP-008 §2.6).
    /// Mutually exclusive with `--all`.
    pub packages: Vec<String>,

    /// Update exactly one installed user application under the Vibe settings root.
    #[arg(short = 'g', long = "global")]
    pub global: bool,

    /// Select an application platform or distribution ID (for example `ubuntu`).
    #[arg(long, value_name = "PLATFORM")]
    pub platform: Option<String>,

    /// Select an application architecture ID within the selected platform.
    #[arg(long, value_name = "ARCH")]
    pub arch: Option<String>,

    /// Select one application installer option by its stable ID.
    #[arg(long, value_name = "ID")]
    pub installer: Option<String>,

    /// Place an owned (archive or source-built) application payload in this
    /// storage library of the user home instead of the default
    /// `<settings>/opt`; refused for a project home.
    #[arg(long, value_name = "ID")]
    pub library: Option<String>,

    /// Application feature, resume and step flags (PROP-065).
    #[command(flatten)]
    pub steps: Box<AppStepArgs>,

    /// `update -g` of an application: leave out the `add` list of your own
    /// `[slot.<slot>.extras.<ecosystem>]` when the new version's ecosystem
    /// adapters are run (PROP-075 `#SLOT-EXTRAS`). Your `drop` list is
    /// still honoured.
    #[arg(long, requires = "global")]
    pub no_extras: bool,

    /// `update -g` of an application: also write the resolved version into
    /// the slot's declared version file (PROP-075 `#VERSION-FILE-WRITE`).
    #[command(flatten)]
    pub version_file: super::WriteVersionFileArgs,

    /// Explicit local source registry for a global application update.
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Update every package in the lockfile: each declared root moves to the
    /// newest version its constraint allows, at its locked source, and its
    /// dependencies follow. The default when no package is named. Mutually
    /// exclusive with `<packages>`.
    #[arg(long, conflicts_with = "packages")]
    pub all: bool,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// Ask the selected application installer to force its update operation.
    #[arg(long)]
    pub force: bool,

    /// Remove the replaced application installation at once instead of
    /// keeping it hidden for `vibe app rollback` (PROP-065
    /// `#update-and-rollback`).
    #[arg(long)]
    pub purge_previous: bool,

    /// The agent that runs the agent steps of a global application
    /// installer: a definition of agents.toml or a built-in preset (PROP-064).
    #[arg(long, value_name = "NAME", requires = "global")]
    pub agent: Option<String>,

    /// Pin the resolved version exactly (`=x.y.z`) in
    /// `vibe.toml` `[requires].packages` instead of preserving the
    /// existing constraint shape. Same flag as `vibe install`'s.
    /// Useful for "bump and pin" — re-resolve to a newer version
    /// AND tighten the manifest constraint to that exact version
    /// in one step. Without this flag, `vibe update` only refreshes
    /// the lockfile pin and leaves the manifest's `^` / `~` /
    /// range constraint untouched (cargo's default behaviour).
    #[arg(long)]
    pub exact: bool,

    /// Choose each updated package's source again from the current registry
    /// configuration (`[[registry]]` / `registry.toml`) instead of keeping the
    /// source recorded in `vibe.lock`. Without it an update may move versions
    /// but never changes where an already locked package comes from — use
    /// this after switching a registry, for example from HTTPS to SSH.
    #[arg(long)]
    pub reselect_source: bool,

    /// Refresh only where locked packages come from: choose each source again
    /// from the current registry configuration while keeping every locked
    /// version exactly. Applies to the named packages, or to every package in
    /// `vibe.lock` when none is named (or with `--all`). A source that lacks
    /// the locked version fails the update; no other version is substituted.
    #[arg(long, conflicts_with_all = ["reselect_source", "exact"])]
    pub sources_only: bool,

    /// Strict authentication gate — same semantics as
    /// `vibe install --auth-required`. When set, a 401 / 403
    /// against an `auth = "none"` (public) registry halts the
    /// update instead of walking past. Useful in CI / cron where
    /// a fallback to a public substitute would mask a private-
    /// registry outage.
    #[arg(long)]
    pub auth_required: bool,

    /// PROP-054 `##OBS-TRACE`: record every compiler pass this invocation runs
    /// into `.vibe/trace/<run-id>/`. The flag is one half of the activation —
    /// the selected project's own `[compile] trace = true` is the other, and
    /// either alone enables it. Neither can switch the other off.
    #[arg(long)]
    pub trace_compile: bool,
}

#[derive(Debug, clap::Args)]
pub struct ReinstallArgs {
    /// Any directory inside the workspace. Discovery bubbles up to the
    /// absolute workspace root; `vibe reinstall` regenerates the boot
    /// artifacts of every node. Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Re-fetch every locked package's content from its source
    /// repository — at the version `vibe.lock` pins, never re-resolving
    /// — bypassing the local cache and overwriting the current
    /// `vibedeps/` files. The escape hatch for a corrupted or
    /// hand-edited `vibedeps/` subtree. Without this flag,
    /// `vibe reinstall` only recomputes the boot artifacts from the
    /// materialised tree already on disk — no fetch, no network — which
    /// is the fix for a stale or wrongly-generated `INDEX.md`.
    #[arg(long)]
    pub force: bool,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// PROP-054 `##OBS-TRACE`: record every compiler pass this invocation runs
    /// into `.vibe/trace/<run-id>/`. The flag is one half of the activation —
    /// the selected project's own `[compile] trace = true` is the other, and
    /// either alone enables it. Neither can switch the other off.
    ///
    /// `--force` is a MATERIALISATION force and unrelated: it re-fetches from
    /// source, while this only asks the compiler to narrate itself.
    #[arg(long)]
    pub trace_compile: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Direct package reference; removes dependencies no longer needed by the workspace.
    /// The version selector is ignored on uninstall. A bare `<name>` is
    /// resolved against this project's `vibe.lock` — or, with `-g`, this
    /// home's application index — and never against a registry's issued
    /// short names, because the verb acts on what is already installed
    /// (PROP-008 §2.6).
    pub package: String,

    /// Uninstall one user application without mutating a project.
    #[arg(short = 'g', long = "global")]
    pub global: bool,

    /// Target agent registration for a global MCP package; `all` removes all owned registrations.
    #[arg(long, requires = "global")]
    pub agent: Option<String>,

    /// Ask the selected application installer to force its uninstall operation.
    #[arg(long)]
    pub force: bool,

    /// Also remove the applications that require this one at runtime, after
    /// reviewing them with --plan-only (PROP-065 `#uninstall-tree`).
    #[arg(long)]
    pub cascade: bool,

    /// Also remove the auto-installed applications nothing requires any more.
    #[arg(long)]
    pub autoremove: bool,

    /// Print the application removal plan and change nothing.
    #[arg(long)]
    pub plan_only: bool,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// PROP-066 `#uninstall`: when the package being removed offers
    /// accepted remap rows no other accepted package offers, copy those
    /// rows into this user home's own `[[remap]]` table — with
    /// `from = "<package>@<version>"` and the original reason — so the
    /// machine keeps fetching from the same places. Answers the question
    /// without a terminal; mutually exclusive with `--drop-remap`.
    #[arg(long, conflicts_with = "drop_remap")]
    pub keep_remap: bool,

    /// PROP-066 `#uninstall`: drop the orphaned remap rows with the
    /// package. Answers the question without a terminal.
    #[arg(long)]
    pub drop_remap: bool,
}
