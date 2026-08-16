//! Proofs that the generated wire maps are canonically ordered — the
//! ordered-maps pass of codegen post-processing (PROP-044 §4.3: one
//! state, one byte sequence — sorted keys). Two proofs, because the
//! property has two halves the wire-parity oracles cannot see: they
//! compare `serde_json::Value`, whose equality ignores key order, so
//! they stay green across this change whatever it does to the bytes.
//!
//! The compile-time half pins the TYPE: a generated map field is passed
//! to a function accepting only `&BTreeMap<…>`. Against the pre-pass
//! tree that is a COMPILER refusal (`HashMap` is not `BTreeMap`), not a
//! value failure — the strongest red form a representation change can
//! have here.
//!
//! The behavioural half pins the BYTES: keys inserted in a non-sorted
//! order must appear ascending in the serialised string. That half has
//! no honest red form — against a `HashMap` it fails only
//! probabilistically (iteration order is randomised per process, and on
//! three keys it matches sorted order roughly one run in six) — so the
//! red is carried entirely by the compiler refusal, and this test
//! stands guard over the behaviour the compiler cannot see.

use std::collections::BTreeMap;

use vibe_wire::generated::journal::e1::journal::FeaturesEntry;

/// The ordered-map witness, exactly as the campaign record quotes it:
/// passing a generated field here compiles precisely when that field's
/// type IS the ordered map.
fn assert_ordered(_map: &std::collections::BTreeMap<String, Vec<String>>) {}

/// The compile-time proof: the generated `features` map field IS a
/// `BTreeMap<String, Vec<String>>`. Before the ordered-maps pass this
/// call site was a compile error — expected `&BTreeMap`, found
/// `&HashMap` — and that refusal is the change's red form; the green
/// run of this test is its after.
#[test]
fn the_generated_map_field_is_an_ordered_map() {
    let entry = FeaturesEntry {
        exclusive: None,
        features: Some(Box::default()),
    };
    assert_ordered(entry.features.as_deref().expect("the features map is set"));
}

/// The behavioural guard: keys inserted in a NON-sorted order (`"z"`,
/// `"m"`, `"a"`) come out ascending in the serialised bytes — the
/// one-state-one-byte-sequence promise, checked on the wire string
/// itself rather than on a reparsed value.
///
/// Recorded honestly, because overselling this test would be a lie:
/// it has no red form. Against a `HashMap` the iteration order is
/// randomised per process, and on three keys it coincides with sorted
/// order in roughly one run of six — a flaky red is not a proof. The
/// red is the compiler refusal in the test above; this test only
/// watches the behaviour once the compiler is satisfied.
#[test]
fn serialised_keys_appear_ascending_when_inserted_unsorted() {
    let entry = FeaturesEntry {
        exclusive: None,
        features: Some(Box::new(BTreeMap::from([
            ("z".to_string(), vec!["zebra".to_string()]),
            ("m".to_string(), vec!["mango".to_string()]),
            ("a".to_string(), vec!["apple".to_string()]),
        ]))),
    };
    let wire = serde_json::to_string(&entry).expect("FeaturesEntry serialises");
    // Positions in the STRING: the promise is about bytes. A quoted
    // single-letter key cannot match the longer value strings — `"zebra"`
    // never contains the substring `"z"` — so each find lands on a key.
    let z = wire.find("\"z\"").expect("the z key is on the wire");
    let m = wire.find("\"m\"").expect("the m key is on the wire");
    let a = wire.find("\"a\"").expect("the a key is on the wire");
    assert!(
        a < m && m < z,
        "map keys must serialise ascending (a < m < z), got {wire}"
    );
}
