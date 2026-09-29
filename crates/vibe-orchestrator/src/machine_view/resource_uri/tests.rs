use super::*;

fn r(value: &str) -> OpaqueText {
    OpaqueText::new(value).unwrap()
}
fn raw(bytes: &[u8]) -> OpaqueResourceId {
    OpaqueResourceId::new(bytes.to_vec()).unwrap()
}
fn declaration(bytes: &[u8], revision: &str, token: &str) -> ResourceUriDeclaration {
    ResourceUriDeclaration {
        resource: raw(bytes),
        captured_revision: r(revision),
        token: token.to_owned(),
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn non_utf8_exact_key_gets_only_declared_client_uri_without_path_authority() {
    let resource = raw(&[0xff, 0, b'/', b'\\', 0x80]);
    let map = OwnerResourceUriMap::new(
        r("scope_authorized"),
        vec![ResourceUriDeclaration {
            resource: resource.clone(),
            captured_revision: r("capture-rev-1"),
            token: "R7_opaque-token".to_owned(),
        }],
    )
    .unwrap();
    let value = map
        .lookup(&r("scope_authorized"), &resource, &r("capture-rev-1"))
        .unwrap();
    assert_eq!(value.uri(), "vibevm-resource:R7_opaque-token");
    assert_eq!(value.resource().bytes(), resource.bytes());
    assert_eq!(value.captured_revision().as_str(), "capture-rev-1");
    assert_eq!(value.authorized_scope().as_str(), "scope_authorized");
    assert_eq!(
        value.status(),
        ResourceUriStatus::OwnerDeclaredUnverifiedSourceBytes
    );
    assert!(!value.uri().contains("file://"));
    assert!(!value.uri().contains('�'));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn scope_revision_and_absent_resource_refuse_with_distinct_reasons() {
    let map = OwnerResourceUriMap::new(
        r("scope_a"),
        vec![declaration(&[0xff], "rev-a", "opaque-1")],
    )
    .unwrap();
    assert_eq!(
        map.lookup(&r("scope_b"), &raw(&[0xff]), &r("rev-a")),
        Err(ResourceUriLookupError::WrongScope)
    );
    assert_eq!(
        map.lookup(&r("scope_a"), &raw(&[0xff]), &r("rev-b")),
        Err(ResourceUriLookupError::WrongRevision)
    );
    assert_eq!(
        map.lookup(&r("scope_a"), &raw(&[0xfe]), &r("rev-a")),
        Err(ResourceUriLookupError::AbsentResource)
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
fn duplicate_bindings_tokens_and_pathlike_spellings_refuse_before_mapping() {
    assert!(matches!(
        OwnerResourceUriMap::new(r("../private"), vec![]),
        Err(ResourceUriMapError::InvalidScope(_))
    ));
    assert!(matches!(
        OwnerResourceUriMap::new(
            r("scope"),
            vec![
                declaration(&[0xff], "rev", "first"),
                declaration(&[0xff], "rev", "second")
            ]
        ),
        Err(ResourceUriMapError::DuplicateResourceRevision)
    ));
    assert!(matches!(
        OwnerResourceUriMap::new(
            r("scope"),
            vec![
                declaration(&[0xff], "rev", "same"),
                declaration(&[0xfe], "rev", "same")
            ]
        ),
        Err(ResourceUriMapError::DuplicateToken(_))
    ));
    for token in [
        "",
        ".",
        "..",
        "../secret",
        "a/b",
        "a\\b",
        "C:path",
        "file://host",
        "a?b",
        "a#b",
        "a%2Fb",
        " space",
        "x\ncontrol",
        "é",
    ] {
        assert!(
            matches!(
                OwnerResourceUriMap::new(r("scope"), vec![declaration(&[0xff], "rev", token)]),
                Err(ResourceUriMapError::InvalidToken(_))
            ),
            "unexpectedly accepted {token:?}"
        );
    }
}
