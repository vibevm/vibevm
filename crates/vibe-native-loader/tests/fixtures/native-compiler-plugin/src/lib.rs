#![deny(unsafe_code)]

use base64::Engine as _;
use quick_xml::Reader;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};
use vibe_ext::{
    CompileReply, CompileReplyFail, CompileReplyOk, CompileReplySkip, CompileRequest, Ir, Manifest,
    ManifestExtension,
};

/// Harmless rlib marker that forces this compiler fixture through Cargo's
/// ordinary loader dev-dependency graph.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_compiler_fixture::fixture_marker(),
///     "vibe-native-loader-compiler-fixture"
/// );
/// ```
pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-compiler-fixture"
}

/// Safe manifest accessor for fixture registration tests.
///
/// ```
/// let manifest = vibe_native_loader_compiler_fixture::fixture_manifest();
/// assert!(manifest.extensions.iter().all(|extension| {
///     extension.point.starts_with("compile:") && extension.ir_schema == Some(1)
/// }));
/// ```
pub fn fixture_manifest() -> Manifest {
    Manifest {
        extensions: [
            ("compiler-ok", "compile:pass"),
            ("compiler-skip", "compile:pass"),
            ("compiler-fail", "compile:pass"),
            ("compiler-panic", "compile:pass"),
            ("compiler-after", "compile:pass"),
            ("compiler-manager-source", "compile:source"),
            ("compiler-minify", "compile:emitted"),
        ]
        .into_iter()
        .map(|(id, point)| ManifestExtension {
            id: id.to_owned(),
            point: point.to_owned(),
            ir_schema: Some(1),
        })
        .collect(),
    }
}

pub fn handle(request: CompileRequest) -> CompileReply {
    if request.execution.id == "compiler-ok" && request.frontend_physical_stem.is_some() {
        return frontend_reply(request);
    }
    match request.execution.id.as_str() {
        "compiler-skip" => CompileReply::Skip(Box::new(CompileReplySkip {
            envelope: 1,
            message: Some("deterministic compiler skip".to_owned()),
        })),
        "compiler-fail" => CompileReply::Fail(Box::new(CompileReplyFail {
            envelope: 1,
            message: Some("deterministic compiler failure".to_owned()),
        })),
        "compiler-panic" => panic!("deterministic compiler fixture panic"),
        "compiler-manager-source" => match request.payload {
            Ir::SourceDocument(mut payload) => {
                payload
                    .doc
                    .text
                    .push_str("\nR5.5 real native source marker\n");
                CompileReply::Ok(Box::new(CompileReplyOk {
                    envelope: 1,
                    payload: Ir::SourceDocument(payload),
                    message: Some("handled compiler-manager-source".to_owned()),
                }))
            }
            _ => CompileReply::Fail(Box::new(CompileReplyFail {
                envelope: 1,
                message: Some("compiler-manager-source requires source IR".to_owned()),
            })),
        },
        "compiler-minify" | "native" => minify_reply(request),
        _ => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 1,
            payload: request.payload,
            message: Some(format!("handled {}", request.execution.id)),
        })),
    }
}

