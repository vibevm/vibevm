//! Pure git failure classification and credential-safe diagnostic rendering.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use super::GitError;
use std::ffi::OsString;
use std::path::Path;
use std::process::Output;

pub(super) fn is_truthy(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

pub(super) fn classify_failure(args: &[&str], output: &Output) -> GitError {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");

    // Extract `--` followed by URL for clone. For fetch, URL is from
    // origin which we don't know here; fall back to an empty string.
    // Scrubbed on the way out of the argv — this is the URL every
    // classified variant below will carry in its `Display`.
    let url = args
        .iter()
        .skip_while(|a| **a != "--")
        .nth(1)
        .map(|s| without_userinfo(s))
        .unwrap_or_default();

    let refname = args
        .iter()
        .skip_while(|a| **a != "--branch")
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_default();

    classify_stderr_message(&combined, url, refname).unwrap_or_else(|| GitError::CommandFailed {
        cmd: render_argv_for_display(args),
        status: output.status.code().unwrap_or(-1),
        stderr: scrub_stderr(combined.trim_end()),
    })
}

/// Pure substring-driven classifier. Receives the combined stderr+stdout
/// of a failed `git` invocation plus the URL/refname extracted from the
/// argv, and returns the matching `GitError` variant — or `None` to let
/// the caller fall back to `CommandFailed { cmd, status, stderr }`.
///
/// Order of checks is significant: more specific matchers run before
/// broader ones (e.g. `unable to access` is a wrapper that frames many
/// of the lower-level failures, so we look for the inner connect-failure
/// substrings first and let `unable to access` ride along on whatever
/// they found).
pub(super) fn classify_stderr_message(
    combined: &str,
    url: String,
    refname: String,
) -> Option<GitError> {
    let lc = combined.to_lowercase();

    // A redirect comes first: it is the most specific reading of the
    // stderr, and on an intercepting corporate network it is also the
    // most misleading one left unclassified — git reports the broken
    // smart-protocol stream (`expected flush after ref listing`) and
    // never says that a gateway answered the request instead of the
    // host. vibe does not follow redirects on the git transports, so a
    // 3xx is a terminal answer, not a hop (PROP-002
    // `#failure-discriminator`).
    if let Some(status) = observed_redirect_status(&lc) {
        return Some(GitError::HttpRedirected {
            url,
            status,
            stderr: scrub_stderr(combined.trim_end()),
        });
    }
    // Then the trust set: a TLS failure is about which certificates
    // this machine trusts, never about the repository or the
    // credentials, and it frames (`unable to access`) the same
    // connect-failure wording the network branch below keys on.
    if let Some(detail) = tls_trust_detail(combined, &lc) {
        return Some(GitError::TlsTrust {
            url,
            detail,
            stderr: scrub_stderr(combined.trim_end()),
        });
    }
    if lc.contains("repository not found")
        || lc.contains("does not appear to be a git repository")
        || lc.contains("returned error: 404")
        || lc.contains("http 404")
        || lc.contains("404 not found")
    {
        return Some(GitError::RepoNotFound { url });
    }
    // Authentication-required signals. Three families:
    //
    //   1. ssh / direct refusal — `Permission denied (publickey).`
    //      / `fatal: Authentication failed`.
    //   2. HTTPS without working credentials — git tried to ask the
    //      operator (or a credential helper) for username/password
    //      and could not get one. With our `GIT_TERMINAL_PROMPT=0` +
    //      silenced helpers (PROP-002 §2.2.1), git prints
    //      `fatal: could not read Username for '<url>'` /
    //      `fatal: could not read Password for ...` and exits
    //      non-zero. Some credential helpers also leave a `User
    //      cancelled dialog.` line on stderr when their GUI
    //      window is dismissed (the original GCM popup case).
    //   3. Direct HTTP status codes from the underlying transport —
    //      `HTTP 401` / `HTTP 403` / `401 Unauthorized` /
    //      `403 Forbidden`, and curl's `The requested URL returned
    //      error: 401`. These appear when the host returns a
    //      structured response without redirecting through the
    //      credential layer (some proxies, some CI runners).
    if lc.contains("permission denied (publickey)")
        || lc.contains("authentication failed")
        || lc.contains("could not read username")
        || lc.contains("could not read password")
        || lc.contains("user cancelled dialog")
        || lc.contains("returned error: 401")
        || lc.contains("returned error: 403")
        || lc.contains("http 401")
        || lc.contains("http 403")
        || lc.contains("401 unauthorized")
        || lc.contains("403 forbidden")
    {
        let status = observed_http_status(&lc);
        return Some(GitError::AuthFailed { url, status });
    }
    // Network / connect-failure shapes seen in the wild from git 2.x +
    // libcurl. `failed to connect` / `could not connect to server` cover
    // the curl path (the wording flipped over the 7.x → 8.x transition);
    // `connection refused` / `connection timed out` / `operation timed
    // out` cover bare-TCP and proxy paths; `could not resolve host` /
    // `network is unreachable` are the historical entries; `could not
    // read from remote repository` is the SSH-side equivalent that ssh
    // emits before git can class it any tighter.
    if lc.contains("could not resolve host")
        || lc.contains("could not read from remote repository")
        || lc.contains("network is unreachable")
        || lc.contains("failed to connect")
        || lc.contains("could not connect to")
        || lc.contains("connection refused")
        || lc.contains("connection timed out")
        || lc.contains("operation timed out")
    {
        return Some(GitError::NetworkUnreachable { url });
    }
    if lc.contains("remote branch") && lc.contains("not found")
        || lc.contains("couldn't find remote ref")
    {
        return Some(GitError::RefNotFound { url, refname });
    }

    None
}

/// The redirect status git's stderr reports, if any.
///
/// Git speaks the smart protocol over HTTP(S) and does **not** follow a
/// redirect for it, so a `301` / `302` / `307` / `308` is the whole
/// answer the host gave. The shapes seen in the wild come from libcurl
/// (`error: RPC failed; HTTP 302 curl 22 The requested URL returned
/// error: 302`) and from git's own transport layer (`unable to access
/// '…': The requested URL returned error: 301`); the reason-phrase forms
/// (`302 Found`) appear when a proxy writes its own line. `lc` is the
/// lower-cased stderr.
fn observed_redirect_status(lc: &str) -> Option<u16> {
    const REDIRECTS: [(u16, &str); 4] = [
        (301, "moved permanently"),
        (302, "found"),
        (307, "temporary redirect"),
        (308, "permanent redirect"),
    ];
    REDIRECTS.into_iter().find_map(|(code, phrase)| {
        (lc.contains(&format!("returned error: {code}"))
            || lc.contains(&format!("http {code}"))
            || lc.contains(&format!("http/1.1 {code}"))
            || lc.contains(&format!("http/2 {code}"))
            || lc.contains(&format!("{code} {phrase}")))
        .then_some(code)
    })
}

/// The line of git's stderr that reports a TLS trust failure, if any —
/// scrubbed, so it is safe to carry in a `Display` type.
///
/// Everything here is about the *trust set* this machine presents to the
/// verifier: an unknown issuer (a corporate TLS-terminating gateway signs
/// with its own CA), an expired or mismatched certificate, or a handshake
/// the platform backend refused. OpenSSL, schannel (Windows) and GnuTLS
/// each word it differently; the substrings below are the stable parts of
/// each wording. `combined` is the original stderr (so the returned line
/// reads as git wrote it); `lc` is its lower-cased twin.
fn tls_trust_detail(combined: &str, lc: &str) -> Option<String> {
    const TLS_MARKERS: [&str; 11] = [
        "unable to get local issuer certificate",
        "unable to get issuer certificate",
        "certificate verify failed",
        "ssl certificate problem",
        "self signed certificate",
        "self-signed certificate",
        "certificate has expired",
        "ssl peer certificate or ssh remote key was not ok",
        "gnutls_handshake() failed",
        "ssl connect error",
        "schannel: ",
    ];
    if !TLS_MARKERS.iter().any(|marker| lc.contains(marker)) {
        return None;
    }
    // Prefer the line that actually carries the diagnosis; fall back to
    // the first non-empty line so the variant is never built empty.
    let line = combined
        .lines()
        .find(|line| {
            let line = line.to_lowercase();
            TLS_MARKERS.iter().any(|marker| line.contains(marker))
        })
        .or_else(|| combined.lines().find(|line| !line.trim().is_empty()))?;
    Some(scrub_stderr(line.trim()))
}

/// Every `scheme://userinfo@host/…` token in `text` reduced to its
/// credential-free form — the same reduction [`without_userinfo`] makes,
/// applied to text vibe did not compose itself.
///
/// Git ≥ 2.31 redacts passwords from its own stderr, but that is the
/// *second* line of defence and it covers only the shapes git recognises:
/// a proxy's own error page, an `ssh` banner or a credential helper's
/// chatter reaches us verbatim. Anything this module may put in front of
/// an operator goes through here first, so the one-funnel property
/// [`without_userinfo`] documents holds for borrowed text too.
pub(crate) fn scrub_stderr(text: &str) -> String {
    if !text.contains("://") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    for ch in text.chars() {
        // Split on everything that cannot occur inside an authority, so a
        // url in quotes, parentheses or a sentence is still one token.
        if ch.is_whitespace() || matches!(ch, '\'' | '"' | '`' | '<' | '>' | '(' | ')' | ',') {
            if !token.is_empty() {
                out.push_str(&without_userinfo(&token));
                token.clear();
            }
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    out.push_str(&without_userinfo(&token));
    out
}

/// The HTTP refusal status git's stderr reports, if any: curl's
/// `The requested URL returned error: 403`, or `HTTP 401` /
/// `401 Unauthorized` shapes. `lc` is the lower-cased stderr.
fn observed_http_status(lc: &str) -> Option<u16> {
    [401_u16, 403].into_iter().find(|code| {
        lc.contains(&format!("returned error: {code}"))
            || lc.contains(&format!("http {code}"))
            || lc.contains(&format!(
                "{code} {}",
                if *code == 401 {
                    "unauthorized"
                } else {
                    "forbidden"
                }
            ))
    })
}

/// A URL as it may be *shown*: the same URL with any userinfo removed.
///
/// The URL this module hands to git routinely carries a credential —
/// `https://x-access-token:<TOKEN>@host/…`, put there by
/// `credentialed_url` / `inject_token` so the spawned `git` can
/// authenticate. That URL is therefore what a failure here is *about*,
/// but it is never what a [`GitError`] may carry: a `GitError` is a
/// `Display` type. It reaches the operator's terminal through
/// `RegistryError::Git`, and from there any log or bug report they paste.
/// Every `GitError` built in this module puts its URL through here first,
/// and so does every rendered argv — one funnel, so a new construction
/// site cannot quietly reintroduce the leak.
///
/// Removes the whole userinfo rather than masking the secret half: the
/// username in these URLs is the fixed literal `x-access-token`, which
/// tells an operator nothing their own `vibe.toml` does not. An
/// scp-style `git@host:org/repo.git` has no `://` and is returned
/// untouched — that `git` is an ssh account name, not a secret, and
/// removing it would make the URL wrong rather than safe.
/// Whether a lowercased `git archive --remote` failure means the server does
/// not offer `upload-archive`, so the caller may fall back to a shallow clone.
///
/// GitHub disables `upload-archive` server-side: the HTTPS smart protocol
/// answers `HTTP 422`, or the local git complains `expected ACK/NAK, got a
/// flush packet` (verified 2026-04-29). Git 2.53 against a plain smart-HTTP
/// server reports `fatal: can't connect to subservice git-upload-archive`.
/// Gitea / Forgejo / SourceHut refusals take the same fallback.
pub(super) fn archive_unsupported(combined_lc: &str) -> bool {
    combined_lc.contains("operation not supported by protocol")
        || combined_lc.contains("upload-archive")
            && (combined_lc.contains("not permitted")
                || combined_lc.contains("not allowed")
                || combined_lc.contains("disabled")
                || combined_lc.contains("can't connect to subservice")
                || combined_lc.contains("cannot connect to subservice"))
        || (combined_lc.contains("http 422") || combined_lc.contains("error: 422"))
            && combined_lc.contains("git archive")
        || combined_lc.contains("git archive")
            && combined_lc.contains("expected ack/nak")
            && combined_lc.contains("flush packet")
}

pub(crate) fn without_userinfo(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };
    let Some((_userinfo, host)) = authority.rsplit_once('@') else {
        return url.to_string();
    };
    match path {
        Some(path) => format!("{scheme}://{host}/{path}"),
        None => format!("{scheme}://{host}"),
    }
}

/// [`without_userinfo`] for one argv element, which may hold a URL either
/// bare (`git clone -- <url>`) or behind a flag (`git archive
/// --remote=<url>`).
pub(super) fn without_userinfo_in_arg(arg: &str) -> String {
    if let Some((flag, value)) = arg.split_once('=')
        && flag.starts_with("--")
    {
        return format!("{flag}={}", without_userinfo(value));
    }
    without_userinfo(arg)
}

pub(super) fn render_argv(binary: &Path, args: &[&str]) -> String {
    let mut out = OsString::from(binary);
    for a in args {
        out.push(" ");
        out.push(without_userinfo_in_arg(a));
    }
    out.to_string_lossy().into_owned()
}

pub(super) fn render_argv_for_display(args: &[&str]) -> String {
    let mut out = String::from("git");
    for a in args {
        out.push(' ');
        out.push_str(&without_userinfo_in_arg(a));
    }
    out
}
