//! Local/embedded application and installer-provider resolution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#sources");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::{
    ApplicationDistributionDecl, ApplicationSourceDecl, InstallerMethod, LauncherDecl, Manifest,
};
use vibe_core::progress::Progress;
use vibe_install::InstallSource;

use super::model::{ApplicationIdentity, ApplicationSourceObservation, PackageIdentity};
use super::options::{InstallChoice, options_json, select_option};
use super::remote::resolve_remote_source;

pub struct ResolvedApplication {
    pub application: ApplicationIdentity,
    pub installer: InstallerMethod,
    pub launchers: Vec<LauncherDecl>,
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    pub distribution: Option<ApplicationDistributionDecl>,
    pub source: Option<ApplicationSourceObservation>,
    pub selected_platform: String,
    pub selected_arch: String,
    pub selected_option_id: String,
    source_materialization: SourceMaterialization,
}

enum SourceMaterialization {
    ReadyArchive { application_root: PathBuf },
    ReadyCommand { application_root: PathBuf },
    Deferred(ApplicationSourceDecl),
}

impl ResolvedApplication {
    pub fn materialize_source(
        self,
        settings_root: &Path,
        offline: bool,
        progress: &Progress,
    ) -> Result<Self> {
        let declaration = match &self.source_materialization {
            SourceMaterialization::ReadyArchive { .. }
            | SourceMaterialization::ReadyCommand { .. } => {
                return Ok(self);
            }
            SourceMaterialization::Deferred(declaration) => declaration.clone(),
        };
        let observed = resolve_remote_source(settings_root, &declaration, offline, progress)?;
        let resolved = resolve_application_with_source(
            &observed.registry_root,
            &format!(
                "{}/{}@={}",
                self.application.package.group,
                self.application.package.name,
                self.application.package.version
            ),
            Some(ApplicationSourceObservation {
                url: observed.url,
                tracked_ref: observed.tracked_ref,
                resolved_commit: observed.resolved_commit,
                source_tree: observed.source_tree,
            }),
            &InstallChoice {
                platform: Some(self.selected_platform.clone()),
                arch: Some(self.selected_arch.clone()),
                installer_id: Some(self.selected_option_id.clone()),
                interactive: false,
            },
        )?;
        if resolved.application != self.application
            || resolved.distribution != self.distribution
            || resolved.selected_option_id != self.selected_option_id
        {
            bail!("tracked application source differs from its published bridge declaration");
        }
        Ok(resolved)
    }

    pub fn artifact_root(&self) -> Result<&Path> {
        match &self.source_materialization {
            SourceMaterialization::ReadyArchive {
                application_root, ..
            } => Ok(application_root),
            SourceMaterialization::Deferred(_) => bail!("application source was not materialized"),
            SourceMaterialization::ReadyCommand { application_root } => Ok(application_root),
        }
    }
}

