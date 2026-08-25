//! Filesystem adapter from one selected workspace node to the pure extension collector.
//!
//! The collector deliberately knows nothing about manifests, lockfiles, or slots. This
//! cell owns that impure boundary and returns only an already ordered, typed registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::lifecycle::{ExtensionPoint, Phase, PhasePoint};
use vibe_core::manifest::{ExtensionKey, Lockfile, Manifest, Materialization};
use vibe_core::{Group, PackageKind, PackageName};
use vibe_lifecycle::{
    ContributionTier, DependencyExtensionSource, DependencyProvider, DependencyProviderId,
    ExtensionProvider, ExtensionRegistry, ExtensionWorld, HostExtensionSource, HostIdentity,
    HostProvider, SelectorSubject, collect_extensions,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_workspace::Workspace;
use vibe_workspace::vibedeps::{in_place_slot_abs_path, slot_abs_path};

/// Effective contribution plan and non-fatal collection notices for one ritual.
#[derive(Debug, Default)]
pub(crate) struct RitualPlan {
    pub(crate) contributions: Vec<LifecycleContributionReport>,
    pub(crate) notices: Vec<String>,
}

impl RitualPlan {
    pub(crate) fn count_for(&self, phase: Phase) -> usize {
        self.contributions
            .iter()
            .filter(|row| row.phase == phase.as_str())
            .count()
    }
}

/// Load the selected node's effective world and plan the requested default phases.
pub(crate) fn plan_default(path: &Path, phases: &[Phase]) -> Result<RitualPlan> {
    let registry = load_registry(path, WorldLoadMode::Default)?;
    let mut contributions = Vec::new();
    for phase in phases {
        contributions.extend(plan_point(
            &registry,
            ExtensionPoint::Phase(PhasePoint::Default(*phase)),
            phase.as_str(),
        ));
    }
    Ok(RitualPlan {
        contributions,
        notices: registry.notices().iter().map(ToString::to_string).collect(),
    })
}

/// Load and plan the independent clean point before any destructive wipe.
pub(crate) fn plan_clean(path: &Path) -> Result<RitualPlan> {
    let registry = load_registry(path, WorldLoadMode::PreClean)?;
    Ok(RitualPlan {
        contributions: plan_point(&registry, ExtensionPoint::Phase(PhasePoint::Clean), "clean"),
        notices: registry.notices().iter().map(ToString::to_string).collect(),
    })
}

fn plan_point(
    registry: &ExtensionRegistry,
    point: ExtensionPoint,
    phase: &str,
) -> Vec<LifecycleContributionReport> {
    registry
        .plan(point, SelectorSubject::unscoped())
        .into_iter()
        .map(|row| {
            let (provider, version) = match row.provider() {
                ExtensionProvider::Dependency(provider) => {
                    (provider.id.to_string(), Some(provider.version.clone()))
                }
                ExtensionProvider::Host(provider) => (
                    provider.identity.to_string(),
                    (!provider.version.is_empty()).then(|| provider.version.clone()),
                ),
            };
            LifecycleContributionReport {
                handler: row.declaration().handler.kind().to_string(),
                key: row.key().to_string(),
                phase: phase.to_string(),
                point: point.to_string(),
                provider,
                status: "planned".to_string(),
                tier: tier_name(row.effective_tier()).to_string(),
                version,
            }
        })
        .collect()
}

const fn tier_name(tier: ContributionTier) -> &'static str {
    match tier {
        ContributionTier::Preset => "preset",
        ContributionTier::Dependency => "dependency",
        ContributionTier::HostDeclaration => "host-declaration",
        ContributionTier::HostActivation => "host-activation",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldLoadMode {
    Default,
    PreClean,
}

fn load_registry(selected: &Path, mode: WorldLoadMode) -> Result<ExtensionRegistry> {
    let selected = super::super::install::resolve_project_root(selected)?;
    let workspace = Workspace::discover(&selected)
        .context("discovering the workspace for lifecycle contribution collection")?;
    let host_manifest = selected_manifest(&workspace, &selected)?.clone();
    let lock_path = workspace.lockfile_path();
    let vibedeps_root = workspace.vibedeps_root();

    let lock_exists = lock_path
        .try_exists()
        .with_context(|| format!("checking lifecycle lockfile `{}`", lock_path.display()))?;
    let vibedeps_exists = vibedeps_root.try_exists().with_context(|| {
        format!(
            "checking lifecycle dependency root `{}`",
            vibedeps_root.display()
        )
    })?;
    if vibedeps_exists && !vibedeps_root.is_dir() {
        bail!(
            "lifecycle dependency root `{}` exists but is not a directory",
            vibedeps_root.display(),
        );
    }
    if vibedeps_exists && !lock_exists {
        bail!(
            "lifecycle dependency root `{}` exists without `{}`; remove the orphaned root or run `vibe install`",
            vibedeps_root.display(),
            lock_path.display(),
        );
    }

    let installed = if mode == WorldLoadMode::PreClean && !vibedeps_exists {
        Vec::new()
    } else {
        let lock = if lock_exists {
            Lockfile::read(&lock_path).context("reading the lifecycle effective-world lockfile")?
        } else {
            Lockfile::empty("lifecycle-empty-world", "1970-01-01T00:00:00Z")
        };
        dependency_sources(&workspace, &host_manifest, &lock, mode)?
    };
    let effective_stack = effective_stack(&host_manifest, &installed, mode)?;
    let mut host = host_source(host_manifest, selected)?;
    if mode == WorldLoadMode::PreClean {
        retain_pre_clean_controls(&mut host, &installed);
    }

    collect_extensions(ExtensionWorld {
        installed,
        host,
        effective_stack,
    })
    .context("collecting lifecycle extensions from the effective world")
}

/// Pre-clean sees the old installed intersection, which may lag one host edit.
/// Retain controls that exactly match declarations in that world and defer
/// future dependency targets to strict Epoch B. Keys stay opaque: this only
/// compares them with keys constructed from typed provider/declaration data.
fn retain_pre_clean_controls(
    host: &mut HostExtensionSource,
    installed: &[DependencyExtensionSource],
) {
    let mut effective_keys: BTreeSet<_> = host
        .declarations
        .iter()
        .filter_map(|declaration| host_extension_key(&host.provider.identity, &declaration.id))
        .collect();
    effective_keys.extend(installed.iter().flat_map(|source| {
        source.declarations.iter().map(|declaration| {
            ExtensionKey::for_package(
                source.provider.id.group(),
                source.provider.id.name(),
                &declaration.id,
            )
        })
    }));
    host.controls
        .uses
        .retain(|activation| effective_keys.contains(&activation.reference));
    host.controls
        .disable
        .retain(|key| effective_keys.contains(key));
}

fn host_extension_key(identity: &HostIdentity, id: &str) -> Option<ExtensionKey> {
    match identity {
        HostIdentity::UngroupedProject(name) => Some(ExtensionKey::for_host(name, id)),
        HostIdentity::Coordinate(provider) => Some(ExtensionKey::for_package(
            provider.group(),
            provider.name(),
            id,
        )),
        HostIdentity::VirtualWorkspace => None,
    }
}

fn selected_manifest<'workspace>(
    workspace: &'workspace Workspace,
    selected: &Path,
) -> Result<&'workspace Manifest> {
    workspace
        .iter_nodes()
        .find(|(rel, _)| workspace.node_abs_path(rel) == selected)
        .map(|(_, manifest)| manifest)
        .with_context(|| {
            format!(
                "selected lifecycle host `{}` is not a node of workspace `{}`",
                selected.display(),
                workspace.root.display()
            )
        })
}

