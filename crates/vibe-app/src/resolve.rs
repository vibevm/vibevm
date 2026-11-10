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
    /// Captured package source display facts; absent means unknown history.
    pub source_evidence: Option<vibe_wire::generated::shared::SourceEvidence>,
    /// Exact admitted source route for operation continuation, never a newest-version lookup.
    pub source: vibe_wire::generated::shared::ApplicationPendingSource,
    /// Package acquisition retains these declarations without creating a configuration instance.
    pub kind: vibe_core::PackageKind,
    pub configuration: Option<vibe_core::manifest::ConfigurationDecl>,
    pub requires: vibe_core::manifest::Requires,
    /// The `[recommends]` the package declares — the companions the
    /// application installer installs beside it, best-effort
    /// (`PROP-077#APP-RECOMMENDS`).
    pub recommends: vibe_core::manifest::Recommends,
    /// The `[suggests]` the package declares: reported once and installed
    /// nowhere (`PROP-077#SUGGESTS-UNCHANGED`).
    pub suggests: vibe_core::manifest::Suggests,
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
    // The containment check guards against a symlink or `..` escape out of
    // the directory the operator named. A **rendered** version is the one
    // legitimate tree outside it: PROP-075 `#TEMPLATE-RENDER-LOCATION`
    // forbids writing a render into a registry, so the template layer
    // materialises it under the settings cache and `#TEMPLATE-DOMAIN-SEAM`
    // hands back the ordinary `ResolvedPackage`. Admitted against that one
    // root by name, never by widening the check.
    if !package_root.starts_with(&root)
        && !vibe_registry::templates::is_rendered_tree(&package_root)
    {
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
    let mut resolved = from_manifest(manifest, package_root, coordinate)?;
    resolved.source_evidence = Some(vibe_wire::generated::shared::SourceEvidence {
        store_reused: Some(false),
        registry: None,
        source_url: Some(vibe_core::progress::transfer::safe_remote(&format!(
            "file://{}",
            resolved.root.display()
        ))),
        source_ref: None,
        resolved_commit: None,
        source_kind: Some(vibe_wire::generated::shared::SourceSelectionKind::Registry),
        via_redirect: None,
        fetched_from: None,
        acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::Directory),
    });
    resolved.source.kind =
        vibe_wire::generated::shared::ApplicationPendingSourceKind::LocalRegistry;
    resolved.source.registry = Some(
        root.to_str()
            .ok_or_else(|| failed("registry path is not lossless UTF-8".into()))?
            .into(),
    );
    Ok(resolved)
}

