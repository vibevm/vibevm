//! The operator-facing wording of a [`GitError`](super::GitError) — the heads and the
//! standing fixes its `#[error]` attributes interpolate.
//!
//! Split out of `mod.rs` under the file-length budget when the redirect
//! and TLS-trust classifications arrived with fixes of their own. The
//! seam is honest rather than arithmetic: everything here is *prose
//! about a failure*, chosen so that the same wording reaches an
//! operator wherever the failure surfaces — bare, or inside the
//! explanation `crate::git_failure` builds around it.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::path::Path;

/// The one fix a redirected git request takes, wherever it surfaces.
pub(crate) const REDIRECTED_FIX: &str = "vibe does not follow redirects for git, so this is the \
     whole answer the endpoint gave. On a corporate network it usually means an external host is \
     intercepted or blocked — read the registry from the corporate mirror (`[source_policy] \
     exclude_hosting` on the public hosting), or configure the proxy and check it with \
     `vibe dl --test-proxy`";

/// The one fix a TLS trust failure takes, wherever it surfaces.
pub(crate) const TLS_TRUST_FIX: &str = "the verifier did not trust the certificate chain this endpoint presented — a \
     TLS-terminating corporate gateway signs with its own CA, which this machine must be \
     told about: check the path with `vibe dl --test-proxy` and name the corporate root \
     certificate in `[proxy] ca_file`";

/// The [`GitError::HttpRedirected`](super::GitError::HttpRedirected) head: the status, and the endpoint
/// when one is known (a `git fetch` against a recorded `origin` reports
/// no url of its own).
///
/// ```
/// use vibe_registry::GitError;
/// let seen = GitError::HttpRedirected {
///     url: "https://h/o/r.git".into(),
///     status: 302,
///     stderr: String::new(),
/// };
/// assert!(seen.to_string().starts_with("`https://h/o/r.git` answered the git request with an HTTP redirect (302)"));
/// let blind = GitError::HttpRedirected { url: String::new(), status: 301, stderr: String::new() };
/// assert!(blind.to_string().starts_with("the remote answered the git request with an HTTP redirect (301)"));
/// ```
pub(super) fn redirected_head(url: &str, status: u16) -> String {
    format!(
        "{} answered the git request with an HTTP redirect ({status}) instead of the git protocol",
        endpoint_label(url)
    )
}

/// The [`GitError::TlsTrust`](super::GitError::TlsTrust) head: the endpoint and git's own wording.
///
/// ```
/// use vibe_registry::GitError;
/// let refused = GitError::TlsTrust {
///     url: "https://h/o/r.git".into(),
///     detail: "SSL certificate problem: unable to get local issuer certificate".into(),
///     stderr: String::new(),
/// };
/// assert!(refused.to_string().starts_with(
///     "TLS trust failure talking to `https://h/o/r.git`: \
///      SSL certificate problem: unable to get local issuer certificate"
/// ));
/// ```
pub(super) fn tls_trust_head(url: &str, detail: &str) -> String {
    format!(
        "TLS trust failure talking to {}: {detail}",
        endpoint_label(url)
    )
}

/// `` `<url>` `` when the endpoint is known, `the remote` when it is not.
fn endpoint_label(url: &str) -> String {
    if url.is_empty() {
        "the remote".to_string()
    } else {
        format!("`{url}`")
    }
}

/// The transport a Git address uses, as an operator would name it.
pub(super) fn transport_of(url: &str) -> &'static str {
    let raw = url.strip_prefix("git+").unwrap_or(url);
    let lower = raw.get(..8).unwrap_or(raw).to_ascii_lowercase();
    if lower.starts_with("https://") {
        "HTTPS"
    } else if lower.starts_with("http://") {
        "HTTP"
    } else if lower.starts_with("ssh://")
        || (!raw.contains("://") && raw.contains(':') && !Path::new(raw).is_absolute())
    {
        "SSH"
    } else {
        "local"
    }
}

/// The [`GitError::AuthFailed`](super::GitError::AuthFailed) head: the actual transport and status.
///
/// ```
/// use vibe_registry::GitError;
/// let https = GitError::AuthFailed { url: "https://h/o/r.git".into(), status: Some(401) };
/// assert!(https.to_string().starts_with("HTTPS authentication failed (HTTP 401)"));
/// assert!(!https.to_string().contains("ssh-agent"));
/// let ssh = GitError::AuthFailed { url: "git@h:o/r.git".into(), status: None };
/// assert!(ssh.to_string().contains("ssh-agent"));
/// ```
pub(super) fn auth_failed_head(url: &str, status: Option<u16>) -> String {
    let status = status
        .map(|code| format!(" (HTTP {code})"))
        .unwrap_or_default();
    format!(
        "{} authentication failed{status} for `{url}`",
        transport_of(url)
    )
}

/// The fix that applies to the transport `url` uses.
pub(super) fn auth_failed_fix(url: &str) -> &'static str {
    match transport_of(url) {
        "HTTPS" | "HTTP" => {
            "the host requires credentials for this repository: if it is private, \
             give the registry a read token (`auth = \"token-env\"` with `token_env`), \
             a credential helper (`auth = \"credential-helper\"`) or an SSH url; \
             if it should be public, check the url and that the repository exists"
        }
        "local" => "check the repository path and its file permissions",
        _ => "load your key into ssh-agent / check ~/.ssh, or fix the [[registry]] auth setting",
    }
}
