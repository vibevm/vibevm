use super::*;

#[test]
fn final_publication_refuses_a_concurrently_reprepared_release_id() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    let mut replacement = release(true);
    replacement.id = 99;
    *host.release.borrow_mut() = Some(replacement);

    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("a different release ID must never be published")
        .to_string();
    assert!(error.contains("identity changed"));
    assert_eq!(host.assets.borrow().len(), 12, "the guard is read-only");
}

#[test]
fn final_publication_refuses_a_changed_verified_asset_id() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    host.assets.borrow_mut()[0].id += 1_000;
    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("asset identity drift must refuse publication")
        .to_string();
    assert!(error.contains("asset set changed"));
}

#[test]
fn final_publication_refuses_a_moved_release_tag() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    *host.moved_to.borrow_mut() = Some("f".repeat(40));

    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("a moved tag must never be published")
        .to_string();
    assert!(error.contains("tag provenance mismatch"));
}