fn frontend_reply(request: CompileRequest) -> CompileReply {
    let stem = request
        .frontend_physical_stem
        .expect("frontend dispatch requires its physical stem");
    let Ir::SourceDocument(payload) = request.payload else {
        return failed("native frontend requires source document IR");
    };
    let source = payload.doc;
    let mut lines = vec![format!("# {stem} {{#root}}")];
    for line in source.text.lines().filter(|line| !line.trim().is_empty()) {
        lines.push(String::new());
        lines.push(line.to_owned());
    }
    let end = u32::try_from(lines.len()).expect("fixture document stays bounded");
    let payload = vibe_ext::__serde_json::json!({
        "shape": "document-document",
        "ir_schema": 1,
        "level": "document",
        "cardinality": "document",
        "doc": {
            "source": source,
            "tree": {
                "nodes": [
                    {"level": 0, "kind": "heading", "heading": "", "trailing": "",
                     "heading_line": 0, "span": {"start": 0, "end": end}, "children": [1]},
                    {"id": "root", "level": 1, "kind": "heading", "heading": stem,
                     "trailing": "", "heading_line": 0,
                     "span": {"start": 0, "end": end}, "parent": 0, "children": []}
                ],
                "anchors": {"root": 1},
                "duplicate_anchors": [],
                "lines": lines,
                "directives": {"aliases": {}, "directives": [], "errors": [], "in_place_uses": []}
            }
        }
    });
    let payload = match vibe_ext::__serde_json::from_value(payload) {
        Ok(payload) => payload,
        Err(_) => return failed("native frontend could not build document IR"),
    };
    CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 1,
        payload,
        message: Some("handled compiler-ok frontend".to_owned()),
    }))
}

fn minify_reply(request: CompileRequest) -> CompileReply {
    let Ir::EmittedArtifact(mut payload) = request.payload else {
        return failed("native XML minify requires emitted artifact IR");
    };
    let decoded = match base64::engine::general_purpose::STANDARD
        .decode(&payload.emitted.bytes_b64)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(decoded) => decoded,
        None => return failed("native XML minify requires canonical UTF-8 emitted bytes"),
    };
    let minified = match minify_for_backend(&payload.emitted.provenance.backend, &decoded) {
        Ok(Some(minified)) => minified,
        Ok(None) => {
            return CompileReply::Skip(Box::new(CompileReplySkip {
                envelope: 1,
                message: Some("native XML minify selects only static-xml".to_owned()),
            }));
        }
        Err(reason) => return failed(&reason),
    };
    payload.emitted.provenance.bytes_digest = emitted_bytes_digest(minified.as_bytes());
    payload.emitted.bytes_b64 = base64::engine::general_purpose::STANDARD.encode(minified);
    CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 1,
        payload: Ir::EmittedArtifact(payload),
        message: Some("native XML minify complete".to_owned()),
    }))
}

fn emitted_bytes_digest(bytes: &[u8]) -> String {
    const DOMAIN: &[u8] = b"vibe-spec/emitted-bytes/v1";
    let mut framed = Vec::with_capacity(DOMAIN.len() + bytes.len() + 16);
    framed.extend_from_slice(&(DOMAIN.len() as u64).to_le_bytes());
    framed.extend_from_slice(DOMAIN);
    framed.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    framed.extend_from_slice(bytes);
    format!("{:x}", Sha256::digest(framed))
}

fn minify_for_backend(backend: &str, tape: &str) -> Result<Option<String>, String> {
    if backend != "static-xml" {
        return Ok(None);
    }
    minify_tape(tape).map(Some)
}

/// Test-only safe projection of the independently implemented fixture logic.
#[doc(hidden)]
pub fn minify_for_test(backend: &str, tape: &str) -> Result<Option<String>, String> {
    minify_for_backend(backend, tape)
}

fn failed(message: &str) -> CompileReply {
    CompileReply::Fail(Box::new(CompileReplyFail {
        envelope: 1,
        message: Some(message.to_owned()),
    }))
}

#[derive(Default)]
struct ElementShape {
    has_element: bool,
    has_non_whitespace_text: bool,
}

fn minify_tape(tape: &str) -> Result<String, String> {
    let mut rendered = String::with_capacity(tape.len());
    let mut cursor = 0;
    while let Some(open) = next_line_comment(tape, cursor) {
        push_minified_gap(&mut rendered, &tape[cursor..open])?;
        let close = tape[open..]
            .find(" -->")
            .map(|offset| open + offset + " -->".len())
            .ok_or_else(|| format!("unterminated engine comment at byte {open}"))?;
        rendered.push_str(&tape[open..close]);
        cursor = close;
    }
    push_minified_gap(&mut rendered, &tape[cursor..])?;
    Ok(rendered)
}

