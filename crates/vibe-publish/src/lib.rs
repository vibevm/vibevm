//! `vibe registry publish <path>` — maintainer-side per-package publishing.
//!
//! Layered design per [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish):
//!
//! - [`RepoCreator`] — host-specific trait for "create a repo in this
//!   org, check whether one exists, produce the URL to push to". Two
//!   impls today: [`GitHubCreator`] (primary, drives the `vibespecs`
//!   org migration); [`GitVerseCreator`] (retained for any future
//!   Gitea-shape host that fully supports the org-scoped POST). New
//!   adapters land as one new `impl RepoCreator` per host.
//! - [`Publisher`] — host-agnostic orchestrator. Reads manifest,
//!   coordinates with [`RepoCreator`] for repo presence + creation,
//!   shells out to `git` for the working-tree → push → tag flow,
//!   classifies errors per the surface in PROP-002.
//! - [`Token`] — token loading per [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy).
//!   Per-host file precedence: `VIBEVM_PUBLISH_TOKEN` env →
//!   `~/.vibevm/<host-prefix>.publish.token` → legacy
//!   `~/.vibevm/git.publish.token`. Token never logged, never
//!   persisted, never leaks out of process.
//!
//! Consuming code (the CLI command) instantiates a `RepoCreator`,
//! constructs a `Publisher`, calls `Publisher::publish`, and renders
//! the [`PublishOutcome`] to the user. Tests use a mock `RepoCreator`
//! to drive every branch without hitting the network.

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#publish");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;
use vibe_core::PackageKind;
use vibe_core::manifest::{Manifest, NamingConvention};

pub mod direct_git;
pub mod git_publish;
pub mod github;
pub mod gitverse;
pub mod post_hook;
pub mod token;

pub use direct_git::DirectGitCreator;
pub use github::GitHubCreator;
pub use gitverse::GitVerseCreator;
pub use post_hook::{HookConfig, HookError, HookReport, fire as fire_index_hook};
pub use token::{Token, TokenSource, host_env_var, load_token, load_token_for_host};

/// Information about a package repository on a host.
///
/// Returned by [`RepoCreator::create_repo`]; `clone_url` feeds the
/// `git remote add` + push flow, `html_url` is for the operator:
///
/// ```
/// use vibe_publish::RepoInfo;
///
/// let info = RepoInfo {
///     html_url: "https://github.com/vibespecs/org.vibevm.wal".to_string(),
///     clone_url: "https://github.com/vibespecs/org.vibevm.wal.git".to_string(),
/// };
/// assert!(info.clone_url.ends_with(".git"));
/// ```
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub html_url: String,
    pub clone_url: String,
}

/// Options carried into [`RepoCreator::create_repo`].
///
/// Fill what the manifest provides, default the rest:
///
/// ```
/// use vibe_publish::CreateOpts;
///
/// let opts = CreateOpts {
///     description: Some("WAL discipline flow".to_string()),
///     default_branch: Some("main".to_string()),
///     ..CreateOpts::default()
/// };
/// assert!(opts.homepage.is_none());
/// ```
#[derive(Debug, Clone, Default)]
pub struct CreateOpts {
    pub description: Option<String>,
    /// Default branch name on the freshly-created repo. `None` lets the
    /// host pick its server-side default.
    pub default_branch: Option<String>,
    /// Optional homepage URL — propagated to the host so adopters can
    /// click through from the repo listing.
    pub homepage: Option<String>,
}

