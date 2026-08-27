//! `vibe update`'s inputs: the delegated install arguments, the project
//! manifest and lockfile readers, and the locked-entry rebuild.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_registry::CachedPackage;

use crate::cli::{InstallArgs, UpdateArgs};

/// Build the `InstallArgs` that `vibe update`'s whole-graph path delegates
/// with, and that `build_install_resolver` reads. `vibe update` carries no
/// `--registry` / `--git` / feature flags, so those default off.
pub(super) fn install_args_from(args: &UpdateArgs) -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path: args.path.clone(),
        registry: None,
        assume_yes: args.assume_yes,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: args.exact,
        auth_required: args.auth_required,
        solver: None,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        force: false,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
    }
}

/// Build the lockfile entry for a re-resolved package. Version, hash and
/// source come from the fresh fetch; the install-scoped `features` /
/// `subskills_active` / `language` are carried from the previous entry —
/// a version bump does not re-evaluate them.
pub(super) fn locked_package(
    cached: &CachedPackage,
    dependencies: &[PackageRef],
    old: Option<&LockedPackage>,
) -> LockedPackage {
    let source_kind = if cached.overridden {
        SourceKind::Override
    } else if cached.is_path_source {
        SourceKind::Path
    } else if cached.is_git_source {
        SourceKind::Git
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: cached.package_meta().kind,
        group: cached.resolved.group.clone(),
        name: vibe_core::PackageName::from_validated(cached.resolved.name.clone()),
        version: cached.resolved.version.clone(),
        registry: cached.registry_name.clone(),
        source_url: vibe_core::SourceUrl::new(cached.source_uri.clone()),
        source_ref: cached.source_ref.clone(),
        resolved_commit: cached.resolved_commit.clone(),
        content_hash: vibe_core::ContentHash::from_validated(cached.content_hash.clone()),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: dependencies.to_vec(),
        admitted_by: old.and_then(|package| package.admitted_by.clone()),
        via_override: old.and_then(|package| package.via_override.clone()),
        overridden: cached.overridden,
        source_kind: Some(source_kind),
        via_redirect: cached.via_redirect.clone(),
        features: old.map(|o| o.features.clone()).unwrap_or_default(),
        subskills_active: old.map(|o| o.subskills_active.clone()).unwrap_or_default(),
        describes: cached
            .package_meta()
            .describes
            .as_ref()
            .map(|p| p.to_string()),
        language: old.and_then(|o| o.language.clone()),
        // A version bump does not change how the package is materialised —
        // carry the freshly-fetched manifest's declared mode (PROP-022 §2.1).
        materialization: cached.package_meta().materialization,
    }
}

pub(super) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

pub(super) fn load_project_manifest(root: &Path) -> Result<Manifest> {
    Ok(Manifest::read(root.join(Manifest::FILENAME))?)
}

pub(super) fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        bail!(
            "no `vibe.lock` in `{}` — nothing to update; run `vibe install` first",
            root.display()
        );
    }
}
