//! Independent derivative admission and explicit host-owned activation.
//! The non-cloneable host carrier is consumed, never replaced by a trust flag.
use sha2::{Digest, Sha256};
use vibe_core::manifest::{DeployTarget, ExtensionHandler};
use vibe_wire::generated::shared::EnvironmentOperationBinding;
use vibe_workspace::derivatives::BoundDerivative;
use vibe_workspace::derivatives::compatibility::{
    Compatibility, CompatibilityReport, ExportTarget,
};

use super::binding::AdmittedLocalDeployBinding;
use super::model::DeployExecution;
use super::protocol::ResolvedDeployArtifact;

/// Exact result admission consumed by the trusted fixed-deploy host route.
pub(crate) struct DerivativeAdmission {
    host: AdmittedLocalDeployBinding,
    result: String,
    descriptor: DeployTarget,
    selected: vibe_extension_registry::MechanismRegistryRow,
    artifact: ResolvedDeployArtifact,
}

/// Held source inputs selected explicitly by a trusted library host.
/// These inputs are data; only the separate host admission grants an effect.
pub(crate) struct DerivativeSources<'a> {
    pub(crate) entry: &'a vibe_safefs::Project,
    pub(crate) base: &'a vibe_safefs::Project,
    pub(crate) local: &'a vibe_safefs::Project,
    pub(crate) result: String,
}

