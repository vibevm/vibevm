use super::*;

const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
const DIGEST_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn source_archive() -> DistributionSourceArchive {
    DistributionSourceArchive {
        path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
        size: 40,
        digest: DIGEST_B.to_string(),
        tree_oid: SOURCE_TREE.to_string(),
    }
}

fn bundle(target: &str) -> BundleDistributionManifest {
    BundleDistributionManifest {
        schema_version: DISTRIBUTION_SCHEMA_VERSION,
        product: DISTRIBUTION_PRODUCT.to_string(),
        repository: DISTRIBUTION_REPOSITORY.to_string(),
        version: "1.2.3".to_string(),
        tag: "v1.2.3".to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        target: target.to_string(),
        components: vec![
            DistributionComponent {
                name: DistributionComponentName::VibeIndex,
                path: "vibe-index".to_string(),
                size: 20,
                digest: DIGEST_B.to_string(),
            },
            DistributionComponent {
                name: DistributionComponentName::Vibe,
                path: "vibe".to_string(),
                size: 10,
                digest: DIGEST_A.to_string(),
            },
            DistributionComponent {
                name: DistributionComponentName::VibeAppProxy,
                path: "vibe-app-proxy".to_string(),
                size: 10,
                digest: DIGEST_A.to_string(),
            },
        ],
        source_archive: source_archive(),
    }
}

fn fragment(target: &str) -> PlatformDistributionFragment {
    PlatformDistributionFragment {
        schema_version: DISTRIBUTION_SCHEMA_VERSION,
        product: DISTRIBUTION_PRODUCT.to_string(),
        repository: DISTRIBUTION_REPOSITORY.to_string(),
        version: "1.2.3".to_string(),
        tag: "v1.2.3".to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        target: target.to_string(),
        asset: DistributionAsset {
            name: format!("vibevm-1.2.3-{target}.zip"),
            size: 30,
            digest: DIGEST_A.to_string(),
        },
        bootstrap: DistributionAsset {
            name: format!("vibe-bootstrap-{target}"),
            size: 10,
            digest: DIGEST_A.to_string(),
        },
        bundle: bundle(target),
    }
}

fn aggregate() -> AggregateDistributionManifest {
    AggregateDistributionManifest {
        schema_version: DISTRIBUTION_SCHEMA_VERSION,
        product: DISTRIBUTION_PRODUCT.to_string(),
        repository: DISTRIBUTION_REPOSITORY.to_string(),
        version: "1.2.3".to_string(),
        tag: "v1.2.3".to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        platforms: SUPPORTED_DISTRIBUTION_TARGETS
            .iter()
            .rev()
            .map(|target| fragment(target))
            .collect(),
    }
}

#[test]
fn bundle_accepts_historical_pair_and_current_triple() {
    bundle(SUPPORTED_DISTRIBUTION_TARGETS[0])
        .validate()
        .unwrap();
    let mut historical = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    historical
        .components
        .retain(|entry| entry.name != DistributionComponentName::VibeAppProxy);
    historical.validate().unwrap();
    let mut invalid = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    invalid.components[1].name = DistributionComponentName::VibeIndex;
    assert!(matches!(
        invalid.validate(),
        Err(ReleaseManifestError::Components { .. })
    ));
}

#[test]
fn aggregate_requires_the_exact_v1_target_matrix() {
    aggregate().validate().unwrap();
    let mut missing = aggregate();
    missing.platforms.pop();
    assert!(matches!(
        missing.validate(),
        Err(ReleaseManifestError::TargetMatrix { .. })
    ));
    let mut duplicate = aggregate();
    duplicate.platforms[3] = duplicate.platforms[0].clone();
    assert!(matches!(
        duplicate.validate(),
        Err(ReleaseManifestError::TargetDuplicate { .. })
    ));
    // The publishing rule is also the republishing rule: a mirror parses a
    // published aggregate and is held to the complete matrix.
    let partial = serde_json::to_vec(&missing).unwrap();
    assert!(matches!(
        AggregateDistributionManifest::from_json_slice(&partial),
        Err(ReleaseManifestError::TargetMatrix { .. })
    ));
}

