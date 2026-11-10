//! Manifest reads from local-directory registry entries.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

pub(super) fn read_local_dep_manifest(
    registry: &LocalRegistry,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Manifest, RegistryError> {
    let path = registry
        .root()
        .join(group.as_str())
        .join(name)
        .join(format!("v{version}"))
        .join(Manifest::FILENAME);
    if !path.exists() {
        // **Call site 2 of PROP-075 `#TEMPLATE-DOMAIN-SEAM`**: a version
        // the registry does not publish but its template produces gets a
        // **skeleton** render — the stage that carries the identity,
        // `[compatibility]`, `[requires]` and the installer options'
        // platforms, and costs no digest request
        // (`#TEMPLATE-TWO-STAGE`).
        if let Some(skeleton) = registry.template_skeleton(group, name, version)? {
            return Ok(skeleton);
        }
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
