//! The [`RepoCreator`] seam — host-specific repository operations for
//! the publish flow, plus the data types that cross it ([`RepoInfo`],
//! [`CreateOpts`]). One impl per supported git host; the orchestrator
//! ([`crate::Publisher`]) drives the trait and never sees a concrete
//! host. Layering per
//! [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#publish");

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

/// An org name that has passed a creator's scope check.
///
/// The only way to obtain one is [`RepoCreator::validate_scope`], so a
/// method that takes `&ValidatedOrg` cannot be reached without the check
/// having run. This is stronger than the prose obligation it replaces:
/// "concrete impls call `validate_scope` before side-effecting work" is a
/// comment the next implementer is free to forget — and forgetting it is
/// exactly the latent hole this type closes. Making the scope check a
/// *type* turns the "never escalate scope" rule
/// ([PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish);
/// scope discipline per [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy))
/// into a compile-time gate: [`repo_exists`](RepoCreator::repo_exists),
/// [`create_repo`](RepoCreator::create_repo), and
/// [`push_url`](RepoCreator::push_url) each demand a `&ValidatedOrg`, so a
/// caller outside this crate cannot reach the host's side-effecting work
/// without a prior scope check. The private field plus the crate-local
/// [`ValidatedOrg::new`] are the whole mechanism: nothing outside
/// `vibe-publish` can mint one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedOrg(String);

impl ValidatedOrg {
    /// The org name, post scope check.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Mint a validated org. Crate-private on purpose: only this crate's
    /// own `validate_scope` impls (the default in this file, plus the
    /// direct-push override that has no org to scope against) may
    /// construct one. Every consumer outside the crate — the orchestrator,
    /// the CLI — must obtain a `ValidatedOrg` by calling
    /// [`RepoCreator::validate_scope`], which is the gate the type exists
    /// to enforce.
    pub(crate) fn new(org: impl Into<String>) -> Self {
        ValidatedOrg(org.into())
    }
}

/// Host-specific operations for the publish flow. One impl per
/// supported git host. Today: [`GithubRepoCreator`] (primary) and
/// [`GitverseRepoCreator`] (legacy / retained). Adapter pattern matches
/// [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish)
/// — adding Gitea / Forgejo / GitLab is one new `impl RepoCreator`,
/// no consumer-side changes.
///
/// **Scope discipline** ([PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy)).
/// Each impl SHOULD constrain operations to a specific organization at
/// construction time via the `expected_org()` hook. The default
/// [`RepoCreator::validate_scope`] then refuses any call addressed to
/// a different org with [`PublishError::ScopeViolation`] and hands back
/// a [`ValidatedOrg`] token that [`repo_exists`](RepoCreator::repo_exists),
/// [`create_repo`](RepoCreator::create_repo), and
/// [`push_url`](RepoCreator::push_url) require — so the check is a type,
/// not a request (see [`ValidatedOrg`]). Adapters that opt out (return
/// `None` from `expected_org()`) trust their caller for the boundary —
/// useful for tests and mocks.
///
/// The canonical implementation shape — a host adapter scoped to one
/// org; the default [`validate_scope`](RepoCreator::validate_scope)
/// guard comes free:
///
/// ```
/// use vibe_publish::{CreateOpts, PublishError, RepoCreator, RepoInfo, ValidatedOrg};
///
/// struct StaticHost;
///
/// impl RepoCreator for StaticHost {
///     fn host_name(&self) -> &str {
///         "example.test"
///     }
///     // The scope check has already run by the time we get here — the
///     // caller holds a `ValidatedOrg` — so no `validate_scope` call
///     // inside the body.
///     fn repo_exists(&self, _org: &ValidatedOrg, _name: &str) -> Result<bool, PublishError> {
///         Ok(false)
///     }
///     fn create_repo(
///         &self,
///         org: &ValidatedOrg,
///         name: &str,
///         _opts: &CreateOpts,
///     ) -> Result<RepoInfo, PublishError> {
///         Ok(RepoInfo {
///             html_url: format!("https://example.test/{}/{name}", org.as_str()),
///             clone_url: format!("https://example.test/{}/{name}.git", org.as_str()),
///         })
///     }
///     fn push_url(&self, org: &ValidatedOrg, name: &str) -> String {
///         format!("https://example.test/{}/{name}.git", org.as_str())
///     }
///     fn expected_org(&self) -> Option<&str> {
///         Some("vibespecs")
///     }
/// }
///
/// let host = StaticHost;
/// // `validate_scope` is the only door into the host methods. It hands
/// // back the typed token those methods require…
/// let org = host.validate_scope("vibespecs").expect("own org passes");
/// assert_eq!(org.as_str(), "vibespecs");
/// assert!(!host.repo_exists(&org, "org.vibevm_wal").unwrap());
/// assert!(host
///     .push_url(&org, "org.vibevm_wal")
///     .ends_with("/vibespecs/org.vibevm_wal.git"));
/// // …and a foreign org is refused at that door, before any host method
/// // can be reached — so the refusal is a property of the type, not a
/// // call an impl can forget.
/// assert!(matches!(
///     host.validate_scope("someone-else"),
///     Err(PublishError::ScopeViolation { .. })
/// ));
/// ```
pub trait RepoCreator {
    /// Human-readable host name for error messages.
    fn host_name(&self) -> &str;

