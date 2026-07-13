//! Argument structs for the package-lifecycle commands — `vibe init`,
//! `list`, `install`, `outdated`, `search`, `update`, `reinstall`,
//! `uninstall`.
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

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
    /// Filter by package kind (flow, feat, stack, tool, mcp).
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
    /// Zero or more package references, each `<kind>:<name>[@<version>]`.
    /// When empty, `vibe install` reads the project's `vibe.toml`
    /// `[requires].packages` and installs every entry — same shape as
    /// `cargo build` / `npm install` against an existing manifest.
    /// When non-empty, each pkgref is added to (or updates the
    /// constraint on) `vibe.toml` `[requires].packages`.
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

    /// Run every installed package's declared install hooks (PROP-020 §2.3)
    /// without an interactive consent prompt, even for groups that are not
    /// allow-listed. This is the non-interactive opt-in: a hook-declaring
    /// package whose group is not trusted otherwise aborts the install
    /// (re-run interactively to consent, allow-list the group, or pass this
    /// flag). `org.vibevm` is always allow-listed and runs regardless.
    #[arg(long)]
    pub allow_hooks: bool,
}

#[derive(Debug, clap::Args)]
pub struct OutdatedArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Strict authentication gate — same semantics as
    /// `vibe install --auth-required` / `vibe update --auth-required`.
    /// When set, a 401 / 403 from an `auth = "none"` (public)
    /// registry halts the probe instead of walking past. The probe
    /// is read-only, so the trade-off is mostly diagnostic clarity
    /// — you want to see "this private registry is down" rather
    /// than silently miss its packages.
    #[arg(long)]
    pub auth_required: bool,
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
    /// rate-limited; reads `vibe.toml` from each repo's HEAD
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

    /// Strict authentication gate — same semantics as
    /// `vibe install --auth-required`. When set, a 401 / 403
    /// against an `auth = "none"` (public) registry halts the
    /// update instead of walking past. Useful in CI / cron where
    /// a fallback to a public substitute would mask a private-
    /// registry outage.
    #[arg(long)]
    pub auth_required: bool,
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
