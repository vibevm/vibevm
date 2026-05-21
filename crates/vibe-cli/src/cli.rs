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

    /// Print version information.
    Version,
}

#[derive(Debug, clap::Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceSubcommand {
    /// Publish the workspace's self-publishing members. Discovers the
    /// workspace enclosing the current directory, selects every node
    /// (root and members) carrying `[package]` whose `publish` posture
    /// does not exclude it, orders them dependency-first via inter-member
    /// `path` dependencies, and publishes each as its own repository in
    /// the workspace's primary `[[registry]]` org — reusing the same
    /// per-package machinery as `vibe registry publish`. Each published
    /// copy carries an `[origin]` provenance marker, a "generated copy"
    /// README banner, and a `.github/PULL_REQUEST_TEMPLATE.md` STOP
    /// notice. Publishing is **not atomic**: on the first failure the
    /// command stops and reports which nodes were already published and
    /// which remain (PROP-007 §2.7). Maintainers only — needs the same
    /// publish token used by `vibe registry publish`.
    Publish(WorkspacePublishArgs),
}

#[derive(Debug, clap::Args)]
pub struct WorkspacePublishArgs {
    /// Restrict the publish to a single workspace node by its
    /// root-relative path (`.` selects the workspace root, e.g.
    /// `packages/flow-wal` selects that member). When omitted, every
    /// self-publishing node is published. A node whose `publish`
    /// posture excludes it is reported as skipped even when named
    /// explicitly here.
    #[arg(long = "member")]
    pub member: Option<String>,

    /// Project directory to discover the workspace from. Discovery
    /// walks up to the enclosing `[workspace]`. Defaults to the
    /// current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would be published — selection, dependency order,
    /// staged content — but make no API calls and push nothing. No
    /// repository is created.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
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

    /// Probe each configured `[[registry]]` for reachability +
    /// authentication status. Read-only diagnostic — does not
    /// fetch or write anything. Per-registry status: `reachable`
    /// (org URL responded), `auth-required` (got 401 / 403 — for
    /// public registries this is "host policy on missing repos
    /// is 401"; for authenticated registries this means the
    /// configured credentials are missing or wrong),
    /// `unreachable` (network / DNS / cert error), or
    /// `missing-token` (registry declares `auth = "token-env"`
    /// but the env-var resolves empty). Useful when first wiring
    /// a private registry to confirm credentials line up.
    Test(RegistryTestArgs),

    /// Create a registry stub that delegates a package to an external
    /// target URL (PROP-002 §2.4.2). Makes the configured `[[registry]]`
    /// org host a stub repo carrying `vibe-redirect.toml` instead of the
    /// package content. Consumers `vibe install <pkgref>` resolve through
    /// the stub transparently; the resolver follows the marker to the
    /// target. Maintainers only — needs the same publish token used by
    /// `vibe registry publish`.
    Redirect(RegistryRedirectArgs),

    /// Mirror target tags into a registry stub (PROP-002 §2.4.2,
    /// `pass-through-tag` policy). Reads the stub's `vibe-redirect.toml`,
    /// enumerates target tags, and pushes the missing ones into the stub
    /// so consumers `vibe install <pkgref>@<ver>` see the same versions
    /// the target offers. Pinned-policy stubs have nothing to sync —
    /// command refuses with a clear message.
    RedirectSync(RegistryRedirectSyncArgs),

    /// Rewrite an existing registry stub's `vibe-redirect.toml` (PROP-002
    /// §2.4.2). Each flag is optional — fields not specified retain their
    /// current value, so this is a true partial update. Changes that
    /// affect resolution outcomes for consumers (`--to` rewriting the
    /// target URL, `--ref-policy` flipping the resolution mode) require
    /// `--trust-redirect` per PROP-002 §2.4.2's trust model: such a switch
    /// is never silent and must be operator-initiated. Refuses if the
    /// computed marker is byte-identical to the stub's current marker.
    RedirectUpdate(RegistryRedirectUpdateArgs),