fn next_line_comment(tape: &str, from: usize) -> Option<usize> {
    let mut search = from;
    while let Some(relative) = tape[search..].find("<!-- ") {
        let open = search + relative;
        let line_start = open == 0 || tape.as_bytes().get(open - 1) == Some(&b'\n');
        let generated = tape[open..].starts_with("<!-- vibe:c1 ")
            || tape[open..].starts_with("<!-- vibe:transforms ")
            || tape[open..].starts_with("<!-- vibe:transforms-pending ");
        if line_start && generated {
            return Some(open);
        }
        search = open + "<!-- ".len();
    }
    None
}

fn push_minified_gap(rendered: &mut String, gap: &str) -> Result<(), String> {
    let leading = gap.len() - gap.trim_start_matches(is_xml_whitespace).len();
    if leading == gap.len() {
        rendered.push_str(gap);
        return Ok(());
    }
    let trailing = gap.len() - gap.trim_end_matches(is_xml_whitespace).len();
    let core = &gap[leading..gap.len() - trailing];
    rendered.push_str(&gap[..leading]);
    rendered.push_str(&minify_document(core)?);
    rendered.push_str(&gap[gap.len() - trailing..]);
    Ok(())
}

fn minify_document(document: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(document);
    reader.config_mut().trim_text(false);
    let mut elements = Vec::<ElementShape>::new();
    let mut stack = Vec::<usize>::new();
    let mut candidates = Vec::<(usize, usize, usize)>::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(|error| error.to_string())?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(_) => {
                if let Some(parent) = stack.last().copied() {
                    elements[parent].has_element = true;
                }
                let index = elements.len();
                elements.push(ElementShape::default());
                stack.push(index);
            }
            Event::Empty(_) => {
                if let Some(parent) = stack.last().copied() {
                    elements[parent].has_element = true;
                }
            }
            Event::End(_) => {
                stack
                    .pop()
                    .ok_or_else(|| format!("unmatched closing element at byte {start}"))?;
            }
            Event::Text(text) => {
                if let Some(parent) = stack.last().copied() {
                    if text.iter().all(|byte| is_xml_whitespace(*byte as char)) {
                        candidates.push((start, end, parent));
                    } else {
                        elements[parent].has_non_whitespace_text = true;
                    }
                }
            }
            Event::CData(text) => {
                if let Some(parent) = stack.last().copied()
                    && !text.iter().all(|byte| is_xml_whitespace(*byte as char))
                {
                    elements[parent].has_non_whitespace_text = true;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err("unclosed XML element".to_owned());
    }
    let mut rendered = String::with_capacity(document.len());
    let mut cursor = 0;
    for (start, end, parent) in candidates {
        let shape = &elements[parent];
        if shape.has_element && !shape.has_non_whitespace_text {
            rendered.push_str(&document[cursor..start]);
            cursor = end;
        }
    }
    rendered.push_str(&document[cursor..]);
    Ok(rendered)
}

fn is_xml_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r' | '\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_minifier_preserves_mixed_content_cdata_and_comments() {
        let tape = "<!-- vibe:c1 frame -->\n<root>\n  <pure>\n    <a/>\n<!-- keep -->\n    <b/>\n  </pure>\n  <mixed>left <b/> right</mixed>\n  <data><![CDATA[ raw <xml> ]]><b/></data>\n</root>\n";
        let output = minify_tape(tape).unwrap();
        assert!(output.contains("<pure><a/><!-- keep --><b/></pure>"));
        assert!(output.contains("<mixed>left <b/> right</mixed>"));
        assert!(output.contains("<![CDATA[ raw <xml> ]]><b/>"));
        assert!(output.starts_with("<!-- vibe:c1 frame -->\n"));
        assert_eq!(minify_for_backend("static-md", tape).unwrap(), None);
    }
}

#[cfg(feature = "abi")]
vibe_ext::vibe_compile_extension!(manifest = fixture_manifest(), handler = handle);