pub fn resolve_global_application(
    settings_root: &Path,
    explicit_registry: Option<&Path>,
    embedded_registry: Option<&Path>,
    spelling: &str,
    choice: &InstallChoice,
    offline: bool,
    progress: &Progress,
) -> Result<ResolvedApplication> {
    let selection = progress.task(format!("Selecting application source for {spelling}"));
    let requested = qualified_ref(spelling)?;
    if let Some(registry_root) = [explicit_registry, embedded_registry]
        .into_iter()
        .flatten()
        .next()
    {
        let registry_root = canonical_directory(registry_root, "application registry")?;
        let registry = crate::registry::application_local_registry(registry_root.clone())?;
        let selected = registry.resolve(&requested)?;
        let manifest = Manifest::read(selected.source_dir.join(Manifest::FILENAME))?;
        if let Some(source) = manifest.application_source.clone() {
            if manifest.application.is_some() {
                let resolved = resolve_deferred_application(&manifest, source, choice);
                match &resolved {
                    Ok(_) => selection.finish(),
                    Err(_) => selection.fail("application bridge selection failed"),
                }
                return resolved;
            }
            selection.detail("published bridge selected an external source registry");
            let observed =
                resolve_remote_source(settings_root, &source, offline, &selection.progress())?;
            let resolved = resolve_application_with_source(
                &observed.registry_root,
                spelling,
                Some(ApplicationSourceObservation {
                    url: observed.url,
                    tracked_ref: observed.tracked_ref,
                    resolved_commit: observed.resolved_commit,
                    source_tree: observed.source_tree,
                }),
                choice,
            );
            match &resolved {
                Ok(_) => selection.finish(),
                Err(_) => selection.fail("application source selection failed"),
            }
            return resolved;
        }
        selection.detail("using the selected local application registry");
        let resolved = resolve_application(&registry_root, spelling, choice);
        match &resolved {
            Ok(_) => selection.finish(),
            Err(_) => selection.fail("application source selection failed"),
        }
        return resolved;
    }

    vibe_core::ensure_default_global_registry()?;
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let synthetic =
        Manifest::parse_str("[project]\nname = \"global-application\"\nversion = \"1.0.0\"\n")?;
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
    )?;
    let store_root = vibe_registry::store::store_root()?;
    selection.detail("resolving the published application package");
    let cached = resolver
        .resolve_and_fetch(&requested, &store_root, None)
        .with_context(|| format!("resolving published application package `{spelling}`"))?;
    if cached.manifest.application.is_some() && !cached.manifest.installer_options.is_empty() {
        let resolved = resolve_published_direct_application(
            cached.manifest,
            &cached.cache_dir,
            &cached.resolved,
            choice,
        );
        match &resolved {
            Ok(_) => selection.finish(),
            Err(_) => selection.fail("application package selection failed"),
        }
        return resolved;
    }
    let source = cached.manifest.application_source.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "published global application `{spelling}` must declare [application_source]"
        )
    })?;
    if cached.manifest.application.is_some() {
        let resolved = resolve_deferred_application(&cached.manifest, source, choice);
        match &resolved {
            Ok(_) => selection.finish(),
            Err(_) => selection.fail("application bridge selection failed"),
        }
        return resolved;
    }
    let observed = resolve_remote_source(settings_root, &source, offline, &selection.progress())?;
    let resolved = resolve_application_with_source(
        &observed.registry_root,
        spelling,
        Some(ApplicationSourceObservation {
            url: observed.url,
            tracked_ref: observed.tracked_ref,
            resolved_commit: observed.resolved_commit,
            source_tree: observed.source_tree,
        }),
        choice,
    );
    match &resolved {
        Ok(_) => selection.finish(),
        Err(_) => selection.fail("application source selection failed"),
    }
    resolved
}

fn resolve_published_direct_application(
    manifest: Manifest,
    cache_dir: &Path,
    resolved: &vibe_registry::ResolvedPackage,
    choice: &InstallChoice,
) -> Result<ResolvedApplication> {
    let package = manifest.require_package()?;
    if package.group != resolved.group
        || package.name != resolved.name
        || package.version != resolved.version
    {
        bail!("cached application package manifest identity differs from its registry coordinate");
    }
    let cached_root = canonical_directory(cache_dir, "cached application package")?;
    resolve_manifest_application(manifest, cached_root, None, choice)
}

