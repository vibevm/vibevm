//! Manifest reads from local-directory registry entries.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

pub(super) fn read_local_dep_manifest(
    ls: &super::super::LocalRegistrySource,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Manifest, RegistryError> {
    let path = ls
        .registry
        .root()
        .join(group.as_str())
        .join(name)
        .join(format!("v{version}"))
        .join(Manifest::FILENAME);
    if !path.exists() {
        return Err(RegistryError::UnknownPackage {
            group: group.clone(),
            name: name.to_string(),
        });
    }
    Manifest::read(&path).map_err(|e| RegistryError::MalformedMeta {
        path: path.clone(),
        reason: e.to_string(),
    })
}
