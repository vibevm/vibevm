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
        stderr: combined.trim_end().to_string(),
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

    if lc.contains("repository not found") || lc.contains("does not appear to be a git repository")
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
    //      `403 Forbidden`. These appear when the host returns a
    //      structured response without redirecting through the
    //      credential layer (some proxies, some CI runners).
    if lc.contains("permission denied (publickey)")
        || lc.contains("authentication failed")
        || lc.contains("could not read username")
        || lc.contains("could not read password")
        || lc.contains("user cancelled dialog")
        || lc.contains("http 401")
        || lc.contains("http 403")
        || lc.contains("401 unauthorized")
        || lc.contains("403 forbidden")
    {
        return Some(GitError::AuthFailed { url });
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

pub(super) fn without_userinfo(url: &str) -> String {
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
