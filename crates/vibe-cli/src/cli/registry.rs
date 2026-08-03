//! Argument structs for `vibe registry …` (PROP-002).
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::Subcommand;

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

    /// Naming convention mapping a pkgref to a repo name under the org.
    /// One of `fqdn` (default), `kind-name`, `name`, `kind/name`.
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
