//! Wire contracts shared by the four independent vibevm distribution builds.
//!
//! Each platform bundle carries a [`BundleDistributionManifest`] named
//! [`DISTRIBUTION_MANIFEST_FILENAME`]. The same build emits a
//! [`PlatformDistributionFragment`], and the release aggregation job combines
//! exactly one fragment for every target in [`SUPPORTED_DISTRIBUTION_TARGETS`]
//! into an [`AggregateDistributionManifest`]. All three documents are strict
//! JSON contracts with deterministic serializers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#instances");

use std::collections::BTreeSet;

use semver::Version;
use serde::{Deserialize, Serialize};

mod error;
pub use error::{RELEASE_MANIFEST_CONTRACT, ReleaseManifestError};
mod validation;
use validation::*;

pub const DISTRIBUTION_MANIFEST_FILENAME: &str = "DISTRIBUTION.json";
pub const DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME: &str = "DISTRIBUTIONS.json";
pub const DISTRIBUTION_SOURCE_ARCHIVE_FILENAME: &str = "vibevm-source.zip";
pub const DISTRIBUTION_BASH_INSTALLER_FILENAME: &str = "install.sh";
pub const DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME: &str = "install.ps1";
/// Independent parser limit for a bundle manifest or platform fragment.
pub const DISTRIBUTION_MANIFEST_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Independent parser limit for the four-platform aggregate manifest.
pub const DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum declared size of either executable carried inside a bundle.
pub const DISTRIBUTION_COMPONENT_MAX_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum declared size of the raw bootstrap executable release asset.
pub const DISTRIBUTION_BOOTSTRAP_MAX_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum compressed size of the canonical source snapshot inside a bundle.
pub const DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum total expanded bytes accepted from the tracked source snapshot.
pub const DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Maximum regular-file count accepted from the tracked source snapshot.
pub const DISTRIBUTION_SOURCE_MAX_FILES: u64 = 200_000;
/// Maximum materialized files plus unique implicit directories in source/.
pub const DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES: u64 = 400_000;
/// Maximum slash-separated component count for one tracked source path.
pub const DISTRIBUTION_SOURCE_MAX_DEPTH: usize = 256;
/// Maximum UTF-8 bytes in one canonical (ASCII-only) tracked source path.
pub const DISTRIBUTION_SOURCE_MAX_PATH_BYTES: usize = 4096;
/// Stay strictly below GitHub Releases' 2 GiB per-file ceiling.
pub const DISTRIBUTION_BUNDLE_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024 - 1;
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
    /// Raw `vibe[.exe]` used by dependency-light hosted bootstraps before
    /// they have a ZIP reader or an installed VVM.
    pub bootstrap: DistributionAsset,
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
        bounded_json(
            &canonical,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
            "bundle manifest",
        )
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        validate_buffer_size(
            "bundle manifest",
            bytes.len() as u64,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

impl PlatformDistributionFragment {
    pub fn new(
        asset: DistributionAsset,
        bootstrap: DistributionAsset,
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
            bootstrap,
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
        validate_buffer_size(
            "platform bundle",
            self.asset.size,
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        )?;
        validate_asset(&self.bootstrap)?;
        validate_buffer_size(
            "raw bootstrap",
            self.bootstrap.size,
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        if self.asset.name == self.bootstrap.name {
            return Err(ReleaseManifestError::AssetNameCollision {
                target: self.target.clone(),
                name: self.asset.name.clone(),
            });
        }
        self.bundle.validate()?;
        let vibe = self
            .bundle
            .components
            .iter()
            .find(|component| component.name == DistributionComponentName::Vibe)
            .ok_or_else(|| ReleaseManifestError::Components {
                target: self.target.clone(),
            })?;
        if self.bootstrap.size != vibe.size {
            return Err(ReleaseManifestError::BootstrapMismatch {
                target: self.target.clone(),
                field: "size",
            });
        }
        if self.bootstrap.digest != vibe.digest {
            return Err(ReleaseManifestError::BootstrapMismatch {
                target: self.target.clone(),
                field: "digest",
            });
        }
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
        bounded_json(
            &canonical,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
            "platform fragment",
        )
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        validate_buffer_size(
            "platform fragment",
            bytes.len() as u64,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
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

        let mut release_asset_names = BTreeSet::new();
        for platform in &self.platforms {
            platform.validate()?;
            for asset in [&platform.asset, &platform.bootstrap] {
                if !release_asset_names.insert(asset.name.clone()) {
                    return Err(ReleaseManifestError::AssetNameCollision {
                        target: platform.target.clone(),
                        name: asset.name.clone(),
                    });
                }
            }
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
        bounded_json(
            &canonical,
            DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
            "aggregate manifest",
        )
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        validate_buffer_size(
            "aggregate manifest",
            bytes.len() as u64,
            DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
        )?;
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

fn bounded_json<T: Serialize>(
    value: &T,
    max: u64,
    field: &'static str,
) -> Result<Vec<u8>, ReleaseManifestError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    validate_buffer_size(field, bytes.len() as u64, max)?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "release_manifest/tests.rs"]
mod tests;
