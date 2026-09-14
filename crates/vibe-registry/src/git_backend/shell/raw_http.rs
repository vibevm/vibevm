//! The HTTPS single-file read that runs *before* `git archive` in
//! [`fetch_file_at_ref`](super::ShellGit::fetch_file_at_ref).
//!
//! **Why this exists.** [PROP-002 §2.12][prop] promises that a resolve
//! walk over N candidate versions costs N `git archive` round-trips,
//! *not* N clones. GitHub does not run the `upload-archive` service, so
//! on GitHub that promise inverted: each archive attempt spent a network
//! round-trip only to come back [`GitError::ArchiveUnsupported`], and the
//! manifest read then paid a full clone anyway. Measured on 2026-09-14,
//! `vibe install org.vibevm.world/redbook` — 26 dependencies, every one
//! of them on `github.com` — spent roughly nine seconds per package (two
//! refused archives plus a clone) and printed nothing at all for the
//! minutes it took to resolve.
//!
//! **What it does.** Both hosts we read packages from serve a single file
//! over ordinary HTTPS, one request, no working tree: GitHub from
//! `raw.githubusercontent.com`, GitVerse from its `contents` API. Reading
//! there costs one round-trip and no subprocess, which is what §2.12
//! asked for in the first place. Every other host, and every answer this
//! module cannot interpret, falls through to the archive → clone ladder
//! below it, unchanged.
//!
//! **When a host says "not now".** A rate limit and a server fault are
//! the two answers that are about the moment rather than about the file,
//! and falling through to the ladder on either is paying a clone for a
//! condition that often clears in under a second. Those two, alone, are
//! retried — briefly, a bounded number of times, and never past the
//! budget in [`retry`]. Everything else keeps the behaviour below.
//!
//! **What it must never do** is turn a cheap read into a wrong answer. A
//! host that cannot find something answers with one code for several
//! different facts — the file is absent in that ref, the ref does not
//! exist, the repository does not exist, the repository exists but is
//! invisible to us — and callers act very differently on each. Only git
//! tells them apart, so an answer we cannot fully explain is a reason to
//! ask git, never a reason to conclude anything.
//! [`absence_is_authoritative`] holds the two narrow conditions under
//! which a miss may be reported directly.
//!
//! [prop]: ../../../../../../vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf");

use std::time::Duration;

use serde::Deserialize;
use vibe_core::manifest::Manifest;

use super::GitError;

/// Timeout for one read, matching `index_client`'s fetch timeout. A read
/// that has not answered within ten seconds is no longer a fast path; the
/// caller drops to git rather than holding the whole resolve walk on it.
const READ_TIMEOUT_SECS: u64 = 10;

/// GitVerse's "no such file" code, delivered inside an HTTP 400 body as
/// `{"code":4305,"message":"File: <path> not found"}`. Verified against
/// the live host on 2026-09-14; the same code comes back for an unknown
/// ref, which is why [`absence_is_authoritative`] gates what may be
/// concluded from it.
const GITVERSE_FILE_NOT_FOUND: i64 = 4305;

/// One host the fast path knows how to read a single file from.
///
/// A table rather than a branch: the two hosts differ only in how a read
/// is addressed and how an answer is shaped, so each row states those
/// three facts and the read itself ([`try_read`]) stays host-blind. A
/// third host is a third row.
struct RawHost {
    /// The host a repository URL must carry, compared case-insensitively.
    host: &'static str,
    /// Where this host serves single files in production.
    default_base: &'static str,
    /// Address one file: `(base, repo, refname, path)` → read URL.
    /// `None` when this host cannot address that combination.
    request_url: fn(&str, &RepoCoords, &str, &str) -> Option<String>,
    /// Turn a 2xx body into the file's bytes, or `None` when the body is
    /// not a shape this reader understands — which means falling back to
    /// git, never an error.
    read_body: fn(&[u8]) -> Option<Vec<u8>>,
    /// Does this non-2xx answer mean "nothing here"? Takes the body as
    /// well as the status, because one of the two hosts carries the
    /// distinction in the body rather than the status line.
    is_miss: fn(u16, &[u8]) -> bool,
}

