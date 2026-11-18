//! Git backend failures.
use super::*;
use specmark::spec;
use thiserror::Error;

/// Errors a [`GitBackend`] operation may surface.
///
/// Variants correspond to stderr patterns stable enough to key on.
/// Anything unclassified surfaces as [`GitError::CommandFailed`] with the
/// raw stderr attached.
#[derive(Debug, Error)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
)]
pub enum GitError {
    #[error(
        "SSH host key rejected (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH; fix: verify the exact endpoint and explicitly select its trusted known-hosts input)"
    )]
    SshHostKeyRejected,
    #[error(
        "SSH authentication rejected (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH; fix: select an authorized identity or explicit agent for this exact endpoint)"
    )]
    SshAuthenticationRejected,
    #[error(
        "native operation unavailable: {operation} (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION; fix: use a separately supported transport or materialisation capability)"
    )]
    NativeUnavailable { operation: &'static str },
    #[error(
        "native context unsupported: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION; fix: supply a supported explicit transport context)"
    )]
    NativeContext { reason: &'static str },
    /// A bounded metadata reader of the controlled transport failed, carrying
    /// **what was run and what Git said**: with a cleared environment and a
    /// selected installation, "the reader failed" told an operator nothing.
    #[error(
        "the selected Git could not answer `git {argv}`{} (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION; fix: check that the selected Git installation supports this operation, and read its own message above): {stderr}",
        match status { Some(code) => format!(" (exit {code})"), None => String::new() }
    )]
    NativeMetadata {
        /// The child's arguments, and Git's own stderr below them.
        argv: String,
        status: Option<i32>,
        stderr: String,
    },
    /// A path-valued OpenSSH setting the selected client reported that this
    /// capture cannot lower to one file it may read. The message names the
    /// client that produced the value, the keyword it reported it under and
    /// the value itself, because the setting may come from any file in the
    /// user's Include closure or from the client's compiled-in defaults.
    #[error(
        "the SSH client `{executable}` reported `{key} {value}`, which this capture cannot use: \
         {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION; \
          fix: {fix})"
    )]
    SshConfigPath {
        executable: String,
        key: String,
        value: String,
        reason: &'static str,
        fix: &'static str,
    },
    #[error(transparent)]
    ContactRefused(#[from] crate::ContactRefusal),
    #[error(
        "checkout at `{path}` has no origin remote (violates spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION; fix: configure the checkout origin or a reachable registry mirror and prepare again)"
    )]
    OriginMissing { path: std::path::PathBuf },
    #[error(
        "this git backend does not implement immutable prepared checkouts (violates spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION; fix: use a backend that supports prepared source admission)"
    )]
    PreparedCheckoutUnsupported,
    /// A prepared source cannot establish the authoritative checkout invariant.
    #[error(
        "prepared checkout refused: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION; fix: correct the source or checkout attributes and prepare again)"
    )]
    PreparedCheckout { reason: String },
    /// The requested object was acquired, but inspecting that exact source
    /// failed. This phase is terminal: another source cannot replace it.
    #[error(
        "acquired source validation failed (violates spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION; fix: correct the acquired source or checkout inputs and prepare again): {source}"
    )]
    PreparedValidation {
        #[source]
        source: Box<GitError>,
    },
    #[error(
        "the `git` executable is not available on PATH; install git \
         (https://git-scm.com/downloads) and retry \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend; \
          fix: install git and ensure it is on PATH)"
    )]
    NotInstalled,

    #[error(
        "remote repository `{url}` not found (does it exist? is access granted?) \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: verify the repo URL and your read access)"
    )]
    RepoNotFound { url: String },

    /// The host refused the read for want of (valid) credentials. The
    /// message names the transport the `url` uses and, for HTTP(S), the
    /// status when one was observed — an HTTPS 401 is not an SSH problem.
    #[error(
        "{} (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth; fix: {})",
        auth_failed_head(url, *status),
        auth_failed_fix(url)
    )]
    AuthFailed {
        url: String,
        /// The HTTP status (401 / 403) when the transport reported one.
        status: Option<u16>,
    },

    /// The host answered the git request with an HTTP redirect rather
    /// than the smart-protocol stream. Git does not follow a redirect
    /// for this transport, so the 3xx is the whole answer — and on a
    /// corporate network it is almost always a gateway answering in the
    /// host's place, which is why the fix names the mirror and the proxy
    /// before it names the url.
    #[error(
        "{} (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; fix: {})",
        redirected_head(url, *status),
        REDIRECTED_FIX
    )]
    HttpRedirected {
        url: String,
        status: u16,
        /// Git's own stderr, kept so the explanation layer can print it
        /// below the diagnosis instead of in place of it.
        stderr: String,
    },

    /// The TLS handshake was refused over the certificate: an unknown
    /// issuer, an expired or mismatched certificate, a handshake the
    /// platform backend would not complete. About the trust set this
    /// machine presents, never about the repository or the credentials.
    #[error(
        "{} (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; fix: {})",
        tls_trust_head(url, detail),
        TLS_TRUST_FIX
    )]
    TlsTrust {
        url: String,
        detail: String,
        /// Git's own stderr — see [`GitError::HttpRedirected`].
        stderr: String,
    },

    #[error(
        "unable to reach `{url}` (network or DNS error) \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check connectivity and the host name)"
    )]
    NetworkUnreachable { url: String },

    #[error(
        "branch / ref `{refname}` not found on `{url}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: verify the ref with `git ls-remote`)"
    )]
    RefNotFound { url: String, refname: String },

    #[error(
        "file `{path}` not found in `{url}` at ref `{refname}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: ensure the file is committed at that ref)"
    )]
    FileNotFoundInRef {
        url: String,
        refname: String,
        path: String,
    },

    #[error(
        "remote `{url}` does not support `git archive` for fetching individual files \
         (uploadarch service refused). Caller should fall back to a clone \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: use the clone fallback or enable upload-archive on the host)"
    )]
    ArchiveUnsupported { url: String },

    #[error(
        "git `{cmd}` exited with status {status} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#windows-ux; \
          fix: re-run the command by hand and read the stderr below):\n{stderr}"
    )]
    CommandFailed {
        cmd: String,
        status: i32,
        stderr: String,
    },

    #[error(
        "I/O error spawning git `{cmd}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend; \
          fix: check the git installation and PATH): {source}"
    )]
    Io {
        cmd: String,
        #[source]
        source: std::io::Error,
    },
}
