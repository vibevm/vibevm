//! Behaviour of the distribution manifest family — the wire contracts
//! shared by the five independent vibevm distribution builds.
//!
//! The shapes are generated from
//! [`schemas/distribution/e1/aggregate_distribution_manifest.jtd.json`](../../../../../schemas/distribution/e1/aggregate_distribution_manifest.jtd.json)
//! into [`crate::generated::distribution::e1::aggregate_distribution_manifest`];
//! this module carries what a shape cannot say about itself — the release
//! constants, the independent parser limits, the relational validation, and
//! the deterministic serializers.
//!
//! Each platform bundle carries a [`BundleDistributionManifest`] named
//! [`DISTRIBUTION_MANIFEST_FILENAME`]. The same build emits a
//! [`PlatformDistributionFragment`], and the release aggregation job combines
//! exactly one fragment for every target in [`SUPPORTED_DISTRIBUTION_TARGETS`]
//! into an [`AggregateDistributionManifest`]. All three documents are strict
//! JSON contracts with deterministic serializers.
//!
//! The complete matrix is the *publishing* rule, and only writers and
//! release-host mirrors are held to it
//! ([`AggregateDistributionManifest::validate`]). An installation reads one
//! platform out of an aggregate, so a release drop may carry an aggregate
//! describing only the targets that site runs
//! ([`AggregateDistributionManifest::validate_for_target`], which documents
//! why that costs no integrity).
//!
//! It lives in this crate rather than beside its callers because the orphan
//! rule leaves no alternative: an inherent `impl` belongs in the crate that
//! defines the type, and `vibe-publish` re-exports these types through
//! `vibe_publish::release_manifest` instead of duplicating them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#instances");

use std::collections::BTreeSet;

use semver::Version;
use serde::Serialize;

pub use crate::generated::distribution::e1::aggregate_distribution_manifest::{
    AggregateDistributionManifest, BundleDistributionManifest, DistributionAsset,
    DistributionComponent, DistributionComponentName, DistributionSourceArchive,
    PlatformDistributionFragment,
};

mod error;
pub use error::{RELEASE_MANIFEST_CONTRACT, ReleaseManifestError};
mod validation;
use validation::*;

