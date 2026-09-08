//! Selected deploy-native mechanism artifact planning and preparation.

use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::{ExtensionHandler, MechanismKey};
use vibe_extension_registry::{MechanismRegistryRow, SelectionStep};
use vibe_native_loader::{NativeLoadError, NativeMechanism};

use super::build_provider::{MechanismBuildTransport, mechanism_build_provider};
use super::cargo::build_cdylib;
use super::path::{
    VerifiedFile, existing_load_image, prebuilt_file, publish_load_image, relative_spelling,
    source_crate,
};
use super::provider::{ProviderFacts, ProviderHome, mechanism_facts};
use super::record::{
    SourceRecordExpectation, SourceRecordInputs, record_path, revalidate_source_record,
    write_source_record,
};
use super::witness::{mechanism_config_witness, record_id, source_witness};
use super::{
    NativeArtifactError, NativeArtifactOrigin, NativeArtifactRecordRoot, NativeBuildExecution,
    NativePlatform, prepare_dependency_ignore,
};

#[path = "mechanism/plan.rs"]
mod plan;
pub use plan::{project_native_mechanisms, project_native_target_mechanisms};

#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
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

    pub fn bindings(&self) -> impl Iterator<Item = &NativeMechanismBinding> {
        self.entries.iter().flat_map(|entry| &entry.bindings)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
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

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
pub struct NativeMechanismPreflight {
    plan: NativeMechanismPlan,
    artifacts: Vec<PreflightArtifact>,
    groups: Vec<SourceGroup>,
    build_provider: Option<String>,
    build_transport: Option<MechanismBuildTransport>,
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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
pub struct PreparedNativeMechanisms {
    pub entries: Vec<PreparedNativeMechanism>,
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
    pub(crate) fn binding_for(
        &self,
        role: vibe_core::manifest::MechanismRole,
        target: &str,
        key: &MechanismKey,
    ) -> Result<Option<(&PreparedNativeMechanism, &NativeMechanismBinding)>, (&str, &'static str)>
    {
        let mut found = None;
        for entry in &self.entries {
            for binding in &entry.bindings {
                if binding.key.role() != role || binding.target != target {
                    continue;
                }
                if found.is_some() || &binding.key != key {
                    return Err((
                        &binding.pin,
                        "prepared carriage has duplicate or mismatched target bindings",
                    ));
                }
                found = Some((entry, binding));
            }
        }
        Ok(found)
    }

    /// Execute build targets with this exact prepared native carriage.
    ///
    /// ```
    /// use std::path::Path;
    /// use vibe_core::manifest::{ExtensionsControl, MechanismRoutes};
    /// use vibe_extension_registry::collect_mechanisms;
    /// use vibe_lifecycle::native::PreparedNativeMechanisms;
    /// use vibe_lifecycle::{
    ///     BuildExecution, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
    /// };
    ///
    /// let world = ExtensionWorld {
    ///     installed: Vec::new(),
    ///     host: HostExtensionSource {
    ///         provider: HostProvider {
    ///             identity: HostIdentity::ungrouped_project("demo"),
    ///             root: Path::new(".").to_path_buf(),
    ///             version: "0.1.0".into(),
    ///             kind: None,
    ///             content_hash: None,
    ///         },
    ///         declarations: Vec::new(),
    ///         controls: ExtensionsControl::default(),
    ///         mechanisms: Vec::new(),
    ///     },
    ///     effective_stack: None,
    /// };
    /// let registry = collect_mechanisms(&world).unwrap();
    /// let routes = MechanismRoutes::default();
    /// let execution = BuildExecution {
    ///     project_root: Path::new("."),
    ///     targets: &[],
    ///     registry: &registry,
    ///     routes: &routes,
    ///     build_root: BuildExecution::default_build_root(),
    ///     offline: true,
    ///     created_at: "2026-09-08T00:00:00Z",
    /// };
    /// assert!(
    ///     PreparedNativeMechanisms::default()
    ///         .execute_build_targets(&execution)
    ///         .unwrap()
    ///         .is_empty()
    /// );
    /// ```
    pub fn execute_build_targets(
        &self,
        execution: &crate::BuildExecution<'_>,
    ) -> Result<Vec<crate::BuildOutcome>, crate::BuildError> {
        crate::mechanism::build::execute_prepared_build_targets(execution, self)
    }

    pub fn plan_deploy_targets(
        &self,
        execution: &crate::DeployExecution<'_>,
    ) -> Result<Vec<crate::DeployPlanReport>, crate::DeployError> {
        crate::mechanism::deploy::plan_prepared_deploy_targets(execution, self)
    }

    pub fn validate_restart(
        &self,
        execution: &crate::DeployExecution<'_>,
    ) -> Result<(), crate::DeployError> {
        crate::mechanism::deploy::validate_restart(execution, self)
    }

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
    let build_transport = if groups.is_empty() {
        None
    } else {
        Some(mechanism_build_provider(execution)?)
    };
    let build_provider = build_transport
        .as_ref()
        .map(|transport| transport.pin().to_owned());
    Ok(NativeMechanismPreflight {
        plan,
        artifacts,
        groups,
        build_provider,
        build_transport,
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
        if let Some(transport) = &self.build_transport {
            transport.admit_prepare(execution.platform.key())?;
        }
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
        finish(self.plan, &self.artifacts, resolved, execution, |file| {
            publish_load_image(
                execution.selected_project_root,
                &file.absolute,
                &file.digest,
                file.bytes,
            )
        })
    }

    /// Rehydrate only existing records and immutable images; never build or publish.
    pub fn rehydrate(
        self,
        execution: &NativeBuildExecution<'_>,
    ) -> Result<PreparedNativeMechanisms, NativeArtifactError> {
        let mut resolved = (0..self.plan.entries.len())
            .map(|_| None)
            .collect::<Vec<_>>();
        for group in &self.groups {
            let source = source_witness(&group.provider)?;
            let id = record_id(&group.provider, &group.crate_wire, execution.platform);
            let build_provider = self
                .build_provider
                .as_deref()
                .ok_or_else(|| selection_error("source group has no preflighted build provider"))?;
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
        finish(self.plan, &self.artifacts, resolved, execution, |file| {
            existing_load_image(
                execution.selected_project_root,
                &file.absolute,
                &file.digest,
                file.bytes,
            )
        })
    }
}

fn finish(
    plan: NativeMechanismPlan,
    artifacts: &[PreflightArtifact],
    mut resolved: Vec<Option<(VerifiedFile, NativeArtifactOrigin, Option<String>)>>,
    execution: &NativeBuildExecution<'_>,
    image: impl Fn(&VerifiedFile) -> Result<PathBuf, NativeArtifactError>,
) -> Result<PreparedNativeMechanisms, NativeArtifactError> {
    let mut entries = Vec::with_capacity(plan.entries.len());
    for (index, planned) in plan.entries.into_iter().enumerate() {
        let provider = &artifacts
            .get(index)
            .ok_or_else(|| selection_error("prepared mechanism lost provider provenance"))?
            .provider;
        let (file, origin, record) = resolved[index]
            .take()
            .ok_or_else(|| selection_error("prepared mechanism lost its artifact"))?;
        let image = image(&file)?;
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

fn selection_error(reason: &str) -> NativeArtifactError {
    NativeArtifactError::MechanismSelection {
        reason: reason.to_owned(),
    }
}