/// Host-specific operations for the publish flow. One impl per
/// supported git host. Today: [`GitHubCreator`] (primary) and
/// [`GitVerseCreator`] (legacy / retained). Adapter pattern matches
/// [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish)
/// — adding Gitea / Forgejo / GitLab is one new `impl RepoCreator`,
/// no consumer-side changes.
///
/// **Scope discipline** ([PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy)).
/// Each impl SHOULD constrain operations to a specific organization at
/// construction time via the `expected_org()` hook. The default
/// [`RepoCreator::validate_scope`] then refuses any call addressed to
/// a different org with [`PublishError::ScopeViolation`]. Adapters
/// that opt out (return `None` from `expected_org()`) trust their
/// caller for the boundary — useful for tests and mocks.
///
/// The canonical implementation shape — a host adapter scoped to one
/// org; the default [`validate_scope`](RepoCreator::validate_scope)
/// guard comes free:
///
/// ```
/// use vibe_publish::{CreateOpts, PublishError, RepoCreator, RepoInfo};
///
/// struct StaticHost;
///
/// impl RepoCreator for StaticHost {
///     fn host_name(&self) -> &str {
///         "example.test"
///     }
///     fn repo_exists(&self, org: &str, _name: &str) -> Result<bool, PublishError> {
///         self.validate_scope(org)?;
///         Ok(false)
///     }
///     fn create_repo(
///         &self,
///         org: &str,
///         name: &str,
///         _opts: &CreateOpts,
///     ) -> Result<RepoInfo, PublishError> {
///         self.validate_scope(org)?;
///         Ok(RepoInfo {
///             html_url: format!("https://example.test/{org}/{name}"),
///             clone_url: format!("https://example.test/{org}/{name}.git"),
///         })
///     }
///     fn push_url(&self, org: &str, name: &str) -> String {
///         format!("https://example.test/{org}/{name}.git")
///     }
///     fn expected_org(&self) -> Option<&str> {
///         Some("vibespecs")
///     }
/// }
///
/// let host = StaticHost;
/// assert!(host.validate_scope("vibespecs").is_ok());
/// assert!(matches!(
///     host.repo_exists("someone-else", "org.vibevm.wal"),
///     Err(PublishError::ScopeViolation { .. })
/// ));
/// ```
pub trait RepoCreator {
    /// Human-readable host name for error messages.
    fn host_name(&self) -> &str;

    /// Whether the org's repo with `name` already exists. Implementations
    /// should distinguish missing-token / missing-org / forbidden errors
    /// from a clean "no, it doesn't" answer.
    fn repo_exists(&self, org: &str, name: &str) -> Result<bool, PublishError>;

    /// Create the repository in the org. Returns the host's metadata
    /// (clone URL, HTML URL) for downstream `git remote add` + push.
    fn create_repo(
        &self,
        org: &str,
        name: &str,
        opts: &CreateOpts,
    ) -> Result<RepoInfo, PublishError>;

    /// URL to use for `git remote add origin` and `git push`. SSH-auth
    /// hosts return the bare SSH URL; HTTPS-token-auth hosts return the
    /// URL with credentials embedded for the duration of the push.
    /// Modern git ≥ 2.31 redacts URL passwords in its own log output
    /// to `***`, so the embedded form is safe in stderr; nonetheless
    /// the URL MUST never appear in any vibevm-produced output (CLI
    /// step lines, JSON events, error messages).
    fn push_url(&self, org: &str, name: &str) -> String;

    /// Org this adapter is scoped to. `Some(org)` enables the default
    /// [`validate_scope`](Self::validate_scope) refusal of any call
    /// addressed to a different org. `None` means the adapter trusts
    /// its caller (used by tests and mocks). Concrete hosting
    /// adapters SHOULD always return `Some` in production usage.
    fn expected_org(&self) -> Option<&str> {
        None
    }

    /// Refuse operations addressed to an org other than this adapter's
    /// configured scope. Default impl uses [`expected_org`](Self::expected_org).
    /// Concrete impls call this from `repo_exists` / `create_repo`
    /// before any side-effecting work.
    fn validate_scope(&self, org: &str) -> Result<(), PublishError> {
        if let Some(want) = self.expected_org()
            && org != want
        {
            return Err(PublishError::ScopeViolation {
                host: self.host_name().to_string(),
                expected_org: want.to_string(),
                attempted_org: org.to_string(),
            });
        }
        Ok(())
    }

    /// When set, signals "no host API in play — push the freshly-built
    /// commit + tag straight to this URL using the local user's git
    /// credentials". [`Publisher::publish`] short-circuits the whole
    /// org-extraction + repo_exists + create_repo dance when this
    /// returns `Some`. Default `None` means the regular host-adapter
    /// flow (token, API, scope-guard) applies. See [`crate::DirectGitCreator`].
    fn direct_repo_url(&self) -> Option<&str> {
        None
    }
}

