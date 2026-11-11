//! This invocation's extra request headers on the direct HTTP client —
//! the branch a fetch takes when the caller handed it no managed client
//! (`PROP-075#DOWNLOAD-HEADERS`).
//!
//! The managed branch keeps its headers on one origin inside the walk it
//! already owns. This one has no walk: the client follows redirects
//! itself, and would resend every header it was given. So when there are
//! headers the client is told to refuse a redirect that leaves the origin
//! the declared URL names, which is exactly what nvm does when
//! `$NVM_AUTH_HEADER` is set and the downloader is one that would carry it
//! across (`nvm.sh`, `nvm_wget_same_origin`).

use vibe_core::download_headers::DownloadHeaders;

/// At most this many redirects, as the managed walk allows.
const HOPS: usize = 10;

/// Attach `headers` — the invocation's, when it declared any — to a
/// request. The names and values are already validated, so this cannot
/// introduce a header the file reader would have refused.
pub(super) fn attach(
    mut request: reqwest::blocking::RequestBuilder,
    headers: Option<&DownloadHeaders>,
) -> reqwest::blocking::RequestBuilder {
    for header in headers.into_iter().flatten() {
        request = request.header(header.name(), header.value());
    }
    request
}

/// A redirect policy that follows only inside the origin `source` names.
/// A redirect out of it is refused rather than followed with the headers
/// stripped: without a walk of its own this side cannot re-issue the
/// request, and a credential that silently becomes an anonymous request
/// reads as a mirror that forgot its gateway rather than as a refusal.
pub(super) fn origin_bound_redirects(source: &str) -> reqwest::redirect::Policy {
    let shown = vibe_core::progress::transfer::safe_remote(source);
    let origin = reqwest::Url::parse(source).ok().map(|url| url.origin());
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() > HOPS {
            return attempt.error(format!("{shown}: too many redirects"));
        }
        match &origin {
            Some(origin) if attempt.url().origin() == *origin => attempt.follow(),
            _ => attempt.error(format!(
                "{shown} redirects to another origin; the headers of \
                 `--download-header-file` would be sent there too, so it is not followed"
            )),
        }
    })
}