    /// Whether the org's repo with `name` already exists. Implementations
    /// should distinguish missing-token / missing-org / forbidden errors
    /// from a clean "no, it doesn't" answer.
    fn repo_exists(&self, org: &ValidatedOrg, name: &str) -> Result<bool, PublishError>;

    /// Create the repository in the org. Returns the host's metadata
    /// (clone URL, HTML URL) for downstream `git remote add` + push.
    fn create_repo(
        &self,
        org: &ValidatedOrg,
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
    fn push_url(&self, org: &ValidatedOrg, name: &str) -> String;

    /// Org this adapter is scoped to. `Some(org)` enables the default
    /// [`validate_scope`](Self::validate_scope) refusal of any call
    /// addressed to a different org. `None` means the adapter trusts
    /// its caller (used by tests and mocks). Concrete hosting
    /// adapters SHOULD always return `Some` in production usage.
    fn expected_org(&self) -> Option<&str> {
        None
    }

    /// Refuse operations addressed to an org other than this adapter's
    /// configured scope, returning a [`ValidatedOrg`] token that
    /// [`repo_exists`](Self::repo_exists), [`create_repo`](Self::create_repo),
    /// and [`push_url`](Self::push_url) require. Default impl uses
    /// [`expected_org`](Self::expected_org). Because every side-effecting
    /// host method takes `&ValidatedOrg` and this is the only way to mint
    /// one, the check is unavoidable for any code outside this crate —
    /// the "never escalate scope" rule of PROP-002 §2.10 enforced as a
    /// type, not a request. Concrete impls no longer call this from their
    /// own method bodies; the *caller* mints once and passes the token on.
    fn validate_scope(&self, org: &str) -> Result<ValidatedOrg, PublishError> {
        if let Some(want) = self.expected_org()
            && org != want
        {
            return Err(PublishError::ScopeViolation {
                host: self.host_name().to_string(),
                expected_org: want.to_string(),
                attempted_org: org.to_string(),
            });
        }
        Ok(ValidatedOrg::new(org))
    }

    /// When set, signals "no host API in play — push the freshly-built
    /// commit + tag straight to this URL using the local user's git
    /// credentials". [`Publisher::publish`] short-circuits the whole
    /// org-extraction + repo_exists + create_repo dance when this
    /// returns `Some`. Default `None` means the regular host-adapter
    /// flow (token, API, scope-guard) applies. See [`crate::DirectRepoCreator`].
    fn direct_repo_url(&self) -> Option<&str> {
        None
    }
}
