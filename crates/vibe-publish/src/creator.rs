//! The [`RepoCreator`] seam — host-specific repository operations for
//! the publish flow, plus the data types that cross it ([`RepoInfo`],
//! [`CreateOpts`]). One impl per supported git host; the orchestrator
//! ([`crate::Publisher`]) drives the trait and never sees a concrete
//! host. Layering per
//! [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#publish");

use crate::PublishError;

/// Information about a package repository on a host.
///
/// Returned by [`RepoCreator::create_repo`]; `clone_url` feeds the
/// `git remote add` + push flow, `html_url` is for the operator:
///
/// ```
/// use vibe_publish::RepoInfo;
///
/// let info = RepoInfo {
///     html_url: "https://github.com/vibespecs/org.vibevm_wal".to_string(),
///     clone_url: "https://github.com/vibespecs/org.vibevm_wal.git".to_string(),
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
///     host.repo_exists("someone-else", "org.vibevm_wal"),
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