/// Inputs to a single publish run.
///
/// [`with_defaults`](PublishConfig::with_defaults) is the blessed
/// constructor — `v` tag prefix, registry-default naming, real run.
/// Flip `dry_run` to plan without touching the host:
///
/// ```
/// use std::path::PathBuf;
/// use vibe_publish::PublishConfig;
///
/// let mut config = PublishConfig::with_defaults(
///     PathBuf::from("fixtures/registry/org.vibevm/wal/v0.1.0"),
///     "https://github.com/vibespecs".to_string(),
/// );
/// config.dry_run = true; // describe the plan, change nothing
/// assert_eq!(config.tag_prefix, "v");
/// ```
#[derive(Debug, Clone)]
pub struct PublishConfig {
    /// Filesystem directory containing `vibe.toml` and the rest
    /// of the package content. The publisher copies this into a fresh
    /// staging clone before pushing.
    pub source_dir: PathBuf,
    /// Org URL (the `[[registry]].url` from `vibe.toml`). The publisher
    /// extracts the org segment for the host API and combines it with
    /// the package repo name for the `git push` URL.
    pub org_url: String,
    /// Convention for mapping `<kind>:<name>` to a repo name under the
    /// org. Defaults to the registry's recorded `naming` field.
    pub naming: NamingConvention,
    /// Tag prefix — `v` is the convention. Surfaces in the tag name as
    /// `<prefix><semver>` (e.g. `v0.1.0`). Customisable for hosts /
    /// registries that pick a different prefix.
    pub tag_prefix: String,
    /// `true` → describe what would happen but make no changes (no API
    /// calls, no pushes, no tags).
    pub dry_run: bool,
}

impl PublishConfig {
    pub fn with_defaults(source_dir: PathBuf, org_url: String) -> Self {
        PublishConfig {
            source_dir,
            org_url,
            naming: NamingConvention::KindName,
            tag_prefix: "v".to_string(),
            dry_run: false,
        }
    }
}

/// Outcome of a successful publish — what was done, on what host, with
/// what URLs. CLI renders this for the operator.
///
/// ```
/// use vibe_core::PackageKind;
/// use vibe_publish::PublishOutcome;
///
/// let outcome = PublishOutcome {
///     kind: PackageKind::Flow,
///     name: "wal".to_string(),
///     version: semver::Version::parse("0.1.0").unwrap(),
///     repo_name: "org.vibevm.wal".to_string(),
///     repo_url: "https://github.com/vibespecs/org.vibevm.wal.git".to_string(),
///     tag: "v0.1.0".to_string(),
///     created_repo: true,
///     host: "github.com".to_string(),
///     dry_run: false,
/// };
/// assert_eq!(outcome.tag, format!("v{}", outcome.version));
/// ```
#[derive(Debug, Clone)]
pub struct PublishOutcome {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    pub repo_name: String,
    pub repo_url: String,
    pub tag: String,
    pub created_repo: bool,
    pub host: String,
    pub dry_run: bool,
}

/// The publish orchestrator.
///
/// Canonical flow — pick a [`RepoCreator`], wrap it, call
/// [`publish`](Publisher::publish), render the outcome. Shown here on
/// the direct-git path against a stand-in local URL (`no_run`: the
/// real call reads the package from disk and shells out to `git`):
///
/// ```no_run
/// use std::path::PathBuf;
/// use vibe_publish::{DirectGitCreator, PublishConfig, Publisher};
///
/// // Operator-provisioned repo: push with local git credentials.
/// let creator = DirectGitCreator::new("file:///tmp/registry/org.vibevm.wal.git");
/// let publisher = Publisher::new(&creator);
///
/// let mut config = PublishConfig::with_defaults(
///     PathBuf::from("fixtures/registry/org.vibevm/wal/v0.1.0"),
///     "file:///tmp/registry".to_string(),
/// );
/// config.dry_run = true; // plan only — no push, no tag
///
/// let outcome = publisher.publish(&config).unwrap();
/// println!("would publish {} as {}", outcome.name, outcome.tag);
/// ```
pub struct Publisher<'c, C: RepoCreator + ?Sized> {
    creator: &'c C,
}

impl<'c, C: RepoCreator + ?Sized> Publisher<'c, C> {
    pub fn new(creator: &'c C) -> Self {
        Publisher { creator }
    }

    /// Publish the package at `config.source_dir` under the org named in
    /// `config.org_url`.
    pub fn publish(&self, config: &PublishConfig) -> Result<PublishOutcome, PublishError> {
        // Step 1: read the package manifest. `publish` only operates on
        // a publishable `[package]` manifest; a `[project]`-only or
        // `[workspace]`-only `vibe.toml` is rejected as source-invalid.
        let manifest_path = config.source_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path).map_err(|e| PublishError::SourceInvalid {
            path: manifest_path.clone(),
            reason: format!("could not read or parse manifest: {e}"),
        })?;
        let meta = manifest
            .require_package()
            .map_err(|e| PublishError::SourceInvalid {
                path: manifest_path.clone(),
                reason: e.to_string(),
            })?;

