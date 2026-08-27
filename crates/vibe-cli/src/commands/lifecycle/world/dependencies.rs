//! How one node's INSTALLED dependencies become extension sources.
//!
//! Split from the collection loop because it answers a different question:
//! the loop decides what the effective world IS, this decides which
//! materialised slots are reachable from the selected node and what each one
//! contributes. These are the reads that legitimately touch disk in a
//! post-install epoch — the current lockfile and the slot manifests the
//! install just wrote — which is why they sit behind their own door.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_workspace::Workspace;

use super::*;

pub(super) fn dependency_sources(
    workspace: &Workspace,
    host: &Manifest,
    lock: &Lockfile,
    mode: WorldLoadMode,
) -> Result<Vec<LoadedDependency>> {
    let by_id: BTreeMap<(&Group, &str), usize> = lock
        .packages
        .iter()
        .enumerate()
        .map(|(index, package)| ((&package.group, package.name.as_str()), index))
        .collect();
    let mut queue = VecDeque::new();
    for (group, name) in host.requires.iter_pkgrefs() {
        let group = group
            .cloned()
            .with_context(|| format!("selected host requirement `{name}` has no group"))?;
        if !by_id.contains_key(&(&group, name)) {
            // A host edit may be ahead of its lock. Pre-clean sees only the
            // old installed intersection; after the install barrier, the same
            // omission is a malformed durable world and must never hide that
            // provider's contributions.
            if mode == WorldLoadMode::PreClean {
                continue;
            }
            bail!(
                "selected host requires `{group}/{name}`, but it is absent from effective-world lock `{}`; run `vibe install`",
                workspace.lockfile_path().display(),
            );
        }
        queue.push_back((group, name.to_string()));
    }
    let mut reachable = BTreeSet::new();

    while let Some((group, name)) = queue.pop_front() {
        if !reachable.insert((group.clone(), name.clone())) {
            continue;
        }
        let index = by_id
            .get(&(&group, name.as_str()))
            .copied()
            .with_context(|| {
                format!(
                    "selected host reaches `{group}/{name}`, but the package is absent from `{}`; run `vibe install`",
                    workspace.lockfile_path().display()
                )
            })?;
        for dependency in &lock.packages[index].dependencies {
            let dependency_group = dependency.group.clone().with_context(|| {
                format!("locked dependency `{dependency}` of `{group}/{name}` has no group")
            })?;
            queue.push_back((dependency_group, dependency.name.to_string()));
        }
    }

    lock.packages
        .iter()
        .filter(|package| reachable.contains(&(package.group.clone(), package.name.to_string())))
        .map(|package| dependency_source(workspace, package))
        .collect()
}

pub(super) fn dependency_source(
    workspace: &Workspace,
    package: &vibe_core::manifest::LockedPackage,
) -> Result<LoadedDependency> {
    let root = match package.materialization {
        Materialization::InPlace => {
            in_place_slot_abs_path(&workspace.root, &package.group, &package.name)
        }
        Materialization::Copy | Materialization::Hardlink => slot_abs_path(
            &workspace.root,
            &package.group,
            &package.name,
            &package.version,
        ),
    };
    if !root.is_dir() {
        bail!(
            "reachable locked package `{}/{}@{}` has no materialised {} slot `{}`; run `vibe install`",
            package.group,
            package.name,
            package.version,
            if package.materialization.is_in_place() {
                "unversioned in-place"
            } else {
                "versioned"
            },
            root.display(),
        );
    }
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).with_context(|| {
        format!(
            "reading reachable slot manifest `{}`; remove or repair slot `{}`, then run `vibe install`",
            root.join(Manifest::FILENAME).display(),
            root.display(),
        )
    })?;
    let declared = manifest.package.as_ref().with_context(|| {
        format!(
            "reachable slot `{}` has no `[package]` identity; remove or repair the slot, then run `vibe install`",
            root.display()
        )
    })?;
    if declared.group != package.group
        || declared.name != package.name
        || declared.version != package.version
        || declared.kind != package.kind
    {
        bail!(
            "reachable slot `{}` declares `{}:{}/{}@{}`, but the lock requires `{}:{}/{}@{}`; remove or repair the slot, then run `vibe install`",
            root.display(),
            declared.kind,
            declared.group,
            declared.name,
            declared.version,
            package.kind,
            package.group,
            package.name,
            package.version,
        );
    }

    Ok(LoadedDependency {
        skills: manifest.skills,
        source: DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(package.group.clone(), package.name.clone()),
                root,
                version: package.version.to_string(),
                kind: package.kind,
                content_hash: package.content_hash.clone(),
            },
            declarations: manifest.extensions,
        },
    })
}

pub(super) fn effective_stack(
    host: &Manifest,
    installed: &[DependencyExtensionSource],
    mode: WorldLoadMode,
) -> Result<Option<DependencyProviderId>> {
    let Some(short_name) = host
        .active
        .as_ref()
        .and_then(|active| active.stack.as_deref())
    else {
        return Ok(None);
    };
    let matches: Vec<_> = installed
        .iter()
        .filter(|source| {
            source.provider.kind == PackageKind::Stack
                && source.provider.id.name().as_str() == short_name
        })
        .map(|source| source.provider.id.clone())
        .collect();
    match matches.as_slice() {
        [id] => Ok(Some(id.clone())),
        [] if mode == WorldLoadMode::PreClean => Ok(None),
        [] => bail!(
            "[active].stack `{short_name}` names no installed reachable stack package; run `vibe install` or correct the short name"
        ),
        many => bail!(
            "[active].stack `{short_name}` is ambiguous across installed reachable stacks: {}; use a unique stack short name",
            many.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
