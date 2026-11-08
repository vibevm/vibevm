//! Retained local source authority is an owner capability, not a portable URL.
use super::*;
#[test]
fn local_metadata_token_rejects_another_owner_and_preserves_semantic_witness() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("org.test/item/v1.0.0");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.test'\nname='item'\nversion='1.0.0'\nkind='tool'\n",
    )
    .unwrap();
    let owner = crate::LocalRegistry::new(root.path().to_path_buf()).unwrap();
    let other = crate::LocalRegistry::new(root.path().to_path_buf()).unwrap();
    let group = Group::parse("org.test").unwrap();
    let retained = owner
        .retain_default_candidate(
            &group,
            "item",
            &"1.0.0".parse().unwrap(),
            LocalCandidateKind::ExplicitRegistry,
        )
        .unwrap();
    assert!(
        retained
            .candidate
            .resolution_for(&owner.candidate_owner)
            .is_ok()
    );
    assert!(
        retained
            .candidate
            .resolution_for(&other.candidate_owner)
            .is_err()
    );
    assert_eq!(
        retained.candidate.metadata_recipe(),
        "parsed-manifest-toml-v1"
    );
    let digest = retained
        .candidate
        .metadata_digest()
        .strip_prefix("sha256:")
        .expect("semantic digest must retain its algorithm tag");
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    retained
        .candidate
        .validate_manifest(retained.candidate.manifest())
        .expect("unchanged admitted metadata must validate");
    assert!(retained.candidate.source_evidence().acquisition.is_none());
    assert!(retained.candidate.source_evidence().store_reused.is_none());
    let mut changed = retained.candidate.manifest().clone();
    changed.package.as_mut().unwrap().description = Some("changed".into());
    assert!(retained.candidate.validate_manifest(&changed).is_err());
}

#[test]
fn all_allowed_inventory_keeps_unused_constructor_failure_as_an_observation() {
    let root = tempfile::tempdir().unwrap();
    let registry = root.path().join("registry");
    let package = registry.join("org.test/item/v1.0.0");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.test'\nname='item'\nversion='1.0.0'\nkind='tool'\n",
    )
    .unwrap();
    let manifest = Manifest::parse_str(&format!("[project]\nname='consumer'\nversion='1.0.0'\n[[registry]]\nname='available'\nurl='file://{}'\n[[registry]]\nname='missing'\nurl='file://{}'\n", registry.display(), root.path().join("absent").display())).unwrap();
    let owner = crate::MultiRegistryResolver::from_manifest(
        &manifest.registries,
        &[],
        &[],
        root.path().join("cache"),
        Arc::new(crate::git_backend::ShellGit::new()),
        0,
    )
    .unwrap();
    let inventory = owner
        .candidate_inventory(
            &Group::parse("org.test").unwrap(),
            "item",
            &SourceInventoryLimits::default(),
            CandidateInventoryScope::AllAllowed,
        )
        .unwrap();
    assert_eq!(inventory.candidates.len(), 1);
    assert!(inventory.candidates[0].default_eligible);
    assert!(inventory.observations.iter().any(|observation| {
        observation.binding.registry_name() == Some("missing")
            && matches!(
                observation.observation,
                SourceCandidateObservationKind::Unavailable { .. }
            )
    }));
}