/// The hosts the fast path knows. `static`, not `const`, so a matched
/// row borrows for `'static` rather than out of a temporary.
static HOSTS: [RawHost; 2] = [
    RawHost {
        host: "github.com",
        default_base: "https://raw.githubusercontent.com",
        request_url: github_request_url,
        // `raw.githubusercontent.com` serves the file itself — the body
        // is already the bytes.
        read_body: |body| Some(body.to_vec()),
        is_miss: |status, _body| status == 404,
    },
    RawHost {
        host: "gitverse.ru",
        default_base: "https://gitverse.ru",
        request_url: gitverse_request_url,
        read_body: gitverse_read_body,
        is_miss: gitverse_is_miss,
    },
];

/// A repository URL the fast path understands, split into what a read
/// needs.
struct RepoCoords {
    owner: String,
    repo: String,
    token: Option<String>,
    /// The caller's URL with any userinfo removed — the only form that
    /// may appear in a log line or an error, since userinfo is where the
    /// token rides.
    plain_url: String,
}

/// One addressed read: everything decided before a socket is opened.
struct AddressedRead {
    /// Where the request goes. Credential-free by construction.
    url: String,
    /// Bearer token for the request header, when the repository URL
    /// carried one.
    token: Option<String>,
    /// How this host's answers are to be read.
    host: &'static RawHost,
    /// The repository URL to name in an error.
    plain_url: String,
}

/// Read `path` at `refname` from `url` over HTTPS, if this is a host and
/// a shape the fast path knows.
///
/// Three outcomes, and the third is the important one:
///
/// - `Some(Ok(bytes))` — the file, read in one round-trip. git was never
///   spawned, and does not even have to be installed.
/// - `Some(Err(FileNotFoundInRef))` — the host said "no such file" and
///   [`absence_is_authoritative`] agreed we may say so. This is the
///   `vibe-redirect.toml` probe's ordinary answer: almost no package
///   carries a redirect marker, and before this path every one of those
///   probes cost a refused archive plus a clone.
/// - `None` — *ask git*. Not this host, not a shape we can address, an
///   answer we cannot interpret, an unexpected status, a timeout, a
///   transport error. The caller continues into the unchanged
///   archive → clone ladder and its diagnosis stands as it always did.
///
/// `client` is the backend's one client, so a walk over a whole
/// dependency graph reuses connections instead of shaking hands anew for
/// every file (see [`build_client`]). `base_override` replaces the
/// matched host's production base and `sleeper` is where the pauses
/// between attempts go; only a test passes either (see
/// [`super::ShellGit::with_raw_base`] and
/// [`super::ShellGit::with_raw_sleeper`]).
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#RAW-READ-FAST-PATH"
)]
pub(super) fn try_read(
    client: &reqwest::blocking::Client,
    base_override: Option<&str>,
    sleeper: &super::Sleeper,
    url: &str,
    refname: &str,
    path: &str,
) -> Option<Result<Vec<u8>, GitError>> {
    let read = address(base_override, url, refname, path)?;

    // Ask, and — for the two answers that are about the moment rather
    // than about the file — ask again. Both bounds are the point: after
    // [`retry::MAX_ATTEMPTS`] asks, or once the pause budget is gone,
    // this is the ladder's problem again and nothing below has changed.
    let mut waited = Duration::ZERO;
    let mut refusal = None;
    for attempt in 1..=retry::MAX_ATTEMPTS {
        let asked = match attempt_read(client, &read, refname, path) {
            Attempt::Settled(outcome) => return outcome,
            Attempt::Again { status, after } => {
                refusal = status;
                after
            }
        };
        let Some(pause) = retry::pause_before_next(attempt, asked, waited) else {
            break;
        };
        tracing::debug!(
            target: "vibe_registry::git",
            url = %read.url,
            status = refusal,
            attempt,
            wait_ms = pause.as_millis() as u64,
            "https read was refused for now; waiting and asking again"
        );
        sleeper.sleep(pause);
        waited += pause;
    }
    match refusal {
        Some(status) => tracing::debug!(
            target: "vibe_registry::git",
            url = %read.url,
            status,
            "https read did not answer with the file; falling back to git"
        ),
        None => tracing::debug!(
            target: "vibe_registry::git",
            url = %read.url,
            "https read never reached the host; falling back to git"
        ),
    }
    None
}

