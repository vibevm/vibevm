//! Filesystem adapter from one selected workspace node to the pure extension collector.
//!
//! The collector deliberately knows nothing about manifests, lockfiles, or slots. This
//! cell owns that impure boundary and returns only an already ordered, typed registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_agent_projection::pkgskill::ProjectSkillBinding;
use vibe_core::lifecycle::{ExtensionPoint, Phase, PhasePoint};
use vibe_core::manifest::{
    ExtensionKey, LlmSection, Lockfile, Manifest, Materialization, SkillDecl,
};
use vibe_core::{Group, PackageKind, PackageName};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, EffectiveManifestKind,
    ExecutablePlan, ExtensionRegistry, ExtensionWorld, HostExtensionSource, HostIdentity,
    HostProvider, SelectorSubject, collect_extensions_with_presets,
};
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
    /// The surface-neutral plan shared with the future MCP adapter.
    pub(crate) shared: vibe_orchestrator::RitualPlan,
    /// Project `[llm]`. Read with the manifest, never resolved here: an
    /// endpoint or credential is touched only inside an actual agent call.
    /// This CLI-only wrapper deliberately keeps provider/model configuration
    /// out of `vibe-orchestrator`.
    pub(crate) llm: Option<LlmSection>,
}

impl std::ops::Deref for RitualPlan {
    type Target = vibe_orchestrator::RitualPlan;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

/// One effective declaration retained for execution in canonical order.
pub(crate) use vibe_orchestrator::PlannedExecution;

/// Load the selected node's effective world and plan the requested default
/// phases, from a workspace the caller ALREADY has.
///
/// This is the production seam. It performs no path resolution, no discovery
/// and no read of the selected manifest: those answers arrived with the
/// prepared `Workspace`, which is also the only copy carrying this
/// invocation's own in-memory `--git` delta. A rediscovery here would be a
/// second byte snapshot of a tree the command is itself changing, and — on a
/// validate-only chain — could succeed where the first attempt failed.
///
/// It DOES read the current lockfile and the materialised dependency slot
/// manifests. That is the point of a post-install epoch: those are the
/// artifacts the install just wrote, and reading them is how the world it
/// produced is collected at all.
pub(crate) fn plan_default_prepared(
    selected_project_root: &Path,
    workspace: &Workspace,
    phases: &[Phase],
) -> Result<RitualPlan> {
    let loaded = load_registry_prepared(selected_project_root, workspace, WorldLoadMode::Default)?;
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
        shared: vibe_orchestrator::RitualPlan {
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
        },
        llm: loaded.llm,
    })
}

/// The compatibility wrapper: ONE ordinary discovery, then the prepared seam.
///
/// `#[cfg(test)]` on purpose. Every production caller now has a prepared
/// workspace, and gating this behind the test cfg makes "no production path
/// rediscovers to plan its world" a fact the compiler enforces rather than one
/// a reviewer has to re-check. Tests that build a project and plan it in one
/// step still want it.
#[cfg(test)]
pub(crate) fn plan_default(path: &Path, phases: &[Phase]) -> Result<RitualPlan> {
    let selected = super::super::install::resolve_project_root(path)?;
    let workspace = discover_for_collection(&selected)?;
    plan_default_prepared(&selected, &workspace, phases)
}

/// Load and plan the independent clean point before any destructive wipe.
pub(crate) fn plan_clean(path: &Path) -> Result<RitualPlan> {
    let loaded = load_registry(path, WorldLoadMode::PreClean)?;
    Ok(RitualPlan {
        shared: vibe_orchestrator::RitualPlan {
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
        },
        llm: loaded.llm,
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
    llm: Option<LlmSection>,
}

/// Strict read-only inspection of one selected node's durable effective world.
pub(crate) fn inspect(path: &Path) -> Result<LoadedRegistry> {
    load_registry(path, WorldLoadMode::Default)
}

/// The one discovery the compatibility paths perform.
fn discover_for_collection(selected: &Path) -> Result<Workspace> {
    Workspace::discover(selected)
        .context("discovering the workspace for lifecycle contribution collection")
}

fn load_registry(selected: &Path, mode: WorldLoadMode) -> Result<LoadedRegistry> {
    let selected = super::super::install::resolve_project_root(selected)?;
    let workspace = discover_for_collection(&selected)?;
    load_registry_prepared(&selected, &workspace, mode)
}

/// Collect one node's effective world from an ALREADY-prepared workspace.
fn load_registry_prepared(
    selected: &Path,
    workspace: &Workspace,
    mode: WorldLoadMode,
) -> Result<LoadedRegistry> {
    let selected = selected.to_path_buf();
    let host_manifest = selected_manifest(workspace, &selected)?.clone();
    let manifest_kind = effective_manifest_kind(&host_manifest);
    let llm = host_manifest.llm.clone();
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
        dependency_sources(workspace, &host_manifest, &lock, mode)?
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
    let world = envelope_world(workspace, &installed);
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
        llm,
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

#[path = "world/dependencies.rs"]
mod dependencies;

#[cfg(test)]
use dependencies::dependency_source;
use dependencies::{dependency_sources, effective_stack};

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "world/prepared_tests.rs"]
mod prepared_tests;
