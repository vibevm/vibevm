use super::*;

fn package(registry: &Path, version: &str) -> PathBuf {
    let root = registry.join(format!("org.continuity/offer/v{version}"));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), format!(
            "[package]\ngroup='org.continuity'\nname='offer'\nkind='remap'\nversion='{version}'\n[remap]\ndescription='fixture'\n[[remap.rows]]\nprefix='https://example.invalid/source'\nurl='https://example.invalid/target'\nreason='fixture'\n"
        )).unwrap();
    root
}

#[test]
fn remap_preflight_keeps_original_owner_and_version_when_catalog_grows() {
    let registry = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    package(registry.path(), "1.0.0");
    let args = InstallArgs {
        registry: Some(registry.path().to_path_buf()),
        path: registry.path().to_path_buf(),
        packages: vec!["org.continuity/offer".into()],
        ..inert_args()
    };
    let selected = claims(&args, true).unwrap();
    package(registry.path(), "1.1.0");
    let candidate = &selected.candidates[0];
    let acquired = selected
        .resolver
        .acquire_candidate_for_install(
            candidate,
            store.path(),
            None,
            &store.path().join("slot"),
            &vibe_core::progress::Progress::default(),
        )
        .unwrap();
    let vibe_registry::PackageAcquisition::Cached(cached) = acquired else {
        panic!("remap must be cached");
    };
    assert_eq!(cached.resolved.version.to_string(), "1.0.0");
    let reference = PackageRef::parse("org.continuity/offer").unwrap();
    checked_offer(&cached, &reference).unwrap();
}

#[test]
fn remap_preflight_refuses_changed_selected_manifest() {
    let registry = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let root = package(registry.path(), "1.0.0");
    let args = InstallArgs {
        registry: Some(registry.path().to_path_buf()),
        path: registry.path().to_path_buf(),
        packages: vec!["org.continuity/offer".into()],
        ..inert_args()
    };
    let selected = claims(&args, true).unwrap();
    let path = root.join("vibe.toml");
    let original = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        original.replace("description='fixture'", "description='changed'"),
    )
    .unwrap();
    assert!(
        selected
            .resolver
            .acquire_candidate_for_install(
                &selected.candidates[0],
                store.path(),
                None,
                &store.path().join("slot"),
                &vibe_core::progress::Progress::default(),
            )
            .is_err()
    );
}