/// What one request settled, from the point of view of the read above.
enum Attempt {
    /// Nothing is gained by asking again: the file itself, a miss that
    /// may be reported, or a decision to ask git. The three outcomes of
    /// [`try_read`], reached on the spot.
    Settled(Option<Result<Vec<u8>, GitError>>),
    /// The host refused *now*, not the file. Carries the status that
    /// said so — `None` when no answer arrived at all — and whatever the
    /// host's own `Retry-After` asked for.
    Again {
        status: Option<u16>,
        after: Option<Duration>,
    },
}

/// One request, and the reading of its answer.
///
/// Every `Settled(None)` here logs its own reason: this is where the
/// read decides it cannot explain what it got, and the line it prints is
/// the one an operator reads when a package took the long way round.
fn attempt_read(
    client: &reqwest::blocking::Client,
    read: &AddressedRead,
    refname: &str,
    path: &str,
) -> Attempt {
    let mut request = client.get(&read.url);
    if let Some(token) = read.token.as_deref() {
        // Both hosts accept, for a private repository, the same token the
        // credentialed git URL carries in its userinfo. It rides the
        // header — never the request URL, never a log line, never the
        // text of an error.
        request = request.bearer_auth(token);
    }
    tracing::debug!(
        target: "vibe_registry::git",
        url = %read.url,
        "reading file over https (no git)"
    );
    let response = match request.send() {
        Ok(r) => r,
        Err(e) => return transport_failure(&read.url, &e, "https read failed"),
    };
    let status = response.status().as_u16();
    // Before the body: reading it consumes the response, headers and all.
    let after = retry::retry_after(response.headers());
    let body = match response.bytes() {
        Ok(b) => b,
        Err(e) => return transport_failure(&read.url, &e, "https read body failed"),
    };

    if (200..300).contains(&status) {
        if let Some(bytes) = (read.host.read_body)(&body) {
            return Attempt::Settled(Some(Ok(bytes)));
        }
        tracing::debug!(
            target: "vibe_registry::git",
            url = %read.url,
            "https read answered a body shape this reader does not know; falling back to git"
        );
        return Attempt::Settled(None);
    }
    if (read.host.is_miss)(status, &body) && absence_is_authoritative(refname, path) {
        return Attempt::Settled(Some(Err(GitError::FileNotFoundInRef {
            url: read.plain_url.clone(),
            refname: refname.to_string(),
            path: path.to_string(),
        })));
    }
    if retry::refused_for_now(status) {
        return Attempt::Again {
            status: Some(status),
            after,
        };
    }
    tracing::debug!(
        target: "vibe_registry::git",
        url = %read.url,
        status,
        "https read did not answer with the file; falling back to git"
    );
    Attempt::Settled(None)
}

/// A request that produced no answer: retryable unless the failure
/// already carries its verdict (see [`retry::worth_another_try`]).
///
/// The log line is deliberately printed only for the settling case. A
/// failure that is about to be retried is announced by the retry line
/// instead, which says what is waited and for how long; one that is not
/// retried says why here, exactly as it always did.
fn transport_failure(url: &str, error: &reqwest::Error, what: &str) -> Attempt {
    if retry::worth_another_try(error) {
        return Attempt::Again {
            status: None,
            after: None,
        };
    }
    tracing::debug!(
        target: "vibe_registry::git",
        url = %url,
        error = %error,
        "{what}; falling back to git"
    );
    Attempt::Settled(None)
}

