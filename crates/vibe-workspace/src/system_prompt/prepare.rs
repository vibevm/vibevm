//! `prepare` — the calling program's preflight: bands P, U and G as
//! separate, ordered materials with their own observed digests.
//!
//! **It returns materials and observations, never a ready-to-send request.**
//! It assembles no credentials, chooses no model and constructs no provider
//! envelope; its grammar is settled with its first real consumer rather than
//! shipped as flags that parse and are ignored. And it is the *calling
//! program's* preflight, run outside the model's turn: PROP-009
//! `#PURE-FILE-READING` and `#REJ-RUN-VIBE` forbid turning boot into a
//! command, so the transport contract never asks an agent to run this
//! (`#SYS-OPERATIONS-SURFACE`, `#SYS-TRANSPORT-CONTRACT`).
//!
//! **Why the bands are separate and never concatenated.** Band U's bytes are
//! the current bytes of files whose identity and order the boot plan fixes,
//! so they change only when those files change — exactly as band P's do. A
//! provider that supports several cache breakpoints can therefore place one
//! at the end of band P and another at the end of band U, and a task change
//! invalidates neither. Handing back one concatenated string would destroy
//! the only property that makes keeping INDEX late-bound free
//! (`#SYS-CONTINUATION-CACHEABLE`).
//!
//! **Read-only, like `check`, and for the same reasons.** It reads the
//! prefix and the applicable continuation rows and nothing else — no home,
//! plan, custody or token path, and no effectful provider. A stale artifact
//! is **refused** with the exact `vibe prompt build` that fixes it rather
//! than rebuilt, because rebuilding would be a write and this command
//! promises none (`#SYS-READ-ONLY-CHECK`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE");

use std::path::Path;

use vibe_core::manifest::TargetOs;
use vibe_wire::generated::system_prompt_meta as wire;

use crate::WorkspaceError;

use super::check::{CheckRequest, PromptCheck};
use super::markers::sha256_hex;

/// One band member: where the bytes came from and what was observed.
///
/// The digest is request-local by design. No continuation row's content is
/// witnessed in the sidecar, so these observations are the only place a
/// row's bytes are ever digested — which is what lets a band-U edit cost
/// nothing (`#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedBand {
    /// The file's path as the plan spells it — workspace-root-relative for a
    /// continuation row, node-relative for the prefix.
    pub path: String,
    /// SHA-256 of exactly the bytes observed for this request.
    pub sha256: String,
    /// The observed byte length.
    pub len: usize,
}

/// A row this request could not resolve, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRow {
    /// The row's path.
    pub path: String,
    /// Its condition, verbatim.
    pub when: String,
    /// Why it stayed unresolved.
    pub reason: String,
}

/// The materials of one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    /// Band P — the prefix bytes, as one member.
    pub prefix: PreparedBand,
    /// Band U — every unconditional continuation row, in plan order.
    pub unconditional: Vec<PreparedBand>,
    /// Band G — the gated rows this session's target selects, in plan order.
    pub gated: Vec<PreparedBand>,
    /// The rows no supplied target could decide.
    pub unresolved: Vec<UnresolvedRow>,
    /// Recommended cache-breakpoint offsets, counted from the first byte of
    /// band P: the sidecar's own segment boundaries, then the end of band U.
    pub cache_boundaries: Vec<usize>,
}

