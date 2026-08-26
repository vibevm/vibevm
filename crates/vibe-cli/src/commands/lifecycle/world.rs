//! Filesystem adapter from one selected workspace node to the pure extension collector.
//!
//! The collector deliberately knows nothing about manifests, lockfiles, or slots. This
//! cell owns that impure boundary and returns only an already ordered, typed registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::lifecycle::{ExtensionPoint, Phase, PhasePoint};
use vibe_core::manifest::{ExtensionKey, Lockfile, Manifest, Materialization, SkillDecl};
use vibe_core::{Group, PackageKind, PackageName};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, EffectiveManifestKind,
    ExecutablePlan, ExtensionRegistry, ExtensionWorld, HostExtensionSource, HostIdentity,
    HostProvider, SelectorSubject, collect_extensions_with_presets,
};
use vibe_mcp::pkgskill::ProjectSkillBinding;
use vibe_wire::generated::lifecycle::e1::context::{
    Project as EnvelopeProject, World as EnvelopeWorld, WorldPackage,
};
use vibe_workspace::Workspace;
use vibe_workspace::vibedeps::{in_place_slot_abs_path, slot_abs_path};

#[path = "world/package_skill.rs"]
mod package_skill;
pub(crate) use package_skill::RECONCILE_KEY as PACKAGE_SKILL_RECONCILE_KEY;
pub(crate) use package_skill::RECOVER_KEY as PACKAGE_SKILL_RECOVER_KEY;

/// Effective contribution plan and non-fatal collection notices for one ritual.
#[derive(Debug)]
pub(crate) struct RitualPlan {
    pub(crate) executions: ExecutablePlan,
    pub(crate) notices: Vec<String>,
    pub(crate) project: EnvelopeProject,
    pub(crate) world: EnvelopeWorld,
    pub(crate) workspace_root: PathBuf,
    pub(crate) package_bindings: BTreeMap<String, ProjectSkillBinding>,
    pub(crate) package_desired_keys: BTreeSet<String>,
    pub(crate) package_phase_planned: bool,
}

impl RitualPlan {
    pub(crate) fn count_for(&self, phase: Phase) -> usize {
        self.executions.count_for(phase.as_str())
    }
}

/// One effective declaration retained for execution in canonical order.
pub(crate) type PlannedExecution = vibe_lifecycle::ExecutableContribution;

/// Load the selected node's effective world and plan the requested default phases.
pub(crate) fn plan_default(path: &Path, phases: &[Phase]) -> Result<RitualPlan> {
    let loaded = load_registry(path, WorldLoadMode::Default)?;
    let executions = ExecutablePlan::from_points(
        &loaded.registry,
        phases.iter().map(|phase| {
            (
                phase.to_string(),
                ExtensionPoint::Phase(PhasePoint::Default(*phase)),
            )
        }),
        SelectorSubject::unscoped(),
    );
    Ok(RitualPlan {
        executions,
        notices: loaded
            .registry
            .notices()
            .iter()
            .map(ToString::to_string)
            .collect(),
        project: loaded.project,
        world: loaded.world,
        workspace_root: loaded.workspace_root,
        package_bindings: loaded.package_bindings,
        package_desired_keys: loaded.package_desired_keys,
        package_phase_planned: phases.contains(&Phase::Package),
    })
}

