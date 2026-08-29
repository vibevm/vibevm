//! Engine-owned emitted transition and complete target-tape inspection.

use std::borrow::Cow;

use sha2::{Digest, Sha256};

use super::super::backend::{BackendError, BackendId};
use super::super::ir::{
    ArtifactFrame, EmissionProvenance, EmittedArtifact, LaneChunk, LaneContribution, LaneIr,
    LaneNode, PreEmissionWitness, PreparedEmissionTarget,
};
use super::super::pass::PassName;
use super::framing;

mod xml;

pub(super) fn transition(
    backend: &BackendId,
    pass: &PassName,
    witness: &PreEmissionWitness,
    lane: &LaneIr,
    emitted: &EmittedArtifact,
) -> Result<(), BackendError> {
    let provenance = emitted.provenance();
    field(
        backend,
        witness.context == *lane.context(),
        "witness context",
    )?;
    field(
        backend,
        witness.source_node_count == lane.source_node_count,
        "source node count",
    )?;
    field(
        backend,
        witness.source_link_digest == lane.source_link_digest,
        "source Link digest",
    )?;
    field(backend, witness.frame == lane.frame, "Lane frame")?;
    compare_contributions(backend, witness, lane)?;
    field(
        backend,
        provenance.context == witness.context,
        "provenance context",
    )?;
    field(backend, provenance.backend == *backend, "backend id")?;
    field(backend, provenance.producer == *pass, "producer pass")?;
    field(
        backend,
        provenance.source_lane_digest == witness.lane_digest,
        "Lane digest",
    )?;
    field(
        backend,
        provenance.renames == witness.frame.renames,
        "ordered renames",
    )?;
    field(
        backend,
        provenance.contributions == witness.emission_witnesses,
        "contribution witnesses",
    )
}

fn compare_contributions(
    backend: &BackendId,
    witness: &PreEmissionWitness,
    lane: &LaneIr,
) -> Result<(), BackendError> {
    field(
        backend,
        witness.contributions.len() == lane.contributions.len(),
        "contribution count",
    )?;
    for (expected, actual) in witness.contributions.iter().zip(&lane.contributions) {
        field(
            backend,
            contribution_matches(expected, actual),
            "ordered contribution field",
        )?;
    }
    Ok(())
}

fn contribution_matches(expected: &LaneContribution, actual: &LaneContribution) -> bool {
    match (expected, actual) {
        (
            LaneContribution::Normal {
                meta: em,
                seed: es,
                seed_address: ea,
                chunks: ec,
            },
            LaneContribution::Normal {
                meta,
                seed,
                seed_address,
                chunks,
            },
        ) => em == meta && es == seed && ea == seed_address && ec == chunks,
        (
            LaneContribution::Simple {
                meta: em,
                address: ea,
                chunks: ec,
            },
            LaneContribution::Simple {
                meta,
                address,
                chunks,
            },
        ) => em == meta && ea == address && ec == chunks,
        (LaneContribution::Elided { meta: em }, LaneContribution::Elided { meta }) => em == meta,
        (
            LaneContribution::Hoisted {
                meta: em,
                target: et,
            },
            LaneContribution::Hoisted { meta, target },
        ) => em == meta && et == target,
        _ => false,
    }
}

pub(super) fn current(
    backend: &BackendId,
    pass: &PassName,
    witness: &PreEmissionWitness,
    bytes: &[u8],
    provenance: &EmissionProvenance,
) -> Result<(), BackendError> {
    common_current(backend, pass, bytes, provenance)?;
    match &witness.prepared_target {
        PreparedEmissionTarget::Markdown => markdown_observation(backend, witness, bytes),
        PreparedEmissionTarget::Xml { documents } => {
            xml::observation(backend, witness, documents, bytes)
        }
        #[cfg(any(test, feature = "test-support"))]
        PreparedEmissionTarget::Custom => Ok(()),
    }
}

fn markdown_observation(
    backend: &BackendId,
    witness: &PreEmissionWitness,
    bytes: &[u8],
) -> Result<(), BackendError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        current_error(backend, format!("STATIC Markdown is not UTF-8: {error}"))
    })?;
    let mut cursor = Cursor::new(text);
    match witness.context.frame() {
        ArtifactFrame::CompatibilityFragment => {
            for contribution in &witness.contributions {
                match contribution {
                    LaneContribution::Normal { chunks, .. }
                    | LaneContribution::Simple { chunks, .. } => {
                        observe_markdown_chunks(backend, &mut cursor, chunks, false)?;
                    }
                    LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => {
                        return Err(current_error(
                            backend,
                            "compatibility fragment contains a framed contribution",
                        ));
                    }
                }
            }
        }
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => {
            observe_markdown_frame(
                backend,
                &mut cursor,
                generated_path,
                source_root,
                &witness.frame.renames,
                witness.transforms_header.as_deref(),
            )?;
            for contribution in &witness.contributions {
                observe_markdown_contribution(backend, &mut cursor, contribution)?;
            }
        }
    }
    cursor.finish(backend, "trailing Markdown tape bytes")
}

