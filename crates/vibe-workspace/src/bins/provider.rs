//! Provider-scoped binary lookup for extension dispatch.

use std::path::Path;

use specmark::spec;
use vibe_core::Group;
use vibe_core::manifest::Manifest;

use super::{BinsError, DeclaredBinary};

/// Resolve `binary_name` from one exact installed provider slot.
///
/// Unlike [`super::find_binary`], this never searches the world by a bare
/// binary name. The slot manifest must agree with the provider coordinate and
/// version retained by extension collection, so two providers may safely
/// declare the same PATH-facing name.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY")]
pub fn find_binary_in_provider_slot(
    provider_slot: &Path,
    provider_group: &Group,
    provider_name: &str,
    provider_version: &str,
    binary_name: &str,
) -> Result<DeclaredBinary, BinsError> {
    find_binary_in_provider_root(
        provider_slot,
        provider_group,
        provider_name,
        provider_version,
        binary_name,
        provider_slot
            .parent()
            .and_then(Path::parent)
            .unwrap_or(provider_slot),
    )
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY")]
pub fn find_binary_in_authored_package_root(
    provider_root: &Path,
    provider_group: &Group,
    provider_name: &str,
    provider_version: &str,
    binary_name: &str,
) -> Result<DeclaredBinary, BinsError> {
    find_binary_in_provider_root(
        provider_root,
        provider_group,
        provider_name,
        provider_version,
        binary_name,
        provider_root,
    )
}

fn find_binary_in_provider_root(
    provider_slot: &Path,
    provider_group: &Group,
    provider_name: &str,
    provider_version: &str,
    binary_name: &str,
    dependency_root: &Path,
) -> Result<DeclaredBinary, BinsError> {
    let manifest_path = provider_slot.join(Manifest::FILENAME);
    let manifest = Manifest::read(&manifest_path).map_err(|error| BinsError::ProviderManifest {
        path: manifest_path,
        detail: error.to_string(),
    })?;
    let package = manifest
        .package
        .as_ref()
        .ok_or_else(|| BinsError::ProviderMismatch {
            slot: provider_slot.to_path_buf(),
            expected: format!("{provider_group}/{provider_name}@{provider_version}"),
            actual: "manifest has no [package] coordinate".to_string(),
        })?;
    let actual = format!("{}/{}@{}", package.group, package.name, package.version);
    let expected = format!("{provider_group}/{provider_name}@{provider_version}");
    if package.group != *provider_group
        || package.name != provider_name
        || package.version.to_string() != provider_version
    {
        return Err(BinsError::ProviderMismatch {
            slot: provider_slot.to_path_buf(),
            expected,
            actual,
        });
    }

    let decl = manifest
        .binaries
        .iter()
        .find(|decl| decl.name == binary_name)
        .cloned()
        .ok_or_else(|| BinsError::UnknownProviderBinary {
            package: format!("{provider_group}/{provider_name}"),
            name: binary_name.to_string(),
            known: manifest
                .binaries
                .iter()
                .map(|decl| decl.name.clone())
                .collect(),
        })?;
    Ok(DeclaredBinary {
        decl,
        package: format!("{provider_group}/{provider_name}"),
        group: provider_group.to_string(),
        vibedeps_root: dependency_root.to_path_buf(),
        slot: provider_slot.to_path_buf(),
    })
}