/// The addressing half of [`try_read`]: which URL this read goes to,
/// which host reads the answer, and what token the request carries.
///
/// Split out from the request so it can be asserted directly. A mapping
/// mistake is the silent kind of mistake — a wrong URL simply misses, and
/// a miss looks exactly like an absent file.
fn address(
    base_override: Option<&str>,
    url: &str,
    refname: &str,
    path: &str,
) -> Option<AddressedRead> {
    let (host, repo) = RepoCoords::parse(url)?;
    // Both the ref and the path land inside a URL. Map only shapes that
    // cannot change that URL's meaning; anything else is git's.
    if !is_plain(refname) || !is_plain(path) {
        return None;
    }
    let base = base_override.unwrap_or(host.default_base);
    let request_url = (host.request_url)(base, &repo, refname, path)?;
    Some(AddressedRead {
        url: request_url,
        token: repo.token,
        host,
        plain_url: repo.plain_url,
    })
}

/// May a host's "nothing here" be reported as
/// [`GitError::FileNotFoundInRef`], or must the read fall back to git?
///
/// Both hosts answer one code for several different facts — GitHub a 404,
/// GitVerse a `4305` — and among those facts are "the ref does not
/// exist", "the repository does not exist" and "you may not see this
/// repository" (a private repository is hidden, not refused). Only git
/// distinguishes them, so the answer may be taken at face value only
/// where both of these hold:
///
/// - **It is not a manifest read.** `vibe.toml` is the read whose failure
///   an operator must see diagnosed exactly — [`GitError::RefNotFound`],
///   [`GitError::RepoNotFound`], [`GitError::AuthFailed`] — so its miss
///   always goes down the git path and keeps that diagnosis honest. The
///   marker probe (`vibe-redirect.toml`) is the opposite case: absence is
///   its ordinary answer, for almost every package, and that is the miss
///   worth serving without spawning git at all.
/// - **The ref names fixed content.** A `v<semver>` tag or a full commit
///   SHA does, so "not in that ref" stays true. A branch does not:
///   `[requires.packages]` git sources and `vibe install --git … --branch
///   <b>` read a moving tip, and a mistyped or since-deleted branch
///   misses for reasons that have nothing to do with the file. Those go
///   to git, which says which.
fn absence_is_authoritative(refname: &str, path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    basename != Manifest::FILENAME && names_fixed_content(refname)
}

/// Does `refname` name content that cannot change under us — a release
/// tag (`v<semver>`, the shape the registry publishes and
/// `list_candidates` reads back) or a full commit SHA?
fn names_fixed_content(refname: &str) -> bool {
    let is_version_tag = refname
        .strip_prefix('v')
        .is_some_and(|rest| semver::Version::parse(rest).is_ok());
    let is_commit_sha = refname.len() == 40 && refname.bytes().all(|b| b.is_ascii_hexdigit());
    is_version_tag || is_commit_sha
}