/// Load and plan the independent clean point before any destructive wipe.
pub(crate) fn plan_clean(path: &Path) -> Result<RitualPlan> {
    let loaded = load_registry(path, WorldLoadMode::PreClean)?;
    Ok(RitualPlan {
        executions: ExecutablePlan::from_points(
            &loaded.registry,
            [(
                "clean".to_string(),
                ExtensionPoint::Phase(PhasePoint::Clean),
            )],
            SelectorSubject::unscoped(),
        ),
        notices: loaded
            .registry
            .notices()
            .iter()
            .map(ToString::to_string)
            .collect(),
        project: loaded.project,
        world: loaded.world,
        workspace_root: loaded.workspace_root,
        package_bindings: BTreeMap::new(),
        package_desired_keys: BTreeSet::new(),
        package_phase_planned: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldLoadMode {
    Default,
    PreClean,
}

pub(crate) struct LoadedRegistry {
    pub(crate) registry: ExtensionRegistry,
    pub(crate) project: EnvelopeProject,
    world: EnvelopeWorld,
    pub(crate) workspace_root: PathBuf,
    pub(crate) host_identity: HostIdentity,
    pub(crate) manifest_kind: EffectiveManifestKind,
    pub(crate) effective_stack: Option<DependencyProviderId>,
    package_bindings: BTreeMap<String, ProjectSkillBinding>,
    package_desired_keys: BTreeSet<String>,
}

/// Strict read-only inspection of one selected node's durable effective world.
pub(crate) fn inspect(path: &Path) -> Result<LoadedRegistry> {
    load_registry(path, WorldLoadMode::Default)
}

fn load_registry(selected: &Path, mode: WorldLoadMode) -> Result<LoadedRegistry> {
    let selected = super::super::install::resolve_project_root(selected)?;
    let workspace = Workspace::discover(&selected)
        .context("discovering the workspace for lifecycle contribution collection")?;
    let host_manifest = selected_manifest(&workspace, &selected)?.clone();
    let manifest_kind = effective_manifest_kind(&host_manifest);
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
            "lifecycle dependency root `{}` exists but is not a directory; remove or repair it, then run `vibe install`",
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

    let loaded_dependencies = if mode == WorldLoadMode::PreClean && !vibedeps_exists {
        Vec::new()
    } else {
        let lock = if lock_exists {
            Lockfile::read(&lock_path).with_context(|| {
                format!(
                    "reading lifecycle effective-world lockfile `{}`; repair or remove it, then run `vibe install`",
                    lock_path.display()
                )
            })?
        } else {
            Lockfile::empty("lifecycle-empty-world", "1970-01-01T00:00:00Z")
        };
        dependency_sources(&workspace, &host_manifest, &lock, mode)?
    };
    let installed = loaded_dependencies
        .iter()
        .map(|dependency| dependency.source.clone())
        .collect::<Vec<_>>();
    let effective_stack = effective_stack(&host_manifest, &installed, mode)?;
    let host_skills = host_manifest.skills.clone();
    let mut host = host_source(host_manifest, selected.clone())?;
    let host_identity = host.provider.identity.clone();
    if mode == WorldLoadMode::PreClean {
        retain_pre_clean_controls(&mut host, &installed);
    }

    let project = envelope_project(&host.provider, &selected);
    let world = envelope_world(&workspace, &installed);
    let (presets, package_bindings, package_desired_keys) = if mode == WorldLoadMode::Default {
        package_skill::presets(
            &selected,
            &host.provider,
            &host_skills,
            &loaded_dependencies,
        )?
    } else {
        (Vec::new(), BTreeMap::new(), BTreeSet::new())
    };
    let registry = collect_extensions_with_presets(
        ExtensionWorld {
            installed,
            host,
            effective_stack: effective_stack.clone(),
        },
        presets,
    )
    .context("collecting lifecycle extensions from the effective world")?;
    Ok(LoadedRegistry {
        registry,
        project,
        world,
        workspace_root: workspace.root.clone(),
        host_identity,
        manifest_kind,
        effective_stack,
        package_bindings,
        package_desired_keys,
    })
}

#[derive(Debug, Clone)]
pub(super) struct LoadedDependency {
    pub(super) source: DependencyExtensionSource,
    pub(super) skills: Vec<SkillDecl>,
}

fn effective_manifest_kind(manifest: &Manifest) -> EffectiveManifestKind {
    if let Some(package) = &manifest.package {
        EffectiveManifestKind::Package(package.kind)
    } else if manifest.project.is_some() {
        EffectiveManifestKind::Project
    } else {
        EffectiveManifestKind::VirtualWorkspace
    }
}

fn envelope_project(provider: &HostProvider, selected: &Path) -> EnvelopeProject {
    let (name, kind) = match &provider.identity {
        HostIdentity::UngroupedProject(name) => (name.clone(), "project".to_string()),
        HostIdentity::Coordinate(identity) => (
            identity.name().to_string(),
            provider
                .kind
                .map_or_else(|| "project".to_string(), |kind| kind.as_str().to_string()),
        ),
        HostIdentity::VirtualWorkspace => {
            ("<virtual-workspace>".to_string(), "workspace".to_string())
        }
    };
    EnvelopeProject {
        kind,
        manifest: vibe_core::machine_json_path(&selected.join(Manifest::FILENAME)),
        name,
        root: vibe_core::machine_json_path(selected),
        spec_roots: vec![vibe_core::machine_json_path(
            &selected.join(vibe_core::layout::current_specs_root()),
        )],
        version: provider.version.clone(),
    }
}

fn envelope_world(workspace: &Workspace, installed: &[DependencyExtensionSource]) -> EnvelopeWorld {
    EnvelopeWorld {
        deps_root: vibe_core::machine_json_path(&workspace.vibedeps_root()),
        lockfile: vibe_core::machine_json_path(&workspace.lockfile_path()),
        packages: installed
            .iter()
            .map(|source| WorldPackage {
                group: source.provider.id.group().to_string(),
                kind: source.provider.kind.as_str().to_string(),
                name: source.provider.id.name().to_string(),
                slot: vibe_core::machine_json_path(&source.provider.root),
                version: source.provider.version.clone(),
            })
            .collect(),
    }
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

fn dependency_source(
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