    /// Generate a local mirror directory containing every package
    /// referenced by `vibe.lock`, suitable for use as a
    /// `[[mirror]] url = "file:///<abs-path>"` for offline / air-gapped
    /// installs.
    Vendor(RegistryVendorArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistryRedirectArgs {
    /// Pkgref (`<kind>:<name>`) to delegate. The version part of the
    /// pkgref is ignored — stubs live on `(kind, name)` and any version
    /// gating is done via stub tags later.
    pub pkgref: String,

    /// Target git URL where the package's actual content lives. Any
    /// git URL `git` accepts (`git@host:org/repo`, `ssh://...`,
    /// `https://...`).
    #[arg(long = "to")]
    pub to: String,

    /// Name of the `[[registry]]` whose org will host the stub. Defaults
    /// to the first registry in `vibe.toml`.
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// Ref policy for the stub. `pass-through-tag` (default): consumer's
    /// resolved stub tag passes through to the target. `pinned`: every
    /// consumer resolves to `--pinned-ref` regardless of stub tag.
    #[arg(long = "ref-policy", default_value = "pass-through-tag")]
    pub ref_policy: String,

    /// Required when `--ref-policy pinned`. Tag, branch, or commit on
    /// the target URL that every consumer pins to.
    #[arg(long = "pinned-ref")]
    pub pinned_ref: Option<String>,

    /// Target-side authentication regime for the redirect. Mirrors the
    /// registry-level auth axis (PROP-002 §2.2.1): `none` (default),
    /// `token-env`, `credential-helper`, `ssh`. Stored in the stub's
    /// `[redirect].auth`.
    #[arg(long = "target-auth")]
    pub target_auth: Option<String>,

    /// Override the env-var name used by `--target-auth token-env`.
    /// Default is derived from the target URL's host.
    #[arg(long = "target-token-env")]
    pub target_token_env: Option<String>,

    /// Free-form text recorded in `[redirect].description` and surfaced
    /// to operators via `vibe show <pkgref>`. Use this to publish
    /// out-of-band contact info for the delegate.
    #[arg(long = "description")]
    pub description: Option<String>,

    /// Mirror current target tags into the freshly-created stub
    /// immediately after creation. Equivalent to running
    /// `vibe registry redirect-sync <pkgref>` once the stub exists.
    /// No-op for `--ref-policy pinned`.
    #[arg(long = "sync")]
    pub sync: bool,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would happen but make no API calls or pushes.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRedirectSyncArgs {
    /// Pkgref (`<kind>:<name>`) of an existing stub to sync.
    pub pkgref: String,

    /// Name of the `[[registry]]` hosting the stub. Defaults to the
    /// first registry in `vibe.toml`.
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would happen but make no API calls or pushes.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRedirectUpdateArgs {
    /// Pkgref (`<kind>:<name>`) of an existing stub to rewrite.
    pub pkgref: String,

    /// New target git URL. Omit to keep the current `target_url`.
    /// Changing the target URL requires `--trust-redirect` because it
    /// switches the content consumers receive — see PROP-002 §2.4.2 on
    /// the trust model.
    #[arg(long = "to")]
    pub to: Option<String>,

    /// Name of the `[[registry]]` hosting the stub. Defaults to the
    /// first registry in `vibe.toml`.
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// New ref policy. `pass-through-tag` or `pinned`. Omit to keep the
    /// current policy. Flipping policy requires `--trust-redirect`.
    #[arg(long = "ref-policy")]
    pub ref_policy: Option<String>,

    /// New pinned ref. Required when switching to `--ref-policy pinned`;
    /// allowed when keeping `pinned` policy (changes the pinned target).
    /// Rejected when current or new policy is `pass-through-tag`.
    #[arg(long = "pinned-ref")]
    pub pinned_ref: Option<String>,

    /// New target-side auth regime. Mirrors the registry-level axis from
    /// PROP-002 §2.2.1 — `none`, `token-env`, `credential-helper`, `ssh`.
    /// Omit to keep the current auth regime.
    #[arg(long = "target-auth")]
    pub target_auth: Option<String>,

    /// Override the env-var name used by `--target-auth token-env`.
    /// Cleared automatically when the new auth regime is not `token-env`.
    #[arg(long = "target-token-env")]
    pub target_token_env: Option<String>,

    /// New description text recorded in `[redirect].description`. Omit
    /// to keep the current description; pass `--clear-description` to
    /// drop it entirely.
    #[arg(long = "description")]
    pub description: Option<String>,

    /// Drop the existing `[redirect].description` field. Mutually
    /// exclusive with `--description`.
    #[arg(long = "clear-description")]
    pub clear_description: bool,

    /// Confirm a deliberate switch of `target_url` or `ref_policy`. Per
    /// PROP-002 §2.4.2, such a switch changes the content consumers
    /// receive — this flag is the operator's explicit acknowledgement.
    /// Without it, requested target/policy changes are rejected with a
    /// pointer at this flag.
    #[arg(long = "trust-redirect")]
    pub trust_redirect: bool,

    /// After pushing the rewritten marker, run `vibe registry
    /// redirect-sync <pkgref>` to mirror target tags into the stub. Most
    /// useful when `--to` migrates the stub to a different target whose
    /// tag set differs from the prior target's. No-op for pinned-policy
    /// stubs after update.
    #[arg(long = "resync")]
    pub resync: bool,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would happen but make no API calls or pushes.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
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
pub struct RegistryTestArgs {
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

    /// Authentication regime for fetching from this registry. One of
    /// `none` (default; public read), `token-env` (read token from
    /// `VIBEVM_REGISTRY_TOKEN_<HOST>` or the explicit `--token-env`
    /// override), `credential-helper` (opt in to system git credential
    /// helpers; GUI prompts allowed), `ssh` (URL must be ssh-form,
    /// auth via ssh-agent / keys). See PROP-002 §2.2.1.
    #[arg(long = "auth")]
    pub auth: Option<String>,

    /// Override the env-var name used by `auth = token-env`. Default
    /// is derived from the registry's host.
    #[arg(long = "token-env")]
    pub token_env: Option<String>,

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
    /// Path to the package directory (containing `vibe.toml`).
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
