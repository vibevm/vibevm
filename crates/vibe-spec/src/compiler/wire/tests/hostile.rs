//! Hostile-input amplification: a refusal must never grow with the input
//! that caused it. Every case here feeds a MULTI-MEGABYTE value through a
//! real production path and judges the RENDERED diagnostic's size, not a
//! comment claiming it is bounded.

use specmark::verifies;
use std::path::PathBuf;

use super::super::{IrWireError, decode, encode_compact};
use super::fixture::{plan_for, world};
use crate::compiler::builtin::compile_artifact;
use crate::compiler::ir::ArtifactTarget;
use crate::compiler::pass::AnyIr;

/// Big enough that any echo is unmistakable, small enough to stay fast.
const HUGE: usize = 4 * 1024 * 1024;

/// Every refusal fits well inside this; the input is 4 MiB.
const BOUND: usize = 512;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid(name: &str) -> String {
    String::from_utf8(std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn assert_bounded(name: &str, bytes: &[u8]) -> String {
    let error = decode(bytes)
        .err()
        .unwrap_or_else(|| panic!("{name}: the hostile carrier must be refused"))
        .to_string();
    assert!(
        error.len() < BOUND,
        "{name}: the refusal is {} bytes for a {HUGE}-byte input",
        error.len()
    );
    error
}

/// A 4 MiB object key repeated: the strict reader refuses it, and the
/// rendered refusal stays a short line.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_duplicate_key_is_refused_without_echoing_it() {
    let key = "k".repeat(HUGE);
    let mutated = valid("document_document.json")
        .replace("\"install\": 3", &format!("\"{key}\": 1, \"{key}\": 3"));
    let error = assert_bounded("duplicate key", mutated.as_bytes());
    assert!(error.contains("duplicate object key"), "{error}");
    assert!(error.contains(&format!("{HUGE} bytes")), "{error}");
}

/// A 4 MiB UNIQUE key is not a duplicate: it reaches the generated parse,
/// which refuses it as an unknown field — and that refusal is bounded too.
/// The reader MOVES a unique key into its set, so this path never doubles it.
#[test]
fn a_multi_megabyte_unique_key_is_refused_without_echoing_it() {
    let key = "k".repeat(HUGE);
    let mutated =
        valid("document_document.json").replace("\"install\": 3", &format!("\"{key}\": 3"));
    assert_bounded("unique key", mutated.as_bytes());
}

/// A 4 MiB ANCHOR NAME passes the scalar law (non-blank, no newline) and is
/// caught by anchor coherence — the gate that used to interpolate it whole.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_anchor_name_is_refused_without_echoing_it() {
    let anchor = "a".repeat(HUGE);
    let mut document: serde_json::Value =
        serde_json::from_str(&valid("document_document.json")).unwrap();
    document["doc"]["tree"]["anchors"] = serde_json::json!({ anchor.clone(): 1 });
    let error = assert_bounded("anchor name", &serde_json::to_vec(&document).unwrap());
    assert!(error.contains("gate `anchor-coherence`"), "{error}");
    assert!(error.contains(&format!("{HUGE} bytes")), "{error}");
}

/// A 4 MiB CONTRIBUTION ORIGIN reaches the origin-relation gate, whose
/// refusal names the origin and the relation error beside it.
#[test]
fn a_multi_megabyte_contribution_origin_is_refused_without_echoing_it() {
    let origin = format!("org.demo/{}", "z".repeat(HUGE));
    let mut document: serde_json::Value =
        serde_json::from_str(&valid("closure_artifact.json")).unwrap();
    document["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!(origin);
    let error = assert_bounded(
        "contribution origin",
        &serde_json::to_vec(&document).unwrap(),
    );
    assert!(error.contains("gate `origin-package-relation`"), "{error}");
}

/// A 4 MiB `raw` address: the address gate refuses it and names it by
/// bounded preview and true byte length.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_raw_address_is_refused_without_echoing_it() {
    let mut document: serde_json::Value =
        serde_json::from_str(&valid("source_document.json")).unwrap();
    document["doc"]["address"]["address"]["raw"] = serde_json::json!(format!(
        "spec://org.demo/lib/manual/{}.md",
        "x".repeat(HUGE)
    ));
    let error = assert_bounded("raw address", &serde_json::to_vec(&document).unwrap());
    assert!(error.contains("gate `address-reparse`"), "{error}");
    assert!(error.contains("raw address ("), "{error}");
}

