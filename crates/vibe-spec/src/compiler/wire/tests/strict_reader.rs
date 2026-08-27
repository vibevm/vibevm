//! Reader strictness: a repeated object key never reaches the generated
//! types. Direct deserialization would silently last-wins every
//! `BTreeMap`/`values` field, so the byte reader rejects the duplicate before
//! any parsing — nothing normalizes silently before domain conversion.

use specmark::verifies;
use std::path::PathBuf;

use super::super::{IrWireError, decode};

/// A literal backslash, spelled without a nested escape (readable in tests).
const BS: &str = "\u{5c}";

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

/// Every map family plus an ordinary struct member, each with a DIFFERENT
/// first value so a last-wins reader would take a different carrier than a
/// first-wins one — either way, the strict reader refuses the bytes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_duplicate_object_key_is_red_for_every_map_family() {
    let names = [
        "duplicate_key_struct_member.json",
        "duplicate_key_directive_aliases.json",
        "duplicate_key_anchors.json",
        "duplicate_key_closure_aliases.json",
        "duplicate_key_embed_documents.json",
        "duplicate_key_source_documents.json",
        "duplicate_key_source_expansions.json",
    ];
    for name in names {
        let bytes = std::fs::read(corpus().join("invalid").join(name)).unwrap();
        let error = decode(&bytes).unwrap_err().to_string();
        assert!(
            error.contains("duplicate object key"),
            "{name} must be red at the strict reader, got {error}"
        );
    }
}

/// The typed variant carries the refusal, so callers can classify without
/// parsing a rendered string.
#[test]
fn the_duplicate_key_refusal_is_typed() {
    let bytes = std::fs::read(corpus().join("invalid").join("duplicate_key_anchors.json")).unwrap();
    match decode(&bytes) {
        Err(IrWireError::StrictReader { detail }) => {
            assert!(
                detail.contains("duplicate object key (7 bytes, starts `install`)"),
                "{detail}"
            );
        }
        other => panic!("expected the typed strict-reader refusal, got {other:?}"),
    }
}

/// Trailing garbage after the JSON document is a reader error too (the
/// strict walk ends the stream before the generated parse).
#[test]
fn trailing_bytes_after_the_document_are_red() {
    let mut bytes = std::fs::read(corpus().join("valid").join("source_document.json")).unwrap();
    bytes.extend_from_slice(b" {}");
    assert!(decode(&bytes).is_err());
}

/// A UNIQUE escaped key is legal: it unescapes to the same key the generated
/// map holds, and the carrier decodes and re-encodes to the same JSON value.
#[test]
fn a_unique_escaped_key_parses_and_round_trips() {
    let original = std::fs::read(corpus().join("valid").join("document_document.json")).unwrap();
    let escaped_install = format!("inst{BS}u0061ll");
    let mutated = String::from_utf8(original.clone())
        .unwrap()
        .replace("\"install\": 3", &format!("\"{escaped_install}\": 3"));
    assert_ne!(
        mutated.as_bytes(),
        original.as_slice(),
        "the escape must land"
    );
    let decoded = decode(mutated.as_bytes())
        .unwrap_or_else(|error| panic!("an escaped unique key is legal: {error}"));
    let round: serde_json::Value =
        serde_json::from_slice(&super::super::encode_compact(&decoded).unwrap()).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(round, authored, "the escape is spelling, not value");
}

/// A literal key plus its `\u`-escaped equivalent is the SAME object key —
/// the typed duplicate refusal, not two legal keys.
#[test]
fn a_literal_and_escaped_spelling_of_one_key_is_a_duplicate() {
    let original = std::fs::read(corpus().join("valid").join("document_document.json")).unwrap();
    let escaped_install = format!("inst{BS}u0061ll");
    let mutated = String::from_utf8(original).unwrap().replace(
        "\"install\": 3",
        &format!("\"install\": 2, \"{escaped_install}\": 3"),
    );
    match decode(mutated.as_bytes()) {
        Err(IrWireError::StrictReader { detail }) => {
            assert!(
                detail.contains("duplicate object key (7 bytes, starts `install`)"),
                "{detail}"
            );
        }
        other => panic!("expected the typed refusal, got {other:?}"),
    }
}

/// A long escaped key keeps the diagnostic bounded.
#[test]
fn a_long_escaped_duplicate_key_is_reported_bounded() {
    let original = std::fs::read(corpus().join("valid").join("document_document.json")).unwrap();
    let long_key: String = "k".repeat(200);
    let escaped_twin = format!("{BS}u006b{}", "k".repeat(199));
    let mutated = String::from_utf8(original).unwrap().replace(
        "\"install\": 3",
        &format!("\"{long_key}\": 1, \"{escaped_twin}\": 3"),
    );
    // `kkk…k` (200) vs its escaped twin — the same unescaped key.
    match decode(mutated.as_bytes()) {
        Err(IrWireError::StrictReader { detail }) => {
            assert!(
                detail.contains('…') && detail.contains("200 bytes"),
                "{detail}"
            );
            // The 200-character key is capped by the shared preview; the
            // whole diagnostic stays a single bounded line (serde_json
            // appends a short "at line L column C" position).
            assert!(
                detail.chars().count() < 160,
                "the preview stays bounded: {detail}"
            );
        }
        other => panic!("expected the typed refusal, got {other:?}"),
    }
}
