//! The client half of the eternal handshake (PROP-044
//! `##ONE-ETERNAL-FILE`): fetch `hello.json` at one probe candidate,
//! parse it through the generated type — the only lawful parser, a
//! hand-written wire is forbidden by `##FORBID-HANDWRITTEN-WIRE` —
//! and interpret it against the epoch THIS build reads.
//!
//! The epoch is never a constant here: it is
//! `FormatId::IndexRepomd.epoch()` from the generated registry, the
//! same source the writer stamped the world with (Р45 — a second
//! copy of a normative value is forbidden). Every way a handshake
//! can exist but not serve this build — an unknown handshake-format
//! string, an unparseable body, no world of this build's epoch — is
//! a LOUD refusal carrying the facts and the fix, never a silent
//! fall-through to `repomd.json`: silence on "the index is there but
//! I cannot read it" would hide a broken or newer index behind
//! "compatibility" (`##FORBID-SILENCE`, `##LAW-NO-LYING`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#truth");

use super::auth::{IndexAuth, refusal_reason};
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::hello::e1::hello::Handshake;

/// What probing `<candidate>/hello.json` decided. The orchestration
/// in [`super::IndexClient::probe`] maps this onto
/// [`super::ProbeOutcome`]: `Found` wins immediately, `Refused` wins
/// immediately (the index is there — its handshake says so), only
/// `Absent` falls through to the next candidate and, ultimately, to
/// the `repomd.json` probe of pre-handshake indexes.
pub(super) enum HandshakeProbe {
    /// No handshake at this candidate (404, 5xx, connect-fail): try
    /// the next candidate, then today's `repomd.json` path.
    Absent,
    /// The handshake is here and serves a world this build reads.
    /// `file_base` is the candidate refined by that world's `path`.
    Found { file_base: String },
    /// The handshake is here but this build cannot use it. The
    /// reason carries the offered epochs, this build's epoch, a
    /// recipe, and `successor` / `min_client` / `notice` when the
    /// document has them.
    Refused { reason: String },
}

/// Probe `<candidate>/hello.json` with the shared probe client. A
/// 401/403 is the private-index refusal exactly as on the `repomd`
/// path (A2-INDEXAUTH), reusing the regime-aware guidance text.
pub(super) fn probe_candidate(
    client: &reqwest::blocking::Client,
    candidate: &str,
    auth: &IndexAuth,
) -> HandshakeProbe {
    let url = format!("{candidate}/hello.json");
    let resp = match client.get(&url).send() {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) => {
            let status = resp.status().as_u16();
            if matches!(status, 401 | 403) {
                let reason = refusal_reason(&url, status, auth);
                tracing::warn!(
                    target: "vibe_registry::index_client",
                    "handshake probe at `{url}` refused: {reason}"
                );
                return HandshakeProbe::Refused { reason };
            }
            // 404 / 5xx / other non-success — this candidate has no
            // handshake; the next candidate, then the repomd path.
            tracing::debug!(
                target: "vibe_registry::index_client",
                "handshake probe at `{url}` non-success ({status}); trying next candidate"
            );
            return HandshakeProbe::Absent;
        }
        Err(e) => {
            tracing::debug!(
                target: "vibe_registry::index_client",
                "handshake probe at `{url}` errored: {e}"
            );
            return HandshakeProbe::Absent;
        }
    };
    let body = match resp.bytes() {
        Ok(body) => body,
        // The request succeeded but the body did not arrive — a
        // transport failure, not a statement by the index; fall
        // through like any connect-fail.
        Err(e) => {
            tracing::debug!(
                target: "vibe_registry::index_client",
                "handshake body from `{url}` errored: {e}"
            );
            return HandshakeProbe::Absent;
        }
    };
    let handshake: Handshake = match serde_json::from_slice(&body) {
        Ok(h) => h,
        Err(e) => {
            let reason = broken_body_reason(&url, &e);
            tracing::warn!(
                target: "vibe_registry::index_client",
                "handshake at `{url}` does not parse: {reason}"
            );
            return HandshakeProbe::Refused { reason };
        }
    };
    interpret(&url, candidate, &handshake)
}