/// The one client every read of this backend goes through.
///
/// A resolve walk reads one or two files per dependency — some hundreds
/// of reads for a 26-package graph, nearly all from the same host. Built
/// per read, the client threw its connection away each time and paid a
/// fresh TLS handshake for the next; held, its pool keeps the connection
/// alive, which is most of what a read costs. Measured on the redbook
/// graph, 2026-09-14: every raw read of the walk now reuses a pooled
/// connection, and the resolution phase fell from 116 s to 90 s.
///
/// `index_client` still builds a client per call, and on a registry whose
/// index is a static mirror it is querying the same host this path reads
/// from — so it is now where nearly all the remaining handshakes come
/// from. That is its own file's call to make, not this one's.
///
/// `None` when the client cannot be built at all — a TLS backend that
/// will not initialise, which is a property of the build and the machine
/// rather than of this request, so the caller caches the answer and every
/// read goes to git.
pub(super) fn build_client() -> Option<reqwest::blocking::Client> {
    match reqwest::blocking::Client::builder()
        .user_agent(concat!("vibe-registry/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(READ_TIMEOUT_SECS))
        .build()
    {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::debug!(
                target: "vibe_registry::git",
                error = %e,
                "could not build the https read client; every read falls back to git"
            );
            None
        }
    }
}

// ---- github.com ----

/// `<base>/<owner>/<repo>/<refname>/<path>` — the raw host takes the ref
/// and the in-repo path literally, slashes and all, so this composes the
/// string rather than pushing URL segments (which would escape the
/// separators inside a `feature/x` branch name). [`is_plain`] has already
/// vouched for both.
///
/// The ref slot is the same one-name-for-any-kind slot git offers, and
/// like git's it is ambiguous in a repository that carries a branch and a
/// tag of the same name — the two may not break the tie identically.
/// Accepted rather than worked around: the registry publishes `v<semver>`
/// tags, nothing in vibevm's world names a branch that way, and the cost
/// of the alternative (an extra ref-resolution round-trip on every read)
/// is the very cost this path exists to remove.
fn github_request_url(base: &str, repo: &RepoCoords, refname: &str, path: &str) -> Option<String> {
    Some(format!(
        "{}/{}/{}/{}/{}",
        base.trim_end_matches('/'),
        repo.owner,
        repo.repo,
        refname,
        path
    ))
}

// ---- gitverse.ru ----

/// `<base>/api/repos/<owner>/<repo>/contents/<path>?ref=<refname>` —
/// verified against the live host on 2026-09-14 with a branch, a tag and
/// a commit SHA, and with a nested path. Composed through
/// [`reqwest::Url`] so segments and the query value are escaped for us,
/// the same way `IndexClient::lookup_purl` escapes a PURL. This host
/// needs no `Accept` header (unlike `api.gitverse.ru`).
fn gitverse_request_url(
    base: &str,
    repo: &RepoCoords,
    refname: &str,
    path: &str,
) -> Option<String> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/repos/", base.trim_end_matches('/'))).ok()?;
    {
        let mut segments = url.path_segments_mut().ok()?;
        segments.pop_if_empty();
        segments.push(&repo.owner).push(&repo.repo).push("contents");
        for segment in path.split('/') {
            segments.push(segment);
        }
    }
    url.query_pairs_mut().append_pair("ref", refname);
    Some(url.to_string())
}

/// GitVerse's `contents` answer for one file.
#[derive(Deserialize)]
struct GitverseContents {
    encoding: String,
    content: String,
}

/// GitVerse's error answer, which carries the real reason in `code`.
#[derive(Deserialize)]
struct GitverseError {
    code: i64,
}

/// A 200 from the `contents` route is JSON, not the file: base64 in
/// `content` under `encoding: "base64"`. Anything else — a different
/// encoding, or the JSON *array* the route answers for a directory —
/// returns `None`, and the read falls back to git.
fn gitverse_read_body(body: &[u8]) -> Option<Vec<u8>> {
    let parsed: GitverseContents = serde_json::from_slice(body).ok()?;
    if parsed.encoding != "base64" {
        return None;
    }
    decode_base64(&parsed.content).ok()
}

/// GitVerse reports a miss as HTTP 400 with [`GITVERSE_FILE_NOT_FOUND`]
/// in the body. A 400 without that code is some other complaint and
/// belongs to git; so do `4004` ("doesn't have repository"), 401/403, and
/// everything in the 5xx range.
fn gitverse_is_miss(status: u16, body: &[u8]) -> bool {
    status == 400
        && serde_json::from_slice::<GitverseError>(body)
            .is_ok_and(|e| e.code == GITVERSE_FILE_NOT_FOUND)
}

