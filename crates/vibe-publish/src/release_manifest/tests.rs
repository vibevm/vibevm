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
            name: format!("vibe-v1.2.3-{target}.tar.gz"),
            size: 30,
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
fn bundle_requires_exactly_the_two_runtime_components() {
    bundle(SUPPORTED_DISTRIBUTION_TARGETS[0])
        .validate()
        .unwrap();
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
        Err(ReleaseManifestError::TargetMatrix { .. })
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
