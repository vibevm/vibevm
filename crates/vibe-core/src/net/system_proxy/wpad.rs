//! WPAD: the candidate `wpad.<domain>/wpad.dat` locations, generated
//! (`PROP-068#proxy-system`).
//!
//! A desktop set to "auto-detect proxy settings" names no URL, so the URL
//! has to be discovered. vibe does the DNS half of WPAD: for each of the
//! host's DNS search domains it asks `http://wpad.<domain>/wpad.dat`,
//! walking up the labels, and the first that answers with a PAC body is
//! the route. Generating the list is pure and lives here; fetching it is
//! the reader behind the seam.
//!
//! **It never walks to a public suffix.** `wpad.com` or `wpad.co.uk` is
//! someone else's host, and asking it for this machine's proxy
//! configuration would hand the route to a stranger — the WPAD hijack the
//! protocol is known for. The walk therefore stops while at least
//! [`WPAD_LABEL_FLOOR`] labels remain, which also keeps the list short
//! enough to try inside one px start.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

/// How many labels a candidate domain must keep. Two — `example.com` is
/// asked, `com` is not.
pub(crate) const WPAD_LABEL_FLOOR: usize = 2;

/// How many candidates are ever generated, however many search domains
/// the host carries: the list is walked with a network timeout each, and
/// px is waiting.
pub(crate) const WPAD_CANDIDATE_LIMIT: usize = 6;

/// The `http://wpad.<domain>/wpad.dat` candidates for a host's DNS search
/// domains and FQDN, in the order they are tried: each domain's own
/// candidate first, then its parents, never a public suffix,
/// deduplicated, at most [`WPAD_CANDIDATE_LIMIT`].
pub(crate) fn wpad_candidates(domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for domain in domains {
        let domain = domain.trim().trim_matches('.').to_ascii_lowercase();
        if domain.is_empty() || domain.contains('/') || domain.contains(':') {
            continue;
        }
        let labels: Vec<&str> = domain.split('.').filter(|l| !l.is_empty()).collect();
        // A bare hostname with no domain part says nothing about a search
        // domain; `wpad.<single label>` is not a candidate either.
        for start in 0..labels.len().saturating_sub(WPAD_LABEL_FLOOR - 1) {
            let rest = &labels[start..];
            if rest.len() < WPAD_LABEL_FLOOR {
                break;
            }
            let candidate = format!("http://wpad.{}/wpad.dat", rest.join("."));
            if !out.contains(&candidate) {
                out.push(candidate);
            }
            if out.len() >= WPAD_CANDIDATE_LIMIT {
                return out;
            }
        }
    }
    out
}

/// Whether a fetched body is a PAC file: it declares the one function
/// every PAC file must declare. The same test `vibe self doctor` applies
/// to a configured PAC, so a captive portal's login page is never
/// mistaken for the corporate proxy configuration.
///
/// ```
/// use vibe_core::net::looks_like_pac;
/// assert!(looks_like_pac("function FindProxyForURL(url, host) { return \"DIRECT\"; }"));
/// assert!(!looks_like_pac("<html><body>Sign in to the guest network</body></html>"));
/// ```
pub fn looks_like_pac(body: &str) -> bool {
    body.contains("FindProxyForURL")
}