/// Standard base64, whitespace-tolerant (the `contents` route wraps its
/// payload in lines). Hand-rolled for the same reason
/// `search::full_scan::decode_base64` is — a single call site does not
/// earn a dependency. Kept private here rather than reached for across
/// the `search` seam; a third caller is the moment to give the two a
/// shared home.
fn decode_base64(input: &str) -> Result<Vec<u8>, &'static str> {
    fn sextet(c: u8) -> Result<u32, &'static str> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("invalid character"),
        }
    }
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !cleaned.len().is_multiple_of(4) {
        return Err("length not a multiple of 4");
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for quad in cleaned.chunks_exact(4) {
        let pad_third = quad[2] == b'=';
        let pad_fourth = quad[3] == b'=';
        let packed = (sextet(quad[0])? << 18)
            | (sextet(quad[1])? << 12)
            | (if pad_third { 0 } else { sextet(quad[2])? } << 6)
            | (if pad_fourth { 0 } else { sextet(quad[3])? });
        out.push((packed >> 16) as u8);
        if !pad_third {
            out.push(((packed >> 8) & 0xff) as u8);
        }
        if !pad_fourth {
            out.push((packed & 0xff) as u8);
        }
    }
    Ok(out)
}

// ---- URL shapes ----

impl RepoCoords {
    /// Split `https://[userinfo@]<host>/<owner>/<repo>[.git][/]` into the
    /// coordinates a read needs, and name the host row that serves it.
    ///
    /// `None` — so, git — for everything else: an `ssh://` or
    /// `git@host:owner/repo.git` URL (there is no HTTPS read to make), an
    /// `http://` one (a token must never cross plaintext, the rule
    /// `inject_token` already keeps), a host outside [`HOSTS`], a path
    /// that is not exactly one owner and one repository, and any URL
    /// carrying a query or a fragment.
    fn parse(url: &str) -> Option<(&'static RawHost, RepoCoords)> {
        let rest = url.strip_prefix("https://")?;
        let (authority, path) = rest.split_once('/')?;
        // Userinfo is everything before the last `@`; the credentialed
        // form `inject_token` / `credentialed_url` produce is
        // `x-access-token:<TOKEN>@<host>`.
        let (userinfo, host) = match authority.rsplit_once('@') {
            Some((user, host)) => (Some(user), host),
            None => (None, authority),
        };
        let entry = HOSTS.iter().find(|h| host.eq_ignore_ascii_case(h.host))?;
        if path.contains('?') || path.contains('#') {
            return None;
        }
        let mut segments = path.trim_end_matches('/').split('/');
        let owner = segments.next().filter(|s| !s.is_empty())?;
        let repo = segments.next().filter(|s| !s.is_empty())?;
        if segments.next().is_some() {
            return None;
        }
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        if repo.is_empty() {
            return None;
        }
        Some((
            entry,
            RepoCoords {
                owner: owner.to_string(),
                repo: repo.to_string(),
                token: userinfo.and_then(token_from_userinfo),
                plain_url: format!("https://{host}/{path}"),
            },
        ))
    }
}

/// The token inside a URL's userinfo: the secret half of
/// `x-access-token:<TOKEN>`, or the whole of a bare `<token>@host` form.
/// Taken verbatim, exactly as git receives it.
fn token_from_userinfo(userinfo: &str) -> Option<String> {
    let candidate = match userinfo.split_once(':') {
        Some((_user, secret)) => secret,
        None => userinfo,
    };
    (!candidate.is_empty()).then(|| candidate.to_string())
}

/// Is this ref or in-repo path safe to place inside a URL verbatim?
///
/// A deliberately narrow allow-list, applied to both: release tags,
/// branch names (slashes included) and commit SHAs all pass, as do the
/// manifest and marker filenames. Anything that could steer a URL
/// elsewhere does not — and falls back to git rather than being escaped
/// into something subtly different from what git would have read. `.` and
/// `..` segments are rejected outright: a read must not be able to climb
/// out of the repository it names.
fn is_plain(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.ends_with('/')
        && s.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'~')
                })
        })
}

/// When a refusal is worth asking again, and how long the read waits
/// before it does — every bound of the backoff, and nothing that makes a
/// request.
mod retry;

#[cfg(test)]
#[path = "raw_http/tests.rs"]
mod tests;