/// Inspect all declared installation routes without selecting or invoking one.
pub fn list_global_installer_options(
    settings_root: &Path,
    explicit_registry: Option<&Path>,
    embedded_registry: Option<&Path>,
    spelling: &str,
    offline: bool,
    progress: &Progress,
) -> Result<serde_json::Value> {
    let requested = qualified_ref(spelling)?;
    if let Some(registry_root) = [explicit_registry, embedded_registry]
        .into_iter()
        .flatten()
        .next()
    {
        let registry_root = canonical_directory(registry_root, "application registry")?;
        let registry = crate::registry::application_local_registry(registry_root.clone())?;
        let selected = registry.resolve(&requested)?;
        let manifest = Manifest::read(selected.source_dir.join(Manifest::FILENAME))?;
        if let Some(source) = &manifest.application_source {
            if manifest.application.is_some() {
                return Ok(options_json(spelling, &manifest.installer_options));
            }
            let observed = resolve_remote_source(settings_root, source, offline, progress)?;
            return list_local_installer_options(&observed.registry_root, spelling);
        }
        return Ok(options_json(spelling, &manifest.installer_options));
    }

    vibe_core::ensure_default_global_registry()?;
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let synthetic =
        Manifest::parse_str("[project]\nname = \"global-application\"\nversion = \"1.0.0\"\n")?;
    let resolver = vibe_package_source::build_install_resolver(
        &vibe_package_source::PackageSourceOptions {
            no_prefer_local: true,
            no_default_registry: true,
            ..Default::default()
        },
        &synthetic,
        None,
        settings_root,
        &global,
        offline,
        &[],
    )?;
    let store_root = vibe_registry::store::store_root()?;
    let cached = resolver
        .resolve_and_fetch(&requested, &store_root, None)
        .with_context(|| format!("resolving published application package `{spelling}`"))?;
    if cached.manifest.application.is_some() && !cached.manifest.installer_options.is_empty() {
        let package = cached.manifest.require_package()?;
        if package.group != cached.resolved.group
            || package.name != cached.resolved.name
            || package.version != cached.resolved.version
        {
            bail!(
                "cached application package manifest identity differs from its registry coordinate"
            );
        }
        return Ok(options_json(spelling, &cached.manifest.installer_options));
    }
    let source = cached.manifest.application_source.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "published global application `{spelling}` must declare [application_source]"
        )
    })?;
    let observed = resolve_remote_source(settings_root, source, offline, progress)?;
    list_local_installer_options(&observed.registry_root, spelling)
}

fn list_local_installer_options(registry_root: &Path, spelling: &str) -> Result<serde_json::Value> {
    let registry_root = canonical_directory(registry_root, "application registry")?;
    let registry = crate::registry::application_local_registry(registry_root)?;
    let selected = registry.resolve(&qualified_ref(spelling)?)?;
    let manifest = Manifest::read(selected.source_dir.join(Manifest::FILENAME))?;
    Ok(options_json(spelling, &manifest.installer_options))
}

pub fn resolve_application(
    registry_root: &Path,
    spelling: &str,
    choice: &InstallChoice,
) -> Result<ResolvedApplication> {
    resolve_application_with_source(registry_root, spelling, None, choice)
}

pub fn resolve_application_with_source(
    registry_root: &Path,
    spelling: &str,
    source: Option<ApplicationSourceObservation>,
    choice: &InstallChoice,
) -> Result<ResolvedApplication> {
    let registry_root = canonical_directory(registry_root, "application registry")?;
    let requested = qualified_ref(spelling)?;
    let registry = crate::registry::application_local_registry(registry_root.clone())
        .context("opening the local application registry")?;
    let selected = registry
        .resolve(&requested)
        .with_context(|| format!("resolving global application package `{spelling}`"))?;
    let source_root = contained_directory(
        &registry_root,
        &selected.source_dir,
        "application package source",
    )?;
    let manifest = Manifest::read(source_root.join(Manifest::FILENAME))
        .context("reading the application package manifest")?;
    let package = manifest.require_package()?;
    if package.group != selected.group
        || package.name != selected.name
        || package.version != selected.version
    {
        bail!(
            "resolved application package manifest identity differs from its registry coordinate"
        );
    }
    resolve_manifest_application(manifest, source_root, source, choice)
}

