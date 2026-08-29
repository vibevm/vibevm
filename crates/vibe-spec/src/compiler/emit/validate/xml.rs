//! Complete c1/XML tape observation against the pre-emission witness.

use quick_xml::events::Event;

use super::{Cursor, current_error, hoisted_coordinate, observe_transforms_header};
use crate::compiler::backend::{BackendError, BackendId};
use crate::compiler::emit::framing;
use crate::compiler::ir::{ArtifactFrame, LaneContribution, PreEmissionWitness};

pub(super) fn observation(
    backend: &BackendId,
    witness: &PreEmissionWitness,
    documents: &[Option<vibe_specdoc::doc::SpecDoc>],
    bytes: &[u8],
) -> Result<(), BackendError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        current_error(backend, format!("STATIC XML tape is not UTF-8: {error}"))
    })?;
    let ArtifactFrame::StaticLane {
        generated_path,
        source_root,
    } = witness.context.frame()
    else {
        return Err(current_error(
            backend,
            "XML backend received compatibility provenance",
        ));
    };
    if documents.len() != witness.contributions.len() {
        return Err(current_error(
            backend,
            "prepared XML observation count differs from contributions",
        ));
    }
    let mut cursor = Cursor::new(text);
    observe_frame(
        backend,
        &mut cursor,
        generated_path,
        source_root,
        &witness.frame.renames,
        witness.transforms_header.as_deref(),
    )?;
    for (index, contribution) in witness.contributions.iter().enumerate() {
        observe_contribution(
            backend,
            &mut cursor,
            contribution,
            documents[index].as_ref(),
        )?;
    }
    cursor.finish(backend, "trailing XML tape bytes")
}

fn observe_frame(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    generated_path: &str,
    source_root: &str,
    renames: &[crate::compiler::ir::OriginRename],
    transforms: Option<&str>,
) -> Result<(), BackendError> {
    for expected in framing::header_payloads(generated_path) {
        observe_comment(backend, cursor, &expected)?;
        cursor.expect(backend, "\n", "c1 header line ending")?;
    }
    // NOT a c1 comment: the transforms header's tokens are already codec-
    // encoded, so it is written — and read — as the same plain comment in
    // both lanes. Routing it through `observe_comment` would demand a
    // `vibe:c1` wrapper the emitter deliberately does not write.
    if let Some(expected) = transforms {
        observe_transforms_header(backend, cursor, expected)?;
        cursor.expect(backend, "\n", "transforms header line ending")?;
    }
    cursor.expect(backend, "\n", "c1 header/frame separator")?;
    observe_comment(backend, cursor, &framing::resolution_payload(source_root))?;
    cursor.expect(backend, "\n\n", "c1 resolution/frame separator")?;
    if !renames.is_empty() {
        observe_comment(backend, cursor, &framing::tombstone_payload(renames))?;
        cursor.expect(backend, "\n\n", "c1 tombstone/frame separator")?;
    }
    Ok(())
}

fn observe_contribution(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    contribution: &LaneContribution,
    expected_document: Option<&vibe_specdoc::doc::SpecDoc>,
) -> Result<(), BackendError> {
    match contribution {
        LaneContribution::Normal { meta, .. } | LaneContribution::Simple { meta, .. } => {
            observe_comment(backend, cursor, &framing::static_marker_payload(meta))?;
            cursor.expect(backend, "\n\n", "c1 STATIC marker/body separator")?;
            let expected_document = expected_document.ok_or_else(|| {
                current_error(
                    backend,
                    "material contribution has no prepared XML document",
                )
            })?;
            let observed = take_document(backend, cursor)?;
            let parsed = vibe_specdoc::from_xml(observed).map_err(|error| {
                current_error(backend, format!("invalid XML contribution: {error}"))
            })?;
            if &parsed != expected_document {
                return Err(current_error(
                    backend,
                    "XML contribution body differs from its pre-emission witness",
                ));
            }
            cursor.expect(backend, "\n\n", "XML contribution separator")
        }
        LaneContribution::Elided { meta } => {
            if expected_document.is_some() {
                return Err(current_error(
                    backend,
                    "elided contribution has an XML document",
                ));
            }
            observe_comment(backend, cursor, &framing::elided_marker_payload(meta))?;
            cursor.expect(backend, "\n\n", "c1 elided contribution separator")
        }
        LaneContribution::Hoisted { meta, target } => {
            if expected_document.is_some() {
                return Err(current_error(
                    backend,
                    "hoisted contribution has an XML document",
                ));
            }
            observe_comment(
                backend,
                cursor,
                &framing::hoisted_marker_payload(&meta.origin),
            )?;
            cursor.expect(
                backend,
                &format!("\n#use spec://{}\n\n", hoisted_coordinate(backend, target)?),
                "c1 hoisted contribution",
            )
        }
    }
}

fn observe_comment(
    backend: &BackendId,
    cursor: &mut Cursor<'_>,
    expected: &str,
) -> Result<(), BackendError> {
    let rest = cursor.remaining();
    let Some(end) = rest.find("-->") else {
        return Err(current_error(backend, "missing or unterminated c1 comment"));
    };
    let comment = &rest[..end + 3];
    let payload = vibe_specdoc::decode_generated_xml_comment(comment)
        .map_err(|error| current_error(backend, format!("invalid c1 comment: {error}")))?;
    if payload.as_deref() != Some(expected) {
        return Err(current_error(backend, "generated c1 comment mismatch"));
    }
    cursor.advance(end + 3);
    Ok(())
}

fn take_document<'a>(
    backend: &BackendId,
    cursor: &mut Cursor<'a>,
) -> Result<&'a str, BackendError> {
    const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
    let rest = cursor.remaining();
    if !rest.starts_with(DECL) {
        return Err(current_error(
            backend,
            "XML contribution declaration mismatch",
        ));
    }
    let mut reader = quick_xml::Reader::from_str(rest);
    reader.config_mut().trim_text(false);
    let mut seen_root = false;
    let mut depth = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Decl(_)) if !seen_root => {}
            Ok(Event::Start(start)) => {
                if !seen_root {
                    if start.name().as_ref() != b"spec" {
                        return Err(current_error(
                            backend,
                            "XML contribution root is not `spec`",
                        ));
                    }
                    seen_root = true;
                }
                depth += 1;
            }
            Ok(Event::End(_)) if seen_root => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    current_error(backend, "XML contribution closes above its root")
                })?;
                if depth == 0 {
                    let end = reader.buffer_position() as usize;
                    let document = &rest[..end];
                    cursor.advance(end);
                    return Ok(document);
                }
            }
            Ok(Event::Eof) => {
                return Err(current_error(backend, "unterminated XML contribution"));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(current_error(
                    backend,
                    format!("malformed XML contribution tape: {error}"),
                ));
            }
        }
    }
}
