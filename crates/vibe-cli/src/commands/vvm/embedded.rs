//! PROP-030 — deriving the ambient embedded registry from the active VVM
//! install. The in-tree `packages/` of a source build (`origin = external`)
//! is a default registry for every project; this module answers "is there
//! one, and where," from the install record alone. The composition that
//! injects it into resolution, and the developer/user precedence, live in
//! the install command (PROP-030 §3, §7) — this is only the discovery half.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-030#registry");

use std::path::{Path, PathBuf};

use super::model::{InstallRecord, Origin};
use super::store::{StoreError, VersionStore};

/// The embedded-registry root an install record implies, if any (PROP-030
/// §2): an `external`-origin install whose `source_path` still holds a
/// `packages/` directory. A `managed` / `binary` origin, a missing
/// `source_path`, or an absent `packages/` all mean "no embedded registry"
/// — the caller then falls back to the declared registries (§8).
//
// pub(crate) + unused until PROP-030 slice 3 wires it into
// `build_install_resolver`; the `#[allow]` retires when that lands.
#[allow(dead_code)]
pub(crate) fn embedded_root_for(record: &InstallRecord) -> Option<PathBuf> {
    if record.origin != Origin::External {
        return None;
    }
    let packages = Path::new(record.source_path.as_ref()?).join("packages");
    packages.is_dir().then_some(packages)
}

/// The embedded-registry root of the **active** install (the one the
/// `current` pointer names), or `None` when no source install is active or
/// its `packages/` is gone. This is the PROP-030 §7 discovery hook: reuse
/// `VersionStore::active` rather than reading the environment.
#[allow(dead_code)]
pub(crate) fn active_embedded_root(store: &VersionStore) -> Result<Option<PathBuf>, StoreError> {
    Ok(store.active()?.as_ref().and_then(embedded_root_for))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::vvm::model::{Kind, Profile};

    fn external_record(source_path: Option<&str>) -> InstallRecord {
        InstallRecord {
            kind: Kind::Branch,
            id: "main".into(),
            instance: 1,
            commit: "c".into(),
            toolchain: "t".into(),
            profile: Profile::Debug,
            installed_at: "now".into(),
            origin: Origin::External,
            source_path: source_path.map(str::to_string),
        }
    }

    #[test]
    fn external_with_packages_dir_resolves_to_it() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("packages")).unwrap();
        let rec = external_record(Some(tmp.path().to_str().unwrap()));
        assert_eq!(embedded_root_for(&rec), Some(tmp.path().join("packages")));
    }

    #[test]
    fn external_without_a_packages_dir_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let rec = external_record(Some(tmp.path().to_str().unwrap()));
        assert_eq!(embedded_root_for(&rec), None);
    }

    #[test]
    fn external_without_a_source_path_is_none() {
        assert_eq!(embedded_root_for(&external_record(None)), None);
    }

    #[test]
    fn a_managed_origin_never_has_an_embedded_registry() {
        // Even with a real `packages/` on disk, only `external` installs
        // carry an embedded registry (PROP-030 §2).
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("packages")).unwrap();
        let mut rec = external_record(Some(tmp.path().to_str().unwrap()));
        rec.origin = Origin::Managed;
        assert_eq!(embedded_root_for(&rec), None);
    }
}
