//! The hosts the fast path knows, and how a read is addressed on each.
//!
//! **Why this is its own file.** The read next door is host-blind on
//! purpose: it asks, and then reads the answer through whichever row
//! matched. Everything host-specific — which host is recognised at all,
//! where its single-file endpoint lives, how a URL is composed for it,
//! what shape a `2xx` body arrives in, and which other answer means
//! "nothing here" — is stated here, one row per host, so that a third
//! host is a third row and nothing in the read itself changes.
//!
//! Nothing here opens a socket, and nothing here decides what a miss may
//! be concluded from ([`super::absence_is_authoritative`] owns that).
//! Addressing is settled before a request is made, and it is worth
//! asserting on its own: a mapping mistake is the silent kind of
//! mistake — a wrong URL simply misses, and a miss looks exactly like an
//! absent file.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf");

use serde::Deserialize;

/// GitVerse's "no such file" code, delivered inside an HTTP 400 body as
/// `{"code":4305,"message":"File: <path> not found"}`. Verified against
/// the live host on 2026-09-14; the same code comes back for an unknown
/// ref, which is why [`super::absence_is_authoritative`] gates what may
/// be concluded from it.
const GITVERSE_FILE_NOT_FOUND: i64 = 4305;

/// One host the fast path knows how to read a single file from.
///
/// A table rather than a branch: the two hosts differ only in how a read
/// is addressed and how an answer is shaped, so each row states those
/// three facts and the read itself ([`super::try_read`]) stays
/// host-blind. A third host is a third row.
pub(super) struct RawHost {
    /// The host a repository URL must carry, compared case-insensitively.
    pub(super) host: &'static str,
    /// Where this host serves single files in production.
    default_base: &'static str,
    /// Address one file: `(base, repo, refname, path)` → read URL.
    /// `None` when this host cannot address that combination.
    request_url: fn(&str, &RepoCoords, &str, &str) -> Option<String>,
    /// Turn a 2xx body into the file's bytes, or `None` when the body is
    /// not a shape this reader understands — which means falling back to
    /// git, never an error.
    pub(super) read_body: fn(&[u8]) -> Option<Vec<u8>>,
    /// Does this non-2xx answer mean "nothing here"? Takes the body as
    /// well as the status, because one of the two hosts carries the
    /// distinction in the body rather than the status line.
    pub(super) is_miss: fn(u16, &[u8]) -> bool,
}

/// The hosts the fast path knows. `static`, not `const`, so a matched
/// row borrows for `'static` rather than out of a temporary.
pub(super) static HOSTS: [RawHost; 2] = [
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
pub(super) struct AddressedRead {
    /// Where the request goes. Credential-free by construction.
    pub(super) url: String,
    /// Bearer token for the request header, when the repository URL
    /// carried one.
    pub(super) token: Option<String>,
    /// How this host's answers are to be read.
    pub(super) host: &'static RawHost,
    /// The repository URL to name in an error.
    pub(super) plain_url: String,
}

/// The addressing half of [`super::try_read`]: which URL this read goes
/// to, which host reads the answer, and what token the request carries.
///
/// Split out from the request so it can be asserted directly. A mapping
/// mistake is the silent kind of mistake — a wrong URL simply misses, and
/// a miss looks exactly like an absent file.
pub(super) fn address(
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
pub(super) fn decode_base64(input: &str) -> Result<Vec<u8>, &'static str> {
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
