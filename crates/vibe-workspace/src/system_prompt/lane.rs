//! The agent lane — the second composed artifact, and the one a harness's
//! own instruction slot takes (`#SYS-AGENT-LANE`).
//!
//! `STATIC.*` followed by every unconditional INDEX file in INDEX order,
//! each wrapped in the marker pair of [`super::markers`]. **No template, no
//! entrypoint, no router block and no import line of its own.**
//!
//! It is not `SYSTEM.md` minus a segment, and the difference is the whole
//! reason there are two artifacts. `SYSTEM.md` is band P — template,
//! entrypoint export, lane — for an API caller with no base prompt. The
//! agent lane is the *lane part* of band P plus the whole of band U, for a
//! harness that has already loaded the entrypoint itself. Neither is
//! derivable from the other by truncation.
//!
//! Composition is pure, and the bytes are the caller's: `vibe agent`
//! recomposes the lane at **every launch** from the files then on disk,
//! which is how band U keeps the "current bytes" meaning
//! `#SYS-NO-FROZEN-INDEX` gives it. A composer that read the files itself
//! would be a second producer of band U and would have to decide freshness,
//! which is the caller's question and not this module's.
//!
//! Under a fixed `[system_prompt] host` the matching `os:` rows have already
//! become unconditional rows of the resolved plan
//! ([`super::ContinuationPlan::resolve_host`]), so they arrive here as
//! ordinary sources and need no special case; under `"portable"` they are
//! absent from the file and the router has the agent read them. The composer
//! performs no OS detection, so the artifact stays a pure function of
//! declared inputs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-AGENT-LANE");

use crate::WorkspaceError;

use super::compose_error;
use super::markers::{
    MarkerKind, marker_field, marker_line, segment_hash12, sha256_hex, verify_lane_markers,
};
use super::verify::{ImportInertness, refuse_managed_block_markers, refuse_marker_shaped_lines};

/// One file the lane delivers: the path a marker names it by, and the bytes
/// as the caller observed them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneSource<'a> {
    /// The file's project-relative path — not a label, because the
    /// truncation rule must tell the model which file to read.
    pub path: &'a str,
    /// The observed bytes, delivered verbatim.
    pub bytes: &'a [u8],
}

/// One delivered segment's place in the composed lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneSegment {
    /// The file this segment delivers.
    pub path: String,
    /// The 12 hexadecimal digits both of its markers carry.
    pub hash12: String,
    /// Byte offset of the body — the first byte after the begin line.
    pub offset: usize,
    /// Byte length of the body, framing excluded.
    pub len: usize,
}

/// The composed agent lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedLane {
    /// The lane bytes — what `AGENT-LANE.md` holds.
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`.
    pub sha256: String,
    /// The segments in delivered order, which is INDEX order.
    pub segments: Vec<LaneSegment>,
    /// The `@`-initial lines the lane carries. This census is why the
    /// `import` delivery tier stays refused in version 1
    /// (`#SYS-IMPORT-INERTNESS`).
    pub inertness: ImportInertness,
}

impl ComposedLane {
    /// The delivered hash of one path, which is what the managed block's
    /// expected-`STATIC` value must equal (`#SYS-LANE-MARKERS`).
    pub fn hash_of(&self, path: &str) -> Option<&str> {
        self.segments
            .iter()
            .find(|segment| segment.path == path)
            .map(|segment| segment.hash12.as_str())
    }
}

/// Compose the agent lane for one node.
///
/// Refusals, each before any byte is handed back: a path a marker line
/// cannot carry, the same path delivered twice, a body that is not UTF-8, a
/// marker-shaped line inside a body, and a standalone `<vibevm>` marker —
/// which would be a router inside the lane, the one thing the lane must
/// never carry.
///
/// The composer finishes by running the offline check of
/// [`verify_lane_markers`] over its own output, so the artifact it returns
/// is one `vibe prompt check` has already accepted: every begin has its end,
/// pairs are unique, and each hash matches its own bytes.
pub fn compose_agent_lane(
    node_id: &str,
    sources: &[LaneSource<'_>],
) -> Result<ComposedLane, WorkspaceError> {
    marker_field("node id", node_id)?;
    let mut text = String::new();
    let mut segments: Vec<LaneSegment> = Vec::with_capacity(sources.len());
    let mut inertness = ImportInertness::default();
    for source in sources {
        marker_field("segment path", source.path)?;
        if segments.iter().any(|seen| seen.path == source.path) {
            return Err(compose_error(
                "agent lane",
                format!(
                    "`{}` is delivered twice — one INDEX order delivers each file once",
                    source.path
                ),
            ));
        }
        // The delivered digest is taken from the file's own bytes through the
        // one shared function, so the managed block's expected value and the
        // marker this loop writes can never be two different numbers.
        let digest = segment_hash12(source.bytes);
        let mut body = String::from_utf8(source.bytes.to_vec()).map_err(|error| {
            compose_error(
                "agent lane",
                format!("the lane source `{}` is not UTF-8: {error}", source.path),
            )
        })?;
        refuse_marker_shaped_lines(source.path, &body)?;
        refuse_managed_block_markers(&body)?;
        inertness.add(source.path, &body);
        // One LF before the end marker when the body does not carry its own,
        // so the marker is a line of its own on every input. The inserted
        // byte sits between the markers and is therefore part of what the
        // hash covers — which is what makes the hash recomputable from the
        // delivered text alone.
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        debug_assert_eq!(digest, super::markers::hash12(body.as_bytes()));
        text.push_str(&marker_line(
            MarkerKind::Begin,
            node_id,
            source.path,
            &digest,
        )?);
        text.push('\n');
        let offset = text.len();
        text.push_str(&body);
        segments.push(LaneSegment {
            path: source.path.to_owned(),
            hash12: digest.clone(),
            offset,
            len: body.len(),
        });
        text.push_str(&marker_line(
            MarkerKind::End,
            node_id,
            source.path,
            &digest,
        )?);
        text.push('\n');
    }
    let bytes = text.into_bytes();
    let verified = verify_lane_markers(&bytes)?;
    debug_assert_eq!(verified.len(), segments.len());
    Ok(ComposedLane {
        sha256: sha256_hex(&bytes),
        bytes,
        segments,
        inertness,
    })
}

#[cfg(test)]
#[path = "lane/tests.rs"]
mod tests;
