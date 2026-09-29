//! Named composition, closure and admission facts; no wire or effect engine.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-ROLE-SEPARATION");
use super::*;
use crate::{ContentHash, PackageRef};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredRoot {
    pub name: Reference,
    pub package: PackageRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceSelection {
    pub name: Reference,
    pub value: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBinding {
    pub name: Reference,
    pub platform: PlatformFacts,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedExport {
    pub name: Reference,
    pub package_root: Reference,
    pub runtime_binding: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionComposition {
    pub roots: Vec<DesiredRoot>,
    pub features: Vec<Reference>,
    pub choices: Vec<ChoiceSelection>,
    pub target_constraints: PlatformConstraints,
    pub runtime_bindings: Vec<RuntimeBinding>,
    pub tool_exports: Vec<NamedExport>,
    pub service_exports: Vec<NamedExport>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDefinition {
    id: DefinitionId,
    composition: DefinitionComposition,
}
impl EnvironmentDefinition {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-DEFINITION",
        r = 1
    )]
    pub fn new(
        id: DefinitionId,
        composition: DefinitionComposition,
    ) -> Result<Self, EnvironmentError> {
        unique(composition.roots.iter().map(|r| r.name.clone()), "roots")?;
        unique(composition.features.clone(), "features")?;
        unique(
            composition.choices.iter().map(|r| r.name.clone()),
            "choices",
        )?;
        unique(
            composition.runtime_bindings.iter().map(|r| r.name.clone()),
            "runtime_bindings",
        )?;
        unique(
            composition.tool_exports.iter().map(|r| r.name.clone()),
            "tool_exports",
        )?;
        unique(
            composition.service_exports.iter().map(|r| r.name.clone()),
            "service_exports",
        )?;
        for export in composition
            .tool_exports
            .iter()
            .chain(&composition.service_exports)
        {
            if !composition
                .roots
                .iter()
                .any(|r| r.name == export.package_root)
                || !composition
                    .runtime_bindings
                    .iter()
                    .any(|r| r.name == export.runtime_binding)
            {
                return Err(invalid(
                    "exports",
                    "export references an undeclared package root or runtime binding",
                ));
            }
        }
        Ok(Self { id, composition })
    }
    pub fn id(&self) -> &DefinitionId {
        &self.id
    }
    pub fn composition(&self) -> &DefinitionComposition {
        &self.composition
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyRole {
    Build,
    Runtime,
    BuildAndRuntime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedPackageFacts {
    pub root: Option<Reference>,
    pub package: PackageRef,
    pub version: semver::Version,
    pub content: ContentHash,
    pub capture: Reference,
    pub configuration: Reference,
    pub role: DependencyRole,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedPackage {
    facts: SelectedPackageFacts,
}
impl SelectedPackage {
    pub fn new(facts: SelectedPackageFacts) -> Result<Self, EnvironmentError> {
        if facts.package.kind.is_none()
            || facts.package.group.is_none()
            || !facts.package.version.is_exact_pin()
            || !facts.package.version.matches(&facts.version)
        {
            return Err(invalid(
                "selected_package",
                "selected package must be fully qualified with its exact selected version",
            ));
        }
        Ok(Self { facts })
    }
    pub fn facts(&self) -> &SelectedPackageFacts {
        &self.facts
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalBackendClosure {
    pub backend: Reference,
    pub closure: Reference,
    pub configuration: Reference,
    pub role: DependencyRole,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureMember {
    PackageCapture(Reference),
    External {
        backend: Reference,
        closure: Reference,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockClosure {
    pub definition: DefinitionId,
    pub packages: Vec<SelectedPackage>,
    pub external_backends: Vec<ExternalBackendClosure>,
    pub target_constraints: PlatformConstraints,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentLock {
    id: LockId,
    closure: LockClosure,
}
impl EnvironmentLock {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LOCK",
        r = 1
    )]
    pub fn new(
        id: LockId,
        definition: &EnvironmentDefinition,
        closure: LockClosure,
    ) -> Result<Self, EnvironmentError> {
        if &closure.definition != definition.id()
            || closure.target_constraints != definition.composition.target_constraints
        {
            return Err(invalid(
                "lock.definition",
                "lock definition or target constraints differ from declared composition",
            ));
        }
        unique(
            closure.packages.iter().filter_map(|p| p.facts.root.clone()),
            "lock.roots",
        )?;
        let mut captures = BTreeMap::new();
        for p in &closure.packages {
            let p = &p.facts;
            if let Some(root) = &p.root {
                let wanted = definition
                    .composition
                    .roots
                    .iter()
                    .find(|r| &r.name == root)
                    .ok_or_else(|| invalid("lock.roots", "selected root is not declared"))?;
                if wanted
                    .package
                    .group
                    .as_ref()
                    .is_some_and(|group| Some(group) != p.package.group.as_ref())
                    || wanted.package.name != p.package.name
                    || wanted
                        .package
                        .kind
                        .is_some_and(|kind| Some(kind) != p.package.kind)
                    || !wanted.package.version.matches(&p.version)
                {
                    return Err(invalid(
                        "lock.roots",
                        "selected package does not satisfy the named root selection",
                    ));
                }
            }
            let value = (&p.package, &p.version, &p.content, &p.configuration);
            if let Some(prior) = captures.insert(&p.capture, value)
                && prior != value
            {
                return Err(invalid(
                    "lock.captures",
                    "one capture reference names conflicting package facts",
                ));
            }
        }
        for root in &definition.composition.roots {
            if !closure
                .packages
                .iter()
                .any(|p| p.facts.root.as_ref() == Some(&root.name))
            {
                return Err(invalid(
                    "lock.roots",
                    "declared root has no selected package",
                ));
            }
        }
        let mut external = BTreeSet::new();
        for b in &closure.external_backends {
            if !external.insert((&b.backend, &b.closure)) {
                return Err(invalid(
                    "lock.external_backends",
                    "duplicate external closure member",
                ));
            }
        }
        Ok(Self { id, closure })
    }
    pub fn id(&self) -> &LockId {
        &self.id
    }
    pub fn closure(&self) -> &LockClosure {
        &self.closure
    }
    pub fn members(&self) -> BTreeSet<ClosureMember> {
        self.closure
            .packages
            .iter()
            .map(|p| ClosureMember::PackageCapture(p.facts.capture.clone()))
            .chain(
                self.closure
                    .external_backends
                    .iter()
                    .map(|e| ClosureMember::External {
                        backend: e.backend.clone(),
                        closure: e.closure.clone(),
                    }),
            )
            .collect()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountAccess {
    ReadOnly,
    ReadWrite,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountCapability {
    pub name: Reference,
    pub root_reference: Reference,
    pub access: MountAccess,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorFacts {
    pub platform: PlatformFacts,
    pub tool: Reference,
    pub provider: Reference,
    pub principal: Reference,
    pub network_capabilities: Vec<Reference>,
    pub mounts: Vec<MountCapability>,
    pub process_capabilities: Vec<Reference>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    id: ExecutionId,
    facts: ExecutorFacts,
}
impl ExecutionContext {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#EXECUTION-CONTEXT",
        r = 1
    )]
    pub fn new(id: ExecutionId, facts: ExecutorFacts) -> Result<Self, EnvironmentError> {
        unique(
            facts.network_capabilities.clone(),
            "execution.network_capabilities",
        )?;
        unique(
            facts.mounts.iter().map(|m| m.name.clone()),
            "execution.mounts",
        )?;
        unique(
            facts.process_capabilities.clone(),
            "execution.process_capabilities",
        )?;
        Ok(Self { id, facts })
    }
    pub fn id(&self) -> &ExecutionId {
        &self.id
    }
    pub fn facts(&self) -> &ExecutorFacts {
        &self.facts
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    WorkspaceProjection,
    UserProfile,
    OfflineLinuxRootfs,
    LiveLinuxSystem,
}
/// Supplied target-path descriptions, not a physical capability or path parser.
/// The named path-policy/storage owner must validate them before effects. In
/// particular Linux prefixes are not parsed using a Windows executor's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRoots {
    filesystem_root: String,
    prefix: String,
    home: Option<String>,
    path_policy_owner: Reference,
    physical_evidence: Option<Reference>,
}
impl TargetRoots {
    pub fn new(
        filesystem_root: impl Into<String>,
        prefix: impl Into<String>,
        home: Option<String>,
        path_policy_owner: Reference,
        physical_evidence: Option<Reference>,
    ) -> Result<Self, EnvironmentError> {
        let filesystem_root = filesystem_root.into();
        let prefix = prefix.into();
        if filesystem_root.trim().is_empty()
            || prefix.trim().is_empty()
            || home.as_ref().is_some_and(|v| v.trim().is_empty())
        {
            return Err(invalid(
                "target.roots",
                "supplied path descriptions must be nonblank",
            ));
        }
        Ok(Self {
            filesystem_root,
            prefix,
            home,
            path_policy_owner,
            physical_evidence,
        })
    }
    pub fn filesystem_root(&self) -> &str {
        &self.filesystem_root
    }
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
    pub fn home(&self) -> Option<&str> {
        self.home.as_deref()
    }
    pub fn path_policy_owner(&self) -> &Reference {
        &self.path_policy_owner
    }
    pub fn physical_evidence(&self) -> Option<&Reference> {
        self.physical_evidence.as_ref()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetFacts {
    pub definition: DefinitionId,
    pub lock: LockId,
    pub generation: Option<GenerationId>,
    pub mode: TargetMode,
    pub platform: PlatformFacts,
    pub roots: TargetRoots,
    pub principal: Reference,
    pub storage_backend: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetBinding {
    instance: TargetInstanceId,
    facts: TargetFacts,
}
impl TargetBinding {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-BINDING",
        r = 1
    )]
    pub fn new(
        instance: TargetInstanceId,
        definition: &EnvironmentDefinition,
        lock: &EnvironmentLock,
        generation: Option<&EnvironmentGeneration>,
        facts: TargetFacts,
    ) -> Result<Self, EnvironmentError> {
        if &facts.definition != definition.id()
            || &facts.lock != lock.id()
            || &lock.closure.definition != definition.id()
        {
            return Err(invalid(
                "target.binding",
                "definition and lock references do not join",
            ));
        }
        if facts.generation.as_ref() != generation.map(EnvironmentGeneration::id)
            || generation.is_some_and(|g| &g.inventory.lock != lock.id())
        {
            return Err(invalid(
                "target.generation",
                "generation is unestablished or belongs to another lock",
            ));
        }
        if matches!(
            facts.mode,
            TargetMode::OfflineLinuxRootfs | TargetMode::LiveLinuxSystem
        ) && matches!(facts.platform.os(),Fact::Known(os) if *os!=crate::manifest::TargetOs::Linux)
        {
            return Err(invalid(
                "target.platform.os",
                "Linux target mode cannot describe another known OS",
            ));
        }
        Ok(Self { instance, facts })
    }
    pub fn instance(&self) -> &TargetInstanceId {
        &self.instance
    }
    pub fn facts(&self) -> &TargetFacts {
        &self.facts
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadMetadata {
    pub name: Reference,
    pub package_capture: Reference,
    pub payload_reference: Reference,
    pub metadata_reference: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationInventory {
    pub lock: LockId,
    pub recipes: Vec<Reference>,
    pub payloads: Vec<PayloadMetadata>,
    pub dependency_closure: Vec<ClosureMember>,
    pub boot_adapters: Vec<Reference>,
    pub service_catalog: Vec<Reference>,
    pub activation_links: Vec<Reference>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentGeneration {
    id: GenerationId,
    inventory: GenerationInventory,
}
impl EnvironmentGeneration {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-GENERATION",
        r = 1
    )]
    pub fn new(
        id: GenerationId,
        lock: &EnvironmentLock,
        inventory: GenerationInventory,
    ) -> Result<Self, EnvironmentError> {
        if &inventory.lock != lock.id() {
            return Err(invalid(
                "generation.lock",
                "generation references another lock",
            ));
        }
        if inventory.recipes.is_empty() {
            return Err(invalid(
                "generation.recipes",
                "realization must name its producing recipe",
            ));
        }
        unique(inventory.recipes.clone(), "generation.recipes")?;
        unique(
            inventory.payloads.iter().map(|p| p.name.clone()),
            "generation.payloads",
        )?;
        let members = lock.members();
        let declared = inventory
            .dependency_closure
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if declared != members || declared.len() != inventory.dependency_closure.len() {
            return Err(invalid(
                "generation.closure",
                "realized closure does not match selected lock",
            ));
        }
        for payload in &inventory.payloads {
            if !members.contains(&ClosureMember::PackageCapture(
                payload.package_capture.clone(),
            )) {
                return Err(invalid(
                    "generation.payloads",
                    "payload references an unselected package capture",
                ));
            }
        }
        unique(inventory.boot_adapters.clone(), "generation.boot_adapters")?;
        unique(
            inventory.service_catalog.clone(),
            "generation.service_catalog",
        )?;
        unique(
            inventory.activation_links.clone(),
            "generation.activation_links",
        )?;
        Ok(Self { id, inventory })
    }
    pub fn id(&self) -> &GenerationId {
        &self.id
    }
    pub fn inventory(&self) -> &GenerationInventory {
        &self.inventory
    }
}
