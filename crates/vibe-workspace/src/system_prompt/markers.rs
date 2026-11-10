//! The lane segment marker grammar — `#SYS-LANE-MARKERS`, owned here.
//!
//! Each segment of the composed agent lane is wrapped:
//!
//! ```text
//! <!-- vibe:lane-begin <node-id> <path> <hash12> -->
//! …segment bytes, verbatim…
//! <!-- vibe:lane-end <node-id> <path> <hash12> -->
//! ```
//!
//! `<path>` is the file's project-relative path rather than a label, because
//! the truncation rule must tell the model **which file** to read and the
//! path is that answer. `<hash12>` is the first 12 hex digits of the SHA-256
//! of the bytes between the two lines — recomputable from the delivered text
//! alone, which is what lets an offline check verify a lane someone else
//! delivered. `<node-id>` is the selected node's identity, without which a
//! nested project's block could match an outer project's lane.
//!
//! The lines carry no `@`, are ordinary HTML/XML comments, and are a pure
//! function of their inputs, so they cost nothing in cache stability;
//! `STATIC.xml` already opens with `<!-- vibe:c1 … -->` lines, so the
//! convention is not new. **Markers exist only in the composed lane** — the
//! files on disk are untouched, because `vibe` does not own `90-user.xml`.
//!
//! Four uses, and exactly one of them is a mechanism. Presence, truncation,
//! duplication and staleness are *instructions to a model*, which can skip a
//! comparison or miss an absent end marker in 300 KB of text. What can be
//! relied on is [`verify_lane_markers`], offline: every begin has its end,
//! pairs are unique, each hash matches its own bytes, and no marker-shaped
//! line occurs inside a segment's own bytes. Order against INDEX is the
//! caller's comparison, because only the caller holds the plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-LANE-MARKERS");

use sha2::{Digest, Sha256};

use crate::WorkspaceError;

use super::compose_error;

/// The begin keyword of a lane segment marker.
pub const LANE_BEGIN: &str = "vibe:lane-begin";

/// The end keyword of a lane segment marker.
pub const LANE_END: &str = "vibe:lane-end";

/// How many hexadecimal digits of a segment's SHA-256 a marker carries.
pub const HASH12_LEN: usize = 12;

/// Which half of a pair a marker line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// `vibe:lane-begin`.
    Begin,
    /// `vibe:lane-end`.
    End,
}

impl MarkerKind {
    /// The keyword this half spells.
    pub fn keyword(self) -> &'static str {
        match self {
            MarkerKind::Begin => LANE_BEGIN,
            MarkerKind::End => LANE_END,
        }
    }
}

/// One marker line's three fields, as parsed back out of delivered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneMarker {
    /// Begin or end.
    pub kind: MarkerKind,
    /// The selected node's identity.
    pub node_id: String,
    /// The segment's project-relative path.
    pub path: String,
    /// The 12 hexadecimal digits the line carries.
    pub hash12: String,
}

/// One verified segment of a delivered lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSegment {
    /// The node identity both markers carry.
    pub node_id: String,
    /// The file this segment delivers.
    pub path: String,
    /// The hash both markers carry, which equals the digest of the body.
    pub hash12: String,
    /// Byte offset of the body — the first byte after the begin line.
    pub offset: usize,
    /// Byte length of the body, framing excluded.
    pub len: usize,
}

/// SHA-256 of `bytes` as 64 lowercase hexadecimal digits.
///
/// The composer's one digest function, so a prefix digest, a sidecar witness
/// and a marker's `hash12` can never be computed two different ways.
///
/// ```
/// use vibe_workspace::system_prompt::sha256_hex;
///
/// assert_eq!(
///     sha256_hex(b"a"),
///     "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
/// );
/// ```
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The first [`HASH12_LEN`] hexadecimal digits of the SHA-256 of `bytes`.
///
/// ```
/// use vibe_workspace::system_prompt::hash12;
///
/// assert_eq!(hash12(b"").len(), 12);
/// assert_eq!(hash12(b"a"), "ca978112ca1b");
/// ```
pub fn hash12(bytes: &[u8]) -> String {
    let mut digest = sha256_hex(bytes);
    digest.truncate(HASH12_LEN);
    digest
}

/// Render one marker line, without its newline.
///
/// ```
/// use vibe_workspace::system_prompt::{MarkerKind, marker_line};
///
/// let line = marker_line(MarkerKind::Begin, "org.vibevm.core/vibevm", "boot/STATIC.xml", "0123456789ab")?;
/// assert_eq!(
///     line,
///     "<!-- vibe:lane-begin org.vibevm.core/vibevm boot/STATIC.xml 0123456789ab -->"
/// );
/// # Ok::<(), vibe_workspace::WorkspaceError>(())
/// ```
pub fn marker_line(
    kind: MarkerKind,
    node_id: &str,
    path: &str,
    hash12: &str,
) -> Result<String, WorkspaceError> {
    marker_field("node id", node_id)?;
    marker_field("segment path", path)?;
    marker_field("segment hash", hash12)?;
    Ok(format!(
        "<!-- {keyword} {node_id} {path} {hash12} -->",
        keyword = kind.keyword()
    ))
}

