//! Package-overlay lifecycle operations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::store::remove_empty_home;
use crate::{FactOrigin, Registry, RegistryError};

/// One package registry file whose source package is no longer installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanReport {
    pub package: String,
    pub file: PathBuf,
    pub entries: usize,
}

/// The deterministic registry path for one `<group>/<name>` package.
pub fn package_file_path(project_root: &Path, package: &str) -> PathBuf {
    let safe_name = package.replace(['/', '\\'], ".");
    project_root
        .join("vibefacts")
        .join(format!("{safe_name}.toml"))
}

/// Report package overlay files whose packages are absent from `installed`.
/// Host `spec.toml` entries never participate in this lifecycle.
pub fn orphans(
    project_root: &Path,
    installed: &BTreeSet<String>,
) -> Result<Vec<OrphanReport>, RegistryError> {
    let registry = Registry::load(project_root)?;
    let mut counts = BTreeMap::<String, usize>::new();
    for entry in registry.entries() {
        if entry.origin != FactOrigin::Package {
            continue;
        }
        let Some(package) = entry.package.as_ref() else {
            return Err(RegistryError::Invariant(format!(
                "package fact `{}` has no source package",
                entry.address
            )));
        };
        *counts.entry(package.clone()).or_default() += 1;
    }
    Ok(counts
        .into_iter()
        .filter(|(package, _)| !installed.contains(package))
        .map(|(package, entries)| OrphanReport {
            file: package_file_path(project_root, &package),
            package,
            entries,
        })
        .collect())
}

/// Remove one package overlay file, then prune an empty `vibefacts/` home.
/// Returns `false` when the package had no overlay file.
pub fn remove_package_file(project_root: &Path, package: &str) -> Result<bool, RegistryError> {
    let path = package_file_path(project_root, package);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|source| RegistryError::Io {
        path: path.clone(),
        source,
    })?;
    remove_empty_home(&project_root.join("vibefacts"))?;
    Ok(true)
}
