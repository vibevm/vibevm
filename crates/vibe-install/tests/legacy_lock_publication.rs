//! Known legacy regeneration uses the existing owned metadata publisher.
use vibe_core::manifest::Lockfile;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn captured_changed_current_rich_future_or_absent_lock_cannot_be_legacy_regenerated() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("vibe.lock");
    let replacement = Lockfile::empty("fixture", "fixed");
    assert!(vibe_install::publish_empty_legacy_lock(root.path(), 6, &replacement).is_err());
    assert!(!path.exists());
    for epoch in [5, 7, 8, 9, 0] {
        let text = format!(
            "# retain\n[meta]\ngenerated_by='other'\ngenerated_at='other'\nschema_version={epoch}\n"
        );
        std::fs::write(&path, &text).unwrap();
        assert!(vibe_install::publish_empty_legacy_lock(root.path(), 6, &replacement).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
    let malformed = "[meta]\nschema_version=6\n[configurations.untrusted]\n";
    std::fs::write(&path, malformed).unwrap();
    assert!(vibe_install::publish_empty_legacy_lock(root.path(), 6, &replacement).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    let manifest = vibe_core::manifest::Manifest::parse_str(
        "[package]\ngroup='org.test'\nname='owner'\nkind='flow'\nversion='1.0.0'\n[configuration]",
    )
    .unwrap();
    let input = vibe_core::manifest::ConfigurationIntentDecl {
        policy: vibe_core::configuration::ChoicePolicy::RequireSelection,
        profile: None,
        features: vec![],
        default_features: false,
        selections: Default::default(),
        parameters: Default::default(),
    };
    let record = vibe_core::manifest::PackageConfiguration::evaluate(
        &manifest,
        &std::collections::BTreeMap::from([("./vibe.toml".into(), input)]),
    )
    .unwrap();
    let mut rich = Lockfile::empty("fixture", "fixed");
    rich.packages.push(serde_json::from_value(serde_json::json!({"kind":"flow","group":"org.test","name":"owner","version":"1.0.0","source_url":"file:///fixture","content_hash":"sha256:aa","files_written":[]})).unwrap());
    rich.meta
        .root_dependencies
        .push(vibe_core::PackageRef::parse("org.test/owner@=1.0.0").unwrap());
    rich.configurations.insert("org.test/owner".into(), record);
    let supported = rich.render_with_comments(None).unwrap();
    assert!(Lockfile::parse_str(&supported).is_ok());
    let under_old = supported.replace("schema_version = 8", "schema_version = 6");
    assert_ne!(under_old, supported);
    assert!(matches!(
        Lockfile::parse_str(&under_old),
        Err(vibe_core::Error::InvalidLockfile { .. })
    ));
    std::fs::write(&path, &under_old).unwrap();
    assert!(vibe_install::publish_empty_legacy_lock(root.path(), 6, &replacement).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), under_old);
}
