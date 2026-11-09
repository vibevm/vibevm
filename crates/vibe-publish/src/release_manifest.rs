//! Wire contracts shared by the five independent vibevm distribution builds.
//!
//! The shapes are generated from
//! [`schemas/distribution/e1/aggregate_distribution_manifest.jtd.json`](../../../schemas/distribution/e1/aggregate_distribution_manifest.jtd.json),
//! registered as `distribution-bundle-manifest` and
//! `distribution-aggregate-manifest` in
//! [`formats/REGISTRY.toml`](../../../formats/REGISTRY.toml); the release
//! constants, the independent parser limits, the relational validation and
//! the deterministic serializers live beside them in
//! `vibe_wire::behaviour::release_manifest`, because an inherent `impl`
//! belongs to the crate that defines the type.
//!
//! This module is that pair's address for the publishing side: the names
//! below are exactly the ones `vibe-publish` has always exported, so
//! `vibe self`, `vibe vvm` and `cargo xtask dist` keep reading one module
//! path while the type definitions are machine-generated.
//!
//! Each platform bundle carries a [`BundleDistributionManifest`] named
//! [`DISTRIBUTION_MANIFEST_FILENAME`]. The same build emits a
//! [`PlatformDistributionFragment`], and the release aggregation job combines
//! exactly one fragment for every target in [`SUPPORTED_DISTRIBUTION_TARGETS`]
//! into an [`AggregateDistributionManifest`]. All three documents are strict
//! JSON contracts with deterministic serializers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#instances");

/// The release's companion tools (`TOOLS.json`, PROP-019
/// `#companion-tools`): px per target as its own asset, beside
/// `DISTRIBUTIONS.json`, whose closed shape older vibe releases refuse to
/// extend.
pub use vibe_wire::generated::distribution_tools::{
    CompanionAsset, CompanionTool, DistributionToolsManifest,
};
/// The companion manifest's asset name.
pub const DISTRIBUTION_TOOLS_MANIFEST_FILENAME: &str = "TOOLS.json";
/// The companion manifest's read bound.
pub const DISTRIBUTION_TOOLS_MANIFEST_MAX_BYTES: u64 = 1024 * 1024;
/// A companion tool's asset bound (px is a frozen Python of tens of MB).
pub const DISTRIBUTION_TOOL_ASSET_MAX_BYTES: u64 = 256 * 1024 * 1024;

pub use vibe_wire::behaviour::release_manifest::{
    AggregateDistributionManifest, BundleDistributionManifest,
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    DISTRIBUTION_BASH_INSTALLER_FILENAME, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_CMD_INSTALLER_FILENAME,
    DISTRIBUTION_COMPONENT_MAX_BYTES, DISTRIBUTION_MANIFEST_FILENAME,
    DISTRIBUTION_MANIFEST_MAX_BYTES, DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
    DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY, DISTRIBUTION_SCHEMA_VERSION,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
    DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES, DISTRIBUTION_SOURCE_MAX_DEPTH,
    DISTRIBUTION_SOURCE_MAX_FILES, DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, PlatformDistributionFragment,
    RELEASE_MANIFEST_CONTRACT, ReleaseManifestError, SUPPORTED_DISTRIBUTION_TARGETS,
};

#[cfg(test)]
#[path = "release_manifest/tests.rs"]
mod tests;
