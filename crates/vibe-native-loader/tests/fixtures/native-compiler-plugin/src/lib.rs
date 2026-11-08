#![deny(unsafe_code)]

use base64::Engine as _;
use quick_xml::Reader;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicUsize, Ordering};

static RAW_INVOCATIONS: AtomicUsize = AtomicUsize::new(0);
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
/// assert!(manifest.extensions.iter().all(|extension| extension.point.starts_with("compile:")));
/// assert!(manifest.extensions.iter().any(|extension| {
///     extension.id == "compiler-schema-one" && extension.ir_schema == Some(1)
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
            ("compiler-counter", "compile:pass"),
            ("compiler-schema-one", "compile:pass"),
            ("compiler-sleep", "compile:pass"),
        ]
        .into_iter()
        .map(|(id, point)| ManifestExtension {
            id: id.to_owned(),
            point: point.to_owned(),
            // Controlled old declaration in a CURRENT fixture, not an old binary.
            ir_schema: Some(if id == "compiler-schema-one" { 1 } else { 2 }),
        })
        .collect(),
    }
}

/// Dispatch one generated compiler request through the fixture handler.
///
/// ```
/// use vibe_ext::{CompileReply, CompileRequest};
/// use vibe_native_loader_compiler_fixture::handle;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let request: CompileRequest = vibe_ext::__serde_json::from_value(
///     vibe_ext::__serde_json::json!({
///         "envelope": 2,
///         "execution": {"id": "compiler-skip", "package": "org.example/compiler", "config": {}},
///         "io": {"scratch": ".vibe/compile/run"},
///         "payload": {
///             "shape": "source-document", "ir_schema": 2, "level": "source",
///             "cardinality": "document",
///             "doc": {
///                 "address": {"kind": "static-entry", "origin": "fixture", "path": "fixture.md"},
///                 "format": "markdown", "text": "fixture compiler input",
///                 "subject": {"declared_path": "fixture.md", "provider": {"kind": "unclaimed"}}
///             }
///         },
///         "point": "compile:pass",
///         "project": {"root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
///             "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]},
///         "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
///     }),
/// )?;
/// assert!(matches!(handle(request), CompileReply::Skip(_)));
/// # Ok(())
/// # }
/// ```
pub fn handle(request: CompileRequest) -> CompileReply {
    if request.execution.id == "compiler-ok" && request.frontend_physical_stem.is_some() {
        return frontend_reply(request);
    }
    match request.execution.id.as_str() {
        "compiler-counter" => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 2,
            payload: request.payload,
            message: Some(format!(
                "raw invocations: {}",
                RAW_INVOCATIONS.load(Ordering::SeqCst)
            )),
        })),
        "compiler-skip" => CompileReply::Skip(Box::new(CompileReplySkip {
            envelope: 2,
            message: Some("deterministic compiler skip".to_owned()),
        })),
        "compiler-fail" => CompileReply::Fail(Box::new(CompileReplyFail {
            envelope: 2,
            message: Some("deterministic compiler failure".to_owned()),
        })),
        "compiler-panic" => panic!("deterministic compiler fixture panic"),
        // A synchronous call that only a process-level stop can interrupt.
        "compiler-sleep" => {
            std::thread::sleep(std::time::Duration::from_secs(30));
            CompileReply::Ok(Box::new(CompileReplyOk {
                envelope: 2,
                payload: request.payload,
                message: Some("handled compiler-sleep".to_owned()),
            }))
        }
        "compiler-manager-source" => match request.payload {
            Ir::SourceDocument(mut payload) => {
                payload
                    .doc
                    .text
                    .push_str("\nR5.5 real native source marker\n");
                CompileReply::Ok(Box::new(CompileReplyOk {
                    envelope: 2,
                    payload: Ir::SourceDocument(payload),
                    message: Some("handled compiler-manager-source".to_owned()),
                }))
            }
            _ => CompileReply::Fail(Box::new(CompileReplyFail {
                envelope: 2,
                message: Some("compiler-manager-source requires source IR".to_owned()),
            })),
        },
        "compiler-minify" | "native" => minify_reply(request),
        _ => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 2,
            payload: request.payload,
            message: Some(format!("handled {}", request.execution.id)),
        })),
    }
}

fn frontend_reply(request: CompileRequest) -> CompileReply {
    let invalid = request
        .execution
        .config
        .get("invalid_frontend")
        .and_then(Option::as_ref)
        == Some(&vibe_ext::__serde_json::Value::Bool(true));
    let Some(stem) = request.frontend_physical_stem else {
        panic!("frontend dispatch requires its physical stem");
    };
    let Ir::SourceDocument(payload) = request.payload else {
        return failed("native frontend requires source document IR");
    };
    let source = payload.doc;
    let mut lines = vec![format!("# {stem} {{#root}}")];
    for line in source.text.lines().filter(|line| !line.trim().is_empty()) {
        lines.push(String::new());
        lines.push(line.to_owned());
    }
    let end = match u32::try_from(lines.len()) {
        Ok(end) => end,
        Err(error) => panic!("fixture document stays bounded: {error:?}"),
    };
    let mut payload = vibe_ext::__serde_json::json!({
        "shape": "document-document",
        "ir_schema": 2,
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
    if invalid {
        payload["doc"]["tree"]["nodes"][0]["span"]["end"] = 999.into();
    }
    let payload = match vibe_ext::__serde_json::from_value(payload) {
        Ok(payload) => payload,
        Err(_) => return failed("native frontend could not build document IR"),
    };
    CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 2,
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
                envelope: 2,
                message: Some("native XML minify selects only static-xml".to_owned()),
            }));
        }
        Err(reason) => return failed(&reason),
    };
    payload.emitted.provenance.bytes_digest = emitted_bytes_digest(minified.as_bytes());
    payload.emitted.bytes_b64 = base64::engine::general_purpose::STANDARD.encode(minified);
    CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 2,
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
///
/// ```
/// use vibe_native_loader_compiler_fixture::minify_for_test;
///
/// # fn main() -> Result<(), String> {
/// let output = minify_for_test("static-xml", "<root>\n  <child/>\n</root>")?;
/// assert_eq!(output.as_deref(), Some("<root><child/></root>"));
/// # Ok(())
/// # }
/// ```
#[doc(hidden)]
pub fn minify_for_test(backend: &str, tape: &str) -> Result<Option<String>, String> {
    minify_for_backend(backend, tape)
}