fn dependency_sources(
    workspace: &Workspace,
    host: &Manifest,
    lock: &Lockfile,
    mode: WorldLoadMode,
) -> Result<Vec<DependencyExtensionSource>> {
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
                "selected host requires `{group}/{name}`, but it is absent from effective-world lock `{}`",
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
                    "selected host reaches `{group}/{name}`, but the package is absent from `{}`",
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

fn dependency_source(
    workspace: &Workspace,
    package: &vibe_core::manifest::LockedPackage,
) -> Result<DependencyExtensionSource> {
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
            "reading reachable slot manifest `{}`",
            root.join(Manifest::FILENAME).display()
        )
    })?;
    let declared = manifest.package.as_ref().with_context(|| {
        format!(
            "reachable slot `{}` has no `[package]` identity",
            root.display()
        )
    })?;
    if declared.group != package.group
        || declared.name != package.name
        || declared.version != package.version
        || declared.kind != package.kind
    {
        bail!(
            "reachable slot `{}` declares `{}:{}/{}@{}`, but the lock requires `{}:{}/{}@{}`",
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

    Ok(DependencyExtensionSource {
        provider: DependencyProvider {
            id: DependencyProviderId::new(package.group.clone(), package.name.clone()),
            root,
            version: package.version.to_string(),
            kind: package.kind,
            content_hash: package.content_hash.clone(),
        },
        declarations: manifest.extensions,
    })
}

fn effective_stack(
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
            "[active].stack `{short_name}` names no installed reachable stack package; install it or correct the short name"
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

fn host_source(manifest: Manifest, root: PathBuf) -> Result<HostExtensionSource> {
    let (identity, version, kind) = if let Some(package) = &manifest.package {
        (
            HostIdentity::coordinate(DependencyProviderId::new(
                package.group.clone(),
                PackageName::parse(&package.name)?,
            )),
            package.version.to_string(),
            Some(package.kind),
        )
    } else if let Some(project) = &manifest.project {
        let identity = match &project.group {
            Some(group) => HostIdentity::coordinate(DependencyProviderId::new(
                group.clone(),
                PackageName::parse(&project.name)?,
            )),
            None => HostIdentity::ungrouped_project(project.name.clone()),
        };
        (identity, project.version.clone(), None)
    } else {
        (HostIdentity::virtual_workspace(), String::new(), None)
    };

    Ok(HostExtensionSource {
        provider: HostProvider {
            identity,
            root,
            version,
            kind,
            content_hash: None,
        },
        declarations: manifest.extensions,
        controls: manifest.extension_controls,
    })
}

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;
