//! Neutral owner-runtime lowering over one [`ExtensionWorldEpoch`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use specmark::spec;
use vibe_core::manifest::{Manifest, MechanismRoutes};
use vibe_extension_registry::{
    DependencyProviderId, ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld,
    MechanismRegistry, RegistryRowIndex, SyntheticPresetSource,
};
use vibe_spec::TransformPlan;
use vibe_wire::generated::lifecycle::e1::context::{Project, World, WorldPackage};

use crate::{Workspace, WorkspaceError};

use super::{
    ExtensionWorldEpoch, OrderedResolutionIdentity, collect_owner_mechanisms, collect_owner_view,
};

#[cfg(test)]
thread_local! {
    static LOWERING_EVENTS: std::cell::RefCell<Option<Vec<OwnerRuntimeId>>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Portable identity of one lane owner. Absolute roots never enter it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OwnerRuntimeId {
    Node { rel: String },
    Unit { provider: DependencyProviderId },
}

impl std::fmt::Display for OwnerRuntimeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Node { rel } => write!(formatter, "node:{rel}"),
            Self::Unit { provider } => write!(formatter, "unit:{provider}"),
        }
    }
}

/// One owner’s single collected registry and every lowering derived from it.
#[derive(Debug)]
pub struct OwnerRuntime {
    id: OwnerRuntimeId,
    registry: ExtensionRegistry,
    compile_order: Box<[RegistryRowIndex]>,
    native_candidates: Box<[RegistryRowIndex]>,
    transform_plan: TransformPlan,
    mechanisms: MechanismRegistry,
    routes: MechanismRoutes,
}

impl OwnerRuntime {
    #[must_use]
    pub const fn id(&self) -> &OwnerRuntimeId {
        &self.id
    }

    #[must_use]
    pub const fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    #[must_use]
    pub const fn transform_plan(&self) -> &TransformPlan {
        &self.transform_plan
    }

    #[must_use]
    pub const fn mechanisms(&self) -> &MechanismRegistry {
        &self.mechanisms
    }

    #[must_use]
    pub const fn routes(&self) -> &MechanismRoutes {
        &self.routes
    }

    #[must_use]
    pub fn compile_order(&self) -> &[RegistryRowIndex] {
        &self.compile_order
    }

    #[must_use]
    pub fn native_candidate_indices(&self) -> &[RegistryRowIndex] {
        &self.native_candidates
    }

    /// Whether this exact owner plan can execute through the native compiler
    /// manager. Native rows outside the compile order (for example phase-only
    /// handlers) do not count.
    pub(crate) fn has_compiler_native_intersection(&self) -> Result<bool, WorkspaceError> {
        let rows = self.rows()?;
        Ok(rows.compile().iter().any(|compile| {
            rows.native()
                .iter()
                .any(|native| std::ptr::eq(*compile, *native))
        }))
    }

    /// Project temporary borrowed slices from the same immutable registry.
    pub fn rows(&self) -> Result<OwnerRuntimeRows<'_>, WorkspaceError> {
        Ok(OwnerRuntimeRows {
            compile: project_rows(self, &self.compile_order, "compile")?,
            native: project_rows(self, &self.native_candidates, "native")?,
        })
    }
}

/// Temporary row guards. Both slices borrow one [`OwnerRuntime::registry`].
pub struct OwnerRuntimeRows<'runtime> {
    compile: Vec<&'runtime ExtensionRegistryRow>,
    native: Vec<&'runtime ExtensionRegistryRow>,
}

impl<'runtime> OwnerRuntimeRows<'runtime> {
    #[must_use]
    pub fn compile(&self) -> &[&'runtime ExtensionRegistryRow] {
        &self.compile
    }

    #[must_use]
    pub fn native(&self) -> &[&'runtime ExtensionRegistryRow] {
        &self.native
    }
}

fn project_rows<'runtime>(
    runtime: &'runtime OwnerRuntime,
    indices: &[RegistryRowIndex],
    family: &'static str,
) -> Result<Vec<&'runtime ExtensionRegistryRow>, WorkspaceError> {
    indices
        .iter()
        .map(|index| {
            runtime
                .registry
                .row_at(index)
                .ok_or_else(|| WorkspaceError::OwnerRuntimeIndex {
                    owner: runtime.id.to_string(),
                    family,
                })
        })
        .collect()
}

