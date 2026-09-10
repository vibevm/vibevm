//! Wire contracts shared by the four independent vibevm distribution builds.
//!
//! Each platform bundle carries a [`BundleDistributionManifest`] named
//! [`DISTRIBUTION_MANIFEST_FILENAME`]. The same build emits a
//! [`PlatformDistributionFragment`], and the release aggregation job combines
//! exactly one fragment for every target in [`SUPPORTED_DISTRIBUTION_TARGETS`]
//! into an [`AggregateDistributionManifest`]. All three documents are strict
//! JSON contracts with deterministic serializers.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use semver::Version;
use serde::{Deserialize, Serialize};

mod error;
pub use error::{RELEASE_MANIFEST_CONTRACT, ReleaseManifestError};

pub const DISTRIBUTION_MANIFEST_FILENAME: &str = "DISTRIBUTION.json";
pub const DISTRIBUTION_SOURCE_ARCHIVE_FILENAME: &str = "vibevm-source.zip";
pub const DISTRIBUTION_SCHEMA_VERSION: u32 = 1;
pub const DISTRIBUTION_PRODUCT: &str = "vibevm";
pub const DISTRIBUTION_REPOSITORY: &str = "vibevm/vibevm";
pub const SUPPORTED_DISTRIBUTION_TARGETS: [&str; 4] = [
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DistributionComponentName {
    #[serde(rename = "vibe")]
    Vibe,
    #[serde(rename = "vibe-index")]
    VibeIndex,
}

impl DistributionComponentName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vibe => "vibe",
            Self::VibeIndex => "vibe-index",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionComponent {
    pub name: DistributionComponentName,
    /// Bundle-relative executable path (`vibe.exe` on Windows, `vibe` elsewhere).
    pub path: String,
    pub size: u64,
    /// Lowercase `sha256:<64 hex digits>` digest of the component bytes.
    pub digest: String,
}

/// Non-executable source snapshot shipped alongside the two runtime binaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionSourceArchive {
    pub path: String,
    pub size: u64,
    /// Lowercase `sha256:<64 hex digits>` digest of the archive bytes.
    pub digest: String,
    /// Lowercase full Git tree object ID represented by the archive.
    pub tree_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleDistributionManifest {
    pub schema_version: u32,
    pub product: String,
    pub repository: String,
    /// SemVer without the Git tag's `v` prefix.
    pub version: String,
    /// Exactly `v<version>`.
    pub tag: String,
    pub source_commit: String,
    pub target: String,
    pub components: Vec<DistributionComponent>,
    pub source_archive: DistributionSourceArchive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionAsset {
    pub name: String,
    pub size: u64,
    /// Lowercase `sha256:<64 hex digits>` digest of the complete bundle.
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformDistributionFragment {
    pub schema_version: u32,
    pub product: String,
    pub repository: String,
    pub version: String,
    pub tag: String,
    pub source_commit: String,
    pub target: String,
    pub asset: DistributionAsset,
    /// The exact manifest embedded in this platform's bundle.
    pub bundle: BundleDistributionManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateDistributionManifest {
    pub schema_version: u32,
    pub product: String,
    pub repository: String,
    pub version: String,
    pub tag: String,
    pub source_commit: String,
    pub platforms: Vec<PlatformDistributionFragment>,
}

impl BundleDistributionManifest {
    pub fn new(
        version: Version,
        source_commit: impl Into<String>,
        target: impl Into<String>,
        components: Vec<DistributionComponent>,
        source_archive: DistributionSourceArchive,
    ) -> Result<Self, ReleaseManifestError> {
        let version = version.to_string();
        let manifest = Self {
            schema_version: DISTRIBUTION_SCHEMA_VERSION,
            product: DISTRIBUTION_PRODUCT.to_string(),
            repository: DISTRIBUTION_REPOSITORY.to_string(),
            tag: format!("v{version}"),
            version,
            source_commit: source_commit.into(),
            target: target.into(),
            components,
            source_archive,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ReleaseManifestError> {
        validate_schema(self.schema_version)?;
        validate_identity(
            &self.product,
            &self.repository,
            &self.version,
            &self.tag,
            &self.source_commit,
        )?;
        validate_target(&self.target)?;
        validate_components(&self.target, &self.components)?;
        validate_source_archive(&self.source_archive)?;
        if self
            .components
            .iter()
            .any(|component| component.path == self.source_archive.path)
        {
            return Err(ReleaseManifestError::SourceArchivePath {
                path: self.source_archive.path.clone(),
            });
        }
        Ok(())
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ReleaseManifestError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.components.sort_by_key(|component| component.name);
        deterministic_json(&canonical)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

impl PlatformDistributionFragment {
    pub fn new(
        asset: DistributionAsset,
        bundle: BundleDistributionManifest,
    ) -> Result<Self, ReleaseManifestError> {
        let fragment = Self {
            schema_version: DISTRIBUTION_SCHEMA_VERSION,
            product: bundle.product.clone(),
            repository: bundle.repository.clone(),
            version: bundle.version.clone(),
            tag: bundle.tag.clone(),
            source_commit: bundle.source_commit.clone(),
            target: bundle.target.clone(),
            asset,
            bundle,
        };
        fragment.validate()?;
        Ok(fragment)
    }

    pub fn validate(&self) -> Result<(), ReleaseManifestError> {
        validate_schema(self.schema_version)?;
        validate_identity(
            &self.product,
            &self.repository,
            &self.version,
            &self.tag,
            &self.source_commit,
        )?;
        validate_target(&self.target)?;
        validate_asset(&self.asset)?;
        self.bundle.validate()?;
        for (field, matches) in [
            ("product", self.product == self.bundle.product),
            ("repository", self.repository == self.bundle.repository),
            ("version", self.version == self.bundle.version),
            ("tag", self.tag == self.bundle.tag),
            (
                "source_commit",
                self.source_commit == self.bundle.source_commit,
            ),
            ("target", self.target == self.bundle.target),
        ] {
            if !matches {
                return Err(ReleaseManifestError::FragmentBundleMismatch {
                    target: self.target.clone(),
                    field,
                });
            }
        }
        Ok(())
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ReleaseManifestError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical
            .bundle
            .components
            .sort_by_key(|component| component.name);
        deterministic_json(&canonical)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

impl AggregateDistributionManifest {
    pub fn from_platforms(
        platforms: Vec<PlatformDistributionFragment>,
    ) -> Result<Self, ReleaseManifestError> {
        let first = platforms
            .first()
            .ok_or_else(|| ReleaseManifestError::TargetMatrix {
                expected: SUPPORTED_DISTRIBUTION_TARGETS
                    .iter()
                    .map(|target| (*target).to_string())
                    .collect(),
                actual: Vec::new(),
            })?;
        let aggregate = Self {
            schema_version: DISTRIBUTION_SCHEMA_VERSION,
            product: first.product.clone(),
            repository: first.repository.clone(),
            version: first.version.clone(),
            tag: first.tag.clone(),
            source_commit: first.source_commit.clone(),
            platforms,
        };
        aggregate.validate()?;
        Ok(aggregate)
    }

    pub fn validate(&self) -> Result<(), ReleaseManifestError> {
        validate_schema(self.schema_version)?;
        validate_identity(
            &self.product,
            &self.repository,
            &self.version,
            &self.tag,
            &self.source_commit,
        )?;

        let expected = SUPPORTED_DISTRIBUTION_TARGETS
            .iter()
            .map(|target| (*target).to_string())
            .collect::<BTreeSet<_>>();
        let actual = self
            .platforms
            .iter()
            .map(|platform| platform.target.clone())
            .collect::<BTreeSet<_>>();
        if self.platforms.len() != SUPPORTED_DISTRIBUTION_TARGETS.len() || actual != expected {
            return Err(ReleaseManifestError::TargetMatrix {
                expected: expected.into_iter().collect(),
                actual: actual.into_iter().collect(),
            });
        }

        for platform in &self.platforms {
            platform.validate()?;
            for (field, matches) in [
                ("product", platform.product == self.product),
                ("repository", platform.repository == self.repository),
                ("version", platform.version == self.version),
                ("tag", platform.tag == self.tag),
                (
                    "source_commit",
                    platform.source_commit == self.source_commit,
                ),
            ] {
                if !matches {
                    return Err(ReleaseManifestError::AggregateIdentityMismatch {
                        target: platform.target.clone(),
                        field,
                    });
                }
            }
        }
        let Some(first) = self.platforms.first() else {
            return Err(ReleaseManifestError::TargetMatrix {
                expected: SUPPORTED_DISTRIBUTION_TARGETS
                    .iter()
                    .map(|target| (*target).to_string())
                    .collect(),
                actual: Vec::new(),
            });
        };
        for platform in &self.platforms {
            if platform.bundle.source_archive != first.bundle.source_archive {
                return Err(ReleaseManifestError::SourceArchiveMismatch {
                    target: platform.target.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ReleaseManifestError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical
            .platforms
            .sort_by_key(|platform| target_sort_key(&platform.target));
        for platform in &mut canonical.platforms {
            platform
                .bundle
                .components
                .sort_by_key(|component| component.name);
        }
        deterministic_json(&canonical)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

fn deterministic_json<T: Serialize>(value: &T) -> Result<Vec<u8>, ReleaseManifestError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn validate_schema(actual: u32) -> Result<(), ReleaseManifestError> {
    if actual != DISTRIBUTION_SCHEMA_VERSION {
        return Err(ReleaseManifestError::SchemaVersion {
            actual,
            expected: DISTRIBUTION_SCHEMA_VERSION,
        });
    }
    Ok(())
}

fn validate_identity(
    product: &str,
    repository: &str,
    version: &str,
    tag: &str,
    source_commit: &str,
) -> Result<(), ReleaseManifestError> {
    if product != DISTRIBUTION_PRODUCT {
        return Err(ReleaseManifestError::Product {
            actual: product.to_string(),
            expected: DISTRIBUTION_PRODUCT,
        });
    }
    if repository != DISTRIBUTION_REPOSITORY {
        return Err(ReleaseManifestError::Repository {
            actual: repository.to_string(),
            expected: DISTRIBUTION_REPOSITORY,
        });
    }
    Version::parse(version).map_err(|error| ReleaseManifestError::Version {
        value: version.to_string(),
        detail: error.to_string(),
    })?;
    if tag != format!("v{version}") {
        return Err(ReleaseManifestError::Tag {
            version: version.to_string(),
            tag: tag.to_string(),
        });
    }
    if !is_lowercase_git_oid(source_commit) {
        return Err(ReleaseManifestError::SourceCommit {
            value: source_commit.to_string(),
        });
    }
    Ok(())
}

fn is_lowercase_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_target(target: &str) -> Result<(), ReleaseManifestError> {
    if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&target) {
        return Err(ReleaseManifestError::UnsupportedTarget {
            target: target.to_string(),
        });
    }
    Ok(())
}

fn target_sort_key(target: &str) -> usize {
    match target {
        "x86_64-pc-windows-msvc" => 0,
        "x86_64-unknown-linux-musl" => 1,
        "x86_64-apple-darwin" => 2,
        "aarch64-apple-darwin" => 3,
        _ => SUPPORTED_DISTRIBUTION_TARGETS.len(),
    }
}

fn validate_components(
    target: &str,
    components: &[DistributionComponent],
) -> Result<(), ReleaseManifestError> {
    let names = components
        .iter()
        .map(|component| component.name)
        .collect::<BTreeSet<_>>();
    let required = [
        DistributionComponentName::Vibe,
        DistributionComponentName::VibeIndex,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if components.len() != required.len() || names != required {
        return Err(ReleaseManifestError::Components {
            target: target.to_string(),
        });
    }
    let paths = components
        .iter()
        .map(|component| component.path.as_str())
        .collect::<BTreeSet<_>>();
    if paths.len() != components.len() {
        return Err(ReleaseManifestError::Components {
            target: target.to_string(),
        });
    }
    for component in components {
        if !is_bundle_relative_file(&component.path) {
            return Err(ReleaseManifestError::ComponentPath {
                component: component.name.as_str().to_string(),
                path: component.path.clone(),
            });
        }
        if component.size == 0 {
            return Err(ReleaseManifestError::EmptyArtifact {
                field: format!("component `{}`", component.name.as_str()),
            });
        }
        validate_digest(
            &format!("component `{}`", component.name.as_str()),
            &component.digest,
        )?;
    }
    Ok(())
}

fn validate_asset(asset: &DistributionAsset) -> Result<(), ReleaseManifestError> {
    if asset.name.trim().is_empty() || asset.name.contains('/') || asset.name.contains('\\') {
        return Err(ReleaseManifestError::AssetName);
    }
    if asset.size == 0 {
        return Err(ReleaseManifestError::EmptyArtifact {
            field: "distribution asset".to_string(),
        });
    }
    validate_digest("distribution asset", &asset.digest)
}

fn validate_source_archive(
    source_archive: &DistributionSourceArchive,
) -> Result<(), ReleaseManifestError> {
    if !is_bundle_relative_file(&source_archive.path) {
        return Err(ReleaseManifestError::SourceArchivePath {
            path: source_archive.path.clone(),
        });
    }
    if source_archive.size == 0 {
        return Err(ReleaseManifestError::EmptyArtifact {
            field: "source archive".to_string(),
        });
    }
    validate_digest("source archive", &source_archive.digest)?;
    if !is_lowercase_git_oid(&source_archive.tree_oid) {
        return Err(ReleaseManifestError::SourceArchiveTreeOid {
            value: source_archive.tree_oid.clone(),
        });
    }
    Ok(())
}

fn validate_digest(field: &str, digest: &str) -> Result<(), ReleaseManifestError> {
    let valid = digest.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    });
    if !valid {
        return Err(ReleaseManifestError::Digest {
            field: field.to_string(),
            digest: digest.to_string(),
        });
    }
    Ok(())
}

fn is_bundle_relative_file(value: &str) -> bool {
    if value.trim().is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.ends_with('/')
        || value.ends_with('\\')
        || value.contains(':')
        || value
            .split(['/', '\\'])
            .any(|segment| segment.is_empty() || segment == "..")
    {
        return false;
    }
    let mut saw_normal = false;
    for component in Path::new(value).components() {
        match component {
            Component::Normal(_) => saw_normal = true,
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    saw_normal
}

#[cfg(test)]
#[path = "release_manifest/tests.rs"]
mod tests;
