//! Where one application command gets its package from: the effective
//! source policy of this run, the registry chain a resolution walks, and
//! the resolver a package's own requirements are solved through.
//!
//! One cell, because the three are one question — an explicit
//! `--registry` wins, then the embedded registry if it carries the
//! package (PROP-030 `#precedence`), then the configured registries.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_app::{ResolvedPackage, SettingsRoot};

pub(crate) fn source_operation(
    invocation: vibe_core::manifest::SourcePolicyDecl,
) -> Result<vibe_package_source::SourceOperation> {
    source_operation_for_project(None, invocation)
}

/// The caller supplies the already selected project, never inferred from app-home paths.
pub(crate) fn source_operation_for_project(
    project_root: Option<&Path>,
    invocation: vibe_core::manifest::SourcePolicyDecl,
) -> Result<vibe_package_source::SourceOperation> {
    let manifest = match project_root {
        Some(root) => vibe_core::manifest::Manifest::read(root.join("vibe.toml"))?,
        None => vibe_core::manifest::Manifest::default(),
    };
    let cwd = std::env::current_dir()?;
    vibe_package_source::source_operation(
        &vibe_package_source::PackageSourceOptions {
            source_policy: invocation,
            ..Default::default()
        },
        &manifest,
        project_root.unwrap_or(&cwd),
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )
}

pub(crate) fn resolve_package_with_operation(
    settings: &SettingsRoot,
    registry: Option<&Path>,
    embedded_root: Option<&Path>,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<ResolvedPackage> {
    if let Some(root) = registry {
        let local = open_local(root)?;
        return Ok(vibe_app::from_local_registry(&local, root, spelling)
            .map_err(vibe_app::AppError::from)?);
    }
    if let Some(root) = embedded_root {
        let local = open_local(root)?;
        if vibe_app::local_registry_carries(&local, spelling)? {
            let mut resolved = vibe_app::from_local_registry(&local, root, spelling)
                .map_err(vibe_app::AppError::from)?;
            if let Some(evidence) = &mut resolved.source_evidence {
                evidence.source_kind =
                    Some(vibe_wire::generated::shared::SourceSelectionKind::Embedded);
            }
            return Ok(resolved);
        }
    }
    Ok(
        vibe_app::from_global_with_operation(settings.path(), spelling, offline, operation)
            .map_err(vibe_app::AppError::from)?,
    )
}

fn open_local(root: &Path) -> Result<vibe_registry::LocalRegistry> {
    crate::registry::application_local_registry(root.to_path_buf())
        .with_context(|| format!("opening the application registry `{}`", root.display()))
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod source_tests;

/// The package resolver of one command: the same explicit (or embedded)
/// registry as the requested package, otherwise the configured registries
/// (PROP-065 `#app-dependencies`: requirements resolve like any package).
pub(crate) struct Packages {
    pub(crate) operation: vibe_package_source::SourceOperation,
    pub(crate) settings: SettingsRoot,
    pub(crate) registry: Option<PathBuf>,
    pub(crate) embedded: Option<PathBuf>,
    pub(crate) offline: bool,
}

impl Packages {
    pub(crate) fn scope(&self) -> vibe_app::ResolverScope<'_> {
        match self.registry.as_deref() {
            Some(root) => vibe_app::ResolverScope::Registry(root),
            None => vibe_app::ResolverScope::Chain {
                embedded: self.embedded.as_deref(),
            },
        }
    }
    pub(crate) fn resolve(&self, spelling: &str) -> Result<ResolvedPackage, vibe_app::AppError> {
        resolve_package_with_operation(
            &self.settings,
            self.registry.as_deref(),
            self.embedded.as_deref(),
            spelling,
            self.offline,
            &self.operation,
        )
        .map_err(|error| match error.downcast::<vibe_app::AppError>() {
            Ok(error) => error,
            Err(error) => vibe_app::ResolveError::Registry {
                spelling: spelling.into(),
                reason: format!("{error:#}"),
            }
            .into(),
        })
    }
}