        let kind = meta.kind;
        let group = meta.group.clone();
        let name = meta.name.clone();
        let version = meta.version.clone();
        let tag = format!("{}{}", config.tag_prefix, version);

        // Direct-git short-circuit: when the adapter declares a direct
        // repo URL, vibevm pushes straight to it using local-git creds.
        // No org extraction (the URL is repo-level, not org-level), no
        // repo_exists probe, no create_repo, no token. Repo presence is
        // the operator's responsibility — `git push` errors out cleanly
        // if the URL is wrong, and `git_publish::push_with_classification`
        // surfaces a structured `PublishError` with the URL inline
        // (credentials redacted per PROP-000 §20).
        if let Some(direct_url) = self.creator.direct_repo_url() {
            // No naming convention probe — the operator supplied the
            // URL, so the host's actual repo path is whatever they
            // chose. For the human-facing `PublishOutcome.repo_name`
            // we fall back to the configured naming convention's form
            // (matches what the operator typically picked when they
            // provisioned the repo); the truth-of-the-matter is the
            // URL itself, surfaced in `repo_url`.
            let repo_name = config
                .naming
                .repo_name(Some(kind), &group, &name)
                .map_err(|e| PublishError::SourceInvalid {
                    path: manifest_path.clone(),
                    reason: format!("could not derive a repo name: {e}"),
                })?;
            if !config.dry_run {
                git_publish::push_release(&config.source_dir, direct_url, &tag, &name, &version)?;
            }
            return Ok(PublishOutcome {
                kind,
                name,
                version,
                repo_name,
                repo_url: direct_url.to_string(),
                tag,
                created_repo: false,
                host: self.creator.host_name().to_string(),
                dry_run: config.dry_run,
            });
        }

        let repo_name = config
            .naming
            .repo_name(Some(kind), &group, &name)
            .map_err(|e| PublishError::SourceInvalid {
                path: manifest_path.clone(),
                reason: format!("could not derive a repo name: {e}"),
            })?;

        // Step 2: derive org segment from the configured org URL.
        let org_segment = extract_org_segment(&config.org_url)?;

        // Step 3: figure out repo presence.
        let exists = self.creator.repo_exists(&org_segment, &repo_name)?;
        let mut created_repo = false;

        let repo_info = if exists {
            tracing::info!(target: "vibe_publish", org = %org_segment, repo = %repo_name, "repo already exists; reusing");
            RepoInfo {
                html_url: format!("{}/{}", config.org_url.trim_end_matches('/'), repo_name),
                clone_url: format!("{}/{}.git", config.org_url.trim_end_matches('/'), repo_name),
            }
        } else if config.dry_run {
            // Synthesise a plausible RepoInfo for the rendered plan.
            // `created_repo = true` here advertises *what would happen* —
            // the repo does not exist, so a non-dry-run would create it.
            // The CLI renders this as "Would create repository …" which
            // is the correct user expectation for the dry-run case.
            tracing::info!(target: "vibe_publish", "dry-run: would create repo");
            created_repo = true;
            RepoInfo {
                html_url: format!("{}/{}", config.org_url.trim_end_matches('/'), repo_name),
                clone_url: format!("{}/{}.git", config.org_url.trim_end_matches('/'), repo_name),
            }
        } else {
            let opts = CreateOpts {
                description: meta.description.clone(),
                default_branch: Some("main".to_string()),
                homepage: meta.homepage.clone(),
            };
            let info = self.creator.create_repo(&org_segment, &repo_name, &opts)?;
            created_repo = true;
            info
        };

        // Step 4: push contents and tag (skipped on dry-run).
        // Push URL is constructed by the host adapter — SSH-auth hosts
        // return the SSH form; HTTPS-token-auth hosts inject credentials
        // for this single push only. Token never appears in stdout /
        // stderr / log lines; modern git redacts URL passwords in its
        // own diagnostics. The push URL MUST NOT appear in any
        // vibevm-produced output (the user-facing PublishOutcome.repo_url
        // carries the public clone URL for display).
        if !config.dry_run {
            let push_url = self.creator.push_url(&org_segment, &repo_name);
            git_publish::push_release(&config.source_dir, &push_url, &tag, &name, &version)?;
        }

