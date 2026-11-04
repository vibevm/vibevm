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

/// Whether a local directory registry carries any version of the package
/// `spelling` names (PROP-030 `#precedence`: a developer's embedded
/// registry is first in the chain, not the whole chain — a package it does
/// not carry falls through to the declared registries).
///
/// ```
/// let tmp = tempfile::tempdir().unwrap();
/// let registry = vibe_registry::LocalRegistry::new(tmp.path().to_path_buf()).unwrap();
/// assert!(!vibe_app::local_registry_carries(&registry, "org.example/demo").unwrap());
/// ```
pub fn local_registry_carries(
    registry: &LocalRegistry,
    spelling: &str,
) -> Result<bool, ResolveError> {
    let requested = qualified_ref(spelling)?;
    let Some(group) = requested.group.as_ref() else {
        return Err(ResolveError::Coordinate {
            spelling: spelling.into(),
            reason: "the package must be fully qualified as <group>/<name>".into(),
        });
    };
    match registry.list_versions(group, requested.name.as_str()) {
        Ok(versions) => Ok(!versions.is_empty()),
        Err(vibe_registry::RegistryError::UnknownPackage { .. }) => Ok(false),
        Err(error) => Err(ResolveError::Registry {
            spelling: spelling.into(),
            reason: error.to_string(),
        }),
    }
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
    if manifest.configuration.is_some() {
        return Err(ResolveError::ConfigurationUnsupported {
            package: coordinate.key(),
            scope: "package".into(),
        });
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

#[cfg(test)]
mod carries_tests {
    use super::*;

    #[test]
    fn common_package_configuration_is_not_discarded_at_app_resolution() {
        let manifest = Manifest::parse_str("[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='app'\n[application]\nid='demo'\n[configuration]\n").unwrap();
        let result = from_manifest(
            manifest,
            PathBuf::from("/unused"),
            PackageCoordinate {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
        );
        let error = result.unwrap_err();
        assert!(matches!(
            error,
            ResolveError::ConfigurationUnsupported { .. }
        ));
        let app: crate::error::AppError = error.into();
        assert_eq!(app.code(), "configuration_unsupported");
        assert!(app.to_string().contains("scope `package`"));
    }

    #[test]
    fn an_absent_package_is_not_carried_and_a_present_one_is() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = LocalRegistry::new(tmp.path().to_path_buf()).unwrap();
        assert!(!local_registry_carries(&registry, "org.example/demo").unwrap());
        let version = tmp.path().join("org.example").join("demo").join("v1.0.0");
        fs::create_dir_all(&version).unwrap();
        fs::write(
            version.join("vibe.toml"),
            "[package]\nname = \"demo\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\nepoch = 1\nformat = \"normal\"\nauthors = [\"t\"]\nlicense = \"UPL-1.0\"\ndescription = \"d\"\nkeywords = []\n\n[compatibility]\nmin_vibe_version = \"1.0.8\"\nrequires_kinds = []\n\n[application]\nid = \"demo\"\n",
        )
        .unwrap();
        assert!(local_registry_carries(&registry, "org.example/demo").unwrap());
        assert!(local_registry_carries(&registry, "demo").is_err());
    }
}
