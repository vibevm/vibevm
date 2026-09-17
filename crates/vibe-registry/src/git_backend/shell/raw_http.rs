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

use vibe_core::manifest::Manifest;

use super::GitError;
use hosts::{AddressedRead, address};

/// Timeout for one read, matching `index_client`'s fetch timeout. A read
/// that has not answered within ten seconds is no longer a fast path; the
/// caller drops to git rather than holding the whole resolve walk on it.
const READ_TIMEOUT_SECS: u64 = 10;

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
/// `index_client` applies the same invocation-local connection-pool rule to
/// its file and server bases. The pools stay separate because their auth and
/// timeout decisions are separate, while repeated reads within each base do
/// not repay TLS setup.
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

/// The two hosts this path knows, how a read is addressed on each, and
/// how each one's answer is read — everything host-specific, and nothing
/// that decides what an answer means.
mod hosts;

/// When a refusal is worth asking again, and how long the read waits
/// before it does — every bound of the backoff, and nothing that makes a
/// request.
mod retry;

#[cfg(test)]
#[path = "raw_http/tests.rs"]
mod tests;