fn observe_markdown_frame(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    generated_path: &str,
    source_root: &str,
    renames: &[super::super::ir::OriginRename],
    transforms: Option<&str>,
) -> Result<(), BackendError> {
    for expected in framing::header_payloads(generated_path) {
        observe_markdown_comment(backend, cursor, &expected)?;
        cursor.expect(backend, "\n", "header line ending")?;
    }
    if let Some(expected) = transforms {
        observe_transforms_header(backend, cursor, expected)?;
        cursor.expect(backend, "\n", "transforms header line ending")?;
    }
    cursor.expect(backend, "\n", "header/frame separator")?;
    observe_markdown_comment(backend, cursor, &framing::resolution_payload(source_root))?;
    cursor.expect(backend, "\n\n", "resolution/frame separator")?;
    if !renames.is_empty() {
        observe_markdown_comment(backend, cursor, &framing::tombstone_payload(renames))?;
        cursor.expect(backend, "\n\n", "tombstone/frame separator")?;
    }
    Ok(())
}

fn observe_markdown_contribution(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    contribution: &LaneContribution,
) -> Result<(), BackendError> {
    match contribution {
        LaneContribution::Normal { meta, chunks, .. }
        | LaneContribution::Simple { meta, chunks, .. } => {
            observe_markdown_comment(backend, cursor, &framing::static_marker_payload(meta))?;
            cursor.expect(backend, "\n\n", "STATIC marker/body separator")?;
            observe_markdown_chunks(backend, cursor, chunks, true)?;
            cursor.expect(backend, "\n\n", "STATIC contribution separator")
        }
        LaneContribution::Elided { meta } => {
            observe_markdown_comment(backend, cursor, &framing::elided_marker_payload(meta))?;
            cursor.expect(backend, "\n\n", "elided contribution separator")
        }
        LaneContribution::Hoisted { meta, target } => {
            observe_markdown_comment(
                backend,
                cursor,
                &framing::hoisted_marker_payload(&meta.origin),
            )?;
            cursor.expect(
                backend,
                &format!("\n#use spec://{}\n\n", hoisted_coordinate(backend, target)?),
                "hoisted contribution",
            )
        }
    }
}

/// Observe the ONE active-transforms header line (R4 architecture §7.1),
/// identically in both lanes.
///
/// The header is spelled the same bytes in Markdown and XML — its tokens are
/// already codec-encoded, so it carries no `--` and no terminal `-` and needs
/// neither the `vibe:c1` wrapper nor a lane-specific form. One observation
/// therefore serves both validators.
///
/// GRAMMAR FIRST, then identity. The payload read off the TAPE is judged by
/// the shared codec — the reserved prefix must open it, and every
/// whitespace-separated token must decode canonically — so a tape whose
/// tokens were spelled raw, lowercase-escaped or otherwise non-canonical is
/// refused with the CODEC's own error rather than a generic mismatch. Only a
/// well-formed payload is then compared to the one the engine's own plan
/// produced. The order matters: an emitter that bypassed the codec would
/// produce a tape that AGREES with its own witness, and only the grammar step
/// can see that.
pub(in crate::compiler::emit) fn observe_transforms_header(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    expected: &str,
) -> Result<(), BackendError> {
    let rest = cursor.remaining();
    let Some(after_open) = rest.strip_prefix("<!-- ") else {
        return Err(current_error(backend, "missing transforms header comment"));
    };
    let Some(end) = after_open.find("-->") else {
        return Err(current_error(
            backend,
            "unterminated transforms header comment",
        ));
    };
    let Some(payload) = after_open[..end].strip_suffix(' ') else {
        return Err(current_error(
            backend,
            "malformed transforms header framing",
        ));
    };
    let Some(tokens) = crate::compiler::transform::header::observed_header_tokens(payload) else {
        return Err(current_error(
            backend,
            "transforms header does not open with its reserved prefix",
        ));
    };
    for token in tokens {
        vibe_specdoc::decode_generated_xml_comment_payload(token).map_err(|error| {
            current_error(backend, format!("invalid transforms header token: {error}"))
        })?;
    }
    if payload != expected {
        return Err(current_error(backend, "transforms header mismatch"));
    }
    cursor.advance("<!-- ".len() + end + "-->".len());
    Ok(())
}