        Ok(PublishOutcome {
            kind,
            name,
            version,
            repo_name,
            repo_url: repo_info.clone_url,
            tag,
            created_repo,
            host: self.creator.host_name().to_string(),
            dry_run: config.dry_run,
        })
    }
}

/// Pull the org segment out of an org URL.
///
/// - `git@gitverse.ru:vibespecs` → `vibespecs`
/// - `git@gitverse.ru:vibespecs/` → `vibespecs`
/// - `https://gitverse.ru/vibespecs` → `vibespecs`
/// - `https://github.com/vibespecs` → `vibespecs`
/// - `ssh://git@gitverse.ru/vibespecs` → `vibespecs`
/// - `git+https://...` → strips the `git+` first
///
/// ```
/// use vibe_publish::extract_org_segment;
///
/// assert_eq!(
///     extract_org_segment("https://github.com/vibespecs").unwrap(),
///     "vibespecs",
/// );
/// assert_eq!(
///     extract_org_segment("git@gitverse.ru:vibespecs").unwrap(),
///     "vibespecs",
/// );
/// ```
pub fn extract_org_segment(org_url: &str) -> Result<String, PublishError> {
    let url = org_url.trim().trim_end_matches('/');
    let url = url.strip_prefix("git+").unwrap_or(url);
    // ssh shorthand `user@host:path`
    if let Some((_, rest)) = url.split_once(':')
        && !url.contains("://")
    {
        return Ok(rest.trim_end_matches('/').to_string());
    }
    if let Some((_, rest)) = url.split_once("://") {
        // schemes://host/<path...>
        if let Some(slash) = rest.find('/') {
            return Ok(rest[slash + 1..].trim_end_matches('/').to_string());
        }
    }
    Err(PublishError::OrgUrlInvalid {
        url: org_url.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Errors — surface tuned for non-admin contributors per PROP-002 §2.10.
// ---------------------------------------------------------------------------

/// Publish failure surface, tuned for non-admin contributors per
/// [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish)
/// — every refusal names the violated expectation and the fix surface:
///
/// ```
/// use vibe_publish::PublishError;
///
/// let err = PublishError::TagCollision {
///     repo: "vibespecs/org.vibevm.wal".to_string(),
///     tag: "v0.1.0".to_string(),
/// };
/// let rendered = err.to_string();
/// assert!(rendered.contains("tag `v0.1.0` already exists"));
/// assert!(rendered.contains("does not force-push tags"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#publish")]
pub enum PublishError {
    #[error(
        "publish refused: source directory `{path}` does not look like a vibevm package — \
         {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: point the command at a directory whose `vibe.toml` carries a `[package]` section)"
    )]
    SourceInvalid { path: PathBuf, reason: String },

    #[error(
        "publish refused: cannot derive an organization segment from `{url}`. \
         Configure `[[registry]].url` to a value `git` accepts (e.g. `git@gitverse.ru:vibespecs`). \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: include the organization segment after the host in `[[registry]].url`)"
    )]
    OrgUrlInvalid { url: String },

    #[error(
        "publish refused: token lacks `repo:create` permission in organization `{org}` on `{host}`. \
         Contact an org owner, or use a token whose scope includes repository creation. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: mint a token with `repo:create` scope for `{org}` or have an org owner elevate it)"
    )]
    AuthForbidden { host: String, org: String },

    #[error(
        "publish refused: no token available for host `{host}`. \
         Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibevm/git.publish.token`. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: export `VIBEVM_PUBLISH_TOKEN` or write `~/.vibevm/<host-prefix>.publish.token`)"
    )]
    AuthMissing { host: String },

    #[error(
        "publish refused: organization `{org}` does not exist on `{host}` \
         (or the token cannot see it). Check spelling — different from \
         a permissions error. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: correct the org segment in `[[registry]].url` or use a token that can see `{org}`)"
    )]
    OrgNotFound { host: String, org: String },

    #[error(
        "publish refused: tag `{tag}` already exists on `{repo}`. \
         Pick a new version — `vibe registry publish` does not force-push tags. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: bump `[package].version` in `vibe.toml` and publish again)"
    )]
    TagCollision { repo: String, tag: String },

    #[error(
        "publish refused: no push access to `{repo}`. Ask a maintainer of \
         that repo to grant you push, or use a different registry. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: request push access on `{repo}` or point `[[registry]].url` at a registry \
         you can write to)"
    )]
    PushDenied { repo: String },

    #[error(
        "publish refused: host `{host}` is unreachable (network or DNS error). \
         Check connectivity and try again. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: restore network/DNS reachability of `{host}`, then re-run the publish)"
    )]
    HostUnreachable { host: String },

    #[error(
        "git operation failed during publish: {0} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: resolve the reported git failure, then re-run `vibe registry publish`)"
    )]
    Git(String),

    #[error(
        "HTTP request to `{host}` failed: {message} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: check connectivity to `{host}` and retry)"
    )]
    HttpFailed { host: String, message: String },

    #[error(
        "unexpected response from `{host}` (status {status}): {body} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: retry the publish — if the status persists, the host API shape changed and \
         the adapter needs updating)"
    )]
    UnexpectedResponse {
        host: String,
        status: u16,
        body: String,
    },

    #[error(
        "filesystem error during publish at `{path}`: {message} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: check permissions and free space at `{path}`, then re-run)"
    )]
    Io { path: PathBuf, message: String },

    #[error(
        "publish refused: scope violation — adapter for `{host}` is scoped to organization \
         `{expected_org}` but the request targeted `{attempted_org}`. The publish utility never \
         operates outside the organization named in `[[registry]].url`. \
         See spec://vibevm/common/PROP-000#token-secrecy."
    )]
    ScopeViolation {
        host: String,
        expected_org: String,
        attempted_org: String,
    },

    #[error(
        "publish refused: no `RepoCreator` adapter for host `{host}`. Configured registry URL \
         points at an unsupported host; add an adapter per PROP-002 §2.10 or use a supported one \
         (today: `github.com`, `gitverse.ru`). \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#publish; \
         fix: point `[[registry]].url` at `github.com` or `gitverse.ru`, or add a \
         `RepoCreator` impl for `{host}`)"
    )]
    UnsupportedHost { host: String },
}

