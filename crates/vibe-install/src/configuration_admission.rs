//! Unsupported package configuration is refused at its actual consumer boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");

use crate::error::{Error, Result};
use std::path::Path;
use vibe_core::{PackageRef, manifest::Manifest};
use vibe_workspace::Workspace;

/// Admit every real workspace consumer and the raw selected snapshot before
/// source edits, migration, freshness, solving or boot publication. A workspace
/// package's own interface declaration is not its dependency configuration.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn workspace_inputs(
    workspace: &Workspace,
    selected_root: &Path,
    raw_selected: &Manifest,
) -> Result<()> {
    let selected =
        workspace
            .node_rel_of(selected_root)
            .ok_or_else(|| Error::SelectedConsumerMissing {
                selected: selected_root.display().to_string(),
                workspace: workspace.root.display().to_string(),
            })?;
    for (consumer, manifest) in workspace
        .iter_nodes()
        .chain(std::iter::once((selected.as_str(), raw_selected)))
    {
        if let Some(dependency) = manifest.requires.configurations.keys().next().or_else(|| {
            manifest
                .requires
                .optional
                .iter()
                .find_map(|(key, optional)| optional.then_some(key))
        }) {
            return Err(Error::ConfigurationInputUnsupported {
                consumer: format!("{consumer}/vibe.toml"),
                dependency: dependency.clone(),
            });
        }
    }
    Ok(())
}

/// Admit the actual manifest consumed as a package, before legacy projection or
/// project effects. Selected installer configuration belongs to the app owner.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn package_manifest(package: &PackageRef, manifest: &Manifest) -> Result<()> {
    manifest_named(package.qualified_name(), manifest)
}
fn manifest_named(package: String, manifest: &Manifest) -> Result<()> {
    if manifest.configuration.is_some() {
        return Err(Error::ConfigurationUnsupported { package });
    }
    if let Some(dependency) = manifest.requires.configurations.keys().next().or_else(|| {
        manifest
            .requires
            .optional
            .iter()
            .find_map(|(key, optional)| optional.then_some(key))
    }) {
        return Err(Error::ConfigurationInputUnsupported {
            consumer: format!("package {package}"),
            dependency: dependency.clone(),
        });
    }
    Ok(())
}

/// Admit a complete consumed package projection before its first effect.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn resolved_packages(packages: &[vibe_workspace::install::ResolvedDep]) -> Result<()> {
    for package in packages {
        admitted_package(
            format!("{}/{}", package.group, package.name),
            &package.manifest,
            package.configuration.as_ref(),
        )?;
    }
    Ok(())
}

pub(crate) fn planned(planned: &crate::PlannedInstall) -> Result<()> {
    workspace_inputs_for_install(&planned.workspace, &planned.project_root, &planned.manifest)?;
    resolved_packages(&planned.resolution)?;
    for fetched in &planned.fetched {
        let package = format!(
            "{}/{}",
            fetched.cached.resolved.group, fetched.cached.resolved.name
        );
        admitted_package(
            package.clone(),
            &fetched.cached.manifest,
            fetched.configuration.as_ref(),
        )?;
        if let Some(prepared) = &fetched.in_place {
            admitted_package(package, prepared.manifest(), fetched.configuration.as_ref())?;
        }
    }
    if planned.fetched.iter().any(|f| f.configuration.is_some()) {
        let mut prospective =
            vibe_core::manifest::Lockfile::empty("configuration-admission", "not-published");
        prospective.meta.schema_version = vibe_core::manifest::CURRENT_SCHEMA_VERSION;
        for fetched in &planned.fetched {
            let resolved = planned
                .resolution
                .iter()
                .find(|r| {
                    r.group == fetched.cached.resolved.group
                        && r.name == fetched.cached.resolved.name
                })
                .ok_or_else(|| vibe_core::Error::InvalidLockfile {
                    reason: "planned configuration has no matching resolution row".into(),
                })?;
            if fetched.configuration != resolved.configuration {
                return Err(vibe_core::Error::InvalidLockfile {
                    reason: "planned configuration carriers disagree".into(),
                }
                .into());
            }
            prospective
                .packages
                .push(crate::record::locked_package_from_fetched(
                    fetched, resolved, None,
                ));
            if let Some(record) = &fetched.configuration {
                prospective.configurations.insert(
                    format!(
                        "{}/{}",
                        fetched.cached.resolved.group, fetched.cached.resolved.name
                    ),
                    record.clone(),
                );
            }
        }
        prospective.validate_configurations()?;
    }
    Ok(())
}