/// An installation takes one platform out of an aggregate, so a release
/// drop may carry only the targets that site runs (`#release-sources`,
/// B-247) — while every rule that is actually load-bearing per entry stays.
#[test]
fn a_consumer_accepts_an_aggregate_holding_its_own_target_alone() {
    let host = SUPPORTED_DISTRIBUTION_TARGETS[0];
    let other = SUPPORTED_DISTRIBUTION_TARGETS[1];

    let mut only_host = aggregate();
    only_host.platforms.retain(|entry| entry.target == host);
    assert_eq!(only_host.platforms.len(), 1);
    only_host.validate_for_target(host).unwrap();
    let bytes = serde_json::to_vec(&only_host).unwrap();
    let parsed = AggregateDistributionManifest::from_json_slice_for_target(&bytes, host).unwrap();
    assert_eq!(parsed.platforms.len(), 1);
    // The complete matrix still parses as a consumer's aggregate.
    aggregate().validate_for_target(host).unwrap();

    // An aggregate that does not describe this host is refused, by name.
    let error = only_host
        .validate_for_target(other)
        .unwrap_err()
        .to_string();
    assert!(error.contains(other), "{error}");
    assert!(error.contains(host), "{error}");
    assert!(matches!(
        AggregateDistributionManifest::from_json_slice_for_target(&bytes, other),
        Err(ReleaseManifestError::TargetAbsent { .. })
    ));

    // Still refused: the same target twice, an unsupported target, and an
    // aggregate with no platforms at all.
    let mut twice = only_host.clone();
    twice.platforms.push(twice.platforms[0].clone());
    assert!(matches!(
        twice.validate_for_target(host),
        Err(ReleaseManifestError::TargetDuplicate { .. })
    ));
    let mut unknown = only_host.clone();
    unknown.platforms[0].target = "powerpc64-unknown-linux-gnu".to_string();
    unknown.platforms[0].bundle.target = "powerpc64-unknown-linux-gnu".to_string();
    assert!(matches!(
        unknown.validate_for_target(host),
        Err(ReleaseManifestError::UnsupportedTarget { .. })
    ));
    let mut empty = only_host.clone();
    empty.platforms.clear();
    assert!(matches!(
        empty.validate_for_target(host),
        Err(ReleaseManifestError::TargetMatrix { .. })
    ));

    // And every per-entry rule: a partial aggregate is validated exactly as
    // a complete one, entry by entry.
    let mut drifted = only_host.clone();
    drifted.platforms[0].asset.name = drifted.platforms[0].bootstrap.name.clone();
    assert!(matches!(
        drifted.validate_for_target(host),
        Err(ReleaseManifestError::AssetNameCollision { .. })
    ));
    let mut mismatched = only_host.clone();
    mismatched.platforms[0].version = "1.2.4".to_string();
    mismatched.platforms[0].tag = "v1.2.4".to_string();
    mismatched.platforms[0].bundle.version = "1.2.4".to_string();
    mismatched.platforms[0].bundle.tag = "v1.2.4".to_string();
    assert!(matches!(
        mismatched.validate_for_target(host),
        Err(ReleaseManifestError::AggregateIdentityMismatch { .. })
    ));
}

#[test]
fn aggregate_rejects_identity_drift_between_platforms() {
    let mut invalid = aggregate();
    invalid.platforms[2].source_commit = "f".repeat(40);
    invalid.platforms[2].bundle.source_commit = "f".repeat(40);
    assert!(matches!(
        invalid.validate(),
        Err(ReleaseManifestError::AggregateIdentityMismatch {
            field: "source_commit",
            ..
        })
    ));
}

#[test]
fn aggregate_requires_globally_distinct_bundle_and_bootstrap_asset_names() {
    let mut invalid = aggregate();
    invalid.platforms[1].bootstrap.name = invalid.platforms[0].bootstrap.name.clone();
    assert!(matches!(
        invalid.validate(),
        Err(ReleaseManifestError::AssetNameCollision { .. })
    ));
}

#[test]
fn fragment_requires_a_distinct_bootstrap_matching_vibe() {
    let target = SUPPORTED_DISTRIBUTION_TARGETS[0];
    fragment(target).validate().unwrap();

    let mut collision = fragment(target);
    collision.bootstrap.name = collision.asset.name.clone();
    assert!(matches!(
        collision.validate(),
        Err(ReleaseManifestError::AssetNameCollision { .. })
    ));

    let mut wrong_size = fragment(target);
    wrong_size.bootstrap.size += 1;
    assert!(matches!(
        wrong_size.validate(),
        Err(ReleaseManifestError::BootstrapMismatch { field: "size", .. })
    ));

    let mut wrong_digest = fragment(target);
    wrong_digest.bootstrap.digest = DIGEST_B.to_string();
    assert!(matches!(
        wrong_digest.validate(),
        Err(ReleaseManifestError::BootstrapMismatch {
            field: "digest",
            ..
        })
    ));

    let mut missing = serde_json::to_value(fragment(target)).unwrap();
    missing.as_object_mut().unwrap().remove("bootstrap");
    assert!(matches!(
        PlatformDistributionFragment::from_json_slice(&serde_json::to_vec(&missing).unwrap()),
        Err(ReleaseManifestError::Json(_))
    ));
}