/// Pull the host segment out of an org URL.
///
/// - `git@github.com:vibespecs` → `github.com`
/// - `https://github.com/vibespecs` → `github.com`
/// - `ssh://git@github.com/vibespecs` → `github.com`
/// - `git+https://github.com/vibespecs` → `github.com` (strips `git+` first)
///
/// ```
/// use vibe_publish::extract_host_segment;
///
/// assert_eq!(
///     extract_host_segment("git@github.com:vibespecs").unwrap(),
///     "github.com",
/// );
/// assert_eq!(
///     extract_host_segment("https://gitverse.ru/vibespecs").unwrap(),
///     "gitverse.ru",
/// );
/// ```
pub fn extract_host_segment(org_url: &str) -> Result<String, PublishError> {
    let url = org_url.trim().trim_end_matches('/');
    let url = url.strip_prefix("git+").unwrap_or(url);
    if let Some((before_colon, _)) = url.split_once(':')
        && !url.contains("://")
    {
        // ssh shorthand `user@host:path`
        if let Some((_, host)) = before_colon.split_once('@') {
            if !host.is_empty() {
                return Ok(host.to_string());
            }
        } else if !before_colon.is_empty() {
            return Ok(before_colon.to_string());
        }
    }
    if let Some((_, rest)) = url.split_once("://") {
        let after_at = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
        if let Some((host, _)) = after_at.split_once('/') {
            if !host.is_empty() {
                return Ok(host.to_string());
            }
        } else if !after_at.is_empty() {
            return Ok(after_at.to_string());
        }
    }
    Err(PublishError::OrgUrlInvalid {
        url: org_url.to_string(),
    })
}