pub const DISTRIBUTION_MANIFEST_FILENAME: &str = "DISTRIBUTION.json";
pub const DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME: &str = "DISTRIBUTIONS.json";
pub const DISTRIBUTION_SOURCE_ARCHIVE_FILENAME: &str = "vibevm-source.zip";
pub const DISTRIBUTION_BASH_INSTALLER_FILENAME: &str = "install.sh";
pub const DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME: &str = "install.ps1";
pub const DISTRIBUTION_CMD_INSTALLER_FILENAME: &str = "install.cmd";
/// Independent parser limit for a bundle manifest or platform fragment.
pub const DISTRIBUTION_MANIFEST_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Independent parser limit for the five-platform aggregate manifest.
pub const DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum declared size of each executable carried inside a bundle.
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
pub const SUPPORTED_DISTRIBUTION_TARGETS: [&str; 5] = [
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

impl DistributionComponentName {
    /// The wire string this component name spells — the same value the
    /// schema's closed vocabulary declares, and the name the installer
    /// scripts read out of `components[].name`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vibe => "vibe",
            Self::VibeIndex => "vibe-index",
            Self::VibeAppProxy => "vibe-app-proxy",
        }
    }
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
        // The constructed value is canonical: components in the order the
        // wire form writes them, so a manifest read back from its own JSON
        // compares equal to the one that produced it whatever order the
        // caller pushed the components in.
        let mut components = components;
        components.sort_by_key(|component| component_sort_key(&component.name));
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
        canonical
            .components
            .sort_by_key(|component| component_sort_key(&component.name));
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
            .sort_by_key(|component| component_sort_key(&component.name));
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

    /// Everything that is true of any aggregate, whole or partial: the
    /// schema, the release identity, at least one platform, every platform
    /// valid in itself and for a target this vibe supports, no target
    /// twice, no release-asset name used twice, every platform bound to the
    /// aggregate's own identity, and one source archive across them all.
    ///
    /// These are the rules the *entries present* must satisfy. Which
    /// entries must be present is the caller's question, and the two
    /// callers answer it differently: [`Self::validate`] for a publisher,
    /// [`Self::validate_for_target`] for a consumer.
    fn validate_entries(&self) -> Result<(), ReleaseManifestError> {
        validate_schema(self.schema_version)?;
        validate_identity(
            &self.product,
            &self.repository,
            &self.version,
            &self.tag,
            &self.source_commit,
        )?;
        let Some(first) = self.platforms.first() else {
            return Err(ReleaseManifestError::TargetMatrix {
                expected: SUPPORTED_DISTRIBUTION_TARGETS
                    .iter()
                    .map(|target| (*target).to_string())
                    .collect(),
                actual: Vec::new(),
            });
        };

        let mut release_asset_names = BTreeSet::new();
        let mut seen_targets = BTreeSet::new();
        for platform in &self.platforms {
            // Refuses a target outside `SUPPORTED_DISTRIBUTION_TARGETS`
            // (`UnsupportedTarget`) before anything else is believed of it.
            platform.validate()?;
            if !seen_targets.insert(platform.target.clone()) {
                return Err(ReleaseManifestError::TargetDuplicate {
                    target: platform.target.clone(),
                });
            }
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
            if platform.bundle.source_archive != first.bundle.source_archive {
                return Err(ReleaseManifestError::SourceArchiveMismatch {
                    target: platform.target.clone(),
                });
            }
        }
        Ok(())
    }

    /// The **publishing** contract: a published release describes every
    /// supported target exactly once, built from one commit
    /// (`PROP-019#release-production`). `from_platforms`, `to_json_bytes`
    /// and `from_json_slice` hold writers and release-host mirrors to it,
    /// so no incomplete release is ever written or republished.
    pub fn validate(&self) -> Result<(), ReleaseManifestError> {
        self.validate_entries()?;
        let expected = SUPPORTED_DISTRIBUTION_TARGETS
            .iter()
            .map(|target| (*target).to_string())
            .collect::<BTreeSet<_>>();
        let actual = self
            .platforms
            .iter()
            .map(|platform| platform.target.clone())
            .collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(ReleaseManifestError::TargetMatrix {
                expected: expected.into_iter().collect(),
                actual: actual.into_iter().collect(),
            });
        }
        Ok(())
    }

    /// The **consuming** contract (`PROP-019#release-sources`): an
    /// installation reads one platform out of an aggregate, so what it
    /// needs is this host's target, described exactly once. An aggregate
    /// that also describes the other four is the published shape and
    /// satisfies this too.
    ///
    /// Why a partial aggregate may be consumed when it may not be
    /// published: nothing authenticates the aggregate document as a whole.
    /// There is no signature and no outer digest over it — it *is* the root
    /// of trust, reached over a transport the operator chose (the canonical
    /// HTTPS release directory, or a drop and a remap row on this machine),
    /// and what it authenticates is each platform's bundle and bootstrap by
    /// name, size and SHA-256, cross-checked afterwards against the bundle's
    /// own embedded manifest. Those digests are per entry: an entry neither
    /// gains nor loses strength from its neighbours' presence. A forger who
    /// can rewrite one entry can write five consistent ones just as cheaply,
    /// so requiring the full matrix buys a consumer nothing — while it
    /// refuses an air-gapped site that copied only the platform it runs
    /// (B-247). The cold-start installers already read an aggregate this
    /// way: `install.ps1` takes the single entry matching its target and
    /// never counts the rest, and `install.sh` refuses with "release
    /// manifest has no bootstrap for `<target>`".
    ///
    /// Still refused: an entry for an unsupported target, the same target
    /// twice, an aggregate with no entries, and an aggregate that does not
    /// describe `target` — that last one by name, because the operator's
    /// question is which file to copy next.
    pub fn validate_for_target(&self, target: &str) -> Result<(), ReleaseManifestError> {
        self.validate_entries()?;
        if !self
            .platforms
            .iter()
            .any(|platform| platform.target == target)
        {
            return Err(ReleaseManifestError::TargetAbsent {
                target: target.to_owned(),
                actual: self
                    .platforms
                    .iter()
                    .map(|platform| platform.target.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            });
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
                .sort_by_key(|component| component_sort_key(&component.name));
        }
        bounded_json(
            &canonical,
            DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
            "aggregate manifest",
        )
    }

    /// Parse a **published** aggregate: the complete matrix, as a release
    /// host serves it and a mirror republishes it.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        let value = Self::parse(bytes)?;
        value.validate()?;
        Ok(value)
    }

    /// Parse an aggregate an installation will read one platform out of —
    /// which may be a drop's partial aggregate
    /// ([`Self::validate_for_target`]).
    pub fn from_json_slice_for_target(
        bytes: &[u8],
        target: &str,
    ) -> Result<Self, ReleaseManifestError> {
        let value = Self::parse(bytes)?;
        value.validate_for_target(target)?;
        Ok(value)
    }

    fn parse(bytes: &[u8]) -> Result<Self, ReleaseManifestError> {
        validate_buffer_size(
            "aggregate manifest",
            bytes.len() as u64,
            DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
        )?;
        Ok(serde_json::from_slice(bytes)?)
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