/// Assemble one request's materials.
///
/// `session_target` is the **explicitly supplied** session operating system.
/// A non-empty band G with none supplied leaves every gated row unresolved
/// and never falls back to the host: an unknown selector must not become
/// `true` (`#SYS-SESSION-TARGET`).
pub fn prepare_request(
    request: &CheckRequest<'_>,
    answer: &PromptCheck,
    session_target: Option<TargetOs>,
) -> Result<PreparedRequest, WorkspaceError> {
    if !answer.opted_in {
        return Err(refusal(
            "the node has no authored `SYSTEM-TEMPLATE.md`, so it has not opted in and there \
             is no prefix to prepare"
                .to_owned(),
        ));
    }
    if !answer.verdict.is_equal() {
        return Err(refusal(format!(
            "the published prefix is `{}` ({}) — run `{}` and prepare again; `prepare` writes \
             nothing, so it refuses rather than rebuilding",
            answer.verdict.as_str(),
            answer.verdict.reasons().join("; "),
            answer.remedy()
        )));
    }
    let prefix_bytes = read(&answer.system.path)?;
    let meta: wire::SystemPromptMeta =
        serde_json::from_slice(&read(&answer.meta.path)?).map_err(|error| {
            refusal(format!(
                "the sidecar beside the prefix does not parse: {error}"
            ))
        })?;

    let mut prepared = PreparedRequest {
        prefix: PreparedBand {
            path: vibe_core::layout::SYSTEM_MD.to_owned(),
            sha256: sha256_hex(&prefix_bytes),
            len: prefix_bytes.len(),
        },
        unconditional: Vec::new(),
        gated: Vec::new(),
        unresolved: Vec::new(),
        cache_boundaries: boundaries(&meta),
    };
    for row in &meta.continuation {
        match (&row.band, &row.when) {
            (wire::Band::Unconditional, _) => {
                prepared
                    .unconditional
                    .push(observe(request.workspace_root, &row.path)?);
            }
            (wire::Band::Gated, when) => {
                let when = when.clone().unwrap_or_default();
                match gate(&when, session_target) {
                    Gate::Selected => prepared
                        .gated
                        .push(observe(request.workspace_root, &row.path)?),
                    Gate::Dropped => {}
                    Gate::Unresolved(reason) => prepared.unresolved.push(UnresolvedRow {
                        path: row.path.clone(),
                        when,
                        reason,
                    }),
                }
            }
        }
    }
    // The end of band U is the second breakpoint, and it is computed from
    // what was actually observed rather than taken from the sidecar: band U
    // is late-bound, so its length is a property of this request.
    let band_u: usize = prepared.unconditional.iter().map(|band| band.len).sum();
    if band_u > 0 {
        prepared.cache_boundaries.push(prepared.prefix.len + band_u);
    }
    Ok(prepared)
}

/// What a session target says about one gated row.
enum Gate {
    /// The row belongs to this request.
    Selected,
    /// The row's condition is decided and false.
    Dropped,
    /// Nothing supplied can decide it.
    Unresolved(String),
}

/// Decide one gated row. The grammar that can reach band G is closed to
/// `os:` — `installed:` is resolved at generation and `target:` is a project
/// property — so an unrecognised condition is reported, never assumed.
fn gate(when: &str, session_target: Option<TargetOs>) -> Gate {
    let Some(name) = when.strip_prefix("os:") else {
        return Gate::Unresolved(format!(
            "`{when}` is not an `os:` gate, and only an `os:` gate can reach the gated band"
        ));
    };
    let Some(target) = session_target else {
        return Gate::Unresolved(
            "no session target was supplied, and the generator's own operating system is never \
             used to decide a session gate — pass `--session-target os=<name>`"
                .to_owned(),
        );
    };
    if name == target.as_str() {
        Gate::Selected
    } else {
        Gate::Dropped
    }
}

/// The sidecar's own recommended breakpoints, as offsets inside band P.
fn boundaries(meta: &wire::SystemPromptMeta) -> Vec<usize> {
    meta.segments
        .iter()
        .filter(|segment| segment.cache_boundary)
        .map(|segment| segment.offset as usize + segment.len as usize)
        .collect()
}

/// Observe one continuation row's current bytes.
fn observe(workspace_root: &Path, relative: &str) -> Result<PreparedBand, WorkspaceError> {
    let bytes = read(&workspace_root.join(relative))?;
    Ok(PreparedBand {
        sha256: sha256_hex(&bytes),
        len: bytes.len(),
        path: relative.to_owned(),
    })
}

fn read(path: &Path) -> Result<Vec<u8>, WorkspaceError> {
    crate::safe_file::preflight_absent_or_regular(path)
        .and_then(|()| crate::safe_file::read_optional(path))
        .map_err(|error| WorkspaceError::SystemPromptCapture {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?
        .ok_or_else(|| WorkspaceError::SystemPromptCapture {
            path: path.to_path_buf(),
            reason: "the boot plan names this file and it is absent, so the band cannot be \
                     prepared"
                .to_owned(),
        })
}

fn refusal(reason: String) -> WorkspaceError {
    WorkspaceError::SystemPromptCompose {
        what: "prepare".to_owned(),
        reason,
    }
}

#[cfg(test)]
#[path = "prepare/tests.rs"]
mod tests;