/// Construct the right [`RepoCreator`] for a given registry URL.
/// Selects the adapter from the URL's host segment per PROP-002 §2.10.
/// Tokens are loaded by the caller and passed in — this function does
/// not touch token storage.
///
/// `expected_org` is the organization segment the adapter will be
/// scoped to (extracted from the same registry URL by the caller via
/// [`extract_org_segment`]). Adapters refuse operations against any
/// other org per [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy).
///
/// ```
/// use vibe_publish::{RepoCreator, Token, creator_for_url, extract_org_segment};
///
/// let org_url = "https://github.com/vibespecs";
/// let org = extract_org_segment(org_url).unwrap();
/// let token = Token::from_explicit("test-token-please-redact");
/// let creator = creator_for_url(org_url, org, token).unwrap();
/// assert_eq!(creator.host_name(), "github.com");
/// assert_eq!(creator.expected_org(), Some("vibespecs"));
/// ```
pub fn creator_for_url(
    org_url: &str,
    expected_org: impl Into<String>,
    token: Token,
) -> Result<Box<dyn RepoCreator>, PublishError> {
    let host = extract_host_segment(org_url)?;
    let expected_org = expected_org.into();
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "github.com" || host_lower.ends_with(".github.com") {
        let creator = GitHubCreator::new(token, expected_org)?;
        return Ok(Box::new(creator));
    }
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        let creator = GitVerseCreator::new(token, expected_org)?;
        return Ok(Box::new(creator));
    }
    Err(PublishError::UnsupportedHost { host })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_org_segment_ssh_shorthand() {
        assert_eq!(
            extract_org_segment("git@gitverse.ru:vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git@gitverse.ru:vibespecs/").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_https() {
        assert_eq!(
            extract_org_segment("https://gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("https://gitverse.ru/vibespecs/").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_ssh_scheme() {
        assert_eq!(
            extract_org_segment("ssh://git@gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_strips_git_plus() {
        assert_eq!(
            extract_org_segment("git+https://gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git+ssh://git@gitverse.ru/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_org_segment_rejects_bare_host() {
        assert!(extract_org_segment("git@gitverse.ru").is_err());
        assert!(extract_org_segment("https://gitverse.ru").is_err());
    }

    #[test]
    fn extract_org_segment_github_shapes() {
        assert_eq!(
            extract_org_segment("https://github.com/vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("git@github.com:vibespecs").unwrap(),
            "vibespecs"
        );
        assert_eq!(
            extract_org_segment("ssh://git@github.com/vibespecs").unwrap(),
            "vibespecs"
        );
    }

    #[test]
    fn extract_host_segment_ssh_shorthand() {
        assert_eq!(
            extract_host_segment("git@github.com:vibespecs").unwrap(),
            "github.com"
        );
        assert_eq!(
            extract_host_segment("git@gitverse.ru:vibespecs").unwrap(),
            "gitverse.ru"
        );
    }

    #[test]
    fn extract_host_segment_https() {
        assert_eq!(
            extract_host_segment("https://github.com/vibespecs").unwrap(),
            "github.com"
        );
        assert_eq!(
            extract_host_segment("https://gitverse.ru/vibespecs").unwrap(),
            "gitverse.ru"
        );
    }

    #[test]
    fn extract_host_segment_ssh_scheme() {
        assert_eq!(
            extract_host_segment("ssh://git@github.com/vibespecs").unwrap(),
            "github.com"
        );
    }

    #[test]
    fn extract_host_segment_strips_git_plus() {
        assert_eq!(
            extract_host_segment("git+https://github.com/vibespecs").unwrap(),
            "github.com"
        );
    }

    #[test]
    fn creator_for_url_picks_github() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = creator_for_url("https://github.com/vibespecs", "vibespecs", token).unwrap();
        assert_eq!(creator.host_name(), "github.com");
        assert_eq!(creator.expected_org(), Some("vibespecs"));
    }

    #[test]
    fn creator_for_url_picks_gitverse() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = creator_for_url("git@gitverse.ru:vibespecs", "vibespecs", token).unwrap();
        assert_eq!(creator.host_name(), "gitverse.ru");
        assert_eq!(creator.expected_org(), Some("vibespecs"));
    }

    #[test]
    fn creator_for_url_rejects_unknown_host() {
        let token = Token::from_explicit("test-token-please-redact");
        match creator_for_url("https://example.invalid/whatever", "whatever", token) {
            Ok(_) => panic!("expected unsupported-host error"),
            Err(PublishError::UnsupportedHost { host }) => assert_eq!(host, "example.invalid"),
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn validate_scope_refuses_other_org() {
        let token = Token::from_explicit("test-token-please-redact");
        let creator = GitHubCreator::new(token, "vibespecs").unwrap();
        let err = creator
            .validate_scope("not-vibespecs")
            .expect_err("scope guard should fire");
        match err {
            PublishError::ScopeViolation {
                expected_org,
                attempted_org,
                ..
            } => {
                assert_eq!(expected_org, "vibespecs");
                assert_eq!(attempted_org, "not-vibespecs");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
