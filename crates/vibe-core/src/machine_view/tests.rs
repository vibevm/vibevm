use super::*;

fn r(value: &str) -> OpaqueText {
    OpaqueText::new(value).unwrap()
}
fn resource(bytes: &[u8]) -> OpaqueResourceId {
    OpaqueResourceId::new(bytes.to_vec()).unwrap()
}
fn projected_span() -> TextSpan {
    TextSpan::new(
        TextPosition::new(3, 2).unwrap(),
        TextPosition::new(3, 11).unwrap(),
    )
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 2
)]
fn repeated_inclusion_shares_owner_address_but_not_occurrence() {
    let logical = LogicalIdentity::OwnerMinted(LogicalOwnerAddress::supplied(
        r("spec-owner"),
        r("spec://org.example/a#rule"),
    ));
    let captured = CapturedSourceRef::supplied(r("source-sha"), r("revision-a"));
    let observed = ObservationRef::supplied(r("projection-a"), r("observation-a"));
    let a = EntityReference::new(
        logical.clone(),
        captured.clone(),
        observed.clone(),
        ContributionOccurrenceRef::supplied(observed.clone(), r("occ-a"), r("include-a"), 0),
    )
    .unwrap();
    let b = EntityReference::new(
        logical,
        captured,
        observed.clone(),
        ContributionOccurrenceRef::supplied(observed, r("occ-b"), r("include-b"), 1),
    )
    .unwrap();
    assert_eq!(a.logical(), b.logical());
    assert_eq!(a.captured(), b.captured());
    assert_eq!(a.observed(), b.observed());
    assert_eq!(a.occurrence().occurrence_id().as_str(), "occ-a");
    assert_eq!(b.occurrence().occurrence_id().as_str(), "occ-b");
    assert_ne!(a.occurrence(), b.occurrence());
    assert_ne!(a, b);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 2
)]
fn missing_owner_and_foreign_occurrence_are_typed_not_minted() {
    assert!(OpaqueText::new("  \n").is_err());
    let logical = LogicalIdentity::ExtractionGap {
        reason: r("owner address unavailable"),
    };
    let captured = CapturedSourceRef::supplied(r("capture"), r("rev"));
    let observed = ObservationRef::supplied(r("projection"), r("observation"));
    let wrong = ContributionOccurrenceRef::supplied(
        ObservationRef::supplied(r("projection"), r("other-observation")),
        r("occ-a"),
        r("include"),
        0,
    );
    assert!(
        EntityReference::new(logical.clone(), captured.clone(), observed.clone(), wrong).is_err()
    );
    let wrong_projection = ContributionOccurrenceRef::supplied(
        ObservationRef::supplied(r("other-projection"), r("observation")),
        r("occ-a"),
        r("include"),
        0,
    );
    assert!(
        EntityReference::new(
            logical.clone(),
            captured.clone(),
            observed.clone(),
            wrong_projection
        )
        .is_err()
    );
    let entity = EntityReference::new(
        logical,
        captured,
        observed.clone(),
        ContributionOccurrenceRef::supplied(observed, r("occ-a"), r("include"), 0),
    )
    .unwrap();
    assert!(matches!(
        entity.logical(),
        LogicalIdentity::ExtractionGap { .. }
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn opaque_non_utf8_resource_and_original_byte_basis_survive() {
    let raw = vec![0xff, b'/', 0, b'\\'];
    let id = OpaqueResourceId::new(raw.clone()).unwrap();
    assert_eq!(id.bytes(), raw);
    let LocationObservation::Available(location) = SourceLocation::original(
        id.clone(),
        r("captured-r1"),
        Some(r("iso-8859-1")),
        ByteSpan::new(9, 19).unwrap(),
    ) else {
        panic!("original byte span was not retained")
    };
    assert_eq!(location.resource(), &id);
    assert_eq!(location.captured_revision().as_str(), "captured-r1");
    assert!(
        matches!(location.basis(), PositionBasis::OriginalBytes { span, .. } if span.start() == 9 && span.end() == 19)
    );
    assert_eq!(
        location.original_from_projected(),
        LocationObservation::Available(location.clone())
    );
    assert!(OpaqueResourceId::new(Vec::new()).is_err());
    assert!(ByteSpan::new(20, 19).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn projected_markdown_cannot_be_reported_as_original_xml_without_owner_map() {
    let markdown = resource(&[0xff, b'm', b'd']);
    let LocationObservation::Available(projected) = SourceLocation::projected(
        markdown.clone(),
        r("capture-rev"),
        r("markdown-projection-r1"),
        Some(r("utf-8")),
        projected_span(),
    ) else {
        panic!("projection was not represented")
    };
    assert!(matches!(
        projected.basis(),
        PositionBasis::ProjectedText { .. }
    ));
    assert_eq!(
        projected.original_from_projected(),
        LocationObservation::Unavailable {
            resource: markdown.clone(),
            captured_revision: r("capture-rev"),
            reason: LocationUnavailable::MissingReverseMap,
        }
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn unknown_encoding_missing_bytes_and_invalid_text_ranges_are_explicit() {
    assert!(TextPosition::new(0, 1).is_err());
    assert!(
        TextSpan::new(
            TextPosition::new(4, 1).unwrap(),
            TextPosition::new(3, 1).unwrap()
        )
        .is_err()
    );
    let id = resource(&[0xff]);
    assert_eq!(
        SourceLocation::original(id.clone(), r("rev"), None, ByteSpan::new(0, 1).unwrap()),
        LocationObservation::Unavailable {
            resource: id.clone(),
            captured_revision: r("rev"),
            reason: LocationUnavailable::UnknownEncoding,
        }
    );
    assert_eq!(
        LocationObservation::raw_bytes_unavailable(id.clone(), r("rev")),
        LocationObservation::Unavailable {
            resource: id,
            captured_revision: r("rev"),
            reason: LocationUnavailable::RawBytesUnavailable,
        }
    );
}
