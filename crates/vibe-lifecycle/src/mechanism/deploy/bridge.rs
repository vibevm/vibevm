//! C.1 owner bridge for one host-admitted bound vibe-bin deployment.
//! Ready precedes destination acquisition; existing keys only inspect.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_wire::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};
use vibe_wire::generated::shared::EnvironmentOperationBinding;

use super::binding::AdmittedLocalDeployBinding;
use super::model::DeployExecution;
use super::state::DeploymentHome;
use super::transaction::bound_plan_hash;
use super::view::DeployStateView;
use super::{DeployError, execute_host_admitted_bound_vibe_bin_with_hook};
use crate::process::{is_valid_run_id, propose_run_id};
use crate::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};

const HOME: [&str; 2] = [".vibe", "deploy-operations"];
const CAP: usize = 16 * 1024;
const C1_CAP: usize = 1024 * 1024;
const BRIDGE_FORMAT: &str = "vibevm.deploy.bound-vibe-bin-bridge/1";
const RESULT_FORMAT: &str = "vibevm.deploy.bound-vibe-bin-result/1";

mod capsule;
pub(crate) use capsule::inspect_host_admitted_bound_vibe_bin_cold;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BoundDeployBridgeError {
    #[error("bound deploy bridge refused: {0}")]
    Refused(String),
    #[error(transparent)]
    Submission(#[from] crate::SubmissionError),
    #[error(transparent)]
    Lease(#[from] crate::LifecycleLeaseError),
    #[error(transparent)]
    Scratch(#[from] crate::process::ScratchError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Deploy(#[from] DeployError),
}
type Result<T> = std::result::Result<T, BoundDeployBridgeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BoundDeployBridgeInspection {
    Completed {
        run_id: String,
        receipt: DeployReceipt,
        replayed: bool,
    },
    RecoveryRequired {
        run_id: String,
    },
    Uncertain {
        run_id: String,
        partial: BoundDeployPartial,
    },
    Retired {
        run_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundDeployPartial {
    pub intent: String,
    pub receipt: String,
    pub sidecar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bridge {
    format: String,
    run_id: String,
    request_sha256: String,
    plan_sha256: String,
    home_id: String,
    binding_id: String,
    principal: String,
    instance_ref: String,
    bound_plan_hash: String,
    target: String,
    project: String,
    package: Option<String>,
    profile: String,
    provider_pin: String,
    artifact_digest: String,
    plan: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRecord {
    format: String,
    run_id: String,
    reservation_ref: String,
    plan_sha256: String,
    home_id: String,
    binding_id: String,
    bound_plan_hash: String,
    receipt_sha256: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn refused(reason: impl std::fmt::Display) -> BoundDeployBridgeError {
    BoundDeployBridgeError::Refused(reason.to_string())
}
fn member(run_id: &str, kind: &str) -> Result<String> {
    if !is_valid_run_id(run_id) {
        return Err(refused("invalid reserved run ID"));
    }
    Ok(format!("{run_id}.bound-vibe-bin-{kind}.json"))
}
fn read<T: for<'de> Deserialize<'de>>(
    lease: &LifecycleLease,
    run_id: &str,
    kind: &str,
) -> Result<Option<T>> {
    let Some(home) = lease.project().dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    let Some(bytes) = lease
        .project()
        .read_file_bounded_in(&home, &member(run_id, kind)?, CAP)
        .map_err(refused)?
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}
fn publish<T: Serialize>(
    lease: &LifecycleLease,
    run_id: &str,
    kind: &str,
    value: &T,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > CAP {
        return Err(refused("owner record exceeds cap"));
    }
    let home = lease.project().ensure_subproject(&HOME).map_err(refused)?;
    let dir = home.root_dir().map_err(refused)?;
    let name = member(run_id, kind)?;
    home.publish_new_in(&dir, &name, &bytes)
        .map_err(|error| refused(format!("owner publication: {:#}", error.into_report())))?;
    let observed = home
        .read_file_bounded_in(&dir, &name, CAP)
        .map_err(refused)?;
    if observed.as_deref() != Some(bytes.as_slice()) {
        return Err(refused("owner record changed on readback"));
    }
    Ok(())
}

pub(super) fn canonical_request(
    execution: &DeployExecution<'_>,
    binding: &EnvironmentOperationBinding,
    principal: &str,
) -> Result<Vec<u8>> {
    let [target_id] = execution.selection.targets.as_slice() else {
        return Err(refused("one selected target required"));
    };
    let target = execution
        .targets
        .iter()
        .find(|row| &row.id == target_id)
        .ok_or_else(|| refused("selected target absent"))?;
    if target.mechanism.to_string() != "deploy:vibe-bin"
        || target.when.is_some()
        || target
            .depends_on
            .as_ref()
            .is_some_and(|rows| !rows.is_empty())
    {
        return Err(refused(
            "only unconditional independent vibe-bin is admitted",
        ));
    }
    let path = |value: &std::path::Path| -> Result<String> {
        if !value.is_absolute() {
            return Err(refused("root is not absolute"));
        }
        value
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| refused("root is not UTF-8"))
    };
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": "vibevm.deploy.bound-vibe-bin-request/1",
        "binding": binding, "principal": principal,
        "project_root": path(execution.project_root)?,
        "settings_root": path(execution.settings_root)?,
        "user_home": path(execution.user_home)?,
        "state_home": path(execution.state_home)?,
        "project": execution.project, "package": execution.package,
        "profile": execution.selection.profile,
        "target": target.id, "artifact": target.artifact,
        "mechanism": target.mechanism.to_string(),
        "provider": target.provider.as_ref().map(ToString::to_string),
        "config": target.config.as_ref().map(|row| row.as_table()),
    }))?;
    if bytes.len() > C1_CAP {
        return Err(refused("request exceeds C.1 cap"));
    }
    Ok(bytes)
}

fn partial(settings: &vibe_safefs::Project, home: &DeploymentHome) -> BoundDeployPartial {
    let view = match DeployStateView::open_bound(settings) {
        Ok(view) => view,
        Err(error) => {
            return BoundDeployPartial {
                intent: error.to_string(),
                receipt: error.to_string(),
                sidecar: error.to_string(),
            };
        }
    };
    let status = |value: std::result::Result<bool, DeployError>| match value {
        Ok(true) => "present".to_owned(),
        Ok(false) => "absent".to_owned(),
        Err(error) => format!("unavailable: {error}"),
    };
    BoundDeployPartial {
        intent: status(view.read_intent(home).map(|row| row.is_some())),
        receipt: status(view.read_receipt(home).map(|row| row.is_some())),
        sidecar: status(view.read_lock_resources(home).map(|row| row.is_some())),
    }
}

fn verified_receipt(
    settings: &vibe_safefs::Project,
    home: &DeploymentHome,
    bridge: &Bridge,
) -> Result<(DeployReceipt, String)> {
    let view = DeployStateView::open_bound(settings)?;
    if view.read_intent(home)?.is_some() {
        return Err(refused("bound intent remains"));
    }
    let receipt = view
        .read_receipt(home)?
        .ok_or_else(|| refused("bound receipt absent"))?;
    let sidecar = view
        .read_lock_resources(home)?
        .ok_or_else(|| refused("bound sidecar absent"))?;
    let committed = sidecar
        .committed
        .ok_or_else(|| refused("bound sidecar not committed"))?;
    if sidecar.pending.is_some()
        || committed.plan_hash != bridge.bound_plan_hash
        || committed
            .operation_binding
            .as_ref()
            .map(|row| row.binding_id.as_str())
            != Some(bridge.binding_id.as_str())
        || receipt.status != ReceiptStatus::Verified
        || receipt.target != bridge.target
        || receipt.identity.project != bridge.project
        || receipt.identity.package != bridge.package
        || receipt.profile != bridge.profile
        || receipt.provider.key != bridge.provider_pin
        || receipt.artifact_digest != bridge.artifact_digest
        || receipt.generation != committed.generation
        || receipt.generation != 0
        || receipt
            .resources
            .iter()
            .map(|row| row.resource.as_str())
            .collect::<Vec<_>>()
            != committed
                .resources
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
    {
        return Err(refused("receipt/sidecar do not prove this plan"));
    }
    let path = format!("state/deployments/{}/receipt.json", home.id());
    let bytes = settings
        .read_file_bounded(&path, CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("receipt bytes vanished"))?;
    Ok((receipt, sha(&bytes)))
}

fn inspect(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: &[u8],
    run_id: String,
    ready: JournalAdmission,
    reservation_ref: Option<String>,
    settings: &vibe_safefs::Project,
    home: &DeploymentHome,
    binding: &EnvironmentOperationBinding,
    roots: &capsule::RootTokens,
) -> Result<BoundDeployBridgeInspection> {
    let bridge = match read::<Bridge>(lease, &run_id, "bridge") {
        Ok(Some(bridge)) => bridge,
        Ok(None) | Err(_) => return Ok(BoundDeployBridgeInspection::RecoveryRequired { run_id }),
    };
    let expected_plan = SubmissionReservations::inspect_reserved_plan_sha256(
        lease, namespace, scope, key, &run_id,
    )?;
    let plan_digest = serde_json::to_vec(&bridge.plan).map(|bytes| sha(&bytes));
    if ready != JournalAdmission::Ready
        || bridge.format != BRIDGE_FORMAT
        || bridge.run_id != run_id
        || bridge.request_sha256 != sha(request)
        || bridge.plan_sha256 != expected_plan
        || plan_digest.ok().as_deref() != Some(expected_plan.as_str())
        || bridge.home_id != home.id()
        || bridge.binding_id != binding.binding_id
        || bridge.instance_ref != binding.instance_ref
        || bridge.principal != scope.principal
        || bridge.plan["home_id"] != bridge.home_id
        || bridge.plan["binding_id"] != bridge.binding_id
        || bridge.plan["principal"] != bridge.principal
        || bridge.plan["instance_ref"] != bridge.instance_ref
        || bridge.plan["target"] != bridge.target
        || bridge.plan["provider_pin"] != bridge.provider_pin
        || bridge.plan["bound_plan_hash"] != bridge.bound_plan_hash
        || bridge.plan["project"] != bridge.project
        || bridge.plan["package"] != serde_json::to_value(&bridge.package)?
        || bridge.plan["profile"] != bridge.profile
        || bridge.plan["artifact_digest"] != bridge.artifact_digest
    {
        return Ok(BoundDeployBridgeInspection::RecoveryRequired { run_id });
    }
    if capsule::verify_existing(lease, &run_id, request, &bridge, binding, roots).is_err() {
        return Ok(BoundDeployBridgeInspection::Uncertain {
            partial: partial(settings, home),
            run_id,
        });
    }
    let result = match read::<ResultRecord>(lease, &run_id, "result") {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => {
            return Ok(BoundDeployBridgeInspection::Uncertain {
                partial: partial(settings, home),
                run_id,
            });
        }
    };
    if result.format != RESULT_FORMAT
        || result.run_id != run_id
        || result.reservation_ref != reservation_ref.unwrap_or_default()
        || result.plan_sha256 != bridge.plan_sha256
        || result.home_id != home.id()
        || result.binding_id != bridge.binding_id
        || result.bound_plan_hash != bridge.bound_plan_hash
    {
        return Ok(BoundDeployBridgeInspection::Uncertain {
            partial: partial(settings, home),
            run_id,
        });
    }
    match verified_receipt(settings, home, &bridge) {
        Ok((receipt, digest)) if digest == result.receipt_sha256 => {
            Ok(BoundDeployBridgeInspection::Completed {
                run_id,
                receipt,
                replayed: true,
            })
        }
        _ => Ok(BoundDeployBridgeInspection::Uncertain {
            partial: partial(settings, home),
            run_id,
        }),
    }
}

/// Host-internal fixed route. No caller-provided factory or generic deploy
/// dispatcher is admitted through this bridge.
pub(crate) fn submit_host_admitted_bound_vibe_bin(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &DeployExecution<'_>,
    request_binding: &EnvironmentOperationBinding,
    admitted: AdmittedLocalDeployBinding,
    principal: &str,
    current_generation: &mut dyn FnMut() -> std::result::Result<Option<String>, DeployError>,
) -> Result<BoundDeployBridgeInspection> {
    lease.ensure_root(execution.project_root, "at protected bound deploy")?;
    if admitted.binding() != request_binding
        || principal != request_binding.principal_ref
        || scope.principal != principal
        || scope.context != request_binding.host_context_id
        || scope.target != request_binding.instance_ref
    {
        return Err(refused("host scope or admitted binding differs"));
    }
    admitted.reobserve_roots().map_err(refused)?;
    let roots = capsule::RootTokens::capture(&admitted)?;
    let request = canonical_request(execution, request_binding, principal)?;
    let target_id = &execution.selection.targets[0];
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        execution.package,
        target_id,
        request_binding,
    )?;
    let settings = admitted.settings().subproject(&[]).map_err(refused)?;
    match SubmissionReservations::inspect_existing_request(lease, namespace, scope, key, &request)?
    {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(BoundDeployBridgeInspection::Retired { run_id: None });
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission,
            reservation_ref,
            ..
        } => {
            return inspect(
                lease,
                namespace,
                scope,
                key,
                &request,
                domain_run_id,
                journal_admission,
                reservation_ref,
                &settings,
                &home,
                request_binding,
                &roots,
            );
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let mut created: Option<Bridge> = None;
    let mut callback = |owner_home: &DeploymentHome,
                        target_request: &crate::mechanism::DeployTargetRequest<'_>,
                        plan: &super::protocol::DeployPlan,
                        pin: &str|
     -> std::result::Result<(), DeployError> {
        let reserve = || -> Result<Bridge> {
            if owner_home.id() != home.id() {
                return Err(refused("preplan home differs"));
            }
            let run_id = propose_run_id(lease.root())?;
            let bound_hash = bound_plan_hash(target_request, pin, plan, request_binding);
            let artifact_digest = target_request
                .artifact
                .ok_or_else(|| refused("bound artifact absent at preplan"))?
                .digest
                .clone();
            let plan_value = serde_json::json!({
                "format": "vibevm.deploy.bound-vibe-bin-plan/1",
                "home_id": home.id(), "binding_id": request_binding.binding_id,
                "principal": principal, "instance_ref": request_binding.instance_ref,
                "target": target_id, "provider_pin": pin, "bound_plan_hash": bound_hash,
                "project": execution.project, "package": execution.package,
                "profile": execution.selection.profile, "artifact_digest": artifact_digest,
                "resources": plan.resources.iter().map(|row| (&row.resource, &row.desired_digest)).collect::<Vec<_>>(),
                "lock_resources": plan.lock_resources,
            });
            let plan_bytes = serde_json::to_vec(&plan_value)?;
            if plan_bytes.len() > C1_CAP {
                return Err(refused("plan exceeds C.1 cap"));
            }
            let decision = SubmissionReservations::reserve(
                lease,
                namespace,
                scope,
                key,
                &request,
                &plan_bytes,
                &run_id,
            )?;
            if !matches!(decision, ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
            {
                return Err(refused("fresh reservation was not Created"));
            }
            let bridge = Bridge {
                format: BRIDGE_FORMAT.into(),
                run_id: run_id.clone(),
                request_sha256: sha(&request),
                plan_sha256: sha(&plan_bytes),
                home_id: home.id().into(),
                binding_id: request_binding.binding_id.clone(),
                principal: principal.into(),
                instance_ref: request_binding.instance_ref.clone(),
                bound_plan_hash: bound_hash,
                target: target_id.clone(),
                project: execution.project.into(),
                package: execution.package.map(str::to_owned),
                profile: execution.selection.profile.clone(),
                provider_pin: pin.into(),
                artifact_digest,
                plan: plan_value,
            };
            publish(lease, &run_id, "bridge", &bridge)?;
            let readback = read::<Bridge>(lease, &run_id, "bridge")?
                .ok_or_else(|| refused("bridge vanished"))?;
            if readback.plan_sha256 != bridge.plan_sha256
                || readback.binding_id != bridge.binding_id
            {
                return Err(refused("bridge changed on readback"));
            }
            capsule::publish(
                lease,
                &run_id,
                &request,
                &plan_bytes,
                request_binding,
                &bridge,
                &roots,
            )?;
            SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)?;
            if SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id)?
                != JournalAdmission::Ready
            {
                return Err(refused("bridge was not Ready"));
            }
            Ok(bridge)
        };
        match reserve() {
            Ok(bridge) => {
                created = Some(bridge);
                Ok(())
            }
            Err(error) => Err(DeployError::ProtectedReservation {
                reason: error.to_string(),
            }),
        }
    };
    let applied = execute_host_admitted_bound_vibe_bin_with_hook(
        execution,
        request_binding,
        admitted,
        principal,
        current_generation,
        Some(&mut callback),
    );
    let Some(bridge) = created else {
        return Err(applied
            .err()
            .unwrap_or_else(|| DeployError::ProtectedReservation {
                reason: "pre-acquire bridge callback was not reached".into(),
            })
            .into());
    };
    if applied.is_err() {
        return Ok(BoundDeployBridgeInspection::Uncertain {
            partial: partial(&settings, &home),
            run_id: bridge.run_id,
        });
    }
    let (receipt, receipt_sha256) = match verified_receipt(&settings, &home, &bridge) {
        Ok(value) => value,
        Err(_) => {
            return Ok(BoundDeployBridgeInspection::Uncertain {
                partial: partial(&settings, &home),
                run_id: bridge.run_id,
            });
        }
    };
    let reservation_ref =
        SubmissionReservations::reservation_ref(lease, namespace, scope, key, &bridge.run_id)?;
    let result = ResultRecord {
        format: RESULT_FORMAT.into(),
        run_id: bridge.run_id.clone(),
        reservation_ref,
        plan_sha256: bridge.plan_sha256.clone(),
        home_id: bridge.home_id.clone(),
        binding_id: bridge.binding_id.clone(),
        bound_plan_hash: bridge.bound_plan_hash.clone(),
        receipt_sha256,
    };
    if publish(lease, &bridge.run_id, "result", &result).is_err() {
        return Ok(BoundDeployBridgeInspection::Uncertain {
            partial: partial(&settings, &home),
            run_id: bridge.run_id,
        });
    }
    Ok(BoundDeployBridgeInspection::Completed {
        run_id: bridge.run_id,
        receipt,
        replayed: false,
    })
}