/// Read only the lock-selected materialized providers when a Fresh migration
/// would publish authored inputs without going through normal fetching.
pub(crate) fn locked_packages(
    workspace: &Workspace,
    lock: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    for package in &lock.packages {
        let slot = if package.materialization.is_in_place() {
            vibe_workspace::vibedeps::in_place_slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
            )
        } else {
            vibe_workspace::vibedeps::slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
                &package.version,
            )
        };
        let manifest = Manifest::read(slot.join(Manifest::FILENAME))?;
        let configuration = lock.configuration_for(package, &manifest)?;
        admitted_package(
            format!("{}/{}", package.group, package.name),
            &manifest,
            configuration.as_ref(),
        )?;
    }
    Ok(())
}

/// Before install source construction, retain supported original inputs while
/// refusing owner-self optionality and unsupported authored source routes. The
/// actual source/solver and candidate still must admit the resulting plan.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn workspace_inputs_for_install(
    workspace: &Workspace,
    selected_root: &Path,
    raw_selected: &Manifest,
) -> Result<()> {
    let selected =
        workspace
            .node_rel_of(selected_root)
            .ok_or_else(|| Error::SelectedConsumerMissing {
                selected: selected_root.display().to_string(),
                workspace: workspace.root.display().to_string(),
            })?;
    for (consumer, manifest) in workspace
        .iter_nodes()
        .chain(std::iter::once((selected.as_str(), raw_selected)))
    {
        for (key, optional) in &manifest.requires.optional {
            if *optional {
                return Err(Error::ConfigurationInputUnsupported {
                    consumer: format!("{consumer}/vibe.toml (owner activation context missing)"),
                    dependency: key.clone(),
                });
            }
        }
        for key in manifest.requires.configurations.keys() {
            let registry = manifest
                .requires
                .packages
                .iter()
                .any(|p| p.qualified_name() == *key)
                || manifest
                    .requires
                    .var_packages
                    .iter()
                    .any(|p| format!("{}/{}", p.group, p.name) == *key);
            if !registry {
                return Err(Error::ConfigurationInputUnsupported {
                    consumer: format!("{consumer}/vibe.toml (unsupported configuration source)"),
                    dependency: key.clone(),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn admitted_package(
    package: String,
    manifest: &Manifest,
    configuration: Option<&vibe_core::manifest::PackageConfiguration>,
) -> Result<()> {
    match configuration {
        Some(record) => {
            record.validate(manifest)?;
            Ok(())
        }
        None => manifest_named(package, manifest),
    }
}

/// Replay uses retained original inputs; a changed authored answer requires
/// planning rather than silently reusing the old derived graph.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
pub fn locked_workspace(
    workspace: &Workspace,
    selected_root: &Path,
    raw_selected: &Manifest,
    lock: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    workspace_inputs_for_install(workspace, selected_root, raw_selected)?;
    lock.validate_configurations()?;
    if let vibe_workspace::freshness::Freshness::Stale(reason) =
        vibe_workspace::freshness::configuration_freshness(workspace, lock)
    {
        return Err(
            vibe_core::manifest::PackageConfigurationError::Unsupported {
                package: "workspace dependency graph".into(),
                reason: format!(
                    "retained input changed ({reason}); run install to plan the authored change"
                ),
            }
            .into(),
        );
    }
    Ok(())
}

/// A previously rich world is checked before deciding Fresh versus replan.
/// A stale authored request is not permission to reconstruct missing history.
/// Missing slots can use the existing exact source store, without creating a
/// project slot; an unavailable unclassified row is read through the old pin.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
pub(crate) fn retained_before_planning<S: crate::InstallSource + ?Sized>(
    source: &S,
    workspace: &Workspace,
    lock: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    if lock.meta.schema_version != vibe_core::manifest::CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    let store = vibe_registry::store::store_root()?;
    for package in &lock.packages {
        let key = format!("{}/{}", package.group, package.name);
        let slot = if package.materialization.is_in_place() {
            vibe_workspace::vibedeps::in_place_slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
            )
        } else {
            vibe_workspace::vibedeps::slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
                &package.version,
            )
        };
        let manifest = if slot.is_dir() {
            Manifest::read(slot.join(Manifest::FILENAME))?
        } else {
            let entry = vibe_registry::store::entry_dir(
                &store,
                &package.group,
                &package.name,
                &package.version,
            );
            if entry.is_dir() {
                let recipe = if package
                    .content_hash
                    .starts_with(vibe_registry::RecipeId::Tree1.label())
                {
                    vibe_registry::RecipeId::Tree1
                } else {
                    vibe_registry::RecipeId::Legacy0
                };
                let hash = vibe_registry::compute_content_hash_with(recipe, &entry)?;
                if hash != package.content_hash.as_str() {
                    return Err(vibe_core::Error::InvalidLockfile {
                        reason: format!(
                            "retained source store for `{key}` disagrees with the exact lock pin"
                        ),
                    }
                    .into());
                }
                Manifest::read(entry.join(Manifest::FILENAME))?
            } else if lock.configurations.contains_key(&key) {
                // Present historical record was validated at read. A later
                // acquisition still must admit its actual candidate; no missing
                // answer/default receipt is invented here.
                continue;
            } else {
                let cached = source.resolve_and_fetch(
                    &package.as_package_ref()?,
                    &store,
                    Some(&package.content_hash),
                )?;
                if cached.content_hash != package.content_hash.as_str() {
                    return Err(vibe_core::Error::InvalidLockfile {
                        reason: format!(
                            "cannot classify retained `{key}` without its exact source pin"
                        ),
                    }
                    .into());
                }
                cached.manifest
            }
        };
        lock.configuration_for(package, &manifest)?;
    }
    Ok(())
}

/// Rediscovery may add, remove or edit a consumer after planning. Keep the
/// prepared original-input set exact; apply never silently re-evaluates it.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
pub(crate) fn unchanged_inputs(
    before: &Workspace,
    after: &Workspace,
    selected_root: &Path,
    selected_after: &Manifest,
) -> Result<()> {
    let selected =
        before
            .node_rel_of(selected_root)
            .ok_or_else(|| Error::SelectedConsumerMissing {
                selected: selected_root.display().to_string(),
                workspace: before.root.display().to_string(),
            })?;
    let targets = before
        .iter_nodes()
        .chain(after.iter_nodes())
        .flat_map(|(_, node)| node.requires.configurations.keys().cloned())
        .chain(selected_after.requires.configurations.keys().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    for target in targets {
        let contributions = |workspace: &Workspace, overlay: bool| {
            workspace
                .iter_nodes()
                // The selected CLI declaration delta was already planned. Only
                // other newly observed inputs must be compared to the old tree.
                .map(|(node, manifest)| {
                    (
                        node,
                        if overlay && node == selected.as_str() {
                            selected_after
                        } else {
                            manifest
                        },
                    )
                })
                .filter(|(_, node)| {
                    node.requires
                        .iter_pkgrefs()
                        .any(|(g, n)| g.is_some_and(|g| format!("{g}/{n}") == target))
                })
                .map(|(node, manifest)| {
                    (
                        node.to_owned(),
                        manifest.requires.configurations.get(&target).cloned(),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let old = contributions(before, true);
        let current = contributions(after, false);
        for consumer in old
            .keys()
            .chain(current.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if old.get(consumer) != current.get(consumer) {
                return Err(Error::ConfigurationInputChanged {
                    consumer: format!("{consumer}/vibe.toml"),
                    dependency: target,
                });
            }
        }
    }
    Ok(())
}