/// Resolve through the configured global registries into the package store.
///
/// `seed_default_registry` is the caller's composition-root reading of the
/// `VIBE_NO_DEFAULT_REGISTRY` posture; this crate reads no environment.
pub fn from_global(
    settings_root: &Path,
    spelling: &str,
    offline: bool,
    seed_default_registry: bool,
) -> Result<ResolvedPackage, ResolveError> {
    let synthetic = Manifest::parse_str("[project]\nname='global-application'\nversion='1.0.0'\n")
        .map_err(|error| ResolveError::Registry {
            spelling: spelling.into(),
            reason: error.to_string(),
        })?;
    let options = vibe_package_source::PackageSourceOptions {
        no_prefer_local: true,
        no_default_registry: true,
        ..Default::default()
    };
    let operation = vibe_package_source::source_operation(
        &options,
        &synthetic,
        settings_root,
        vibe_package_source::deferred_user_registry(seed_default_registry),
    )
    .map_err(
        |error| match error.downcast::<vibe_registry::RegistryError>() {
            Ok(error) => ResolveError::SourcePolicy(error),
            Err(error) => ResolveError::Registry {
                spelling: spelling.into(),
                reason: format!("{error:#}"),
            },
        },
    )?;
    from_global_with_operation(settings_root, spelling, offline, &operation)
}
/// The install resolver of the configured global registries — the one
/// [`from_global_with_operation`] resolves through, built here so a
/// read-only enumeration can use the identical source chain without
/// resolving or fetching anything (PROP-075 `#VERSIONS-LIST`).
pub fn global_resolver(
    settings_root: &Path,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<vibe_package_source::InstallResolver, ResolveError> {
    let failed = |reason: String| ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    let synthetic =
        Manifest::parse_str("[project]\nname = \"global-application\"\nversion = \"1.0.0\"\n")
            .map_err(|error| failed(error.to_string()))?;
    let options = vibe_package_source::PackageSourceOptions {
        no_prefer_local: true,
        no_default_registry: true,
        ..Default::default()
    };
    vibe_package_source::build_install_resolver_with_operation(
        vibe_package_source::InstallResolverRequest {
            options: &options,
            manifest: &synthetic,
            embedded_root: None,
            project_root: settings_root,
            offline,
            locked: &[],
            progress: &vibe_core::progress::Progress::default(),
        },
        operation,
    )
    .map_err(
        |error| match error.downcast::<vibe_registry::RegistryError>() {
            Ok(error) => ResolveError::SourcePolicy(error),
            Err(error) => failed(format!("{error:#}")),
        },
    )
}

/// Every version the configured global registries publish for
/// `<group>/<name>`, ascending — the read-only enumeration one listing
/// needs, with nothing resolved and nothing fetched
/// (PROP-075 `#VERSIONS-LIST`).
///
/// Both halves of the chain already union a template package's produced
/// versions in, because `#TEMPLATE-DOMAIN-SEAM`'s call sites 1 and 4 are
/// inside the registries this walks.
pub fn versions_from_global_with_operation(
    settings_root: &Path,
    group: &vibe_core::Group,
    name: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Vec<semver::Version>, ResolveError> {
    let spelling = format!("{group}/{name}");
    let resolver = global_resolver(settings_root, &spelling, offline, operation)?;
    let failed = |reason: String| ResolveError::Registry {
        spelling: spelling.clone(),
        reason,
    };
    let from_multi =
        |multi: &vibe_registry::MultiRegistryResolver| match multi.list_versions(group, name) {
            Ok(found) => Ok(found),
            Err(vibe_registry::RegistryError::UnknownPackage { .. }) => Ok(Vec::new()),
            Err(error) => Err(failed(error.to_string())),
        };
    let mut versions = Vec::new();
    match &resolver {
        vibe_package_source::InstallResolver::Local(local, _) => {
            versions.extend(local_versions(local, group, name)?);
        }
        vibe_package_source::InstallResolver::LockedLocal { local, held, .. } => {
            versions.extend(local_versions(local, group, name)?);
            versions.extend(from_multi(held)?);
        }
        vibe_package_source::InstallResolver::Multi(multi, _) => {
            versions.extend(from_multi(multi)?);
        }
        vibe_package_source::InstallResolver::Embedded {
            locals, declared, ..
        } => {
            for local in locals {
                versions.extend(local_versions(local, group, name)?);
            }
            if let Some(multi) = declared {
                versions.extend(from_multi(multi)?);
            }
        }
    }
    versions.sort();
    versions.dedup();
    Ok(versions)
}

/// The roots of the **directory-shaped** registries the configured chain
/// reads, in precedence order — the places a template is discovered.
///
/// PROP-075 `#TEMPLATE-DISCOVERY` reads a template off the manifests of a
/// package directory, which is what a local-directory registry, a
/// `monorepo` working copy and a mirrored layout registry all are. A
/// per-package **git** registry has no such root and contributes none.
pub fn directory_roots_from_global_with_operation(
    settings_root: &Path,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Vec<PathBuf>, ResolveError> {
    let resolver = global_resolver(
        settings_root,
        "(the configured registries)",
        offline,
        operation,
    )?;
    let mut roots: Vec<PathBuf> = Vec::new();
    let declared_roots = |multi: &vibe_registry::MultiRegistryResolver,
                          roots: &mut Vec<PathBuf>| {
        if let Ok(sources) = multi.sources() {
            for source in sources {
                if let vibe_registry::multi_registry_resolver::RegistrySource::Local(local) = source
                {
                    roots.push(local.registry.root().to_path_buf());
                }
            }
        }
    };
    match &resolver {
        vibe_package_source::InstallResolver::Local(local, _) => {
            roots.push(local.root().to_path_buf());
        }
        vibe_package_source::InstallResolver::LockedLocal { local, held, .. } => {
            roots.push(local.root().to_path_buf());
            declared_roots(held, &mut roots);
        }
        vibe_package_source::InstallResolver::Multi(multi, _) => {
            declared_roots(multi, &mut roots);
        }
        vibe_package_source::InstallResolver::Embedded {
            locals, declared, ..
        } => {
            for local in locals {
                roots.push(local.root().to_path_buf());
            }
            if let Some(multi) = declared {
                declared_roots(multi, &mut roots);
            }
        }
    }
    roots.dedup();
    Ok(roots)
}

/// One local-directory registry's versions; a package it does not carry
/// contributes nothing rather than failing the enumeration.
fn local_versions(
    registry: &LocalRegistry,
    group: &vibe_core::Group,
    name: &str,
) -> Result<Vec<semver::Version>, ResolveError> {
    match registry.list_versions(group, name) {
        Ok(versions) => Ok(versions),
        Err(vibe_registry::RegistryError::UnknownPackage { .. }) => Ok(Vec::new()),
        Err(error) => Err(ResolveError::Registry {
            spelling: format!("{group}/{name}"),
            reason: error.to_string(),
        }),
    }
}

pub fn from_global_with_operation(
    settings_root: &Path,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<ResolvedPackage, ResolveError> {
    let requested = qualified_ref(spelling)?;
    let failed = |reason: String| ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    let resolver = global_resolver(settings_root, spelling, offline, operation)?;
    let store = vibe_registry::store::store_root().map_err(ResolveError::SourcePolicy)?;
    let cached = resolver
        .resolve_and_fetch(&requested, &store, None)
        .map_err(ResolveError::SourcePolicy)?;
    let root = canonical_dir(&cached.cache_dir).map_err(&failed)?;
    let coordinate = PackageCoordinate {
        group: cached.resolved.group.to_string(),
        name: cached.resolved.name.clone(),
        version: cached.resolved.version.to_string(),
    };
    let evidence = cached.source_evidence();
    let mut resolved = from_manifest(cached.manifest, root, coordinate)?;
    resolved.source_evidence = Some(evidence);
    resolved.source.kind =
        vibe_wire::generated::shared::ApplicationPendingSourceKind::AcquiredStore;
    Ok(resolved)
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
    let source = vibe_wire::generated::shared::ApplicationPendingSource {
        directory: root
            .to_str()
            .ok_or_else(|| unusable("source directory is not lossless UTF-8"))?
            .into(),
        kind: vibe_wire::generated::shared::ApplicationPendingSourceKind::Directory,
        registry: None,
        project: None,
    };
    Ok(ResolvedPackage {
        source_evidence: None,
        source,
        kind: package.kind,
        configuration: manifest.configuration,
        requires: manifest.requires,
        recommends: manifest.recommends,
        suggests: manifest.suggests,
        package: coordinate,
        root,
        application,
        options: manifest.installer_options,
    })
}

pub(crate) fn canonical_dir(path: &Path) -> Result<PathBuf, String> {
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
        let resolved = result.unwrap();
        assert!(resolved.configuration.is_some());
        assert_eq!(resolved.kind, vibe_core::PackageKind::App);
        assert_eq!(
            crate::configuration::describe(&resolved)
                .configuration
                .unwrap()
                .features
                .len(),
            0
        );
    }

    #[test]
    fn original_dependency_input_is_not_dropped_by_app_source_projection() {
        let m=Manifest::parse_str("[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='app'\n[application]\nid='demo'\n[requires.packages.'org.example/dep']\nversion='*'\nconfiguration={policy='require-selection',parameters={value='SYN_INPUT'}}\n").unwrap();
        let resolved = from_manifest(
            m,
            "/unused".into(),
            PackageCoordinate {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
        )
        .unwrap();
        assert_eq!(
            resolved.requires.configurations["org.example/dep"].parameters["value"],
            vibe_core::manifest::ParameterValue::Text("SYN_INPUT".into())
        );
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
