//! Per-package registry projection used as a derivation input.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{FactOrigin, FactStatus, Registry};

/// Package-owned registry entries keyed by their full fact address.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageOverlay {
    entries: BTreeMap<String, Option<FactStatus>>,
}

impl PackageOverlay {
    /// The consumer status for `address`; indeterminate entries do not override.
    pub fn status_for(&self, address: &str) -> Option<FactStatus> {
        self.entries.get(address).copied().flatten()
    }

    /// Whether the overlay carries this address, including indeterminate entries.
    pub fn contains_address(&self, address: &str) -> bool {
        self.entries.contains_key(address)
    }

    /// Whether any entry belongs to the document denoted by `address_prefix`.
    pub fn contains_document(&self, address_prefix: &str) -> bool {
        self.entries
            .range(address_prefix.to_string()..)
            .next()
            .is_some_and(|(address, _)| address.starts_with(address_prefix))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Registry {
    /// Validate and project prospective missing entries without writing registry bytes.
    /// Existing entries, including indeterminate entries, retain precedence.
    pub fn package_overlay_with_additions(
        &self,
        package: &str,
        additions: &[crate::FactEntry],
    ) -> Result<PackageOverlay, crate::RegistryError> {
        let mut overlay = self.package_overlay(package);
        for entry in additions {
            entry.validate()?;
            if entry.origin != FactOrigin::Package || entry.package.as_deref() != Some(package) {
                return Err(crate::RegistryError::Invariant(
                    "prospective overlay contains another source".into(),
                ));
            }
            overlay
                .entries
                .entry(entry.address.clone())
                .or_insert(entry.status);
        }
        Ok(overlay)
    }

    /// Project the `origin = "package"` records of one package.
    pub fn package_overlay(&self, package: &str) -> PackageOverlay {
        PackageOverlay {
            entries: self
                .entries()
                .filter(|entry| {
                    entry.origin == FactOrigin::Package && entry.package.as_deref() == Some(package)
                })
                .map(|entry| (entry.address.clone(), entry.status))
                .collect(),
        }
    }
}

/// Hash the exact registry-file bytes for one package.
///
/// Absence (or an unreadable path) is represented as `None`, matching the
/// derived-manifest wire where no package overlay has no field at all.
pub fn overlay_file_hash(project_root: &Path, package: &str) -> Option<String> {
    let path = project_root
        .join(vibe_core::layout::current_vibefacts_root())
        .join(format!("{}.toml", package.replace('/', ".")));
    let bytes = fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    Some(format!("sha256:{hex}"))
}

#[cfg(test)]
mod prospective_tests {
    use super::*;
    #[test]
    fn prospective_overlay_preserves_indeterminate_and_rejects_other_package() {
        let root = tempfile::tempdir().unwrap();
        let mut registry = Registry::default();
        let mut entry = crate::FactEntry {
            address: "spec://org.example/rules/law#LAW".into(),
            origin: FactOrigin::Package,
            package: Some("org.example/rules".into()),
            status: None,
            comment: Some("keep".into()),
        };
        registry.upsert(root.path(), entry.clone()).unwrap();
        let before = overlay_file_hash(root.path(), "org.example/rules");
        entry.status = Some(FactStatus::parse("impl/done").unwrap());
        let projected = registry
            .package_overlay_with_additions("org.example/rules", &[entry.clone()])
            .unwrap();
        assert!(projected.contains_address(&entry.address));
        assert_eq!(projected.status_for(&entry.address), None);
        assert!(
            registry
                .package_overlay_with_additions("org.other/rules", &[entry])
                .is_err()
        );
        assert_eq!(before, overlay_file_hash(root.path(), "org.example/rules"));
    }
}