#[test]
fn independent_limits_reject_oversized_declarations_before_io() {
    let target = SUPPORTED_DISTRIBUTION_TARGETS[0];

    let mut component = bundle(target);
    component.components[0].size = DISTRIBUTION_COMPONENT_MAX_BYTES + 1;
    assert!(matches!(
        component.validate(),
        Err(ReleaseManifestError::ArtifactTooLarge { .. })
    ));

    let mut source = bundle(target);
    source.source_archive.size = DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES + 1;
    assert!(matches!(
        source.validate(),
        Err(ReleaseManifestError::ArtifactTooLarge { .. })
    ));

    let mut platform = fragment(target);
    platform.asset.size = DISTRIBUTION_BUNDLE_MAX_BYTES + 1;
    assert!(matches!(
        platform.validate(),
        Err(ReleaseManifestError::ArtifactTooLarge { .. })
    ));

    let oversized_json = vec![b' '; DISTRIBUTION_MANIFEST_MAX_BYTES as usize + 1];
    assert!(matches!(
        PlatformDistributionFragment::from_json_slice(&oversized_json),
        Err(ReleaseManifestError::ArtifactTooLarge { .. })
    ));
}

#[test]
fn aggregate_requires_identical_source_archive_across_all_platforms() {
    for mutation in 0..4 {
        let mut invalid = aggregate();
        let archive = &mut invalid.platforms[2].bundle.source_archive;
        match mutation {
            0 => archive.path = "other-source.zip".to_string(),
            1 => archive.size += 1,
            2 => archive.digest = format!("sha256:{}", "c".repeat(64)),
            3 => archive.tree_oid = "d".repeat(40),
            _ => unreachable!(),
        }
        assert!(matches!(
            invalid.validate(),
            Err(ReleaseManifestError::SourceArchiveMismatch { .. })
        ));
    }
}

#[test]
fn serialization_is_deterministic_and_canonicalizes_vector_order() {
    let first = aggregate().to_json_bytes().unwrap();
    let mut reordered = aggregate();
    reordered.platforms.reverse();
    for platform in &mut reordered.platforms {
        platform.bundle.components.reverse();
    }
    let second = reordered.to_json_bytes().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.last(), Some(&b'\n'));
    AggregateDistributionManifest::from_json_slice(&first).unwrap();
}

#[test]
fn strict_json_rejects_unknown_fields() {
    let mut value = serde_json::to_value(bundle(SUPPORTED_DISTRIBUTION_TARGETS[0])).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("surprise".to_string(), serde_json::json!(true));
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(matches!(
        BundleDistributionManifest::from_json_slice(&bytes),
        Err(ReleaseManifestError::Json(_))
    ));
}

#[test]
fn source_archive_is_required_and_strict() {
    let valid = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    valid.validate().unwrap();

    let mut missing = serde_json::to_value(&valid).unwrap();
    missing.as_object_mut().unwrap().remove("source_archive");
    assert!(matches!(
        BundleDistributionManifest::from_json_slice(&serde_json::to_vec(&missing).unwrap()),
        Err(ReleaseManifestError::Json(_))
    ));

    let mut unsafe_path = valid.clone();
    unsafe_path.source_archive.path = "../source.zip".to_string();
    assert!(matches!(
        unsafe_path.validate(),
        Err(ReleaseManifestError::SourceArchivePath { .. })
    ));

    let mut empty = valid.clone();
    empty.source_archive.size = 0;
    assert!(matches!(
        empty.validate(),
        Err(ReleaseManifestError::EmptyArtifact { .. })
    ));

    let mut bad_digest = valid.clone();
    bad_digest.source_archive.digest = "sha256:not-a-digest".to_string();
    assert!(matches!(
        bad_digest.validate(),
        Err(ReleaseManifestError::Digest { .. })
    ));

    let mut bad_tree = valid;
    bad_tree.source_archive.tree_oid = "A".repeat(40);
    assert!(matches!(
        bad_tree.validate(),
        Err(ReleaseManifestError::SourceArchiveTreeOid { .. })
    ));
}

#[test]
fn identity_and_source_commit_are_strict() {
    let mut sha256 = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    sha256.source_commit = "c".repeat(64);
    sha256.validate().unwrap();

    let mut invalid = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    invalid.product = "other".to_string();
    assert!(matches!(
        invalid.validate(),
        Err(ReleaseManifestError::Product { .. })
    ));

    let mut short = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    short.source_commit = "0123456789abcdef".to_string();
    assert!(matches!(
        short.validate(),
        Err(ReleaseManifestError::SourceCommit { .. })
    ));

    let mut uppercase = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    uppercase.source_commit = "A".repeat(40);
    assert!(matches!(
        uppercase.validate(),
        Err(ReleaseManifestError::SourceCommit { .. })
    ));

    let mut non_hex = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    non_hex.source_commit = "z".repeat(40);
    assert!(matches!(
        non_hex.validate(),
        Err(ReleaseManifestError::SourceCommit { .. })
    ));
}

#[test]
fn tag_must_be_the_v_prefixed_version() {
    let mut invalid = bundle(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    invalid.tag = "release-1.2.3".to_string();
    assert!(matches!(
        invalid.validate(),
        Err(ReleaseManifestError::Tag { .. })
    ));
}
