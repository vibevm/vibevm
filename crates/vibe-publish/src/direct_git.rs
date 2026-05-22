//! [`RepoCreator`] adapter for "no-API" publish — push directly to a
//! known git URL using the local user's git credentials (SSH agent,
//! `credential.helper`, …). No HTTP API call, no token loading, no
//! organisation-scope plumbing. Use case: an operator who has already
//! provisioned the package repo on the host (manually, or via a host
//! we don't have an API adapter for) and just wants vibevm to do the
//! tag-and-push dance against it.
//!
//! Wiring: the consumer (CLI) checks for a non-empty `--repo-url`,
//! constructs a [`DirectGitCreator`] with that URL, and feeds it into
//! [`crate::Publisher`] in place of the regular host adapter. When
//! [`Publisher::publish`](crate::Publisher::publish) sees
//! [`RepoCreator::direct_repo_url`] return `Some`, it short-circuits
//! the org-extraction + create-repo + token-aware push-URL flow and
//! pushes the freshly-built commit + tag straight to the supplied URL.
//!
//! Per [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish):
//! adapters are pluggable; this is the explicit "operator-managed
//! repository" plug. The token-secrecy invariant
//! ([PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy))
//! is upheld trivially because no token participates in this path —
//! the credential surface is local-git's, not vibevm's.

use crate::{CreateOpts, PublishError, RepoCreator, RepoInfo, extract_host_segment};

/// Direct-push adapter. Constructed from a single repo URL; carries
/// no token, no API client, no org scoping. Every [`RepoCreator`]
/// method except [`RepoCreator::push_url`] is a no-op or refusal.
pub struct DirectGitCreator {
    /// Repo URL the operator supplied. Used verbatim as the push URL
    /// — vibevm never rewrites or augments it.
    repo_url: String,
    /// Host segment extracted from `repo_url` for diagnostics. Falls
    /// back to a literal `"git"` when the URL doesn't parse as one of
    /// the known shapes (e.g. `file:///` for tests against a local
    /// bare repo).
    host_name: String,
}

impl DirectGitCreator {
    /// Build a creator for the given URL. The URL is used as-is at
    /// push time; vibevm does not inspect, normalise, or canonicalise
    /// it. A trailing `/` does no harm — git handles both forms.
    pub fn new(repo_url: impl Into<String>) -> Self {
        let repo_url = repo_url.into();
        let host_name = extract_host_segment(&repo_url).unwrap_or_else(|_| "git".to_string());
        DirectGitCreator {
            repo_url,
            host_name,
        }
    }

    /// The configured URL. Exposed so the CLI can echo it in the
    /// outcome envelope.
    pub fn repo_url(&self) -> &str {
        &self.repo_url
    }
}

impl RepoCreator for DirectGitCreator {
    fn host_name(&self) -> &str {
        &self.host_name
    }

    fn expected_org(&self) -> Option<&str> {
        // Direct-push is operator-managed; there is no org scoping.
        None
    }

    fn validate_scope(&self, _org: &str) -> Result<(), PublishError> {
        // No scoping to enforce. The publish flow that calls this
        // never derives an org from a URL on the direct path —
        // see [`Publisher::publish`] short-circuit on `direct_repo_url`.
        Ok(())
    }

    fn repo_exists(&self, _org: &str, _name: &str) -> Result<bool, PublishError> {
        // The operator told vibevm "this repo exists, push to it"
        // by supplying the URL. Treat as existing; if the push fails
        // because the URL is wrong, `git_publish::push_with_classification`
        // surfaces a network / push-denied / not-found error with
        // the URL inline (credentials redacted per PROP-000 §20).
        Ok(true)
    }

    fn create_repo(
        &self,
        _org: &str,
        _name: &str,
        _opts: &CreateOpts,
    ) -> Result<RepoInfo, PublishError> {
        // Unreachable on the direct path — `repo_exists` always
        // returns `Ok(true)` so [`Publisher::publish`] never falls
        // into this arm. Reach it only via misuse (wired through a
        // Publisher that ignored `direct_repo_url`); raise a clear
        // error so the bug is loud.
        Err(PublishError::Git(format!(
            "internal error: DirectGitCreator::create_repo invoked for `{}` — \
             direct-push adapter does not provision repositories. The publish \
             pipeline should have short-circuited at `direct_repo_url`.",
            self.repo_url
        )))
    }

    fn push_url(&self, _org: &str, _name: &str) -> String {
        // The configured URL is the push URL, verbatim. Local git
        // resolves credentials via SSH agent / credential.helper /
        // whatever else it is wired to use.
        self.repo_url.clone()
    }

    fn direct_repo_url(&self) -> Option<&str> {
        Some(&self.repo_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_name_for_https_url() {
        let c = DirectGitCreator::new("https://example.org/foo/bar.git");
        assert_eq!(c.host_name(), "example.org");
    }

    #[test]
    fn host_name_for_ssh_shorthand() {
        let c = DirectGitCreator::new("git@example.org:foo/bar.git");
        assert_eq!(c.host_name(), "example.org");
    }

    #[test]
    fn host_name_falls_back_for_local_bare_repo() {
        // `file:///` URLs do not carry a meaningful host; fallback
        // is the literal `"git"`. Useful in tests that point at a
        // local bare repo for hermetic e2e coverage.
        let c = DirectGitCreator::new("file:///tmp/origin.git");
        assert_eq!(c.host_name(), "git");
    }

    #[test]
    fn direct_repo_url_round_trips() {
        let c = DirectGitCreator::new("ssh://git@example.org/foo.git");
        assert_eq!(c.direct_repo_url(), Some("ssh://git@example.org/foo.git"));
        assert_eq!(c.repo_url(), "ssh://git@example.org/foo.git");
    }

    #[test]
    fn validate_scope_is_a_no_op() {
        let c = DirectGitCreator::new("https://example.org/foo.git");
        assert!(c.validate_scope("anything").is_ok());
        assert!(c.expected_org().is_none());
    }

    #[test]
    fn repo_exists_returns_true_unconditionally() {
        let c = DirectGitCreator::new("https://example.org/foo.git");
        assert!(c.repo_exists("ignored-org", "ignored-name").unwrap());
    }

    #[test]
    fn push_url_returns_configured_url_verbatim() {
        let c = DirectGitCreator::new("https://example.org/foo/bar.git");
        // org and name args are ignored on this path.
        assert_eq!(
            c.push_url("ignored", "also-ignored"),
            "https://example.org/foo/bar.git"
        );
    }

    #[test]
    fn create_repo_raises_clear_error() {
        let c = DirectGitCreator::new("https://example.org/foo.git");
        let err = c
            .create_repo("ignored", "name", &CreateOpts::default())
            .expect_err("create_repo must refuse on direct-push adapter");
        match err {
            PublishError::Git(msg) => {
                assert!(msg.contains("DirectGitCreator"));
                assert!(msg.contains("https://example.org/foo.git"));
            }
            other => panic!("expected PublishError::Git, got: {other:?}"),
        }
    }
}
