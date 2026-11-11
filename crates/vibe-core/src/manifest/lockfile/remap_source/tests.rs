#![cfg(test)]
use super::*;
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn legacy_source_absence_stays_unknown_and_new_binding_requires_epoch_nine() {
    for epoch in [7, 8] {
        let text = format!(
            "[meta]\nschema_version={epoch}\ngenerated_by='fixture'\ngenerated_at='fixed'\n[[remap]]\npackage='org.example/policy'\nversion='1.0.0'\ncontent_hash='sha256:ab'\n"
        );
        let mut lock = Lockfile::parse_str(&text).unwrap();
        assert_eq!(lock.meta.schema_version, epoch);
        assert!(matches!(
            lock.remaps[0].locked_package(),
            Err(LockedRemapSourceError::MissingSource { .. })
        ));
        let source: LockedRemapSource = toml::from_str("source_url='https://example.invalid/policy.git'\nsource_ref='main'\nresolved_commit='0123456789012345678901234567890123456789'\nsource_kind='git'\n").unwrap();
        lock.remaps[0] = lock.remaps[0].clone().bind_source(source).unwrap();
        assert!(
            Lockfile::parse_str(&toml::to_string(&lock).unwrap()).is_err(),
            "binding under older epoch must refuse"
        );
        let written = lock.for_write().unwrap();
        assert_eq!(written.meta.schema_version, 9);
        let admitted = Lockfile::parse_str(&toml::to_string(&written).unwrap()).unwrap();
        let held = admitted.remaps[0].locked_package().unwrap();
        assert_eq!(held.source_ref.as_deref(), Some("main"));
        assert_eq!(held.source_kind, Some(SourceKind::Git));
        assert_eq!(held.kind, PackageKind::Remap);
        assert_eq!(held.content_hash.as_str(), "sha256:ab");
    }
}
#[test]
fn new_fields_do_not_force_ordinary_legacy_or_configuration_epoch_changes() {
    let legacy = Lockfile::empty("fixture", "fixed");
    assert_eq!(legacy.for_write().unwrap().meta.schema_version, 7);
    assert_eq!(CONFIGURATION_SCHEMA_VERSION, 8);
    assert_eq!(crate::manifest::REMAP_SOURCE_SCHEMA_VERSION, 9);
    assert_eq!(crate::manifest::TEMPLATE_SCHEMA_VERSION, 10);
    // PROP-075 `#TEMPLATE-NO-DIGEST` (AV-FMT-033) added epoch11 for a
    // template record with no digest document. The point of this test is
    // that an ordinary lock is untouched by it, which the first
    // assertion above still measures.
    assert_eq!(CURRENT_SCHEMA_VERSION, 11);
}
#[test]
fn explicit_binding_rejects_credentials_without_printing_them() {
    let row: LockedRemap =
        toml::from_str("package='org.example/policy'\nversion='1.0.0'\ncontent_hash='sha256:ab'")
            .unwrap();
    let source: LockedRemapSource = toml::from_str(
        "source_url='https://user:SYN_REMAP_SECRET@example.invalid/policy'\nsource_ref='main'",
    )
    .unwrap();
    let error = row.bind_source(source).unwrap_err().to_string();
    assert!(error.contains("credentials"));
    assert!(!error.contains("SYN_REMAP_SECRET"));
}