/// Match a parsed handshake against what this build reads.
fn interpret(url: &str, candidate: &str, handshake: &Handshake) -> HandshakeProbe {
    let expected = expected_vibe();
    if handshake.vibe != expected {
        let reason = unknown_vibe_reason(url, &handshake.vibe, &expected);
        tracing::warn!(
            target: "vibe_registry::index_client",
            "handshake at `{url}` is a format this build does not read: {reason}"
        );
        return HandshakeProbe::Refused { reason };
    }
    let own = own_world_epoch();
    match handshake.worlds.iter().find(|w| w.epoch == own) {
        Some(world) => HandshakeProbe::Found {
            file_base: refine_file_base(candidate, &world.path),
        },
        None => {
            let reason = epoch_refusal(url, handshake, own);
            tracing::warn!(
                target: "vibe_registry::index_client",
                "handshake at `{url}` serves no world this build reads: {reason}"
            );
            HandshakeProbe::Refused { reason }
        }
    }
}

/// The epoch of the catalog world this build reads — from the
/// generated registry, never a client-side constant (Р45).
fn own_world_epoch() -> u32 {
    FormatId::IndexRepomd.epoch()
}

/// The handshake-format string this build understands, spelled by
/// the generated registry the same way the writer spells it.
fn expected_vibe() -> String {
    format!("hello/{}", FormatId::Handshake.epoch())
}

/// Refine the winning candidate's base by the world's `path`.
/// `"."` — the degenerate single-world form — keeps the candidate
/// untouched: gluing a `/.` onto a URL is a real defect class of its
/// own. A path of only slashes (empty included) is another spelling
/// of "here" and is treated the same way.
pub(super) fn refine_file_base(candidate: &str, world_path: &str) -> String {
    if world_path == "." {
        return candidate.to_string();
    }
    let path = world_path.trim_matches('/');
    if path.is_empty() {
        return candidate.to_string();
    }
    format!("{candidate}/{path}")
}

/// The index offers no world this build reads: name what it offers,
/// what this build reads, and the way out. `successor` is NAMED,
/// never followed (Р42: auto-following needs a cycle watchman and a
/// trust rule, neither decided; the named address makes the human's
/// next move one command).
fn epoch_refusal(url: &str, handshake: &Handshake, own: u32) -> String {
    let offered = if handshake.worlds.is_empty() {
        "none".to_string()
    } else {
        handshake
            .worlds
            .iter()
            .map(|w| w.epoch.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut reason = format!(
        "index at `{url}` offers worlds of epochs [{offered}], but this build reads \
         epoch {own} — update vibe to a build that reads one of those epochs, or point \
         this registry at an index serving a world of epoch {own}"
    );
    if let Some(min_client) = &handshake.min_client {
        reason.push_str(&format!(
            "; the index names {min_client} as the oldest client able to read it"
        ));
    }
    if let Some(successor) = &handshake.successor {
        reason.push_str(&format!(
            "; the handshake has moved to {successor} — not followed automatically, \
             set the registry's index URL there by hand to follow the move"
        ));
    }
    if let Some(notice) = &handshake.notice {
        reason.push_str(&format!("; index notice: {notice}"));
    }
    reason
}

/// A handshake-format string this build does not read: the document
/// itself is newer than the client (D8 — a client that understands
/// no value of `vibe` refuses loudly).
fn unknown_vibe_reason(url: &str, got: &str, expected: &str) -> String {
    format!(
        "index at `{url}` answers with a handshake of format `{got}`, but this build \
         reads handshake format `{expected}` — update vibe"
    )
}

/// HTTP 200 with a body that does not parse as a handshake: the
/// index is there and broken. Falling back to `repomd.json` here
/// would hide a broken index behind "compatibility" — the exact
/// silence `##FORBID-SILENCE` forbids.
fn broken_body_reason(url: &str, error: &serde_json::Error) -> String {
    format!(
        "index at `{url}` answered the handshake with HTTP 200, but the body does not \
         parse as a handshake ({error}) — the index is there and broken; check the \
         index publication at that URL"
    )
}
