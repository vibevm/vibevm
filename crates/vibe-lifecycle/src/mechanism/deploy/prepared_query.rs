//! Explicit preparation followed by a retained, read-only deployment query.
use super::protocol::{DeployFingerprint, DeployPlan, DeployTargetRequest, ResolvedDeployArtifact};
use super::state::DeploymentHome;
use super::view::DeployStateView;
use super::*;
use std::path::PathBuf;
use vibe_wire::generated::{deploy_intent::DeployIntent, deploy_receipt::DeployReceipt};

struct TargetCapture {
    target: vibe_core::manifest::DeployTarget,
    observation: Box<dyn super::protocol::PreparedDeployObservation>,
    home: DeploymentHome,
    receipt: Option<DeployReceipt>,
    intent: Option<DeployIntent>,
    artifact: ResolvedDeployArtifact,
    plan: DeployPlan,
    fingerprint: DeployFingerprint,
}
/// Host-retained observations. It cannot be supplied to apply or recovery.
pub struct PreparedDeployQuery {
    project: PathBuf,
    settings: PathBuf,
    user_home: PathBuf,
    state_home: PathBuf,
    profile: String,
    clients: ClientExecutables,
    targets: Vec<TargetCapture>,
    reports: Vec<DeployPlanReport>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDeployRow {
    pub report: DeployPlanReport,
    pub config_digest: String,
    pub fingerprint: String,
    pub reversible: bool,
}
fn need(
    target: &str,
    reason: vibe_wire::generated::adapter_plan::e1::error::ErrorReason,
) -> DeployError {
    crate::mechanism::MechanismError::from(
        crate::mechanism::error::DeployProviderError::NeedsPreparation {
            target: target.into(),
            reason,
        },
    )
    .into()
}
pub fn prepare_deploy_query(
    execution: &DeployExecution<'_>,
) -> Result<PreparedDeployQuery, DeployError> {
    prepare_deploy_query_with_client_state_owner(execution, None)
}
/// Trusted embedding injection; no request can supply a client-state owner.
pub fn prepare_deploy_query_with_client_state_owner(
    execution: &DeployExecution<'_>,
    owner: Option<std::sync::Arc<dyn ClientStateOwner>>,
) -> Result<PreparedDeployQuery, DeployError> {
    let resolved = resolve_query_selection(execution)?;
    if let Some(owner) = owner {
        for selected in &resolved {
            selected.provider.set_client_state_owner(owner.clone());
        }
    }
    // Uses the incumbent selection, artifact and receipt owners exactly once
    // for preparation; no alternative provider resolver is introduced.
    let reports = plan::prepare_resolved(execution, &resolved)?;
    let state = DeployStateView::open(execution.state_home)?;
    let mut targets = Vec::new();
    for selected in resolved {
        let home = home_of(execution, &selected.target.id);
        let receipt = state.read_receipt(&home)?;
        let intent = state.read_intent(&home)?;
        let artifact = artifact::resolve_artifact(execution.project_root, selected.target)?;
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            prior_receipt: receipt.as_ref(),
            recovery_intent: intent.as_ref(),
            artifact: Some(&artifact),
            staging: None,
        };
        let (plan, fingerprint) = selected.provider.preparation_result(&request)?;
        let observation = match selected.provider.prepared_query_observation() {
            Some(observation) => observation,
            None => {
                // Exact retained selected row lookup, not another resolution.
                let pin = selected.pin.parse().expect("selected provider pin");
                let row = execution
                    .registry
                    .find(&pin)
                    .expect("retained selected row");
                Box::new(PureBuiltinObservation(builtin_provider(
                    row.handler(),
                    &selected.target.mechanism.to_string(),
                    &selected.pin,
                )?))
            }
        };
        targets.push(TargetCapture {
            target: selected.target.clone(),
            observation,
            home,
            receipt,
            intent,
            artifact,
            plan,
            fingerprint,
        });
    }
    Ok(PreparedDeployQuery {
        project: execution.project_root.into(),
        settings: execution.settings_root.into(),
        user_home: execution.user_home.into(),
        state_home: execution.state_home.into(),
        profile: execution.selection.profile.clone(),
        clients: execution.clients.clone(),
        targets,
        reports,
    })
}
impl PreparedDeployQuery {
    /// Historical preparation identities only, not current plan rows.
    pub fn evidence_identity(&self) -> String {
        self.targets
            .iter()
            .map(|t| t.fingerprint.digest.as_str())
            .collect::<Vec<_>>()
            .join(":")
    }

    pub fn plan(&self) -> Result<Vec<PreparedDeployRow>, DeployError> {
        let state = DeployStateView::open(&self.state_home)?;
        let mut rows = Vec::new();
        for (capture, report) in self.targets.iter().zip(&self.reports) {
            let receipt = state.read_receipt(&capture.home)?;
            let intent = state.read_intent(&capture.home)?;
            let artifact = artifact::resolve_artifact(&self.project, &capture.target)?;
            if receipt != capture.receipt
                || intent != capture.intent
                || artifact != capture.artifact
            {
                return Err(need(
                    &capture.target.id,
                    vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientCaptureStale,
                ));
            }
            let request = DeployTargetRequest {
                target: &capture.target,
                profile: &self.profile,
                project_root: &self.project,
                settings_root: &self.settings,
                user_home: &self.user_home,
                clients: &self.clients,
                prior_receipt: receipt.as_ref(),
                recovery_intent: intent.as_ref(),
                artifact: Some(&artifact),
                staging: None,
            };
            let (plan, fingerprint) = capture.observation.current(&request)?;
            if plan != capture.plan || fingerprint != capture.fingerprint {
                return Err(need(
                    &capture.target.id,
                    vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientCaptureStale,
                ));
            }
            rows.push(PreparedDeployRow {
                report: report.clone(),
                config_digest: plan.config_digest,
                fingerprint: fingerprint.digest,
                reversible: plan.reversible,
            });
        }
        Ok(rows)
    }
}

/// Only stateless builtins reach this wrapper. Stateful client adapters export
/// their finite observation directly, and native rows refuse before loading.
struct PureBuiltinObservation(Box<dyn crate::mechanism::DeployProvider + Send + Sync>);
impl super::protocol::PreparedDeployObservation for PureBuiltinObservation {
    fn current(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<(DeployPlan, DeployFingerprint), crate::mechanism::MechanismError> {
        let plan = self.0.plan(request)?;
        let fingerprint = self.0.fingerprint(request, &plan)?;
        Ok((plan, fingerprint))
    }
}
