//! Every valid corpus document round-trips wire→domain→wire exactly, and
//! pretty and compact decode to the same generated value.

use specmark::verifies;
use std::path::PathBuf;

use super::super::decode;
use super::super::decode_unverified;
use super::super::encode_compact;
use super::super::encode_pretty;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// wire → domain → wire, compared as JSON values: the authored corpus bytes
/// against the re-encoded conversion output.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn every_valid_document_round_trips_wire_domain_wire() {
    for name in valid_names() {
        let bytes = std::fs::read(corpus().join("valid").join(&name)).unwrap();
        let decoded = decode(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let compact = encode_compact(&decoded).unwrap();
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let round: serde_json::Value = serde_json::from_slice(&compact).unwrap();
        assert_eq!(round, original, "{name} loses data across the conversion");
    }
}

/// Pretty and compact are the same generated value: one projection, two
/// serializer choices.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn pretty_and_compact_decode_to_the_same_generated_value() {
    for name in valid_names() {
        let bytes = std::fs::read(corpus().join("valid").join(&name)).unwrap();
        let decoded = decode(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let compact: serde_json::Value =
            serde_json::from_slice(&encode_compact(&decoded).unwrap()).unwrap();
        let pretty: serde_json::Value =
            serde_json::from_slice(&encode_pretty(&decoded).unwrap()).unwrap();
        assert_eq!(compact, pretty, "{name}");
    }
}

/// The strict reader itself is red on the malformed corpus fixtures, and the
/// reader's refusal surfaces as the typed Reader error.
#[test]
fn reader_reds_stay_red_through_decode() {
    for name in [
        "level_mismatch.json",
        "cardinality_mismatch.json",
        "unknown_shape.json",
        "unknown_field.json",
    ] {
        let bytes = std::fs::read(corpus().join("invalid").join(name)).unwrap();
        assert!(decode(&bytes).is_err(), "{name} must be red");
    }
}

/// The conversion itself is faithful on every corpus document, verifier or
/// not: wire → domain → wire is JSON-identical through the typed path.
#[test]
fn the_typed_conversion_alone_is_json_faithful_on_every_document() {
    for name in valid_names() {
        let bytes = std::fs::read(corpus().join("valid").join(&name)).unwrap();
        let decoded = decode_unverified(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let compact = encode_compact(&decoded).unwrap();
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let round: serde_json::Value = serde_json::from_slice(&compact).unwrap();
        assert_eq!(round, original, "{name} loses data before verification");
    }
}

/// The repaired corpus `input_digest` is the production link pass's own
/// replay digest, recomputed here through the landed `derive_result` — not a
/// copied constant (R6.2b repair 1).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_corpus_link_digest_is_the_production_replay_digest() {
    use crate::compiler::ir::LinkState;
    use crate::compiler::link::derived_link_digest_for_test;
    use crate::compiler::pass::AnyIr;

    let bytes = std::fs::read(corpus().join("valid").join("closure_artifact.json")).unwrap();
    let ir = decode(&bytes).unwrap();
    let AnyIr::Closure(closure) = &ir else {
        panic!("closure_artifact.json is the closure carrier");
    };
    let LinkState::Linked(result) = &closure.link else {
        panic!("the corpus closure is linked");
    };
    let recomputed = derived_link_digest_for_test(closure);
    let hex: String = recomputed
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let carried: String = result
        .input_digest
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        hex, carried,
        "the corpus digest must equal the production replay"
    );
    // And the landed full verifier (which replays the digest) accepted it above.
}
