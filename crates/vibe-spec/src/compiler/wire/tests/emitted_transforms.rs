//! The T9 emitted-transform chain on the wire: it round-trips byte-exact in
//! APPLICATION order, it is REQUIRED rather than defaulted, an empty chain is
//! the honest spelling of "no emitted transform changed these bytes", and every
//! element obeys the same scalar law `producer` obeys.
//!
//! Two corpus documents carry the property rather than one hand-built value:
//! `emitted_artifact.json` is the untransformed tape with an empty chain, and
//! `emitted_artifact_transformed.json` is the same artifact after two emitted
//! transforms rewrote it — different bytes, a recomputed digest, and the two
//! pass names in the order they applied. The invalid sibling
//! `emitted_transform_name_blank.json` pins the refusal.

use std::path::PathBuf;

use super::super::super::pass::AnyIr;
use super::super::{IrWireError, decode, encode_compact};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid(name: &str) -> Vec<u8> {
    std::fs::read(corpus().join("valid").join(name)).unwrap()
}

fn invalid(name: &str) -> Vec<u8> {
    std::fs::read(corpus().join("invalid").join(name)).unwrap()
}

/// The chain of one decoded emitted carrier, as owned strings.
fn chain_of(bytes: &[u8]) -> Vec<String> {
    let ir = decode(bytes).unwrap_or_else(|error| panic!("the carrier decodes: {error}"));
    let AnyIr::Emitted(emitted) = &ir else {
        panic!("the emitted corpus documents are emitted carriers")
    };
    emitted
        .provenance()
        .emitted_transforms
        .iter()
        .map(|name| name.as_str().to_string())
        .collect()
}

/// The chain crosses the strict round-trip exactly, and the whole document
/// re-encodes to the authored bytes — so nothing is dropped, reordered or
/// re-derived on the way through.
#[test]
fn a_carried_transform_chain_survives_the_strict_round_trip_exactly() {
    let bytes = valid("emitted_artifact_transformed.json");
    assert_eq!(
        chain_of(&bytes),
        [
            "transform:emitted:org.demo/tools#append",
            "transform:emitted:org.demo/tools#again",
        ]
    );

    let ir = decode(&bytes).unwrap();
    let round: serde_json::Value = serde_json::from_slice(&encode_compact(&ir).unwrap()).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(round, authored);
}

/// APPLICATION order, not a sorted set: the corpus chain is deliberately
/// authored in an order that is not the sorted one, so a conversion that
/// sorted or deduplicated the list would be caught here rather than staying
/// invisible behind a happily-alphabetical fixture.
#[test]
fn the_chain_is_application_order_and_is_never_sorted() {
    let carried = chain_of(&valid("emitted_artifact_transformed.json"));
    let mut sorted = carried.clone();
    sorted.sort();
    assert_ne!(
        carried, sorted,
        "the fixture must be authored out of sorted order to be able to catch a sort"
    );
}

/// The untransformed tape spells the absence as an EMPTY chain — the whole
/// point of the member being empty at emission: an artifact nothing rewrote is
/// spelled exactly as it was before this member existed.
#[test]
fn an_untransformed_artifact_carries_an_empty_chain() {
    assert!(chain_of(&valid("emitted_artifact.json")).is_empty());
}

/// A blank element is refused at the scalar-identity gate, corpus-pinned: a
/// blank pass name can neither position nor attribute anything, so it must not
/// cross into a provenance record that exists to be read later.
#[test]
fn a_blank_transform_name_is_refused_by_the_scalar_gate() {
    let error = decode(&invalid("emitted_transform_name_blank.json")).unwrap_err();
    let IrWireError::Gate { gate, detail } = &error else {
        panic!("the spelling phase owns the refusal, got {error:?}")
    };
    assert_eq!(*gate, "scalar-ids");
    assert!(detail.contains("emitted transform name"), "{detail}");
}

/// The same law over the rest of the scalar contract, and a forward-slashed
/// control through the same call so the red is the spelling and not the edit.
#[test]
fn newline_and_nul_bearing_transform_names_are_refused() {
    for spelling in ["transform:emitted:a\nb", "transform:emitted:a\0b", "  ", ""] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&valid("emitted_artifact_transformed.json")).unwrap();
        document["emitted"]["provenance"]["emitted_transforms"][1] = serde_json::json!(spelling);
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Gate { gate, detail } = &error else {
            panic!("{spelling:?}: the spelling phase owns it, got {error:?}")
        };
        assert_eq!(*gate, "scalar-ids");
        assert!(detail.contains("emitted transform name"), "{detail}");
    }

    let mut control: serde_json::Value =
        serde_json::from_slice(&valid("emitted_artifact_transformed.json")).unwrap();
    control["emitted"]["provenance"]["emitted_transforms"][1] =
        serde_json::json!("transform:emitted:org.demo/tools#other");
    decode(&serde_json::to_vec(&control).unwrap()).unwrap();
}

/// A carrier that omits the member is refused at the strict generated reader.
/// A defaulted empty chain would silently claim that nothing rewrote bytes that
/// something did — the one lie this record exists to prevent.
#[test]
fn a_missing_chain_is_refused_rather_than_defaulted() {
    for name in ["emitted_artifact.json", "emitted_artifact_transformed.json"] {
        let mut document: serde_json::Value = serde_json::from_slice(&valid(name)).unwrap();
        assert!(
            document["emitted"]["provenance"]
                .as_object_mut()
                .unwrap()
                .remove("emitted_transforms")
                .is_some(),
            "{name} must really carry the member to drop"
        );
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Reader { detail } = &error else {
            panic!("{name}: the refusal is the strict reader's, got {error:?}")
        };
        assert!(detail.contains("missing field"), "{name}: {detail}");
        assert!(detail.contains("emitted_transforms"), "{name}: {detail}");
    }
}

/// The typed value survives domain → wire → domain unchanged, chain included:
/// the wire carries the record, it does not merely tolerate it.
#[test]
fn the_chain_round_trips_domain_wire_domain_as_the_same_value() {
    let ir = decode(&valid("emitted_artifact_transformed.json")).unwrap();
    let back = decode(&encode_compact(&ir).unwrap()).unwrap();
    let (AnyIr::Emitted(original), AnyIr::Emitted(actual)) = (&ir, &back) else {
        panic!("the emitted carrier stays an emitted carrier")
    };
    assert_eq!(original, actual, "the whole emitted value survives");
    assert_eq!(
        actual.provenance().emitted_transforms.len(),
        2,
        "and the surviving value really carried a nonempty chain"
    );
}
