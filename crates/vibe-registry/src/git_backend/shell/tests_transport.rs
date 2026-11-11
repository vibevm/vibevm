//! What the stderr classifier makes of the transport failures a
//! corporate network produces — redirects, TLS trust, the HTTP statuses
//! and the connect failures. Driven by the exact wording git + libcurl
//! emit, because substring classification is only as good as the
//! strings it was shown.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use specmark::verifies;

use super::*;

/// The classifier over one stderr body, with a known endpoint.
fn classify_at(stderr: &str, url: &str) -> Option<GitError> {
    classify_stderr_message(stderr, url.into(), "v26.13.0".into())
}

/// The exact body the owner's corporate network produced: libcurl
/// reports the status, git reports the broken ref listing, and nothing
/// in either line says "a gateway answered instead of the host".
const INTERCEPTED_302: &str = "error: RPC failed; HTTP 302 curl 22 The requested URL returned \
                               error: 302\nfatal: expected flush after ref listing";

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn every_redirect_status_classifies_as_a_redirect() {
    // One body per status, in the two shapes the transports produce:
    // libcurl's `returned error: <code>` and git's own `HTTP <code>`.
    for (code, body) in [
        (
            301_u16,
            "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: 301"
                .to_string(),
        ),
        (302, INTERCEPTED_302.to_string()),
        (
            307,
            "error: RPC failed; HTTP 307 curl 22 The requested URL returned error: 307".to_string(),
        ),
        (
            308,
            "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: 308"
                .to_string(),
        ),
    ] {
        let classified = classify_at(&body, "https://h/o/r.git");
        let Some(GitError::HttpRedirected { status, stderr, .. }) = &classified else {
            panic!("expected HttpRedirected for {code}, got {classified:?}");
        };
        assert_eq!(*status, code);
        // The raw text survives the classification — the explanation
        // layer prints it below the diagnosis, never instead of it.
        assert_eq!(stderr.trim_end(), body.trim_end());
        let text = classified.unwrap().to_string();
        assert!(text.contains(&format!("HTTP redirect ({code})")), "{text}");
        assert!(text.contains("exclude_hosting"), "{text}");
        assert!(text.contains("vibe dl --test-proxy"), "{text}");
    }
}

/// A reason-phrase line (a proxy writing its own status line) reads the
/// same as a libcurl-reported status.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn a_reason_phrase_status_line_is_still_a_redirect() {
    let classified = classify_at(
        "fatal: unable to access 'https://h/o/r.git/': HTTP/1.1 302 Found",
        "https://h/o/r.git",
    );
    assert!(matches!(
        classified,
        Some(GitError::HttpRedirected { status: 302, .. })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn the_tls_trust_wordings_classify_as_the_trust_set() {
    for body in [
        "fatal: unable to access 'https://h/o/r.git/': SSL certificate problem: unable to get \
         local issuer certificate",
        "fatal: unable to access 'https://h/o/r.git/': server certificate verification failed. \
         CAfile: none CRLfile: none; certificate verify failed",
        "fatal: unable to access 'https://h/o/r.git/': SSL certificate problem: self signed \
         certificate in certificate chain",
        "fatal: unable to access 'https://h/o/r.git/': schannel: next InitializeSecurityContext \
         failed: CRYPT_E_NO_REVOCATION_CHECK (0x80092012) - the certificate chain could not be \
         checked",
    ] {
        let classified = classify_at(body, "https://h/o/r.git");
        let Some(GitError::TlsTrust { detail, stderr, .. }) = &classified else {
            panic!("expected TlsTrust, got {classified:?}");
        };
        assert!(!detail.is_empty(), "{body}");
        assert_eq!(stderr.trim_end(), body);
        let text = classified.unwrap().to_string();
        assert!(text.contains("TLS trust failure"), "{text}");
        assert!(text.contains("ca_file"), "{text}");
        assert!(text.contains("vibe dl --test-proxy"), "{text}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn the_http_refusal_statuses_keep_their_own_classification() {
    for (code, body) in [
        (
            401_u16,
            "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: 401",
        ),
        (
            403,
            "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: 403",
        ),
    ] {
        let classified = classify_at(body, "https://h/o/r.git");
        let Some(GitError::AuthFailed { status, .. }) = &classified else {
            panic!("expected AuthFailed for {code}, got {classified:?}");
        };
        assert_eq!(*status, Some(code));
    }
}

/// A 404 over HTTP means the same thing `remote: Repository not found.`
/// means — the endpoint has no such repository — and must not fall
/// through to the unclassified bucket.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn a_404_is_a_missing_repository() {
    for body in [
        "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: 404",
        "error: RPC failed; HTTP 404 curl 22 The requested URL returned error: 404",
        "remote: Repository not found.\nfatal: repository 'https://h/o/r.git/' not found",
    ] {
        assert!(
            matches!(
                classify_at(body, "https://h/o/r.git"),
                Some(GitError::RepoNotFound { .. })
            ),
            "{body}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn dns_timeout_and_refusal_stay_network_failures() {
    for body in [
        "fatal: unable to access 'https://h/o/r.git/': Could not resolve host: h",
        "fatal: unable to access 'https://h/o/r.git/': Failed to connect to h port 443 after \
         75003 ms: Operation timed out",
        "fatal: unable to access 'https://h/o/r.git/': Failed to connect to 127.0.0.1 port 1 \
         after 0 ms: Connection refused",
    ] {
        assert!(
            matches!(
                classify_at(body, "https://h/o/r.git"),
                Some(GitError::NetworkUnreachable { .. })
            ),
            "{body}"
        );
    }
}

/// A redirect is read before the connect-failure wording that frames it
/// — an intercepting gateway produces both, and only the former names
/// what actually happened.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn a_redirect_outranks_the_wrapper_wordings_around_it() {
    let body = "fatal: unable to access 'https://h/o/r.git/': The requested URL returned error: \
                302\nfatal: could not read Username for 'https://h': terminal prompts disabled";
    assert!(matches!(
        classify_at(body, "https://h/o/r.git"),
        Some(GitError::HttpRedirected { status: 302, .. })
    ));
}

/// Nothing classified above may carry a credential: the stderr a
/// variant keeps goes through the same scrubbing funnel vibe's own urls
/// do.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn a_credential_in_borrowed_stderr_is_scrubbed() {
    let body = "fatal: unable to access \
                'https://x-access-token:SECRET@h/o/r.git/': The requested URL returned error: 302";
    let classified = classify_at(body, "https://h/o/r.git").unwrap();
    let GitError::HttpRedirected { stderr, .. } = &classified else {
        panic!("expected HttpRedirected, got {classified:?}");
    };
    assert!(!stderr.contains("SECRET"), "{stderr}");
    assert!(stderr.contains("https://h/o/r.git/"), "{stderr}");
    assert!(!classified.to_string().contains("SECRET"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn scrubbing_leaves_text_without_a_url_alone() {
    assert_eq!(scrub_stderr("fatal: nothing here"), "fatal: nothing here");
    assert_eq!(
        scrub_stderr("see <https://u:p@h/x> and \"https://h2/y\""),
        "see <https://h/x> and \"https://h2/y\""
    );
}