/// A marker field must be one non-empty token that keeps the line a
/// well-formed XML comment: no ASCII whitespace, and no `--` or `>`, which
/// would end or corrupt the comment. Refusing names the field and the value
/// rather than sanitising it, because a silently rewritten path would send a
/// model to the wrong file.
pub(super) fn marker_field(what: &str, value: &str) -> Result<(), WorkspaceError> {
    let bad = if value.is_empty() {
        Some("it is empty")
    } else if value.chars().any(|c| c.is_ascii_whitespace()) {
        Some("it contains whitespace, and a marker field is one token")
    } else if value.contains("--") {
        Some("it contains `--`, which cannot appear inside an XML comment")
    } else if value.contains('>') || value.contains('<') {
        Some("it contains `<` or `>`, which would corrupt the comment")
    } else {
        None
    };
    match bad {
        None => Ok(()),
        Some(reason) => Err(compose_error(
            "lane segment marker",
            format!("the {what} `{value}` cannot be written into a marker line: {reason}"),
        )),
    }
}

/// Parse one line as a marker, or `None` when it is ordinary text.
///
/// Matching is on the **trimmed** line, so a marker is recognised wherever a
/// harness indented it, and a line that merely mentions the keyword inside
/// other text is not a marker.
pub fn parse_marker_line(line: &str) -> Option<LaneMarker> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let mut fields = inner.split_ascii_whitespace();
    let kind = match fields.next()? {
        LANE_BEGIN => MarkerKind::Begin,
        LANE_END => MarkerKind::End,
        _ => return None,
    };
    let node_id = fields.next()?.to_owned();
    let path = fields.next()?.to_owned();
    let hash12 = fields.next()?.to_owned();
    if fields.next().is_some() {
        return None;
    }
    Some(LaneMarker {
        kind,
        node_id,
        path,
        hash12,
    })
}

/// Verify a delivered lane's markers offline — the one part of
/// `#SYS-LANE-MARKERS` that is a mechanism rather than an instruction.
///
/// Refuses, each naming the line: a begin with no matching end (truncation),
/// an end with no begin, two pairs for one `(node id, path)` (duplication), a
/// hash that does not match the bytes it wraps, and a marker-shaped line
/// inside a segment's own bytes. Returns the verified segments in delivered
/// order, so a caller holding the boot plan can compare that order with
/// INDEX's — the one check only the caller can make.
pub fn verify_lane_markers(bytes: &[u8]) -> Result<Vec<VerifiedSegment>, WorkspaceError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        compose_error(
            "agent lane",
            format!("the delivered lane is not UTF-8: {error}"),
        )
    })?;
    let mut segments: Vec<VerifiedSegment> = Vec::new();
    let mut open: Option<(LaneMarker, usize, usize)> = None;
    let mut offset = 0usize;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let line_no = index + 1;
        let next = offset + line.len();
        match (parse_marker_line(line), open.as_ref()) {
            (Some(marker), None) if marker.kind == MarkerKind::Begin => {
                open = Some((marker, next, line_no));
            }
            (Some(marker), None) => {
                return Err(marker_defect(
                    line_no,
                    &format!(
                        "a `{LANE_END}` line for `{}` has no `{LANE_BEGIN}` before it",
                        marker.path
                    ),
                ));
            }
            (Some(marker), Some((begun, body_start, begun_line))) => {
                if marker.kind == MarkerKind::Begin
                    || marker.node_id != begun.node_id
                    || marker.path != begun.path
                    || marker.hash12 != begun.hash12
                {
                    return Err(marker_defect(
                        line_no,
                        &format!(
                            "a marker-shaped line occurs inside the segment opened for `{}` at \
                             line {begun_line}",
                            begun.path
                        ),
                    ));
                }
                let body = &text[*body_start..offset];
                let actual = hash12(body.as_bytes());
                if actual != begun.hash12 {
                    return Err(marker_defect(
                        line_no,
                        &format!(
                            "segment `{}` carries hash `{}` but its delivered bytes hash to \
                             `{actual}`",
                            begun.path, begun.hash12
                        ),
                    ));
                }
                if segments
                    .iter()
                    .any(|seen| seen.node_id == begun.node_id && seen.path == begun.path)
                {
                    return Err(marker_defect(
                        line_no,
                        &format!("segment `{}` is delivered twice", begun.path),
                    ));
                }
                segments.push(VerifiedSegment {
                    node_id: begun.node_id.clone(),
                    path: begun.path.clone(),
                    hash12: begun.hash12.clone(),
                    offset: *body_start,
                    len: offset - *body_start,
                });
                open = None;
            }
            (None, _) => {}
        }
        offset = next;
    }
    if let Some((begun, _, begun_line)) = open {
        return Err(marker_defect(
            begun_line,
            &format!(
                "segment `{}` was opened and never closed — the delivered lane is truncated, \
                 so that file must be read from disk",
                begun.path
            ),
        ));
    }
    Ok(segments)
}

fn marker_defect(line_no: usize, reason: &str) -> WorkspaceError {
    compose_error("agent lane markers", format!("line {line_no}: {reason}"))
}

#[cfg(test)]
#[path = "markers/tests.rs"]
mod tests;
