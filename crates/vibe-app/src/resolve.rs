//! Resolution of one fully qualified app package to its package tree: an
//! explicit local registry (`--registry`, or the embedded registry), else
//! the configured global registries.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::PackageRef;
use vibe_core::manifest::{ApplicationDecl, InstallerOptionDecl, Manifest};
use vibe_install::InstallSource;
use vibe_registry::LocalRegistry;

use crate::error::ResolveError;
use crate::index::PackageCoordinate;

/// An app package ready for selection: identity, declarations and the
/// package tree that package-relative sources resolve against.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
pub struct ResolvedPackage {
    pub package: PackageCoordinate,
    pub root: PathBuf,
    pub application: ApplicationDecl,
    pub options: Vec<InstallerOptionDecl>,
}

/// Parse a global application coordinate; the group is mandatory.
///
/// ```
/// let requested = vibe_app::qualified_ref("org.example/demo@1").unwrap();
/// assert_eq!(requested.name.as_str(), "demo");
/// assert!(vibe_app::qualified_ref("demo").is_err());
/// ```
pub fn qualified_ref(spelling: &str) -> Result<PackageRef, ResolveError> {
    let requested = PackageRef::parse(spelling).map_err(|error| ResolveError::Coordinate {
        spelling: spelling.into(),
        reason: error.to_string(),
    })?;
    if requested.group.is_none() {
        return Err(ResolveError::Coordinate {
            spelling: spelling.into(),
            reason: "the package must be fully qualified as <group>/<name>".into(),
        });
    }
    Ok(requested)
}

/// Resolve from a local directory registry the caller constructed.
pub fn from_local_registry(
    registry: &LocalRegistry,
    registry_root: &Path,
    spelling: &str,
) -> Result<ResolvedPackage, ResolveError> {
    let requested = qualified_ref(spelling)?;
    let failed = |reason: String| ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    let root = canonical_dir(registry_root).map_err(&failed)?;
    let selected = registry
        .resolve(&requested)
        .map_err(|error| failed(error.to_string()))?;
    let package_root = canonical_dir(&selected.source_dir).map_err(&failed)?;
    if !package_root.starts_with(&root) {
        return Err(failed(
            "the package tree escapes the selected registry".into(),
        ));
    }
    let manifest = Manifest::read(package_root.join(Manifest::FILENAME))
        .map_err(|error| failed(error.to_string()))?;
    let coordinate = PackageCoordinate {
        group: selected.group.to_string(),
        name: selected.name.clone(),
        version: selected.version.to_string(),
    };
    from_manifest(manifest, package_root, coordinate)
}

/// Resolve through the configured global registries into the package store.
pub fn from_global(
    settings_root: &Path,
    spelling: &str,
    offline: bool,
) -> Result<ResolvedPackage, ResolveError> {
    let requested = qualified_ref(spelling)?;
    let failed = |reason: String| ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    vibe_core::ensure_default_global_registry().map_err(|error| failed(error.to_string()))?;
    let global =
        vibe_core::GlobalRegistryConfig::load().map_err(|error| failed(error.to_string()))?;
    let synthetic =
        Manifest::parse_str("[project]\nname = \"global-application\"\nversion = \"1.0.0\"\n")
            .map_err(|error| failed(error.to_string()))?;
    let options = vibe_package_source::PackageSourceOptions {
        no_prefer_local: true,
        no_default_registry: true,
        ..Default::default()
    };
    let resolver = vibe_package_source::build_install_resolver(
        &options,
        &synthetic,
        None,
        settings_root,
        &global,
        offline,
        &[],
    )
    .map_err(|error| failed(format!("{error:#}")))?;
    let store = vibe_registry::store::store_root().map_err(|error| failed(error.to_string()))?;
    let cached = resolver
        .resolve_and_fetch(&requested, &store, None)
        .map_err(|error| failed(error.to_string()))?;
    let root = canonical_dir(&cached.cache_dir).map_err(&failed)?;
    let coordinate = PackageCoordinate {
        group: cached.resolved.group.to_string(),
        name: cached.resolved.name.clone(),
        version: cached.resolved.version.to_string(),
    };
    from_manifest(cached.manifest, root, coordinate)
}

/// Check the manifest against the registry coordinate and take its
/// application declarations.
pub fn from_manifest(
    manifest: Manifest,
    root: PathBuf,
    coordinate: PackageCoordinate,
) -> Result<ResolvedPackage, ResolveError> {
    let unusable = |reason: &str| ResolveError::Manifest {
        package: coordinate.key(),
        reason: reason.into(),
    };
    let package = manifest
        .package
        .as_ref()
        .ok_or_else(|| unusable("the manifest has no [package] table"))?;
    if package.group.to_string() != coordinate.group
        || package.name != coordinate.name
        || package.version.to_string() != coordinate.version
    {
        return Err(unusable(
            "the manifest identity differs from its registry coordinate",
        ));
    }
    let application = manifest
        .application
        .clone()
        .ok_or_else(|| unusable("the package declares no [application]"))?;
    Ok(ResolvedPackage {
        package: coordinate,
        root,
        application,
        options: manifest.installer_options,
    })
}

fn canonical_dir(path: &Path) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("reading `{}`: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("`{}` is a symbolic link", path.display()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("resolving `{}`: {error}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("`{}` is not a directory", path.display()));
    }
    Ok(strip_verbatim(canonical))
}

/// Windows `canonicalize` yields `\\?\` paths; strip the prefix so they
/// compare and display like ordinary paths.
pub(crate) fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest.to_string()),
        _ => path,
    }
}