fn resolve_manifest_application(
    manifest: Manifest,
    source_root: PathBuf,
    source: Option<ApplicationSourceObservation>,
    choice: &InstallChoice,
) -> Result<ResolvedApplication> {
    let package = manifest.require_package()?;
    let declaration = manifest
        .application
        .clone()
        .ok_or_else(|| anyhow::anyhow!("package declares no [application]"))?;
    let selected_option = select_option(&manifest.installer_options, choice)?;
    let installer = selected_option.installation.clone();
    let source_materialization = if matches!(installer, InstallerMethod::Shell { .. }) {
        SourceMaterialization::ReadyCommand {
            application_root: source_root.clone(),
        }
    } else {
        SourceMaterialization::ReadyArchive {
            application_root: source_root,
        }
    };
    let application = ApplicationIdentity {
        id: declaration.id.clone(),
        package: PackageIdentity {
            group: package.group.to_string(),
            name: package.name.clone(),
            version: package.version.to_string(),
        },
        installer_package: PackageIdentity {
            group: package.group.to_string(),
            name: package.name.clone(),
            version: package.version.to_string(),
        },
        commands: declaration.commands.clone(),
    };
    Ok(ResolvedApplication {
        application,
        installer,
        launchers: selected_option.launchers,
        pre_install: selected_option.pre_install,
        post_install: selected_option.post_install,
        distribution: declaration.distribution,
        source,
        selected_platform: selected_option.platform,
        selected_arch: selected_option.arch,
        selected_option_id: selected_option.id,
        source_materialization,
    })
}

fn resolve_deferred_application(
    manifest: &Manifest,
    source: ApplicationSourceDecl,
    choice: &InstallChoice,
) -> Result<ResolvedApplication> {
    let package = manifest.require_package()?;
    let declaration = manifest
        .application
        .clone()
        .ok_or_else(|| anyhow::anyhow!("published application bridge has no [application]"))?;
    let selected_option = select_option(&manifest.installer_options, choice)?;
    let installer = selected_option.installation.clone();
    Ok(ResolvedApplication {
        application: ApplicationIdentity {
            id: declaration.id,
            package: PackageIdentity {
                group: package.group.to_string(),
                name: package.name.clone(),
                version: package.version.to_string(),
            },
            installer_package: PackageIdentity {
                group: package.group.to_string(),
                name: package.name.clone(),
                version: package.version.to_string(),
            },
            commands: declaration.commands,
        },
        installer,
        launchers: selected_option.launchers,
        pre_install: selected_option.pre_install,
        post_install: selected_option.post_install,
        distribution: declaration.distribution,
        source: None,
        selected_platform: selected_option.platform,
        selected_arch: selected_option.arch,
        selected_option_id: selected_option.id,
        source_materialization: SourceMaterialization::Deferred(source),
    })
}

pub fn qualified_ref(spelling: &str) -> Result<PackageRef> {
    let requested = PackageRef::parse(spelling)
        .with_context(|| format!("parsing global application package `{spelling}`"))?;
    if requested.group.is_none() {
        bail!("global application package must be fully qualified as <group>/<name>");
    }
    Ok(requested)
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {label} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label} is a symbolic link");
    }
    let canonical = crate::commands::init::strip_unc_public(
        fs::canonicalize(path)
            .with_context(|| format!("resolving {label} `{}`", path.display()))?,
    );
    if !canonical.is_dir() {
        bail!("{label} is not a directory");
    }
    Ok(canonical)
}

fn contained_directory(root: &Path, path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {label} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label} is a symbolic link");
    }
    let canonical = canonical_directory(path, label)?;
    if !canonical.starts_with(root) {
        bail!("{label} escapes the selected registry");
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_direct_app_resolves_artifact_from_cached_package() {
        let manifest = Manifest::parse_str(
            r#"
[package]
name = "demo"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "demo"

[[installer_options]]
id = "archive"
name = "ZIP archive"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"

[installer_options.installation]
type = "zip"
source = "payload.zip"
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
"#,
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let resolved = vibe_registry::ResolvedPackage {
            group: vibe_core::Group::parse("org.example").unwrap(),
            name: "demo".into(),
            version: semver::Version::parse("1.0.0").unwrap(),
            source_dir: directory.path().into(),
        };
        let application = resolve_published_direct_application(
            manifest,
            directory.path(),
            &resolved,
            &InstallChoice {
                platform: Some("linux".into()),
                arch: Some("x86_64".into()),
                installer_id: Some("archive".into()),
                interactive: false,
            },
        )
        .unwrap();
        assert_eq!(application.selected_option_id, "archive");
        assert_eq!(application.artifact_root().unwrap(), directory.path());
        assert!(application.source.is_none());
    }
}
