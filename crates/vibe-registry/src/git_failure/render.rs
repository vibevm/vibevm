//! Turning a [`super::GitFailureDetail`] into the block an operator
//! reads — and, below it, the bounded tail of what git itself said.
//!
//! The order is the contract (PROP-002 `#failure-discriminator`): the
//! errand, the registry and its real url, the package, the translated
//! cause and its fix, the allowed registries, the public-host remark,
//! and only then git's stderr. The raw text is evidence under a
//! conclusion, never the conclusion.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::fmt::Write as _;

use super::{GitFailureDetail, roster};
use crate::git_backend::shell::scrub_stderr;
use crate::git_backend::{GitError, REDIRECTED_FIX, TLS_TRUST_FIX};

/// How many trailing lines of git's own stderr the explanation keeps.
///
/// The tail is evidence, not the answer: enough to recognise the shape
/// an operator already knows (`error: RPC failed; … fatal: expected
/// flush after ref listing`) without burying the explanation now
/// standing above it. A `git clone` that fails after a long transfer can
/// emit hundreds of progress lines and only their end carries anything.
const STDERR_TAIL_LINES: usize = 12;

/// Everything above the error variant's spec/fix trailer.
pub(super) fn explanation(detail: &GitFailureDetail) -> String {
    let mut out = format!("{} failed", detail.operation);
    let _ = write!(
        out,
        "\n  registry: {} — {}",
        detail.registry,
        endpoint(&detail.url)
    );
    if let Some(package) = detail.operation.package() {
        let version = detail
            .operation
            .version()
            .map(|version| format!(" version {version}"))
            .unwrap_or_default();
        let _ = write!(out, "\n  package:  {package}{version}");
    }
    let (cause, fix) = translate(&detail.source, &detail.url);
    let _ = write!(out, "\n  cause:    {cause}");
    let _ = write!(out, "\n  fix:      {fix}");
    if let Some(allowed) = detail.roster.allowed_line() {
        let _ = write!(out, "\n  {allowed}");
    }
    if let Some(remark) = public_host_remark(detail) {
        let _ = write!(out, "\n  note:     {remark}");
    }
    out
}

/// `` `<url>` `` — or an honest admission when the failing invocation
/// carried no address of its own. Never the word `origin`: that is
/// git's name for a remote, not something an operator can open.
fn endpoint(url: &str) -> String {
    if url.is_empty() {
        "(no url recorded for this attempt)".to_string()
    } else {
        format!("`{url}`")
    }
}

/// The remark requirement 6 asks for: this attempt went to a public
/// host although the project declares a site registry of its own.
fn public_host_remark(detail: &GitFailureDetail) -> Option<String> {
    let host = roster::host_of(&detail.url)?;
    if !roster::is_public_hosting(&detail.url) && !detail.roster.is_public_host(&host) {
        return None;
    }
    let site = detail.roster.site_registries();
    let named = site
        .iter()
        .map(|row| format!("`{}`", row.name))
        .collect::<Vec<_>>()
        .join(", ");
    (!site.is_empty()).then(|| {
        format!(
            "this attempt went to the public host `{host}`, although this project \
             configures a site registry ({named}) — if the mirror should serve this \
             package, exclude the public hosting with `[source_policy] exclude_hosting`"
        )
    })
}

/// The cause in the operator's terms, and the fix that answers it.
///
/// Every arm reads the same way: *what the endpoint did*, then *what to
/// change*. The two classifications that carry their own standing fix
/// ([`REDIRECTED_FIX`], [`TLS_TRUST_FIX`]) reuse it verbatim rather than
/// paraphrase it, so the wording an operator sees here is the wording
/// they see wherever else that failure surfaces.
fn translate(error: &GitError, url: &str) -> (String, String) {
    match error {
        GitError::HttpRedirected { status, .. } => (
            format!("the server redirected (HTTP {status}) instead of answering the git request"),
            REDIRECTED_FIX.to_string(),
        ),
        GitError::TlsTrust { detail, .. } => (
            format!("the TLS certificate chain was not trusted — {detail}"),
            TLS_TRUST_FIX.to_string(),
        ),
        GitError::AuthFailed { status, .. } => (
            match status {
                Some(code) => format!("the endpoint refused the read (HTTP {code})"),
                None => "the endpoint refused the read for want of credentials".to_string(),
            },
            "give this registry credentials it accepts — `auth = \"token-env\"` with \
             `token_env`, `auth = \"credential-helper\"`, or an SSH url with a key in \
             ssh-agent; if it should be readable without any, check that the url and the \
             repository are the ones you meant"
                .to_string(),
        ),
        GitError::RepoNotFound { .. } => (
            "the endpoint has no such repository — it does not exist, or this identity \
             may not see it"
                .to_string(),
            "check the registry url and the package's repository name, and that the \
             account this run authenticates as has read access"
                .to_string(),
        ),
        GitError::RefNotFound { refname, .. } => (
            format!("the repository has no ref `{refname}`"),
            "check the version tag with `git ls-remote --tags <url>`; a registry that \
             publishes `v<semver>` tags has one per released version"
                .to_string(),
        ),
        GitError::NetworkUnreachable { .. } => (
            "the host did not answer — name resolution, the connection, or a proxy in \
             front of it"
                .to_string(),
            "check connectivity to the host, and whether this machine needs a proxy for \
             it: `vibe dl --test-proxy` reports the path vibe would take"
                .to_string(),
        ),
        GitError::CommandFailed { status, .. } => (
            format!("git exited with status {status} and vibe could not classify why"),
            format!(
                "read git's own output below; the endpoint is {}, and `vibe dl \
                 --test-proxy` reports whether this machine can reach it at all",
                endpoint(url)
            ),
        ),
        other => (other.to_string(), "act on the git error above".to_string()),
    }
}

/// Git's stderr tail, below the explanation and introduced as such.
pub(super) fn git_said(detail: &GitFailureDetail) -> String {
    let Some(stderr) = stderr_of(&detail.source) else {
        return String::new();
    };
    let scrubbed = scrub_stderr(stderr.trim_end());
    let lines: Vec<&str> = scrubbed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let kept = lines.len().saturating_sub(STDERR_TAIL_LINES);
    let mut out = String::from("\ngit said:");
    if kept > 0 {
        let _ = write!(out, "\n  … {kept} earlier line(s) omitted");
    }
    for line in &lines[kept..] {
        let _ = write!(out, "\n  {}", line.trim_end());
    }
    out
}

/// Git's own stderr, as the classified failure carries it.
fn stderr_of(error: &GitError) -> Option<&str> {
    match error {
        GitError::CommandFailed { stderr, .. }
        | GitError::HttpRedirected { stderr, .. }
        | GitError::TlsTrust { stderr, .. } => Some(stderr),
        _ => None,
    }
}
