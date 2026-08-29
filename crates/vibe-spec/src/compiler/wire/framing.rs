//! The builtin backends' intrinsic tape framing — the strongest honest
//! framing grammar computable from the wire document alone, SHARED with the
//! emitters (the engine's `emit::framing` builders and the reversible marker
//! cell), never a second trace/IR DTO.
//!
//! Three established rows, each judged by the grammar its own emitter really
//! writes:
//!
//! - builtin `StaticLane` — the context-owned header/preamble/tombstone
//!   prologue must be EXACT, and the ordered contribution marker/kind
//!   sequence must reconcile to the carried `EmissionContributionWitness`
//!   metadata. Arbitrary UTF-8, marker-free Markdown, or generically
//!   well-formed XML is NOT backend framing;
//! - builtin `CompatibilityFragment` — flattened reversible Markdown: valid
//!   UTF-8 whose marker blocks balance, with no static-lane header or
//!   contribution marker to demand;
//! - a custom target stays opaque after identity and digest, and never
//!   reaches this cell at all.
//!
//! The relational remainder — that block bodies are exactly the lane's
//! occurrence texts in order — needs the Lane the emitted carrier does not
//! carry; that replay stays the manager's transition evidence (R6.3), like
//! the production emit validator.

use quick_xml::Reader;

use crate::compiler::emit::framing as engine;
use crate::compiler::emit::framing::CommentSyntax;
use crate::compiler::ir::{ArtifactFrame, EmissionContributionWitness, EmissionProvenance};
use crate::doctree::FenceTracker;
use crate::markers::{ControlLine, ControlScanner};

use super::bounded::display;
use super::emitted::{GateRefusal, gate_emit_identity};

/// The builtin backends' tape law: exact prologue, reconciled marker
/// sequence, backend-specific document grammar. A custom target stays
/// opaque — identity and digest only — and never reaches this cell.
pub(super) fn builtin(provenance: &EmissionProvenance, bytes: &[u8]) -> Result<(), GateRefusal> {
    let syntax = syntax_of(provenance);
    let text = std::str::from_utf8(bytes).map_err(|_| {
        gate_emit_identity(format!(
            "a `{}` tape is re-read by its backend and must be valid UTF-8",
            provenance.backend.as_str()
        ))
    })?;

    // The BUILTIN compatibility row: `static_md::emit_markdown` flattens the
    // Lane's chunks and writes NO static-lane header and NO contribution
    // marker for a `CompatibilityFragment`. Demanding either would invent a
    // law the emitter never obeys, so the honest intrinsic grammar here is the
    // reversible Markdown boundary the fragment really carries.
    if matches!(
        provenance.context.frame(),
        ArtifactFrame::CompatibilityFragment
    ) {
        return marker_blocks_balanced(text);
    }

    let prologue = prologue_length(syntax, provenance, text)?;
    reconcile_markers(syntax, &text[prologue..], provenance)?;

    match syntax {
        CommentSyntax::Markdown => marker_blocks_balanced(&text[prologue..]),
        CommentSyntax::Xml => well_formed_xml(text),
    }
}

/// How many bytes of `text` the context-owned prologue occupies, or a refusal.
///
/// Three fixed parts and one optional one: the provenance header block, then
/// — for an artifact whose owner plan was nonempty — the ONE active-transforms
/// header (R4 architecture §7.1), then the blank separator, the resolution
/// preamble and any tombstone.
///
/// The emitted carrier does NOT carry the active plan, and §7.1 is explicit
/// that nothing ever parses the header back to recover one. So the strongest
/// honest law computable here is: the header line is optional, and when
/// present it must be a well-formed one — its reserved prefix and its
/// codec-canonical tokens, judged by the shared codec itself. A tape that
/// invents a header, or spells one raw, still refuses.
fn prologue_length(
    syntax: CommentSyntax,
    provenance: &EmissionProvenance,
    text: &str,
) -> Result<usize, GateRefusal> {
    let ArtifactFrame::StaticLane {
        generated_path,
        source_root,
    } = provenance.context.frame()
    else {
        return Ok(0);
    };
    let head = engine::static_header_block(syntax, generated_path);
    if !text.starts_with(&head) {
        return Err(prologue_refusal());
    }
    let mut length = head.len();
    length += observed_transforms_header(&text[length..])?;
    let mut tail = String::from("\n");
    tail.push_str(&engine::resolution_preamble(syntax, source_root));
    if !provenance.renames.is_empty() {
        tail.push_str(&engine::tombstone(syntax, &provenance.renames));
    }
    if !text[length..].starts_with(&tail) {
        return Err(prologue_refusal());
    }
    Ok(length + tail.len())
}

/// The byte length of the optional active-transforms header at this cursor —
/// `0` when none is written, a refusal when one is written malformed.
fn observed_transforms_header(rest: &str) -> Result<usize, GateRefusal> {
    let opening = format!(
        "<!-- {} ",
        crate::compiler::transform::header::TRANSFORMS_HEADER_PREFIX
    );
    if !rest.starts_with(&opening) {
        return Ok(0);
    }
    let Some(end) = rest.find(" -->\n") else {
        return Err(gate_emit_identity(
            "the tape opens an active-transforms header it never terminates",
        ));
    };
    let payload = &rest["<!-- ".len()..end];
    let Some(tokens) = crate::compiler::transform::header::observed_header_tokens(payload) else {
        return Err(gate_emit_identity(
            "the tape's transforms header does not open with its reserved prefix",
        ));
    };
    for token in tokens {
        vibe_specdoc::decode_generated_xml_comment_payload(token).map_err(|error| {
            gate_emit_identity(format!(
                "the tape's transforms header carries a non-canonical token: {error}"
            ))
        })?;
    }
    Ok(end + " -->\n".len())
}