fn observe_markdown_comment(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    expected: &str,
) -> Result<(), BackendError> {
    let rest = cursor.remaining();
    let Some(after_open) = rest.strip_prefix("<!-- ") else {
        return Err(current_error(backend, "missing generated Markdown comment"));
    };
    let Some(end) = after_open.find("-->") else {
        return Err(current_error(
            backend,
            "unterminated generated Markdown comment",
        ));
    };
    let raw = &after_open[..end];
    let payload = raw
        .strip_suffix('\n')
        .or_else(|| raw.strip_suffix(' '))
        .unwrap_or(raw);
    if payload != expected {
        return Err(current_error(
            backend,
            "generated Markdown comment mismatch",
        ));
    }
    cursor.advance("<!-- ".len() + end + "-->".len());
    Ok(())
}

fn observe_markdown_chunks(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    chunks: &[LaneChunk],
    trim_end: bool,
) -> Result<(), BackendError> {
    let mut segments: Vec<Cow<'_, str>> = Vec::new();
    for chunk in chunks {
        match chunk {
            LaneChunk::NormalOpen { marker, .. } => {
                segments.push(Cow::Owned(format!(
                    "<!-- vibe:begin {} -->\n",
                    marker.as_str()
                )));
            }
            LaneChunk::NormalClose { marker, .. } => {
                segments.push(Cow::Owned(format!(
                    "<!-- vibe:end {} -->\n",
                    marker.as_str()
                )));
            }
            LaneChunk::Node(node) => match node.as_ref() {
                LaneNode::Normal { body, .. } | LaneNode::Simple { body, .. } => {
                    segments.push(Cow::Borrowed(body));
                }
            },
            LaneChunk::ForcedNewline { .. } => segments.push(Cow::Borrowed("\n")),
        }
    }
    observe_segments(backend, cursor, &segments, trim_end)
}

fn observe_segments(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    segments: &[Cow<'_, str>],
    trim_end: bool,
) -> Result<(), BackendError> {
    let total = segments.iter().map(|segment| segment.len()).sum::<usize>();
    let trimmed = if trim_end {
        trailing_whitespace_bytes(segments)
    } else {
        0
    };
    let mut remaining = total - trimmed;
    for segment in segments {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(segment.len());
        cursor.expect(
            backend,
            &segment[..take],
            "ordered Markdown contribution body",
        )?;
        remaining -= take;
    }
    Ok(())
}

fn trailing_whitespace_bytes(segments: &[Cow<'_, str>]) -> usize {
    let mut total = 0;
    for segment in segments.iter().rev() {
        let trimmed = segment.trim_end();
        total += segment.len() - trimmed.len();
        if !trimmed.is_empty() {
            break;
        }
    }
    total
}

fn hoisted_coordinate(
    backend: &BackendId,
    target: &crate::SpecAddress,
) -> Result<String, BackendError> {
    match &target.authority {
        crate::Authority::Package {
            group,
            name,
            version: None,
        } => Ok(format!("{group}/{name}")),
        _ => Err(current_error(
            backend,
            "hoisted target is not an unversioned package document",
        )),
    }
}

pub(super) fn common_current(
    backend: &BackendId,
    pass: &PassName,
    bytes: &[u8],
    provenance: &EmissionProvenance,
) -> Result<(), BackendError> {
    if provenance.backend != *backend || provenance.producer != *pass {
        return Err(current_error(backend, "backend/pass provenance mismatch"));
    }
    if provenance.context.target().backend_id() != backend.as_str() {
        return Err(current_error(backend, "target/backend identity mismatch"));
    }
    if provenance.bytes_digest != independent_bytes_digest(bytes) {
        return Err(current_error(backend, "byte digest mismatch"));
    }
    Ok(())
}

fn independent_bytes_digest(bytes: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    let domain = b"vibe-spec/emitted-bytes/v1";
    digest.update((domain.len() as u64).to_le_bytes());
    digest.update(domain);
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    digest.finalize().into()
}

fn field(backend: &BackendId, condition: bool, field: &'static str) -> Result<(), BackendError> {
    if condition {
        Ok(())
    } else {
        Err(BackendError::Transition {
            backend: backend.as_str().to_string(),
            field,
        })
    }
}

fn current_error(backend: &BackendId, reason: impl Into<String>) -> BackendError {
    BackendError::Current {
        backend: backend.as_str().to_string(),
        reason: reason.into(),
    }
}

pub(in crate::compiler::emit) struct Cursor<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, offset: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.text[self.offset..]
    }

    fn advance(&mut self, bytes: usize) {
        self.offset += bytes;
    }

    fn expect(
        &mut self,
        backend: &BackendId,
        expected: &str,
        part: &'static str,
    ) -> Result<(), BackendError> {
        if self.remaining().starts_with(expected) {
            self.advance(expected.len());
            Ok(())
        } else {
            Err(current_error(backend, format!("{part} mismatch")))
        }
    }

    fn finish(self, backend: &BackendId, reason: &'static str) -> Result<(), BackendError> {
        if self.offset == self.text.len() {
            Ok(())
        } else {
            Err(current_error(backend, reason))
        }
    }
}
