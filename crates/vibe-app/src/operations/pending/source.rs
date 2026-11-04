//! Re-admit the exact saved source through its original source owner.
use super::*;
use crate::OperationContext;
use vibe_core::manifest::Manifest;
pub(in crate::operations) fn source(
    ctx: &OperationContext<'_>,
    op: &w::ApplicationPendingOperation,
) -> Result<ResolvedPackage, AppError> {
    read_source(ctx, &op.source, &op.target.key)
}
pub(in crate::operations) fn read_source(
    ctx: &OperationContext<'_>,
    route: &w::ApplicationPendingSource,
    key: &str,
) -> Result<ResolvedPackage, AppError> {
    let package = coordinate(key)?;
    let refuse = |reason| failure(ctx.settings, &package, reason);
    let declared = PathBuf::from(&route.directory);
    if !declared.is_absolute() {
        return Err(refuse("retained source is not absolute".into()));
    }
    let directory = crate::resolve::canonical_dir(&declared)
        .map_err(|e| refuse(format!("saved source is unavailable: {e}")))?;
    let mut resolved = match route.kind {
        w::ApplicationPendingSourceKind::ProjectSlot => ctx.source_reader.ok_or_else(|| {
            refuse("project-slot continuation requires the actual project/lock source owner".into())
        })?(route, &package)?,
        w::ApplicationPendingSourceKind::LocalRegistry => {
            let root = route
                .registry
                .as_ref()
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .ok_or_else(|| refuse("retained registry route is missing or relative".into()))?;
            let registry = vibe_registry::LocalRegistry::new(root.clone())
                .map_err(|e| refuse(e.to_string()))?;
            crate::from_local_registry(
                &registry,
                &root,
                &format!("{}/{}@={}", package.group, package.name, package.version),
            )?
        }
        w::ApplicationPendingSourceKind::AcquiredStore => {
            let requested = crate::qualified_ref(key)?;
            let group = requested
                .group
                .ok_or_else(|| refuse("retained store key has no group".into()))?;
            let version =
                semver::Version::parse(&package.version).map_err(|e| refuse(e.to_string()))?;
            let root = vibe_registry::store::store_root().map_err(|e| refuse(e.to_string()))?;
            let entry = vibe_registry::store::entry_dir(&root, &group, &package.name, &version);
            let expected = crate::resolve::canonical_dir(&entry)
                .map_err(|e| refuse(format!("the exact store entry is unavailable: {e}")))?;
            if expected != directory {
                return Err(refuse(
                    "saved source no longer names the exact entry of the selected package store"
                        .into(),
                ));
            }
            let manifest = Manifest::read(directory.join(Manifest::FILENAME))
                .map_err(|e| refuse(e.to_string()))?;
            crate::from_manifest(manifest, directory.clone(), package.clone())?
        }
        w::ApplicationPendingSourceKind::Directory => {
            let manifest = Manifest::read(directory.join(Manifest::FILENAME))
                .map_err(|e| refuse(e.to_string()))?;
            crate::from_manifest(manifest, directory.clone(), package.clone())?
        }
    };
    let actual = crate::resolve::canonical_dir(&resolved.root).map_err(refuse)?;
    if actual != directory || resolved.package != package {
        return Err(failure(
            ctx.settings,
            &package,
            "the retained source route now admits a different directory or coordinate".into(),
        ));
    }
    resolved.root = actual;
    resolved.source = route.clone();
    Ok(resolved)
}
