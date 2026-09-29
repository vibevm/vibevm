use super::*;
use crate::machine_view::{ResourceUriDeclaration, ResourceUriStatus};

fn r(value: &str) -> OpaqueText {
    OpaqueText::new(value).unwrap()
}
fn neutral() -> SourceLocation {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-source-location/e1/valid/unavailable-reverse-map.json"
    )))
    .unwrap()
}
fn owner() -> OwnerResourceUriMap {
    OwnerResourceUriMap::new(
        r("scope_1"),
        vec![ResourceUriDeclaration {
            resource: OpaqueResourceId::new(vec![255, 0, 47, 92]).unwrap(),
            captured_revision: r("raw-rev"),
            token: "sample_1".to_owned(),
        }],
    )
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn exact_non_utf8_owner_mapping_publishes_only_unverified_client_uri() {
    let original = neutral();
    let mapped =
        publish_source_location_uri(Some(&owner()), &r("scope_1"), original.clone()).unwrap();
    assert_eq!(mapped.resource_id_octets, original.resource_id_octets);
    assert_eq!(mapped.captured_revision, original.captured_revision);
    assert_eq!(mapped.location, original.location);
    assert_eq!(
        mapped.client_uri.as_deref(),
        Some("vibevm-resource:sample_1")
    );
    let UriMapping::OwnerDeclaredUnverified(status) = &mapped.uri_mapping else {
        panic!("mapping was silently upgraded or discarded")
    };
    assert_eq!(status.authorized_scope_id, "scope_1");
    assert_eq!(
        check_published_source_location_uri(Some(&owner()), &r("scope_1"), mapped.clone()),
        Ok(mapped)
    );
    assert_eq!(
        owner()
            .lookup(
                &r("scope_1"),
                &OpaqueResourceId::new(vec![255, 0, 47, 92]).unwrap(),
                &r("raw-rev")
            )
            .unwrap()
            .status(),
        ResourceUriStatus::OwnerDeclaredUnverifiedSourceBytes
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn unavailable_owner_scope_revision_and_resource_remain_null_with_typed_reasons() {
    for (map, scope, change, reason) in [
        (
            None,
            "scope_1",
            "none",
            UriUnavailableReason::NoOwnerMapping,
        ),
        (
            Some(owner()),
            "scope_2",
            "none",
            UriUnavailableReason::WrongScope,
        ),
        (
            Some(owner()),
            "scope_1",
            "revision",
            UriUnavailableReason::WrongRevision,
        ),
        (
            Some(owner()),
            "scope_1",
            "resource",
            UriUnavailableReason::AbsentResource,
        ),
    ] {
        let mut input = neutral();
        if change == "revision" {
            input.captured_revision = "foreign-rev".to_owned();
        }
        if change == "resource" {
            input.resource_id_octets = vec![254, 0, 47, 92];
        }
        let published = publish_source_location_uri(map.as_ref(), &r(scope), input).unwrap();
        assert!(published.client_uri.is_none());
        assert!(matches!(
            &published.uri_mapping,
            UriMapping::Unavailable(value) if value.reason == reason
        ));
        assert!(check_published_source_location_uri(map.as_ref(), &r(scope), published).is_ok());
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn forged_uri_status_scope_resource_and_revision_retain_original_evidence() {
    let mapped = publish_source_location_uri(Some(&owner()), &r("scope_1"), neutral()).unwrap();
    assert_eq!(
        publish_source_location_uri(Some(&owner()), &r("scope_1"), mapped.clone())
            .unwrap_err()
            .fault,
        SourceUriFault::PreclaimedMapping
    );
    let mut cases = Vec::new();
    let mut forged = mapped.clone();
    forged.client_uri = Some("vibevm-resource:another".to_owned());
    cases.push(forged);
    let mut forged = mapped.clone();
    forged.uri_mapping =
        UriMapping::OwnerDeclaredUnverified(Box::new(UriMappingOwnerDeclaredUnverified {
            authorized_scope_id: "scope_2".to_owned(),
        }));
    cases.push(forged);
    let mut forged = mapped.clone();
    forged.uri_mapping = UriMapping::Unavailable(Box::new(UriMappingUnavailable {
        reason: UriUnavailableReason::AbsentResource,
    }));
    cases.push(forged);
    let mut forged = mapped.clone();
    forged.resource_id_octets[0] = 254;
    cases.push(forged);
    let mut forged = mapped;
    forged.captured_revision = "foreign-rev".to_owned();
    cases.push(forged);
    for original in cases {
        let failure =
            check_published_source_location_uri(Some(&owner()), &r("scope_1"), original.clone())
                .unwrap_err();
        assert_eq!(failure.original, original);
        assert!(matches!(
            failure.fault,
            SourceUriFault::ForgedPublication | SourceUriFault::InvalidSource { .. }
        ));
    }
}