fn prologue_refusal() -> GateRefusal {
    gate_emit_identity(
        "the tape does not open with the context-owned header/preamble prologue the context and renames declare",
    )
}

/// The backend's comment syntax, chosen exactly as the emitters choose it.
fn syntax_of(provenance: &EmissionProvenance) -> CommentSyntax {
    if provenance.context.target().is_static_markdown() {
        CommentSyntax::Markdown
    } else {
        CommentSyntax::Xml
    }
}

/// Every marker-prefixed line the scanner can read must be the NEXT expected
/// contribution marker, in carried order, with none extra and none missing.
fn reconcile_markers(
    syntax: CommentSyntax,
    body: &str,
    provenance: &EmissionProvenance,
) -> Result<(), GateRefusal> {
    let expected: Vec<String> = provenance
        .contributions
        .iter()
        .map(|witness| match witness {
            EmissionContributionWitness::Normal { meta, .. }
            | EmissionContributionWitness::Simple { meta, .. } => {
                engine::static_marker(syntax, meta)
            }
            EmissionContributionWitness::Elided { meta } => engine::elided_marker(syntax, meta),
            EmissionContributionWitness::Hoisted { meta, .. } => {
                engine::hoisted_marker(syntax, &meta.origin)
            }
        })
        .collect();

    let mut cursor = 0usize;
    let mut fence = FenceTracker::default();
    for line in body.lines() {
        let fenced = fence.classify(line);
        if fenced || !is_marker_line(syntax, line) {
            continue;
        }
        if expected.get(cursor).is_some_and(|marker| marker == line) {
            cursor += 1;
        } else {
            return Err(gate_emit_identity(format!(
                "the tape's contribution marker sequence does not reconcile with the carried emission witnesses (marker {} of {})",
                cursor + 1,
                expected.len()
            )));
        }
    }
    if cursor != expected.len() {
        return Err(gate_emit_identity(format!(
            "the tape carries {} of {} carried contribution markers",
            cursor,
            expected.len()
        )));
    }
    Ok(())
}

/// A contribution-marker line under this backend's comment syntax. Markdown
/// spells `static`/`elided` and `hoisted` differently — both are contribution
/// markers, and neither collides with the reversible `vibe:begin`/`vibe:end`
/// grammar that rides the body. Every engine XML comment rides the encoded
/// `vibe:c1` channel; the prologue is exact-stripped, so a surviving one is a
/// marker.
fn is_marker_line(syntax: CommentSyntax, line: &str) -> bool {
    match syntax {
        CommentSyntax::Markdown => {
            line.starts_with("<!-- vibe:static ") || line.starts_with("<!-- vibe:hoisted ")
        }
        CommentSyntax::Xml => line.starts_with("<!-- vibe:c1 "),
    }
}

/// The same predicate, for the tests that mutate a REAL emitted tape.
#[cfg(test)]
pub(super) fn is_contribution_marker(provenance: &EmissionProvenance, line: &str) -> bool {
    is_marker_line(syntax_of(provenance), line)
}

/// The tape past its exact context-owned prologue — the region the marker
/// sequence is reconciled over. (The XML prologue is itself spelled in the
/// `vibe:c1` comment channel, so a whole-tape scan would count it.)
#[cfg(test)]
pub(super) fn tape_body<'a>(provenance: &EmissionProvenance, text: &'a str) -> &'a str {
    match prologue_length(syntax_of(provenance), provenance, text) {
        Ok(prologue) => &text[prologue..],
        Err(_) => text,
    }
}

/// The reversible-marker grammar on a static-md tape: every `vibe:begin` the
/// scanner can read must be closed by the matching `vibe:end` before the
/// tape ends. A close naming nobody, or an open riding inside an open block,
/// is BODY under the shared grammar — the engine's own readers treat it as
/// content, so the decoder is never stricter.
fn marker_blocks_balanced(body: &str) -> Result<(), GateRefusal> {
    let mut scanner = ControlScanner::default();
    let mut open: Option<String> = None;
    for line in body.lines() {
        let position = scanner.step(line, open.is_some());
        let Some(control) = position
            .readable()
            .then_some(position.control.as_ref())
            .flatten()
        else {
            continue;
        };
        match (&open, control) {
            (None, ControlLine::Open(key)) => open = Some(key.clone()),
            (Some(current), ControlLine::Close(closed)) if closed == current => {
                open = None;
            }
            _ => {}
        }
    }
    if open.is_some() {
        return Err(gate_emit_identity(
            "a static-md tape must close every reversible marker block it opens",
        ));
    }
    Ok(())
}

/// `static-xml`: a well-formed non-empty XML document in the engine's XML
/// framing (balanced end names; the error preview stays bounded).
fn well_formed_xml(text: &str) -> Result<(), GateRefusal> {
    if text.trim().is_empty() {
        return Err(gate_emit_identity(
            "a static-xml tape is a document, never empty",
        ));
    }
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => continue,
            // quick-xml's error text is derived from attacker-controlled
            // bytes, so it renders through the bounded sink rather than being
            // built in full and then cut.
            Err(source) => {
                return Err(gate_emit_identity(format!(
                    "a static-xml tape must be well-formed XML: {}",
                    display(source)
                )));
            }
        }
    }
    Ok(())
}
