use std::collections::BTreeSet;
use std::path::{Component, Path};

use semver::Version;

use super::{
    DISTRIBUTION_COMPONENT_MAX_BYTES, DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY,
    DISTRIBUTION_SCHEMA_VERSION, DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES, DistributionAsset,
    DistributionComponent, DistributionComponentName, DistributionSourceArchive,
    ReleaseManifestError, SUPPORTED_DISTRIBUTION_TARGETS,
};

pub(super) fn validate_schema(actual: u32) -> Result<(), ReleaseManifestError> {
    if actual != DISTRIBUTION_SCHEMA_VERSION {
        return Err(ReleaseManifestError::SchemaVersion {
            actual,
            expected: DISTRIBUTION_SCHEMA_VERSION,
        });
    }
    Ok(())
}

pub(super) fn validate_identity(
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

pub(super) fn validate_target(target: &str) -> Result<(), ReleaseManifestError> {
    if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&target) {
        return Err(ReleaseManifestError::UnsupportedTarget {
            target: target.to_string(),
        });
    }
    Ok(())
}

pub(super) fn target_sort_key(target: &str) -> usize {
    match target {
        "x86_64-pc-windows-msvc" => 0,
        "x86_64-unknown-linux-musl" => 1,
        "x86_64-apple-darwin" => 2,
        "aarch64-apple-darwin" => 3,
        _ => SUPPORTED_DISTRIBUTION_TARGETS.len(),
    }
}

pub(super) fn validate_components(
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
        validate_buffer_size(
            &format!("component `{}`", component.name.as_str()),
            component.size,
            DISTRIBUTION_COMPONENT_MAX_BYTES,
        )?;
        validate_digest(
            &format!("component `{}`", component.name.as_str()),
            &component.digest,
        )?;
    }
    Ok(())
}

pub(super) fn validate_asset(asset: &DistributionAsset) -> Result<(), ReleaseManifestError> {
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

pub(super) fn validate_source_archive(
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
    validate_buffer_size(
        "source archive",
        source_archive.size,
        DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
    )?;
    validate_digest("source archive", &source_archive.digest)?;
    if !is_lowercase_git_oid(&source_archive.tree_oid) {
        return Err(ReleaseManifestError::SourceArchiveTreeOid {
            value: source_archive.tree_oid.clone(),
        });
    }
    Ok(())
}

pub(super) fn validate_buffer_size(
    field: &str,
    size: u64,
    max: u64,
) -> Result<(), ReleaseManifestError> {
    if size > max {
        return Err(ReleaseManifestError::ArtifactTooLarge {
            field: field.to_string(),
            size,
            max,
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

fn is_lowercase_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
