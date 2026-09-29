use super::*;
use crate::generated::machine::e1::source_location::{
    LocationProjectedText, LocationUnavailable as WireLocationUnavailable,
};

fn entity_fixture(name: &str) -> EntityReference {
    let text = match name {
        "mismatch" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-entity-reference/e1/semantic-invalid/mismatched-observation.json"
        )),
        "epoch" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-entity-reference/e1/semantic-invalid/unsupported-schema.json"
        )),
        _ => unreachable!(),
    };
    serde_json::from_str(text).unwrap()
}

fn location_fixture(name: &str) -> SourceLocation {
    let text = match name {
        "offset" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-source-location/e1/semantic-invalid/noncanonical-offset.json"
        )),
        "epoch" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-source-location/e1/semantic-invalid/unsupported-schema.json"
        )),
        _ => unreachable!(),
    };
    serde_json::from_str(text).unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 1
)]
fn repeated_logical_entity_retains_two_distinct_occurrences() {
    let mut first = entity_fixture("epoch");
    first.schema = 1;
    let mut repeated = first.clone();
    repeated.occurrence.ordinal = "1".to_owned();
    let first = first.validate_machine_entity_reference().unwrap();
    let repeated = repeated.validate_machine_entity_reference().unwrap();
    assert_eq!(first.logical(), repeated.logical());
    assert_eq!(first.captured(), repeated.captured());
    assert_ne!(first.occurrence(), repeated.occurrence());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 1
)]
fn mismatched_observation_epoch_and_bad_ordinals_refuse() {
    let mismatch = entity_fixture("mismatch");
    assert_eq!(
        mismatch
            .validate_machine_entity_reference()
            .unwrap_err()
            .field,
        "occurrence.observed"
    );
    let mut value = entity_fixture("epoch");
    assert_eq!(
        value.validate_machine_entity_reference().unwrap_err().field,
        "schema"
    );
    value.schema = 1;
    for ordinal in ["01", "-1", "18446744073709551616"] {
        value.occurrence.ordinal = ordinal.to_owned();
        assert_eq!(
            value.validate_machine_entity_reference().unwrap_err().field,
            "occurrence.ordinal"
        );
    }
    value.occurrence.ordinal = "0".to_owned();
    value.reference_id = "row\0unsafe".to_owned();
    assert_eq!(
        value.validate_machine_entity_reference().unwrap_err().field,
        "reference_id"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn non_utf8_resource_and_exact_original_byte_bounds_survive() {
    let mut value = location_fixture("offset");
    assert_eq!(
        value.validate_machine_source_location().unwrap_err().field,
        "location.start"
    );
    let Location::OriginalBytes(row) = &mut value.location else {
        panic!("fixture lost original byte basis")
    };
    row.start = "1".to_owned();
    let LocationObservation::Available(location) =
        value.validate_machine_source_location().unwrap()
    else {
        panic!("known original bytes became unavailable")
    };
    assert_eq!(location.resource().bytes(), &[255, 0, 47, 92]);
    if let Location::OriginalBytes(row) = &mut value.location {
        row.end = "18446744073709551616".to_owned();
    }
    assert_eq!(
        value.validate_machine_source_location().unwrap_err().field,
        "location.end"
    );
    if let Location::OriginalBytes(row) = &mut value.location {
        row.end = "0".to_owned();
    }
    assert_eq!(
        value.validate_machine_source_location().unwrap_err().field,
        "location.span"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn projected_position_never_becomes_original_without_reverse_map() {
    let mut value = location_fixture("epoch");
    value.schema = 1;
    value.location = Location::ProjectedText(Box::new(LocationProjectedText {
        projection: "xml-to-markdown".to_owned(),
        encoding: "utf-8".to_owned(),
        start_line: "1".to_owned(),
        start_column: "1".to_owned(),
        end_line: "2".to_owned(),
        end_column: "3".to_owned(),
    }));
    let LocationObservation::Available(projected) =
        value.validate_machine_source_location().unwrap()
    else {
        panic!("projected location was unavailable")
    };
    assert_eq!(projected.resource().bytes(), &[255, 0, 47, 92]);
    assert!(matches!(
        projected.original_from_projected(),
        LocationObservation::Unavailable {
            reason: LocationUnavailable::MissingReverseMap,
            ..
        }
    ));
    value.location = Location::Unavailable(Box::new(WireLocationUnavailable {
        reason: Reason::RawBytesUnavailable,
    }));
    let LocationObservation::Unavailable {
        resource,
        reason: LocationUnavailable::RawBytesUnavailable,
        ..
    } = value.validate_machine_source_location().unwrap()
    else {
        panic!("unavailable bytes were fabricated as a location")
    };
    assert_eq!(resource.bytes(), &[255, 0, 47, 92]);
}
