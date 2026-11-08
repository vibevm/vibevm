//! Pure pin/catalog admission; no transport, mutable preference or registry creation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::{RegistryPin, SourcePolicyError};
use crate::manifest::{LockedPackage, RegistrySection};

pub fn validate_registry_aliases(
    rows: &[RegistrySection],
    scope: &'static str,
) -> Result<(), SourcePolicyError> {
    let mut names = std::collections::BTreeSet::new();
    for row in rows {
        RegistryPin::new(row.name.clone())?;
        if !names.insert(&row.name) {
            return Err(SourcePolicyError::DuplicateRegistry {
                registry: row.name.clone(),
                scope,
            });
        }
    }
    Ok(())
}
/// A held matching alias is checked before any current catalog is needed.
pub fn validate_locked_registry_pin(
    package: &str,
    pin: &RegistryPin,
    held: &LockedPackage,
) -> Result<(), SourcePolicyError> {
    if held.registry.as_deref() != Some(pin.as_str()) {
        return Err(SourcePolicyError::LockedConflict {
            package: package.into(),
            requested: pin.to_string(),
            held: held.registry.clone(),
        });
    }
    Ok(())
}
pub fn validate_new_registry_pin(
    package: &str,
    pin: &RegistryPin,
    catalog: &[RegistrySection],
) -> Result<(), SourcePolicyError> {
    validate_registry_aliases(catalog, "effective")?;
    let row = catalog
        .iter()
        .find(|row| row.name == pin.as_str())
        .ok_or_else(|| SourcePolicyError::UnknownRegistry {
            package: package.into(),
            registry: pin.to_string(),
        })?;
    if !row.enabled {
        return Err(SourcePolicyError::DisabledRegistry {
            package: package.into(),
            registry: pin.to_string(),
        });
    }
    Ok(())
}