/// Synthetic structural carrier for real loader/minifier transport tests.
/// It deliberately does not attest a compiler-produced lane or body origin.
///
/// ```
/// use vibe_ext::Ir;
/// let Ir::EmittedArtifact(value) =
///     vibe_native_loader_compiler_fixture::emitted_payload_for_test("<spec/>")?
/// else { panic!("fixture carrier"); };
/// assert_eq!(value.ir_schema, 2);
/// # Ok::<(), vibe_ext::__serde_json::Error>(())
/// ```
#[doc(hidden)]
pub fn emitted_payload_for_test(text: &str) -> Result<Ir, vibe_ext::__serde_json::Error> {
    vibe_ext::__serde_json::from_value(vibe_ext::__serde_json::json!({
        "shape": "emitted-artifact", "ir_schema": 2, "level": "emitted", "cardinality": "artifact",
        "emitted": {
            "provenance": {
                "context": { "artifact": "fixture-xml", "target": "static-xml",
                    "frame": { "kind": "static-lane", "generated_path": "boot/STATIC.xml", "source_root": "vibevm/vibespecs" },
                    "mode": "qualify-per-node" },
                "backend": "static-xml", "producer": "emit:static-xml",
                "source_lane_digest": "0".repeat(64), "renames": [], "contributions": [], "emitted_transforms": [],
                "bytes_digest": emitted_bytes_digest(text.as_bytes())
            },
            "bytes_b64": base64::engine::general_purpose::STANDARD.encode(text)
        }
    }))
}

fn failed(message: &str) -> CompileReply {
    CompileReply::Fail(Box::new(CompileReplyFail {
        envelope: 2,
        message: Some(message.to_owned()),
    }))
}

#[derive(Default)]
struct ElementShape {
    has_element: bool,
    has_non_whitespace_text: bool,
}

fn minify_tape(tape: &str) -> Result<String, String> {
    // The same token owner can read the complete multi-document artifact.
    // Outside-element framing remains bytes; only eligible interior whitespace
    // is removed. A paragraph comment or CDATA sample cannot split a document.
    minify_document(tape)
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
    fn line_leading_generated_embeds_and_cdata_are_not_outer_tape_frames() {
        let paragraph = "<p>alpha\n<!-- vibe:c1 embed: spec://org.probe/p/payload#x -->\nβ\n<!-- vibe:c1 /embed: spec://org.probe/p/payload#x -->\nomega</p>";
        let literal =
            "<fence><![CDATA[\n<!-- vibe:c1 vibe:static literal — docs/example -->\n]]></fence>";
        let tape = format!(
            "<!-- vibe:c1 frame-one -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\">\n  {paragraph}\n  {literal}\n</spec>\n<!-- vibe:c1 frame-two -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\">\n  <p>second</p>\n</spec>\n"
        );
        let output = minify_tape(&tape).expect(
            "complete XML contributions must not be split at paragraph or CDATA marker lookalikes",
        );
        assert!(
            output.contains(paragraph),
            "paragraph text/comments remain exact: {output}"
        );
        assert!(output.contains(literal), "CDATA remains literal: {output}");
        assert_eq!(output.matches("<!-- vibe:c1 frame-one -->").count(), 1);
        assert_eq!(output.matches("<!-- vibe:c1 frame-two -->").count(), 1);
        assert_eq!(output.matches("<?xml version=\"1.0\"?>").count(), 2);
        assert!(
            !output.contains("\n  <p>second"),
            "ordinary element indentation is still minified"
        );
    }

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

// Fixture-only dispatch instrumentation. The incumbent ABI emitter still owns
// ABI1 pointers, panic containment and exact-once frees. Generated readers and
// existing behavior admission remain the protocol owners, just as in the SDK
// compiler macro; the counter is BEFORE deserialization/handler dispatch.
#[cfg(feature = "abi")]
fn __vibe_ext_manifest_value() -> Manifest {
    fixture_manifest()
}

#[cfg(feature = "abi")]
fn __vibe_ext_dispatch(bytes: &[u8]) -> Option<Vec<u8>> {
    RAW_INVOCATIONS.fetch_add(1, Ordering::SeqCst);
    let request: CompileRequest = vibe_ext::__serde_json::from_slice(bytes).ok()?;
    let shape = vibe_ext::__native_compile::validate_request(&request).ok()?;
    let pass_transition = request.point == "compile:pass";
    let reply = handle(request);
    if pass_transition {
        vibe_ext::__native_compile::validate_reply(&reply).ok()?;
    } else {
        vibe_ext::__native_compile::validate_reply_for_shape(shape, &reply).ok()?;
    }
    vibe_ext::__serde_json::to_vec(&reply).ok()
}

#[cfg(feature = "abi")]
vibe_ext::__vibe_ext_emit_abi!(
    panic_message = "compiler fixture requires unwind panic containment"
);
