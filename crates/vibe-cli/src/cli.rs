//! Command-line argument schema.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

    /// Remove an installed package from the current project.
    Uninstall(UninstallArgs),

    /// Re-fetch and apply changes for one or more installed packages.
    Update(UpdateArgs),

    /// Run the spec-consistency linter against the project tree.
    Check(CheckArgs),

    /// Inspect computed project state (effective spec, configuration).
    Show(ShowArgs),

    /// Manage the registry cache (clone, sync).
    Registry(RegistryArgs),

    /// Print version information.
    Version,
}

#[derive(Debug, clap::Args)]
pub struct RegistryArgs {
    #[command(subcommand)]
    pub command: RegistrySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RegistrySubcommand {
    /// Force a `git fetch` on the configured registry cache.
    Sync(RegistrySyncArgs),

    /// Publish a package directory as a tagged release in the
    /// configured registry organization. Maintainers only — needs a
    /// publish token (see RUNTIME-GUIDE.md).
    Publish(RegistryPublishArgs),

    /// Print the project's configured `[[registry]]` / `[[mirror]]` /
    /// `[[override]]` entries and the host adapter each registry will
    /// dispatch to.
    List(RegistryListArgs),

    /// Add a new `[[registry]]` block to `vibe.toml`.
    Add(RegistryAddArgs),

    /// Add a `[[mirror]]` block targeting a registry (or `*` for any).
    SetMirror(RegistrySetMirrorArgs),

    /// Remove a `[[registry]]` or `[[mirror]]` block from `vibe.toml`.
    Remove(RegistryRemoveArgs),

    /// Generate a local mirror directory containing every package
    /// referenced by `vibe.lock`, suitable for use as a
    /// `[[mirror]] url = "file:///<abs-path>"` for offline / air-gapped
    /// installs.
    Vendor(RegistryVendorArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistryVendorArgs {
    /// Output directory for the vendor mirror. Each package becomes a
    /// bare repo at `<out>/<kind>-<name>.git/` (or whatever the
    /// registry's naming convention produces). Defaults to
    /// `<project>/vendor/`.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// If `--out` exists and is non-empty, wipe it before vendoring.
    /// Without this flag, a non-empty target dir is a hard error —
    /// vibe never silently overwrites the operator's content.
    #[arg(long)]
    pub force: bool,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistrySyncArgs {
    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryListArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryAddArgs {
    /// Local alias for the new registry. Used in lockfile `registry`
    /// fields and to target `[[mirror]]` / `[[override]]` entries.
    pub name: String,

    /// Organization-root URL — any git URL `git` accepts
    /// (`git@host:org`, `ssh://...`, `https://...`).
    pub url: String,

    /// Registry-level ref (reserved for a future registry-metadata
    /// branch). Defaults to `main`.
    #[arg(long = "ref")]
    pub registry_ref: Option<String>,

    /// Naming convention mapping `<kind>:<name>` to a repo name under
    /// the org. One of `kind-name` (default), `name`, `kind/name`.
    #[arg(long = "naming")]
    pub naming: Option<String>,

    /// Where to insert the new registry in the priority list.
    /// `primary` makes it the first registry (the new default for
    /// publish + the first stop on resolve fallback). `append` adds
    /// it at the end. Defaults to `append`.
    #[arg(long = "position", default_value = "append")]
    pub position: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistrySetMirrorArgs {
    /// Target registry name (matches a `[[registry]].name`) or `*` for
    /// any registry.
    pub of: String,

    /// Mirror URL. Any git URL `git` accepts.
    pub url: String,

    /// Priority within the target registry's mirror chain — lower =
    /// tried first. Defaults to 0.
    #[arg(long = "priority", default_value_t = 0)]
    pub priority: i32,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveArgs {
    /// What to remove. Subcommand-style: `registry <name>` removes the
    /// `[[registry]]` with that name; `mirror <of> <url>` removes the
    /// `[[mirror]]` block matching exactly on `(of, url)`.
    #[command(subcommand)]
    pub target: RegistryRemoveTarget,
}

#[derive(Debug, Subcommand)]
pub enum RegistryRemoveTarget {
    /// Remove a `[[registry]]` named `<NAME>`. Refuses if any
    /// `[[mirror]]` targets this registry by name (those would be
    /// orphaned). Wildcard `of = "*"` mirrors are unaffected.
    Registry(RegistryRemoveRegistryArgs),

    /// Remove a `[[mirror]]` exactly matching `(<OF>, <URL>)`.
    Mirror(RegistryRemoveMirrorArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveRegistryArgs {
    /// `[[registry]].name` to remove.
    pub name: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveMirrorArgs {
    /// `[[mirror]].of` of the entry to remove.
    pub of: String,

    /// `[[mirror]].url` of the entry to remove (exact match).
    pub url: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryPublishArgs {
    /// Path to the package directory (containing `vibe-package.toml`).
    #[arg(required = true)]
    pub source: PathBuf,

    /// Name of the `[[registry]]` to publish into. Defaults to the
    /// first registry in `vibe.toml`. Conflicts with `--repo-url`.
    #[arg(long = "registry", conflicts_with = "repo_url")]
    pub registry: Option<String>,

    /// Push directly to the given git URL — SSH or HTTPS — bypassing
    /// every host API. The repo must already exist on the host. Git
    /// authentication is the local user's: SSH agent, `credential.helper`,
    /// `~/.netrc`, whatever the local git is wired to use. No publish
    /// token is loaded on this path. Conflicts with `--registry`.
    #[arg(long = "repo-url", conflicts_with = "registry")]
    pub repo_url: Option<String>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would happen but make no API calls or pushes.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Directory to initialize (defaults to the current working directory).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Pre-set the active stack name (still requires installation separately).
    #[arg(long)]
    pub stack: Option<String>,

    /// Project name; defaults to the basename of the target directory.
    #[arg(long)]
    pub name: Option<String>,

    /// Override the default registry URL written into `vibe.toml`.
    /// When unset, `vibe init` writes two `[[registry]]` blocks: the
    /// `vibespecs` organisation on GitHub (primary, drives `vibe
    /// registry publish` and the first stop on resolve fallback) and
    /// `vibespecs-gitverse` on GitVerse (secondary, queried on
    /// `UnknownPackage` fall-through). Setting this flag replaces both
    /// defaults with a single `[[registry]]` pointing at the supplied
    /// URL. Conflicts with `--no-registry`.
    #[arg(long = "registry-url", conflicts_with = "no_registry")]
    pub registry_url: Option<String>,

    /// Override the default ref (`main`) recorded under `[registry]`.
    /// Conflicts with `--no-registry`.
    #[arg(long = "registry-ref", conflicts_with = "no_registry")]
    pub registry_ref: Option<String>,

    /// Do not write a `[registry]` section into `vibe.toml`. The
    /// project will then require `--registry <path>` on every
    /// `vibe install`, or a manual edit to `vibe.toml` later.
    #[arg(long = "no-registry")]
    pub no_registry: bool,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Filter by package kind (flow, feat, stack, tool).
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

#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    /// One or more package references, each `<kind>:<name>[@<version>]`.
    #[arg(required = true)]
    pub packages: Vec<String>,

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
}

#[derive(Debug, clap::Args)]
pub struct OutdatedArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("query_mode")
        .args(["query", "purl"])
        .required(true)
        .multiple(false)
))]
pub struct SearchArgs {
    /// Free-text query. Tokenised on the server side: lowercase ASCII
    /// alphanumeric runs, common stopwords filtered, single-character
    /// tokens dropped. At least one 2+ character non-stopword must
    /// remain after filtering for the query to match anything.
    /// Mutually exclusive with `--purl`.
    pub query: Vec<String>,

    /// Direct PURL lookup — find every package whose `[package].describes`
    /// or any subskill's `describes` equals this Package URL. Mutually
    /// exclusive with the positional free-text query. Hits carry a
    /// `binding_site` field (`package` vs `subskill`) so consumers see
    /// where the match originated.
    #[arg(long)]
    pub purl: Option<String>,

    /// Restrict results to a single package kind (`flow`, `feat`,
    /// `stack`, `tool`). Applies only to free-text search; PURL
    /// lookup ignores it.
    #[arg(long)]
    pub kind: Option<String>,

    /// Restrict to one configured `[[registry]]` by name. Default: walk
    /// every registry that has `VIBEVM_INDEX_URL_<R>` set in the
    /// environment.
    #[arg(long)]
    pub registry: Option<String>,

    /// Maximum hits to fetch from each registry's index server. The
    /// server may apply its own cap; the union is then deduplicated
    /// by `(kind, name)` keeping the highest-score hit. Defaults to
    /// 20 — large enough to be useful, small enough that no single
    /// registry dominates. Ignored on `--purl` lookups (PURL match
    /// is exact and saturates well below any reasonable limit).
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// For registries without a configured `VIBEVM_INDEX_URL_<R>`,
    /// fall back to a naive org-walk via the host's REST API. v0
    /// supports GitHub-hosted registries only (`github.com`); other
    /// hosts are reported as unsupported. Slower than an index;
    /// rate-limited; reads `vibe-package.toml` from each repo's HEAD
    /// via the GitHub Contents API. Ignored on `--purl` lookups.
    #[arg(long = "full-scan")]
    pub full_scan: bool,

    /// Bypass the persistent search cache under
    /// `~/.vibe/search-cache/`. Reads still go to the network even
    /// when a fresh entry exists; freshly fetched results are not
    /// written back. Useful for testing or when an index update
    /// landed and the operator wants to force a refresh without
    /// waiting for the TTL to expire.
    #[arg(long = "no-cache")]
    pub no_cache: bool,

    /// Override the default cache TTL (1 hour) — entries older than
    /// this many seconds are treated as misses. Ignored when
    /// `--no-cache` is set.
    #[arg(long = "cache-ttl")]
    pub cache_ttl: Option<u64>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

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
    /// `--scope` is also explicit.
    #[arg(long)]
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
pub struct McpServeArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    /// The server reloads the lockfile fresh on every tool call so a
    /// concurrent `vibe install` run becomes visible without a
    /// restart.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    #[command(subcommand)]
    pub command: ShowSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ShowSubcommand {
    /// Print the effective spec — every spec/boot file plus every
    /// installed package's `files_written`, concatenated with
    /// `spec://` provenance headers in stable order.
    Effective(ShowEffectiveArgs),

    /// Print the effective configuration with per-value provenance
    /// (default / vibe.toml / env-var).
    Config(ShowConfigArgs),

    /// Print every active feature recorded in the lockfile, grouped
    /// by package. Per PROP-003 §2.10 / `vibe show features`.
    Features(ShowFeaturesArgs),

    /// Print every active subskill recorded in the lockfile, grouped
    /// by package, with delivery mode and any `describes` PURL.
    Subskills(ShowSubskillsArgs),

    /// Print every PURL the project's lockfile binds to (the union of
    /// per-package `describes` declarations). Useful as a sanity
    /// check for upstream-version drift.
    Purls(ShowPurlsArgs),
}

#[derive(Debug, clap::Args)]
pub struct ShowEffectiveArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowConfigArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowFeaturesArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowSubskillsArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ShowPurlsArgs {
    /// Project root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// WAL is "stale" past this age. Default 24h matches the boot
    /// snippet's freshness rule.
    #[arg(long = "wal-max-age-hours", default_value_t = 24)]
    pub wal_max_age_hours: u64,

    /// REVIEW marker age threshold in days (`<!-- REVIEW: YYYY-MM-DD ... -->`).
    /// Default 14d per `VIBEVM-SPEC.md` §12.
    #[arg(long = "review-max-age-days", default_value_t = 14)]
    pub review_max_age_days: u64,
}

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// Package references `<kind>:<name>` to update. Each must be
    /// currently installed. Mutually exclusive with `--all`.
    pub packages: Vec<String>,

    /// Update every package in the lockfile. Mutually exclusive with
    /// `<packages>`.
    #[arg(long, conflicts_with = "packages")]
    pub all: bool,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Package reference `<kind>:<name>` (version is ignored on uninstall).
    pub package: String,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,
}