impl DerivativeSources<'_> {
    pub(crate) fn read(
        &self,
    ) -> Result<vibe_workspace::derivatives::MaterializedDerivative, String> {
        let current = vibe_workspace::derivatives::MaterializedDerivative::read(
            self.entry,
            self.base,
            self.local,
            4 * 1024 * 1024,
        )
        .map_err(|error| error.to_string())?;
        if current.result().record().result_identity != self.result {
            return Err("selected derivative result changed".into());
        }
        Ok(current)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActivationState {
    NotRequested,
    Refused(String),
    /// A decision only: no payload, launcher, provider or native image ran.
    AdmittedAtFixtureStop,
}

#[derive(Debug)]
pub(crate) struct DerivativeDecision {
    pub compatibility: CompatibilityReport,
    pub activation: ActivationState,
}

fn own_artifact(result: &BoundDerivative, artifact: &ResolvedDeployArtifact) -> Result<(), String> {
    let local = result.binding().local();
    let [file] = local.files.as_slice() else {
        return Err("bound builtin admission requires one produced executable file".into());
    };
    if local.resource != artifact.id || result.binding().base().resource == artifact.id {
        return Err(
            "result requires its own artifact-record identity, distinct from the base".into(),
        );
    }
    if file.path != artifact.relative
        || file.bytes.len() as u64 != artifact.bytes
        || format!("{:x}", Sha256::digest(&file.bytes)) != artifact.digest
    {
        return Err("engine artifact is not the exact retained result bytes".into());
    }
    Ok(())
}

impl DerivativeAdmission {
    /// The trusted host explicitly selects THIS result and target. Package
    /// installation, signatures, wire bindings and compatibility cannot call
    /// this without the incumbent pinned host admission.
    pub(crate) fn bind(
        result: &BoundDerivative,
        execution: &DeployExecution<'_>,
        target: &DeployTarget,
        host: AdmittedLocalDeployBinding,
    ) -> Result<Self, String> {
        if execution.selection.targets != [target.id.clone()]
            || target.mechanism.to_string() != "deploy:vibe-bin"
            || target
                .depends_on
                .as_ref()
                .is_some_and(|deps| !deps.is_empty())
            || !execution.targets.iter().any(|row| row == target)
        {
            return Err(
                "only the explicitly selected independent builtin target is admitted".into(),
            );
        }
        let selected = vibe_extension_registry::resolve_mechanism(
            execution.registry,
            &target.mechanism,
            target.provider.as_ref(),
            execution.routes,
        )
        .map_err(|error| error.to_string())?;
        if !matches!(selected.row().handler(), ExtensionHandler::Builtin { name } if name == crate::mechanism::BUILTIN_VIBE_BIN_NAME)
        {
            return Err("native/plugin/skill effects require their own owner admission".into());
        }
        let (artifact, _) = super::artifact::resolve_bound_project_artifact(host.project(), target)
            .map_err(|error| error.to_string())?;
        own_artifact(result, &artifact)?;
        Ok(Self {
            host,
            result: result.record().result_identity.clone(),
            descriptor: target.clone(),
            selected: selected.row().clone(),
            artifact,
        })
    }
}

/// Explicit opt-in consumer of a fresh result admission. This is crate-private:
/// no public command or automatic source discovery grants activation.
impl DerivativeAdmission {
    #[cfg(test)]
    pub(crate) fn execute(
        self,
        entry: &vibe_safefs::Project,
        base: &vibe_safefs::Project,
        local: &vibe_safefs::Project,
        execution: &DeployExecution<'_>,
        binding: &EnvironmentOperationBinding,
        principal: &str,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, super::DeployError>,
    ) -> Result<Vec<super::DeployOutcome>, super::DeployError> {
        self.execute_with_hook(
            entry,
            base,
            local,
            execution,
            binding,
            principal,
            current_generation,
            None,
        )
    }

    pub(crate) fn execute_with_hook(
        self,
        entry: &vibe_safefs::Project,
        base: &vibe_safefs::Project,
        local: &vibe_safefs::Project,
        execution: &DeployExecution<'_>,
        binding: &EnvironmentOperationBinding,
        principal: &str,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, super::DeployError>,
        before_acquire: Option<&mut super::BoundPreAcquireHook<'_>>,
    ) -> Result<Vec<super::DeployOutcome>, super::DeployError> {
        let refusal = |reason: String| super::DeployError::BoundGrantChanged {
            target: self.descriptor.id.clone(),
            reason,
        };
        if execution.selection.targets != [self.descriptor.id.clone()]
            || !execution
                .targets
                .iter()
                .any(|target| target == &self.descriptor)
        {
            return Err(refusal("selected derivative descriptor changed".into()));
        }
        let selected = vibe_extension_registry::resolve_mechanism(
            execution.registry,
            &self.descriptor.mechanism,
            self.descriptor.provider.as_ref(),
            execution.routes,
        )?;
        if selected.row().pin() != self.selected.pin()
            || selected.row().provider() != self.selected.provider()
            || selected.row().declaration() != self.selected.declaration()
        {
            return Err(refusal("admitted derivative provider changed".into()));
        }
        // The incumbent executor observes generation before planning, after
        // planning and at its commit boundaries under destination ownership.
        // Reopen BOTH authored sources at each of those observations. A retained
        // result or verify_tree alone cannot stand in for current source proof.
        let mut checked_generation = || {
            let generation = current_generation()?;
            let current = vibe_workspace::derivatives::MaterializedDerivative::read(
                entry,
                base,
                local,
                4 * 1024 * 1024,
            )
            .map_err(|error| refusal(error.to_string()))?;
            if current.result().record().result_identity != self.result {
                return Err(refusal(
                    "admission belongs to another derivative result".into(),
                ));
            }
            let project = vibe_safefs::Project::open(execution.project_root)
                .map_err(|error| refusal(error.to_string()))?;
            let (artifact, _) =
                super::artifact::resolve_bound_project_artifact(&project, &self.descriptor)?;
            own_artifact(current.result(), &artifact).map_err(&refusal)?;
            if artifact != self.artifact {
                return Err(refusal("admitted derivative artifact changed".into()));
            }
            Ok(generation)
        };
        super::execute_host_admitted_bound_vibe_bin_with_hook(
            execution,
            binding,
            self.host,
            principal,
            &mut checked_generation,
            before_acquire,
        )
    }
}

/// Result inspection survives every refusal. Success is consumed at this
/// literal safe-stop and cannot be passed to an effectful executor.
#[allow(dead_code)]
pub(crate) fn decide(
    result: &BoundDerivative,
    export: Option<&ExportTarget>,
    activation: Option<(
        &DeployExecution<'_>,
        &DeployTarget,
        &EnvironmentOperationBinding,
        &str,
        Option<&str>,
    )>,
    grant: Option<DerivativeAdmission>,
) -> DerivativeDecision {
    let compatibility = result.compatibility(export);
    let activation = match activation {
        None => ActivationState::NotRequested,
        Some((execution, target, binding, principal, generation)) => {
            let admit = || -> Result<(), String> {
                if compatibility.compatibility != Compatibility::Compatible {
                    return Err("selected export is not compatible".into());
                }
                let grant = grant.ok_or("own result admission is absent")?;
                if grant.result != result.record().result_identity || &grant.descriptor != target {
                    return Err("grant belongs to a different result or descriptor".into());
                }
                if execution.selection.targets != [target.id.clone()]
                    || !execution.targets.iter().any(|row| row == &grant.descriptor)
                {
                    return Err("selected deployment descriptor changed".into());
                }
                grant
                    .host
                    .preflight(execution, binding, principal, generation)
                    .map_err(|error| error.to_string())?;
                // Execution can carry a different registry/routes from bind.
                // Reuse the resolver's availability/disable law before locks.
                let current = vibe_extension_registry::resolve_mechanism(
                    execution.registry,
                    &target.mechanism,
                    target.provider.as_ref(),
                    execution.routes,
                )
                .map_err(|error| error.to_string())?;
                if current.row().pin() != grant.selected.pin()
                    || current.row().provider() != grant.selected.provider()
                    || current.row().declaration() != grant.selected.declaration()
                    || !matches!(current.row().handler(), ExtensionHandler::Builtin { name } if name == crate::mechanism::BUILTIN_VIBE_BIN_NAME)
                {
                    return Err("admitted builtin provider selection changed".into());
                }
                let locked = grant
                    .host
                    .acquire_destination(generation)
                    .map_err(|error| error.to_string())?;
                let (current, _) =
                    super::artifact::resolve_bound_project_artifact(locked.project(), target)
                        .map_err(|error| error.to_string())?;
                own_artifact(result, &current)?;
                if current != grant.artifact {
                    return Err("admitted artifact identity changed".into());
                }
                locked
                    .reobserve_before_commit(generation)
                    .map_err(|error| error.to_string())?;
                Ok(())
            };
            match admit() {
                Ok(()) => ActivationState::AdmittedAtFixtureStop,
                Err(reason) => ActivationState::Refused(reason),
            }
        }
    };
    DerivativeDecision {
        compatibility,
        activation,
    }
}

#[cfg(test)]
mod m_10_b_2_cases;

#[cfg(test)]
mod m_10_d_2_cases;
