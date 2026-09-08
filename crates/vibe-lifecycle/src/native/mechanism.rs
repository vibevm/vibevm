//! Selected deploy-native mechanism artifact planning and preparation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vibe_core::manifest::{
    ArtifactPackageTarget, DeployTarget, ExtensionHandler, MechanismKey, MechanismRoutes,
};
use vibe_extension_registry::{
    MechanismRegistry, MechanismRegistryRow, SelectionStep, resolve_mechanism,
};
use vibe_native_loader::{NativeLoadError, NativeMechanism};

use super::cargo::build_cdylib;
use super::path::{
    VerifiedFile, prebuilt_file, publish_load_image, relative_spelling, source_crate,
};
use super::provider::{ProviderFacts, ProviderHome, mechanism_facts};
use super::record::{
    SourceRecordExpectation, SourceRecordInputs, record_path, revalidate_source_record,
    write_source_record,
};
use super::witness::{mechanism_config_witness, record_id, source_witness};
use super::{
    NativeArtifactError, NativeArtifactOrigin, NativeArtifactRecordRoot, NativeBuildExecution,
    NativePlatform, prepare_dependency_ignore, select_build_provider,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMechanismBinding {
    pub target: String,
    pub key: MechanismKey,
    pub pin: String,
    pub descriptor_id: String,
    pub protocol: u32,
    pub via: SelectionStep,
    pub displaced_default: Option<String>,
}

#[derive(Debug, Clone)]
struct PlannedMechanism {
    row: MechanismRegistryRow,
    bindings: Vec<NativeMechanismBinding>,
}

#[derive(Debug, Clone, Default)]
pub struct NativeMechanismPlan {
    entries: Vec<PlannedMechanism>,
}

impl NativeMechanismPlan {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.entries.iter().map(|entry| entry.bindings.len()).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMechanismArtifactClaim {
    Source {
        provider: String,
        provider_root: PathBuf,
        crate_dir: String,
    },
    Prebuilt {
        provider: String,
        relative: String,
    },
}

pub struct NativeMechanismPreflight {
    plan: NativeMechanismPlan,
    artifacts: Vec<PreflightArtifact>,
    groups: Vec<SourceGroup>,
    build_provider: Option<String>,
    claims: Vec<NativeMechanismArtifactClaim>,
}

struct PreflightArtifact {
    provider: ProviderFacts,
    state: ArtifactState,
}

enum ArtifactState {
    Prebuilt(VerifiedFile),
    Source,
}

struct SourceGroup {
    provider: ProviderFacts,
    crate_dir: PathBuf,
    crate_wire: String,
    entries: Vec<usize>,
    config: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedNativeMechanism {
    pub bindings: Vec<NativeMechanismBinding>,
    pub provider: String,
    pub provider_version: String,
    pub provider_hash: Option<String>,
    pub provider_root: PathBuf,
    pub record_root: NativeArtifactRecordRoot,
    pub platform: NativePlatform,
    pub origin: NativeArtifactOrigin,
    pub record: Option<String>,
    pub image: PathBuf,
    pub digest: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedNativeMechanisms {
    pub entries: Vec<PreparedNativeMechanism>,
}

pub fn project_native_mechanisms(
    package: &[ArtifactPackageTarget],
    deploy: &[DeployTarget],
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<NativeMechanismPlan, NativeArtifactError> {
    for target in package {
        select(
            target.mechanism.clone(),
            target.provider.as_ref(),
            registry,
            routes,
        )?;
    }
    let mut plan = NativeMechanismPlan::default();
    let mut selected_pins = BTreeMap::<MechanismKey, String>::new();
    for target in deploy {
        let selected = select(
            target.mechanism.clone(),
            target.provider.as_ref(),
            registry,
            routes,
        )?;
        let row = selected.row();
        let pin = row.pin().to_string();
        if let Some(previous) = selected_pins.insert(target.mechanism.clone(), pin.clone())
            && previous != pin
        {
            return Err(selection_error(
                "active deploy targets select different exact provider pins for one logical mechanism key",
            ));
        }
        if row.is_builtin() || !matches!(row.handler(), ExtensionHandler::Native { .. }) {
            continue;
        }
        let binding = NativeMechanismBinding {
            target: target.id.clone(),
            key: target.mechanism.clone(),
            pin: pin.clone(),
            descriptor_id: row.declaration().id.clone(),
            protocol: row.protocol(),
            via: selected.via(),
            displaced_default: selected
                .displaced_default()
                .map(|default| default.pin().to_string()),
        };
        if let Some(existing) = plan
            .entries
            .iter_mut()
            .find(|entry| entry.row.pin().to_string() == pin)
        {
            if existing.row.key() != row.key()
                || existing.row.handler() != row.handler()
                || existing.row.declaration().id != row.declaration().id
            {
                return Err(selection_error(
                    "one exact pin resolved to conflicting rows",
                ));
            }
            existing.bindings.push(binding);
        } else {
            plan.entries.push(PlannedMechanism {
                row: row.clone(),
                bindings: vec![binding],
            });
        }
    }
    Ok(plan)
}

impl PreparedNativeMechanism {
    /// Admit one binding from this already-prepared immutable image.
    ///
    /// The process loader is the existing ABI-1 cache. This path performs no
    /// selection, build, artifact-record read, or image publication.
    pub(crate) fn admit(
        &self,
        binding: &NativeMechanismBinding,
    ) -> Result<NativeMechanism, NativeLoadError> {
        super::process_loader().admit_mechanism(
            &self.image,
            &binding.pin,
            &binding.descriptor_id,
            &binding.key,
        )
    }
}

impl PreparedNativeMechanisms {
    /// Execute the selected deploy set through this process's prepared images.
    pub fn execute_deploy_targets(
        &self,
        execution: &crate::DeployExecution<'_>,
    ) -> Result<Vec<crate::DeployOutcome>, crate::DeployError> {
        crate::mechanism::deploy::execute_prepared_deploy_targets(execution, self)
    }

    /// Reverse the selected deploy set while its prepared images are retained.
    pub fn undeploy_targets(
        &self,
        execution: &crate::DeployExecution<'_>,
    ) -> Result<Vec<crate::RemovalOutcome>, crate::DeployError> {
        crate::mechanism::deploy::undeploy_prepared_targets(execution, self)
    }
}

fn select<'a>(
    key: MechanismKey,
    pin: Option<&vibe_core::manifest::ProviderPin>,
    registry: &'a MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<vibe_extension_registry::MechanismSelection<'a>, NativeArtifactError> {
    resolve_mechanism(registry, &key, pin, routes)
        .map_err(|error| selection_error(&error.to_string()))
}

pub fn preflight_native_mechanisms(
    plan: NativeMechanismPlan,
    execution: &NativeBuildExecution<'_>,
) -> Result<NativeMechanismPreflight, NativeArtifactError> {
    let mut artifacts = Vec::with_capacity(plan.entries.len());
    let mut groups: Vec<SourceGroup> = Vec::new();
    let mut claims = Vec::new();
    for (index, entry) in plan.entries.iter().enumerate() {
        let provider = mechanism_facts(&entry.row)?;
        let ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } = entry.row.handler()
        else {
            return Err(selection_error("selected mechanism is not native"));
        };
        if let Some(path) = prebuilt
            .as_ref()
            .and_then(|paths| paths.get(execution.platform.key()))
        {
            let file = prebuilt_file(
                &entry.row.pin().to_string(),
                &provider,
                path,
                execution.platform,
            )?;
            claims.push(NativeMechanismArtifactClaim::Prebuilt {
                provider: provider.identity.clone(),
                relative: file.relative.clone(),
            });
            artifacts.push(PreflightArtifact {
                provider,
                state: ArtifactState::Prebuilt(file),
            });
            continue;
        }
        let Some(crate_dir) = crate_dir else {
            return Err(selection_error(
                "selected native mechanism has no current artifact",
            ));
        };
        let crate_wire =
            relative_spelling(crate_dir).map_err(|reason| NativeArtifactError::CrateDirectory {
                provider: provider.identity.clone(),
                crate_dir: crate_dir.display().to_string(),
                reason,
            })?;
        source_crate(&provider, crate_dir)?;
        source_witness(&provider)?;
        let group = groups
            .iter()
            .position(|group| group.provider == provider && group.crate_wire == crate_wire)
            .unwrap_or_else(|| {
                groups.push(SourceGroup {
                    provider: provider.clone(),
                    crate_dir: crate_dir.clone(),
                    crate_wire: crate_wire.clone(),
                    entries: Vec::new(),
                    config: String::new(),
                });
                groups.len() - 1
            });
        groups[group].entries.push(index);
        artifacts.push(PreflightArtifact {
            provider: provider.clone(),
            state: ArtifactState::Source,
        });
    }
    for group in &mut groups {
        group.entries.sort_by_key(|index| {
            let row = &plan.entries[*index].row;
            (row.key().to_string(), row.pin().to_string())
        });
        let rows = group
            .entries
            .iter()
            .map(|index| &plan.entries[*index].row)
            .collect::<Vec<_>>();
        group.config = mechanism_config_witness(&rows);
        claims.push(NativeMechanismArtifactClaim::Source {
            provider: group.provider.identity.clone(),
            provider_root: group.provider.root.clone(),
            crate_dir: group.crate_wire.clone(),
        });
    }
    let build_provider = if groups.is_empty() {
        None
    } else {
        Some(select_build_provider(execution)?.pin())
    };
    Ok(NativeMechanismPreflight {
        plan,
        artifacts,
        groups,
        build_provider,
        claims,
    })
}

impl NativeMechanismPreflight {
    #[must_use]
    pub fn claims(&self) -> &[NativeMechanismArtifactClaim] {
        &self.claims
    }

    #[must_use]
    pub fn build_provider(&self) -> Option<&str> {
        self.build_provider.as_deref()
    }

    pub fn prepare(
        self,
        execution: &NativeBuildExecution<'_>,
    ) -> Result<PreparedNativeMechanisms, NativeArtifactError> {
        let mut resolved: Vec<Option<(VerifiedFile, NativeArtifactOrigin, Option<String>)>> =
            (0..self.plan.entries.len()).map(|_| None).collect();
        for group in &self.groups {
            let source_before = source_witness(&group.provider)?;
            let id = record_id(&group.provider, &group.crate_wire, execution.platform);
            let build_provider = self
                .build_provider
                .as_deref()
                .ok_or_else(|| selection_error("source group has no preflighted build provider"))?;
            match revalidate_source_record(&SourceRecordExpectation {
                selected_project_root: execution.selected_project_root,
                provider: &group.provider,
                platform: execution.platform,
                record_id: &id,
                build_provider,
                source_witness: &source_before,
                config_witness: &group.config,
            }) {
                Ok(_) | Err(NativeArtifactError::SourceRecordMissing { .. }) => {}
                Err(error) => return Err(error),
            }
            prepare_dependency_ignore(&group.provider)?;
            let (provider_root, manifest) = source_crate(&group.provider, &group.crate_dir)?;
            let built = build_cdylib(
                &group.provider,
                &manifest,
                &provider_root,
                execution.platform,
                execution.offline,
            )?;
            let source = source_witness(&group.provider)?;
            write_source_record(
                &SourceRecordInputs {
                    selected_project_root: execution.selected_project_root,
                    provider: &group.provider,
                    crate_dir: &group.crate_wire,
                    platform: execution.platform,
                    record_id: &id,
                    build_provider,
                    source_witness: &source,
                    config_witness: &group.config,
                    created_at: execution.created_at,
                },
                &built,
            )?;
            let file = revalidate_source_record(&SourceRecordExpectation {
                selected_project_root: execution.selected_project_root,
                provider: &group.provider,
                platform: execution.platform,
                record_id: &id,
                build_provider,
                source_witness: &source,
                config_witness: &group.config,
            })?;
            for index in &group.entries {
                resolved[*index] = Some((
                    file.clone(),
                    NativeArtifactOrigin::SourceRecord,
                    Some(record_path(&id)),
                ));
            }
        }
        for (index, artifact) in self.artifacts.iter().enumerate() {
            if let ArtifactState::Prebuilt(file) = &artifact.state {
                resolved[index] = Some((file.clone(), NativeArtifactOrigin::Prebuilt, None));
            }
        }
        let mut entries = Vec::with_capacity(self.plan.entries.len());
        for (index, planned) in self.plan.entries.into_iter().enumerate() {
            let artifact = self.artifacts.get(index);
            let provider = artifact
                .map(|value| &value.provider)
                .ok_or_else(|| selection_error("prepared mechanism lost provider provenance"))?;
            let (file, origin, record) = resolved[index]
                .take()
                .ok_or_else(|| selection_error("prepared mechanism lost its artifact"))?;
            let image = publish_load_image(
                execution.selected_project_root,
                &file.absolute,
                &file.digest,
                file.bytes,
            )?;
            entries.push(PreparedNativeMechanism {
                bindings: planned.bindings,
                provider: provider.identity.clone(),
                provider_version: provider.version.clone(),
                provider_hash: provider.content_hash.clone(),
                provider_root: provider.root.clone(),
                record_root: match provider.home {
                    ProviderHome::Dependency => NativeArtifactRecordRoot::Slot,
                    ProviderHome::Host => NativeArtifactRecordRoot::Project,
                },
                platform: execution.platform,
                origin,
                record,
                image,
                digest: file.digest,
                bytes: file.bytes,
            });
        }
        Ok(PreparedNativeMechanisms { entries })
    }
}

fn selection_error(reason: &str) -> NativeArtifactError {
    NativeArtifactError::MechanismSelection {
        reason: reason.to_owned(),
    }
}
