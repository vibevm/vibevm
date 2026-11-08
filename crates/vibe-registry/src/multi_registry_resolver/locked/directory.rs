//! Held directory sources — a path declaration or a local-directory registry
//! row — resolved from the held lock row itself. Split from `locked.rs` along
//! the directory/git seam for the file-length budget; same methods.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
impl MultiRegistryResolver {
    pub(super) fn locked_directory(
        &self,
        row: &LockedPackage,
    ) -> Result<Option<PathBuf>, RegistryError> {
        if row.source_kind == Some(SourceKind::Path) {
            let declared = self
                .path_packages
                .get(&format!("{}/{}", row.group, row.name))
                .ok_or_else(|| refusal("held path source has no resolved workspace path"))?;
            if declared.workspace_rel != row.source_url.as_str() {
                return Err(refusal("path declaration changed from the held source"));
            }
            return Ok(Some(declared.package_dir.clone()));
        }
        if row.source_ref.is_none() && is_local_directory_url(row.source_url.as_str()) {
            return local_path_from_url(row.source_url.as_str()).map(Some);
        }
        Ok(None)
    }
    pub(super) fn locked_directory_registry(
        &self,
        row: &LockedPackage,
        directory: &Path,
    ) -> Result<Option<LocalRegistry>, RegistryError> {
        if row.source_kind == Some(SourceKind::Path) {
            return Ok(None);
        }
        let Some(name_dir) = directory.parent() else {
            return Ok(None);
        };
        let Some(group_dir) = name_dir.parent() else {
            return Ok(None);
        };
        let Some(root) = group_dir.parent() else {
            return Ok(None);
        };
        let version_dir = format!("v{}", row.version);
        if directory.file_name().and_then(|s| s.to_str()) != Some(version_dir.as_str())
            || name_dir.file_name().and_then(|s| s.to_str()) != Some(row.name.as_str())
            || group_dir.file_name().and_then(|s| s.to_str()) != Some(row.group.as_str())
        {
            return Ok(None);
        }
        LocalRegistry::new(root).map(Some)
    }
    pub(super) fn resolve_locked_directory(
        &self,
        pkg: &PackageRef,
        row: &LockedPackage,
        directory: &Path,
    ) -> Result<ResolvedPackage, RegistryError> {
        if let Some(registry) = self.locked_directory_registry(row, directory)? {
            return registry.resolve(pkg);
        }
        let manifest = Manifest::read(directory.join(Manifest::FILENAME))?;
        let meta = manifest.require_package()?;
        if meta.group != row.group || meta.name != row.name {
            return Err(refusal("held directory has a different package identity"));
        }
        Ok(ResolvedPackage {
            group: row.group.clone(),
            name: row.name.to_string(),
            version: meta.version.clone(),
            source_dir: directory.into(),
        })
    }
}