/// A 4 MiB backend id fails the charset in the scalar phase; the refusal
/// names it by preview, never by echo.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_backend_id_is_refused_without_echoing_it() {
    let id = "b".repeat(HUGE);
    let mut document: serde_json::Value =
        serde_json::from_str(&valid("closure_artifact_compat.json")).unwrap();
    document["closure"]["context"]["target"] = serde_json::json!(id);
    document["closure"]["context"]["artifact"] = serde_json::json!(id);
    let error = assert_bounded("backend id", &serde_json::to_vec(&document).unwrap());
    assert!(error.contains("gate `scalar-ids`"), "{error}");
    assert!(error.contains(&format!("{HUGE} bytes")), "{error}");
}

/// A 4 MiB anchor that is SCALAR-VALID and structurally coherent: it passes
/// every conversion gate, is CONSTRUCTED, and is then refused by the
/// immutable verifier itself. That refusal is the one repair 4 bounds — the
/// verifier's `Display` would otherwise quote the whole id.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_verifier_refusal_stays_bounded() {
    let anchor = "d".repeat(HUGE);
    let mut document: serde_json::Value =
        serde_json::from_str(&valid("document_document.json")).unwrap();
    let tree = &mut document["doc"]["tree"];
    // Give the FACT node and a later heading the same huge id, keeping the
    // derived views exact: the index names the first occurrence and the
    // duplicate record really repeats, so every gate and every construction
    // law accepts it. A fact on either side is what makes the repeat a build
    // error, so only the immutable verifier objects.
    tree["nodes"][2]["id"] = serde_json::json!(anchor);
    tree["nodes"][3]["id"] = serde_json::json!(anchor);
    tree["anchors"] = serde_json::json!({ "root": 1, anchor.clone(): 2 });
    tree["duplicate_anchors"] = serde_json::json!([anchor]);

    let bytes = serde_json::to_vec(&document).unwrap();
    let error = decode(&bytes).expect_err("the verifier refuses a duplicated id");
    assert!(
        matches!(error, IrWireError::Verification(_)),
        "the refusal must come from the verifier, got {error}"
    );
    let rendered = error.to_string();
    assert!(
        rendered.len() < BOUND,
        "the verifier refusal is {} bytes for a {HUGE}-byte id",
        rendered.len()
    );
    assert!(rendered.contains("DuplicateId"), "{rendered}");
    assert!(
        !rendered.contains(&"d".repeat(200)),
        "the id itself must not ride the diagnostic"
    );
}

/// A 4 MiB `static-xml` tape whose XML is malformed: quick-xml's own error
/// text is derived from the input, so it renders through the bounded sink
/// rather than being built in full and then cut.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_multi_megabyte_xml_parse_error_stays_bounded() {
    use super::super::emitted::{digest_hex, encode_base64};
    use crate::compiler::emit::emitted_bytes_digest;

    let emitted = compile_artifact(plan_for(ArtifactTarget::StaticXml), &world()).unwrap();
    let tape = String::from_utf8(emitted.bytes().to_vec()).unwrap();
    // Keep the exact prologue, then hand the reader a huge unterminated tag
    // so the failure is quick-xml's and its message quotes the input.
    let broken = format!("{tape}<{}", "a".repeat(HUGE));
    let mut document: serde_json::Value =
        serde_json::from_slice(&encode_compact(&AnyIr::Emitted(emitted)).unwrap()).unwrap();
    document["emitted"]["bytes_b64"] = serde_json::json!(encode_base64(broken.as_bytes()));
    document["emitted"]["provenance"]["bytes_digest"] =
        serde_json::json!(digest_hex(&emitted_bytes_digest(broken.as_bytes())));
    let error = assert_bounded("xml tape", &serde_json::to_vec(&document).unwrap());
    assert!(error.contains("gate `emit-identity`"), "{error}");
}
