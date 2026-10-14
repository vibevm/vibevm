//! Literal fixture decision boundary. No provider invocation or deployment.
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

/// No production caller is wired before the fixture safe-stop is qualified.
#[allow(dead_code)]
pub(crate) struct DerivativeAdmission {
    host: AdmittedLocalDeployBinding,
    result: String,
    descriptor: DeployTarget,
    selected: vibe_extension_registry::MechanismRegistryRow,
    artifact: ResolvedDeployArtifact,
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
    #[allow(dead_code)]
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
