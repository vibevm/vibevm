use specmark::spec;
use thiserror::Error;

pub const RELEASE_MANIFEST_CONTRACT: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-019#instances";

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#instances")]
pub enum ReleaseManifestError {
    #[error(
        "invalid distribution manifest JSON: {0} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: regenerate the manifest with the matching vibe release tooling)"
    )]
    Json(#[from] serde_json::Error),
    #[error(
        "unsupported distribution schema_version {actual}; expected {expected} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: use a producer and consumer that implement the same schema version)"
    )]
    SchemaVersion { actual: u32, expected: u32 },
    #[error(
        "invalid distribution version `{value}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: supply a valid semantic version without a `v` prefix)"
    )]
    Version { value: String, detail: String },
    #[error(
        "distribution tag `{tag}` does not match version `{version}`; expected `v{version}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: derive the release tag directly from the semantic version)"
    )]
    Tag { version: String, tag: String },
    #[error(
        "distribution product `{actual}` does not match required product `{expected}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: rebuild the manifest for the vibevm product)"
    )]
    Product {
        actual: String,
        expected: &'static str,
    },
    #[error(
        "distribution repository `{actual}` does not match required repository `{expected}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: rebuild the manifest from the canonical vibevm/vibevm repository)"
    )]
    Repository {
        actual: String,
        expected: &'static str,
    },
    #[error(
        "invalid distribution source_commit `{value}`; expected a lowercase 40- or 64-hex object id \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: record the full lowercase Git object ID used by the build)"
    )]
    SourceCommit { value: String },
    #[error(
        "unsupported distribution target `{target}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: build one of the four targets in SUPPORTED_DISTRIBUTION_TARGETS)"
    )]
    UnsupportedTarget { target: String },
    #[error(
        "bundle for `{target}` must contain exactly `vibe` and `vibe-index` once each \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: rebuild the bundle with both required executable components)"
    )]
    Components { target: String },
    #[error(
        "component `{component}` has invalid bundle-relative path `{path}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: use a non-empty relative path without parent traversal)"
    )]
    ComponentPath { component: String, path: String },
    #[error(
        "{field} must have a non-zero size \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: rebuild and measure the non-empty artifact before manifest generation)"
    )]
    EmptyArtifact { field: String },
    #[error(
        "{field} declares {size} bytes, exceeding the independent {max}-byte distribution limit \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: reject the artifact before download or buffering and publish a bounded distribution)"
    )]
    ArtifactTooLarge { field: String, size: u64, max: u64 },
    #[error(
        "{field} has invalid digest `{digest}`; expected `sha256:<64 lowercase hex digits>` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: recompute the SHA-256 digest from the exact artifact bytes)"
    )]
    Digest { field: String, digest: String },
    #[error(
        "platform fragment `{target}` does not match its embedded bundle: {field} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: regenerate the fragment and embedded manifest in one build)"
    )]
    FragmentBundleMismatch { target: String, field: &'static str },
    #[error(
        "aggregate platform `{target}` does not match aggregate {field} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: aggregate only fragments from the same version, tag, repository, and commit)"
    )]
    AggregateIdentityMismatch { target: String, field: &'static str },
    #[error(
        "aggregate must contain each supported distribution target exactly once; expected {expected:?}, got {actual:?} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: provide exactly one successful fragment for every supported target)"
    )]
    TargetMatrix {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    #[error(
        "source archive has invalid bundle-relative path `{path}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: use a non-executable archive path without root or parent traversal)"
    )]
    SourceArchivePath { path: String },
    #[error(
        "source archive has invalid Git tree object ID `{value}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: record the full lowercase 40- or 64-hex tree OID represented by the archive)"
    )]
    SourceArchiveTreeOid { value: String },
    #[error(
        "platform `{target}` carries a different source archive than the other release bundles \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: build all four platform bundles from the same source archive, tree, digest, and commit)"
    )]
    SourceArchiveMismatch { target: String },
    #[error(
        "distribution asset name must be non-empty and contain no path separators \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: use the platform bundle's plain release-asset file name)"
    )]
    AssetName,
    #[error(
        "platform `{target}` reuses release asset name `{name}` for its bundle and bootstrap \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: publish the full ZIP and raw bootstrap executable under distinct names)"
    )]
    AssetNameCollision { target: String, name: String },
    #[error(
        "platform `{target}` bootstrap does not match the embedded `vibe` component: {field} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#instances; \
         fix: publish the exact same built vibe executable both inside the bundle and as the raw bootstrap asset)"
    )]
    BootstrapMismatch { target: String, field: &'static str },
}