/// Explicit inputs to owner lowering. There is intentionally no `Default`.
pub struct OwnerRuntimeLowering {
    selected_node: String,
    node_presets: BTreeMap<String, Vec<SyntheticPresetSource>>,
}

impl OwnerRuntimeLowering {
    #[must_use]
    pub fn new(
        selected_node: impl Into<String>,
        node_presets: BTreeMap<String, Vec<SyntheticPresetSource>>,
    ) -> Self {
        Self {
            selected_node: selected_node.into(),
            node_presets,
        }
    }

    /// Compatibility input for APIs that historically selected no member and
    /// admitted no package-skill presets. It is explicit at every call site.
    #[must_use]
    pub fn compatibility_root_without_presets() -> Self {
        Self::new(".", BTreeMap::new())
    }
}

/// Every neutral owner runtime plus the selected request facts observed once.
#[derive(Debug)]
pub struct LoweredOwnerRuntimes {
    selected_node: String,
    workspace_root: PathBuf,
    selected_root: PathBuf,
    project: Project,
    world: World,
    nodes: BTreeMap<String, OwnerRuntime>,
    units: BTreeMap<DependencyProviderId, OwnerRuntime>,
    resolution_identity: OrderedResolutionIdentity,
}

impl LoweredOwnerRuntimes {
    #[must_use]
    pub fn selected_node(&self) -> &str {
        &self.selected_node
    }

    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn selected_root(&self) -> &Path {
        &self.selected_root
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub const fn nodes(&self) -> &BTreeMap<String, OwnerRuntime> {
        &self.nodes
    }

    #[must_use]
    pub const fn units(&self) -> &BTreeMap<DependencyProviderId, OwnerRuntime> {
        &self.units
    }

    pub fn node(&self, rel: &str) -> Result<&OwnerRuntime, WorkspaceError> {
        self.nodes
            .get(rel)
            .ok_or_else(|| WorkspaceError::UnknownRuntimeNode {
                rel: rel.to_owned(),
                role: "requested",
            })
    }

    pub fn unit(&self, provider: &DependencyProviderId) -> Result<&OwnerRuntime, WorkspaceError> {
        self.units
            .get(provider)
            .ok_or_else(|| WorkspaceError::UnknownRuntimeUnit {
                provider: provider.to_string(),
            })
    }

    /// Bind injected run facts by move. No world is reread or recollected.
    #[must_use]
    pub fn bind_run(self, run: OwnerRuntimeRunFacts) -> OwnerRuntimeEpoch {
        OwnerRuntimeEpoch {
            lowered: self,
            run,
            replay_identity: Arc::new(()),
        }
    }
}

/// Run-shaped facts owned by INSTALL later. This slice only stores them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRuntimeRunFacts {
    pub run_id: String,
    pub state_root: PathBuf,
    pub platform: String,
    pub offline: bool,
    pub created_at: String,
}

/// Bound owner-runtime epoch transported by later INSTALL work.
#[derive(Debug)]
pub struct OwnerRuntimeEpoch {
    lowered: LoweredOwnerRuntimes,
    run: OwnerRuntimeRunFacts,
    replay_identity: Arc<()>,
}

pub(crate) struct OwnerRuntimeEpochToken(Arc<()>);

impl OwnerRuntimeEpoch {
    #[must_use]
    pub const fn lowered(&self) -> &LoweredOwnerRuntimes {
        &self.lowered
    }

    #[must_use]
    pub const fn run(&self) -> &OwnerRuntimeRunFacts {
        &self.run
    }

    pub(crate) fn replay_token(&self) -> OwnerRuntimeEpochToken {
        OwnerRuntimeEpochToken(Arc::clone(&self.replay_identity))
    }

    pub(crate) fn matches_replay_token(&self, token: &OwnerRuntimeEpochToken) -> bool {
        Arc::ptr_eq(&self.replay_identity, &token.0)
    }

