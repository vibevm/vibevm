//! Four generated machine-view readers: structure only, no domain authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES");

use serde::de::DeserializeOwned;
use vibe_wire::generated::machine::e1::{
    bound_page::BoundPage, composite_snapshot::CompositeSnapshot,
    entity_reference::EntityReference, source_location::SourceLocation,
};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn fixture(family: &str, class: &str, name: &str) -> String {
    format!("formats/corpora/machine-{family}/e1/{class}/{name}.json")
}

fn reader_valid<T: DeserializeOwned + serde::Serialize>(family: &str, name: &str) -> T {
    let path = fixture(family, "valid", name);
    let row: T = wire_support::round_trip(&path);
    assert_eq!(
        serde_json::to_value(&row).unwrap(),
        wire_support::read_json(&path)
    );
    row
}

fn reader_refuses<T: DeserializeOwned>(family: &str, name: &str) {
    let path = fixture(family, "structural-invalid", name);
    assert!(
        serde_json::from_value::<T>(wire_support::read_json(&path)).is_err(),
        "{path}"
    );
}

fn semantic_only<T: DeserializeOwned>(family: &str, name: &str) {
    let path = fixture(family, "semantic-invalid", name);
    assert!(
        serde_json::from_value::<T>(wire_support::read_json(&path)).is_ok(),
        "{path}: shape should pass before owner semantics"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn four_roots_roundtrip_exact_shapes_and_non_utf8_octets() {
    let first: EntityReference = reader_valid("entity-reference", "first-occurrence");
    let repeated: EntityReference =
        reader_valid("entity-reference", "repeated-logical-distinct-occurrence");
    assert_eq!(first.logical, repeated.logical);
    assert_eq!(first.captured, repeated.captured);
    assert_ne!(first.occurrence, repeated.occurrence);
    let _: EntityReference = reader_valid("entity-reference", "missing-owner-gap");
    let _: CompositeSnapshot = reader_valid("composite-snapshot", "single-owner");
    let _: CompositeSnapshot = reader_valid("composite-snapshot", "two-owner-unverified");
    let _: CompositeSnapshot = reader_valid("composite-snapshot", "missing-required-refusal");
    let first: BoundPage = reader_valid("bound-page", "first-page");
    let final_page: BoundPage = reader_valid("bound-page", "final-page");
    assert_eq!(
        first.next_cursor_id.as_deref(),
        final_page.page_cursor_id.as_deref()
    );
    let _: BoundPage = reader_valid("bound-page", "partial-children");
    let raw: SourceLocation = reader_valid("source-location", "non-utf8-original");
    assert_eq!(raw.resource_id_octets, [255, 0, 47, 92]);
    let _: SourceLocation = reader_valid("source-location", "projected-text");
    let _: SourceLocation = reader_valid("source-location", "unavailable-reverse-map");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn strict_structural_cases_refuse_unknown_missing_and_wrong_primitive() {
    for name in ["unknown-root", "wrong-ordinal-type"] {
        reader_refuses::<EntityReference>("entity-reference", name);
    }
    for name in ["unknown-witness-field", "missing-required-nullable"] {
        reader_refuses::<CompositeSnapshot>("composite-snapshot", name);
    }
    for name in ["missing-required-nullable", "unknown-row-field"] {
        reader_refuses::<BoundPage>("bound-page", name);
    }
    for name in ["octet-out-of-range", "unknown-root"] {
        reader_refuses::<SourceLocation>("source-location", name);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
fn semantic_invalid_inputs_remain_reader_valid_for_later_owner_refusal() {
    for name in ["unsupported-schema", "mismatched-observation"] {
        semantic_only::<EntityReference>("entity-reference", name);
    }
    for name in ["unsupported-schema", "conflicting-claim-outcome"] {
        semantic_only::<CompositeSnapshot>("composite-snapshot", name);
    }
    for name in ["unsupported-schema", "false-complete-redacted"] {
        semantic_only::<BoundPage>("bound-page", name);
    }
    for name in ["unsupported-schema", "noncanonical-offset"] {
        semantic_only::<SourceLocation>("source-location", name);
    }
}
