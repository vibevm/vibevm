//! Post-durable lifecycle refresh through the ordinary locked metadata planner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#AI-NATIVE-AUTHORITY-BRIDGE");
use anyhow::{Context, Result};
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_workspace::Workspace;

pub(crate) fn refresh(ctx: &crate::output::Context, root: &Path) -> Result<()> {
    refresh_inner(ctx, root).with_context(|| format!(
        "lifecycle changes are already durable; specification authority refresh failed; rerun vibe install --path {}", root.display()
    ))
}
fn refresh_inner(ctx: &crate::output::Context, root: &Path) -> Result<()> {
    let workspace = Workspace::discover(root)?;
    let manifest = Manifest::read(root.join(Manifest::FILENAME))?;
    let lock_path = workspace.root.join(Lockfile::FILENAME);
    let lock = if lock_path.exists() {
        Lockfile::read(lock_path)?
    } else {
        Lockfile::empty(vibe_orchestrator::generated_by(), "authority")
    };
    let scratch = tempfile::Builder::new()
        .prefix("vibe-lifecycle-authority-")
        .tempdir()?;
    let options = vibe_package_source::PackageSourceOptions {
        storage: Some(vibe_package_source::SourceStorage {
            acquisition_root: scratch.path().into(),
            read_only_store: Some(vibe_registry::store::store_root()?),
        }),
        ..Default::default()
    };
    let resolver = vibe_package_source::build_install_resolver_with_sources(
        vibe_package_source::InstallResolverRequest {
            options: &options,
            manifest: &manifest,
            embedded_root: None,
            project_root: root,
            offline: true,
            locked: &lock.packages,
            progress: &ctx.progress(),
        },
        vibe_package_source::deferred_user_registry(false),
    )?
    .with_locked_checkouts(vibe_install::locked_checkout_paths(
        &workspace.root,
        &lock.packages,
    ));
    let source = vibe_package_source::RegistryPackageSource::new(
        resolver,
        Box::new(super::install::CliQualifier),
    );
    let request = vibe_install::InstallRequest {
        roots: Vec::new(),
        features: Default::default(),
        language: None,
        exact: false,
        no_recommends: false,
        generated_by: vibe_orchestrator::generated_by(),
        offline: true,
    };
    let format = super::install::resolve_spec_format(
        &manifest,
        vibe_core::user_config::UserConfig::load()?
            .install
            .spec_format,
    );
    vibe_orchestrator::refresh_specmap_authority(
        &workspace,
        root,
        &manifest,
        &lock,
        &source,
        scratch.path(),
        &request,
        format,
        &vibe_install::NullObserver,
    )?;
    scratch.close()?;
    Ok(())
}