    /// Refuse an explicit composition resolution that is not the exact world
    /// from which these retained owner runtimes were lowered.
    pub(crate) fn assert_resolution(
        &self,
        workspace_root: &Path,
        resolution: &[crate::install::ResolvedDep],
    ) -> Result<(), WorkspaceError> {
        let supplied = ExtensionWorldEpoch::from_resolution(workspace_root, resolution)
            .map_err(world_error)?;
        if supplied.resolution_identity() != &self.lowered.resolution_identity {
            return Err(WorkspaceError::OwnerRuntimeResolutionMismatch);
        }
        Ok(())
    }

    pub fn selected(&self) -> Result<OwnerRuntimeView<'_>, WorkspaceError> {
        self.node(&self.lowered.selected_node)
    }

    pub fn node(&self, rel: &str) -> Result<OwnerRuntimeView<'_>, WorkspaceError> {
        let runtime = self.lowered.node(rel)?;
        Ok(OwnerRuntimeView {
            runtime,
            epoch: self,
        })
    }

    pub fn unit(
        &self,
        provider: &DependencyProviderId,
    ) -> Result<OwnerRuntimeView<'_>, WorkspaceError> {
        let runtime = self.lowered.unit(provider)?;
        Ok(OwnerRuntimeView {
            runtime,
            epoch: self,
        })
    }
}

/// One owner logically paired with the physically common request/run facts.
pub struct OwnerRuntimeView<'epoch> {
    runtime: &'epoch OwnerRuntime,
    epoch: &'epoch OwnerRuntimeEpoch,
}

impl<'epoch> OwnerRuntimeView<'epoch> {
    #[must_use]
    pub const fn runtime(&self) -> &'epoch OwnerRuntime {
        self.runtime
    }

    #[must_use]
    pub const fn project(&self) -> &'epoch Project {
        &self.epoch.lowered.project
    }

    #[must_use]
    pub const fn world(&self) -> &'epoch World {
        &self.epoch.lowered.world
    }

    #[must_use]
    pub const fn run(&self) -> &'epoch OwnerRuntimeRunFacts {
        &self.epoch.run
    }

    #[must_use]
    pub fn selected_root(&self) -> &'epoch Path {
        &self.epoch.lowered.selected_root
    }

    #[must_use]
    pub fn workspace_root(&self) -> &'epoch Path {
        &self.epoch.lowered.workspace_root
    }
}

/// Lower every node and package unit exactly once from one extension-world epoch.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
pub fn lower_owner_runtimes(
    workspace: &Workspace,
    epoch: &ExtensionWorldEpoch,
    lowering: OwnerRuntimeLowering,
) -> Result<LoweredOwnerRuntimes, WorkspaceError> {
    let OwnerRuntimeLowering {
        selected_node,
        mut node_presets,
    } = lowering;
    let mut nodes = BTreeMap::new();
    let mut selected_facts = None;

    for (rel, manifest) in workspace.iter_nodes() {
        let root = workspace.node_abs_path(rel);
        let view = epoch
            .node_owner_view(&root, manifest)
            .map_err(world_error)?;
        if rel == selected_node {
            selected_facts = Some(request_facts(workspace, &view));
        }
        let presets = node_presets.remove(rel).unwrap_or_default();
        let runtime = lower_owner(
            OwnerRuntimeId::Node {
                rel: rel.to_owned(),
            },
            view,
            manifest.mechanism_routes.clone(),
            presets,
        )?;
        nodes.insert(rel.to_owned(), runtime);
    }

    if let Some((rel, _)) = node_presets.into_iter().next() {
        return Err(WorkspaceError::UnknownRuntimeNode {
            rel,
            role: "preset",
        });
    }
    let Some((selected_root, project, world)) = selected_facts else {
        return Err(WorkspaceError::UnknownRuntimeNode {
            rel: selected_node,
            role: "selected",
        });
    };

    let mut owners = epoch.lane_owners().cloned().collect::<Vec<_>>();
    owners.sort();
    let mut units = BTreeMap::new();
    for owner in owners {
        let view = epoch.package_owner_view(&owner).map_err(world_error)?;
        let routes = epoch
            .package_manifest(&owner)
            .map_err(world_error)?
            .mechanism_routes
            .clone();
        let runtime = lower_owner(
            OwnerRuntimeId::Unit {
                provider: owner.clone(),
            },
            view,
            routes,
            Vec::new(),
        )?;
        units.insert(owner, runtime);
    }

    Ok(LoweredOwnerRuntimes {
        selected_node,
        workspace_root: workspace.root.clone(),
        selected_root,
        project,
        world,
        nodes,
        units,
        resolution_identity: epoch.resolution_identity().clone(),
    })
}

