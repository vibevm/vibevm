//! Operation-owned deferred declarations, policy and backend construction.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use crate::builder::{effective_registry_config, project_local_layer};
use crate::{PackageSourceOptions, SourceOperation};
use anyhow::{Context, Result, bail};
use std::{path::Path, sync::Arc};
use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource, manifest::Manifest};
use vibe_registry::{MultiRegistryResolver, source_policy_provider::SourcePolicyProvider};
/// Open the declared multi-registry walk from a precomputed effective config —
/// shared by the plain multi-registry path and the embedded composition.
pub(super) fn open_multi_from(
    manifest: &Manifest,
    options: &PackageSourceOptions,
    offline: bool,
    locked: &[vibe_core::manifest::LockedPackage],
    project_root: &Path,
    progress: &vibe_core::progress::Progress,
    operation: &SourceOperation,
) -> Result<MultiRegistryResolver> {
    let cache = match &options.storage {
        Some(storage) => storage.acquisition_root.join("registries"),
        None => vibe_registry::default_cache_root()?,
    };
    let read_store = match &options.storage {
        Some(storage) => storage.read_only_store.clone(),
        None => Some(vibe_registry::store::store_root()?),
    };
    let metadata = operation.transport.acquisition_root.join("metadata-store");
    let provider = Arc::new(Declarations {
        operation: operation.clone(),
        manifest: manifest.clone(),
        project_root: project_root.to_path_buf(),
        offline,
    });
    let mut resolver =
        MultiRegistryResolver::from_deferred_catalog(provider, operation.backend(), cache, 3600)
            .with_source_policy_provider(operation.policy.clone())
            .with_metadata_store_root(metadata)
            .with_strict_auth(options.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone())
            .with_project_overrides(manifest.overrides.clone())
            .with_offline(offline)
            .with_locked_packages(locked.to_vec())
            .with_locked_checkouts(vibe_install::locked_checkout_paths(project_root, locked))
            .with_mutable_locked_sources(vibe_install::mutable_locked_sources(project_root, locked))
            .with_progress(progress.clone());
    if let Some(store) = read_store {
        resolver = resolver.with_store_root(store);
    }
    Ok(resolver)
}
struct Declarations {
    operation: SourceOperation,
    manifest: Manifest,
    project_root: std::path::PathBuf,
    offline: bool,
}
impl vibe_registry::multi_registry_resolver::RegistryCatalogProvider for Declarations {
    fn declarations(
        &self,
    ) -> std::result::Result<
        vibe_registry::multi_registry_resolver::RegistryDeclarations,
        vibe_registry::RegistryError,
    > {
        // Preference/exclusion observation precedes any new-name source choice.
        let _policy = self.operation.policy.policy()?;
        let global = self.operation.policy.global.catalog().map_err(|_| {
            vibe_registry::RegistryError::MalformedMeta {
                path: "registry.toml".into(),
                reason: "current registry catalog cannot be decoded".into(),
            }
        })?;
        let local = project_local_layer(&self.project_root).map_err(|error| {
            vibe_registry::RegistryError::MalformedMeta {
                path: vibe_core::project_local_registry_path(&self.project_root),
                reason: format!("{error:#}"),
            }
        })?;
        let effective = effective_registry_config(&self.manifest, &global, &local, self.offline);
        let admission = if self.operation.controlled()? {
            Some(self.operation.admission()?)
        } else {
            None
        };
        Ok(
            vibe_registry::multi_registry_resolver::RegistryDeclarations {
                registries: effective.registries,
                mirrors: effective.mirrors,
                overrides: effective.overrides,
                admission,
            },
        )
    }
}
/// Capture invocation data only. No binary, config closure or catalog is opened.
pub fn source_operation(
    options: &PackageSourceOptions,
    manifest: &Manifest,
    project_root: &Path,
    global: GlobalRegistrySource,
) -> Result<SourceOperation> {
    let environment = std::env::vars_os().collect();
    let cwd = std::env::current_dir().context("capturing source invocation directory")?;
    let acquisition_root = match &options.storage {
        Some(storage) => storage.acquisition_root.join("native"),
        None => vibe_registry::default_cache_root()?.join(".native"),
    };
    if !acquisition_root.is_absolute() {
        bail!("operation acquisition root must be absolute");
    }
    let _selected_project = project_root;
    Ok(SourceOperation::new(
        global,
        manifest.source_policy.clone(),
        options.source_policy.clone(),
        crate::transport::TransportInputs {
            cwd,
            acquisition_root,
            environment,
        },
        options.storage.clone(),
    )?)
}
/// Ordinary source operations use memory defaults on a missing file; explicit
/// registry management commands retain their own deliberate seeding behavior.
///
/// `seed_defaults` is the composition root's reading of the
/// `VIBE_NO_DEFAULT_REGISTRY` posture (`true` when it is unset): this library
/// never reads the ambient environment itself.
pub fn deferred_user_registry(seed_defaults: bool) -> GlobalRegistrySource {
    let missing = if !seed_defaults {
        GlobalRegistryConfig::default()
    } else {
        GlobalRegistryConfig {
            registries: vibe_core::default_registries(),
            ..Default::default()
        }
    };
    GlobalRegistrySource::deferred(vibe_core::settings::registry_config_path(), missing)
}
