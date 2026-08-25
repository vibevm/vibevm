use std::path::Path;

use specmark::spec;
use vibe_core::PackageName;
use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_core::manifest::{ExtensionDecl, ExtensionHandler};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProviderId, ExtensionProvider, ExtensionWorld,
    HandlerExecution, HostExtensionSource, SelectorSubject, SlotTarget, SyntheticHookIdentity,
    collect_extensions,
};
use vibe_workspace::install::ResolvedDep;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub struct SlotLifecyclePlanEntry {
    pub key: String,
    pub reference: String,
    pub point: String,
    pub provider: String,
    pub handler: String,
    pub tier: String,
    pub version: Option<String>,
    pub slot_target: SlotTarget,
    pub(super) execution: HandlerExecution,
}

#[derive(Debug, Clone, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub struct SlotLifecyclePlan {
    pub entries: Vec<SlotLifecyclePlanEntry>,
}

impl SlotLifecyclePlan {
    pub(super) fn for_targets(
        &self,
        targets: &[vibe_workspace::install::SlotLifecycleTarget],
    ) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .filter(|entry| {
                    targets.iter().any(|target| {
                        entry.slot_target.group == target.group.to_string()
                            && entry.slot_target.name == target.name
                            && entry.slot_target.version == target.version.to_string()
                    })
                })
                .cloned()
                .collect(),
        }
    }
}

/// Presentation-neutral pre-dispatch ritual observer.
///
/// ```
/// use vibe_install::{SlotLifecycleObserver, SlotLifecyclePlan};
/// struct Observer;
/// impl SlotLifecycleObserver for Observer {
///     fn observe(&self, plan: &SlotLifecyclePlan) -> Result<(), String> {
///         assert!(plan.entries.iter().all(|entry| entry.point.starts_with("slot:")));
///         Ok(())
///     }
/// }
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub trait SlotLifecycleObserver {
    fn observe(&self, plan: &SlotLifecyclePlan) -> std::result::Result<(), String>;

    fn outcome(&self, _report: &super::SlotLifecycleReport) -> std::result::Result<(), String> {
        Ok(())
    }
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct NoSlotLifecycleObserver;
impl SlotLifecycleObserver for NoSlotLifecycleObserver {
    fn observe(&self, _plan: &SlotLifecyclePlan) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub(super) fn build_slot_plan(
    installed: &[DependencyExtensionSource],
    host: &HostExtensionSource,
    resolution: &[ResolvedDep],
) -> Result<SlotLifecyclePlan> {
    let mut entries = Vec::new();
    for point in [SlotPoint::PreInstall, SlotPoint::PostInstall] {
        for target_dep in resolution {
            append_target_plan(&mut entries, installed, host, target_dep, point)?;
        }
    }
    Ok(SlotLifecyclePlan { entries })
}

fn append_target_plan(
    entries: &mut Vec<SlotLifecyclePlanEntry>,
    installed: &[DependencyExtensionSource],
    host: &HostExtensionSource,
    target_dep: &ResolvedDep,
    point: SlotPoint,
) -> Result<()> {
    let target_id = DependencyProviderId::new(
        target_dep.group.clone(),
        PackageName::parse(&target_dep.name)?,
    );
    let mut event_world = installed.to_vec();
    let target_source = event_world
        .iter_mut()
        .find(|source| source.provider.id == target_id)
        .ok_or_else(|| {
            Error::Lifecycle(format!(
                "planned slot target `{target_id}` is absent from provisional world"
            ))
        })?;
    if let Some(base) = match point {
        SlotPoint::PreInstall => target_dep.manifest.hooks.pre_install.clone(),
        SlotPoint::PostInstall => target_dep.manifest.hooks.post_install.clone(),
    } {
        target_source.declarations.push(ExtensionDecl {
            id: SyntheticHookIdentity::from(point).id().into(),
            point: ExtensionPoint::Slot(point),
            handler: ExtensionHandler::Script { base },
            config: None,
            auto: None,
            inputs: None,
            applies_to: None,
            compiler_internals: None,
            pass: None,
            when: None,
        });
    }
    let target = slot_target(target_dep, &target_source.provider.root);
    let registry = collect_extensions(ExtensionWorld {
        installed: event_world,
        host: host.clone(),
        effective_stack: None,
    })
    .map_err(|error| Error::Lifecycle(error.to_string()))?;
    for row in registry.plan(
        ExtensionPoint::Slot(point),
        SelectorSubject::package(&target_id),
    ) {
        let execution = HandlerExecution::from_row(row).with_slot_target(target.clone());
        entries.push(SlotLifecyclePlanEntry {
            key: execution.key(),
            reference: execution.reference(),
            point: row.declaration().point.to_string(),
            provider: row.provider().to_string(),
            handler: row.declaration().handler.kind().to_string(),
            tier: tier_name(row.effective_tier()).into(),
            version: provider_version(row.provider()),
            slot_target: target.clone(),
            execution,
        });
    }
    Ok(())
}

fn slot_target(dep: &ResolvedDep, root: &Path) -> SlotTarget {
    SlotTarget {
        group: dep.group.to_string(),
        name: dep.name.clone(),
        version: dep.version.to_string(),
        kind: dep.kind.to_string(),
        root: vibe_core::machine_json_path(root),
    }
}

fn tier_name(tier: vibe_lifecycle::ContributionTier) -> &'static str {
    match tier {
        vibe_lifecycle::ContributionTier::Dependency => "dependency",
        vibe_lifecycle::ContributionTier::Preset => "preset",
        vibe_lifecycle::ContributionTier::HostDeclaration => "host-declaration",
        vibe_lifecycle::ContributionTier::HostActivation => "host-activation",
    }
}

fn provider_version(provider: &ExtensionProvider) -> Option<String> {
    match provider {
        ExtensionProvider::Dependency(provider) => Some(provider.version.clone()),
        ExtensionProvider::Host(provider) => {
            (!provider.version.is_empty()).then(|| provider.version.clone())
        }
    }
}