fn lower_owner(
    id: OwnerRuntimeId,
    view: ExtensionWorld,
    routes: MechanismRoutes,
    presets: Vec<SyntheticPresetSource>,
) -> Result<OwnerRuntime, WorkspaceError> {
    let mechanisms = collect_owner_mechanisms(&view).map_err(world_error)?;
    let registry = collect_owner_view(view, presets).map_err(world_error)?;
    let compile_order = registry.enabled_compile_indices();
    let compile_rows = compile_order
        .iter()
        .map(|index| {
            registry
                .row_at(index)
                .ok_or_else(|| WorkspaceError::OwnerRuntimeIndex {
                    owner: id.to_string(),
                    family: "compile",
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let transform_plan = TransformPlan::from_effective_rows(&compile_rows).map_err(|source| {
        WorkspaceError::TransformPlan {
            owner: id.to_string(),
            source,
        }
    })?;
    let native_candidates = registry.enabled_native_indices();
    let runtime = OwnerRuntime {
        id,
        registry,
        compile_order: compile_order.into_boxed_slice(),
        native_candidates: native_candidates.into_boxed_slice(),
        transform_plan,
        mechanisms,
        routes,
    };
    #[cfg(test)]
    LOWERING_EVENTS.with(|events| {
        if let Some(events) = events.borrow_mut().as_mut() {
            events.push(runtime.id.clone());
        }
    });
    Ok(runtime)
}

#[cfg(test)]
pub(super) fn observe_lowerings<T>(run: impl FnOnce() -> T) -> (T, Vec<OwnerRuntimeId>) {
    struct ObservationGuard;
    impl Drop for ObservationGuard {
        fn drop(&mut self) {
            LOWERING_EVENTS.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }

    LOWERING_EVENTS.with(|events| {
        assert!(events.borrow().is_none(), "lowering observer is not nested");
        *events.borrow_mut() = Some(Vec::new());
    });
    let _guard = ObservationGuard;
    let output = run();
    let events = LOWERING_EVENTS.with(|slot| slot.borrow_mut().take().unwrap_or_default());
    (output, events)
}

fn request_facts(workspace: &Workspace, view: &ExtensionWorld) -> (PathBuf, Project, World) {
    let provider = &view.host.provider;
    let selected_root = provider.root.clone();
    let (name, kind) = match &provider.identity {
        vibe_extension_registry::HostIdentity::UngroupedProject(name) => {
            (name.clone(), "project".to_owned())
        }
        vibe_extension_registry::HostIdentity::Coordinate(identity) => (
            identity.name().to_string(),
            provider
                .kind
                .map_or_else(|| "project".to_owned(), |kind| kind.as_str().to_owned()),
        ),
        vibe_extension_registry::HostIdentity::VirtualWorkspace => {
            ("<virtual-workspace>".to_owned(), "workspace".to_owned())
        }
    };
    let project = Project {
        kind,
        manifest: vibe_core::machine_json_path(&selected_root.join(Manifest::FILENAME)),
        name,
        root: vibe_core::machine_json_path(&selected_root),
        spec_roots: vec![vibe_core::machine_json_path(
            &selected_root.join(vibe_core::layout::current_specs_root()),
        )],
        version: provider.version.clone(),
    };
    let world = World {
        deps_root: vibe_core::machine_json_path(&workspace.vibedeps_root()),
        lockfile: vibe_core::machine_json_path(&workspace.lockfile_path()),
        packages: view
            .installed
            .iter()
            .map(|source| WorldPackage {
                group: source.provider.id.group().to_string(),
                kind: source.provider.kind.as_str().to_owned(),
                name: source.provider.id.name().to_string(),
                slot: vibe_core::machine_json_path(&source.provider.root),
                version: source.provider.version.clone(),
            })
            .collect(),
    };
    (selected_root, project, world)
}

fn world_error(source: super::ExtensionWorldError) -> WorkspaceError {
    WorkspaceError::ExtensionWorld {
        source: Box::new(source),
    }
}
