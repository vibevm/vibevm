//! Fresh authority over retained closed builtin ownership, without resolution.
use super::*;
use crate::mechanism::deploy::sidecar::LockResources;
use crate::mechanism::deploy::state::{DeployState, InverseRecord};
use crate::mechanism::deploy::transaction::Transaction;
use crate::mechanism::vibebin::bound::BoundVibeBinStore;
use crate::mechanism::vibebin::bound_provider::BoundVibeBinProvider;
use crate::mechanism::{DeployProvider, DeployTargetRequest, EffectClass};
use vibe_core::manifest::DeployTarget;

pub(crate) struct BoundCleanupPlan {
    home: DeploymentHome,
    receipt: DeployReceipt,
    current: DeployReceipt,
    target: DeployTarget,
    locks: Option<LockResources>,
    inverse: Option<InverseRecord>,
    roots: capsule::RootTokens,
    result: ResultRecord,
}
impl BoundCleanupPlan {
    pub(crate) fn receipt(&self) -> &DeployReceipt {
        &self.receipt
    }
    pub(crate) fn validate_host(&self, host: &AdmittedLocalDeployBinding) -> Result<()> {
        if self.receipt.operation_binding.as_ref() != Some(host.binding())
            || capsule::RootTokens::capture(host)? != self.roots
        {
            return Err(refused(
                "fresh cleanup host differs from retained scope/roots",
            ));
        }
        host.reobserve_roots().map_err(refused)
    }
}

fn generation(bridge: &Bridge) -> Result<u32> {
    bridge
        .plan
        .get("generation")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .try_into()
        .map_err(refused)
}
fn archived(
    settings: &vibe_safefs::Project,
    home: &DeploymentHome,
    bridge: &Bridge,
    result: &ResultRecord,
) -> Result<DeployReceipt> {
    let receipt = DeployStateView::open_bound(settings)?
        .read_bound_generation(home, generation(bridge)?)?
        .ok_or_else(|| refused("verified bound generation history absent"))?;
    if sha(&serde_json::to_vec_pretty(&receipt)?) != result.receipt_sha256
        || receipt.status != ReceiptStatus::Verified
        || receipt.target != bridge.target
        || receipt.identity.project != bridge.project
        || receipt.identity.package != bridge.package
        || receipt.profile != bridge.profile
        || receipt.provider.key != bridge.provider_pin
        || receipt.artifact_digest != bridge.artifact_digest
        || receipt
            .operation_binding
            .as_ref()
            .map(|b| b.binding_id.as_str())
            != Some(bridge.binding_id.as_str())
    {
        return Err(refused(
            "retained verified receipt differs from exact bridge result",
        ));
    }
    Ok(receipt)
}

fn cleaned(before: &DeployReceipt, current: &DeployReceipt) -> bool {
    let mut expected = before.clone();
    expected.status = ReceiptStatus::RolledBack;
    expected.resources.clear();
    expected.prior_state_handle = None;
    expected.finalized_at = current.finalized_at;
    expected.evidence = current.evidence.clone();
    expected == *current
}

/// Completion of retained cleanup is joined through the same immutable
/// bridge result owner, so a modified current receipt cannot invent success.
pub(super) fn cleaned_receipt(
    lease: &LifecycleLease,
    settings: &vibe_safefs::Project,
    home: &DeploymentHome,
    bridge: &Bridge,
    result: &ResultRecord,
) -> Result<DeployReceipt> {
    let original = archived(settings, home, bridge, result)?;
    let state =
        DeployState::observe_bound(settings)?.ok_or_else(|| refused("cleanup state absent"))?;
    let current = state
        .read_receipt(home)?
        .ok_or_else(|| refused("cleanup receipt absent"))?;
    if !cleaned(&original, &current)
        || state.read_inverse(home)?.is_some()
        || state.read_intent(home)?.is_some()
        || state.read_lock_resources(home)?.is_some()
    {
        return Err(refused("cleanup has not finalized retained ownership"));
    }
    let mut expected = result.clone();
    expected.receipt_sha256 = sha(&serde_json::to_vec_pretty(&current)?);
    let observed = read::<ResultRecord>(lease, &result.run_id, "cleanup-result")?
        .ok_or_else(|| refused("cleanup result publication remains uncertain"))?;
    if serde_json::to_vec(&expected)? != serde_json::to_vec(&observed)? {
        return Err(refused(
            "cleanup result differs from exact finalized receipt",
        ));
    }
    Ok(current)
}

pub(super) fn historical_receipt(
    settings: &vibe_safefs::Project,
    home: &DeploymentHome,
    bridge: &Bridge,
    result: &ResultRecord,
) -> Result<DeployReceipt> {
    let view = DeployStateView::open_bound(settings)?;
    let current = view
        .read_receipt(home)?
        .ok_or_else(|| refused("current receipt absent"))?;
    if current.generation <= generation(bridge)?
        || current.status != ReceiptStatus::Verified
        || view
            .read_bound_generation(home, current.generation)?
            .as_ref()
            != Some(&current)
    {
        return Err(refused(
            "current owner does not prove a later verified generation",
        ));
    }
    archived(settings, home, bridge, result)
}

pub(crate) fn prepare_bound_cleanup(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    admitted: &AdmittedLocalDeployBinding,
) -> Result<BoundCleanupPlan> {
    // Verifies C1 request, capsule/root join and the genuine original result.
    let observation =
        inspect_host_admitted_bound_vibe_bin_cold(lease, namespace.id(), scope, key, admitted)?;
    let run_id = match observation {
        Some(BoundDeployBridgeInspection::Completed { run_id, .. })
        | Some(BoundDeployBridgeInspection::Uncertain { run_id, .. }) => run_id,
        _ => return Err(refused("cleanup has no qualified retained owner run")),
    };
    let bridge = read::<Bridge>(lease, &run_id, "bridge")?
        .ok_or_else(|| refused("cleanup bridge absent"))?;
    let result = read::<ResultRecord>(lease, &run_id, "result")?
        .ok_or_else(|| refused("cleanup result absent"))?;
    let roots = capsule::RootTokens::capture(admitted)?;
    let request = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?
        .ok_or_else(|| refused("cleanup reservation absent"))?;
    if result.format != RESULT_FORMAT
        || bridge.format != BRIDGE_FORMAT
        || result.run_id != run_id
        || result.home_id != bridge.home_id
        || result.binding_id != bridge.binding_id
        || result.bound_plan_hash != bridge.bound_plan_hash
        || result.plan_sha256 != request.plan_sha256
        || bridge.plan_sha256 != request.plan_sha256
        || bridge.request_sha256 != request.request_sha256
        || request.journal_admission != JournalAdmission::Ready
        || result.reservation_ref
            != SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id)?
    {
        return Err(refused("cleanup retained result/reservation differs"));
    }
    // The cold reader validated the capsule. Recheck it here even for an
    // uncertain current receipt, so corrupt retention never widens cleanup.
    capsule::verify_retained(lease, &run_id, &bridge, admitted.binding(), &roots)?;
    let settings = admitted.settings();
    let home = DeploymentHome::new_bound(
        &settings.root_path().join("state/deployments"),
        &bridge.project,
        bridge.package.as_deref(),
        &bridge.target,
        admitted.binding(),
    )?;
    if home.id() != bridge.home_id {
        return Err(refused("cleanup home differs"));
    }
    let receipt = archived(settings, &home, &bridge, &result)?;
    let state =
        DeployState::observe_bound(settings)?.ok_or_else(|| refused("cleanup state absent"))?;
    let current = state
        .read_receipt(&home)?
        .ok_or_else(|| refused("cleanup receipt absent"))?;
    let inverse = state.read_inverse(&home)?;
    let marker = InverseRecord::bound_cleanup(&receipt)?;
    if inverse.as_ref().is_some_and(|old| old != &marker)
        || state.read_intent(&home)?.is_some()
        || (current != receipt && !cleaned(&receipt, &current))
    {
        return Err(refused(
            "cleanup current receipt or partial ownership differs",
        ));
    }
    let locks = state.read_lock_resources(&home)?;
    let finalized_partial = cleaned(&receipt, &current) && locks.is_none();
    if locks.as_ref().is_some_and(|locks| locks.pending.is_some())
        || (!finalized_partial
            && !locks
                .as_ref()
                .and_then(|locks| locks.committed.as_ref())
                .is_some_and(|row| {
                    row.operation_binding.as_ref() == Some(admitted.binding())
                        && row.generation == receipt.generation
                        && row.plan_hash == bridge.bound_plan_hash
                        && row.native.is_none()
                        && row.resources
                            == receipt
                                .resources
                                .iter()
                                .map(|r| r.resource.clone())
                                .collect::<Vec<_>>()
                }))
    {
        return Err(refused("cleanup sidecar differs"));
    }
    let target = capsule::retained_target(lease, &run_id)?;
    Ok(BoundCleanupPlan {
        home,
        receipt,
        current,
        target,
        locks,
        inverse,
        roots,
        result,
    })
}

pub(crate) fn execute_bound_cleanup(
    plan: BoundCleanupPlan,
    admitted: AdmittedLocalDeployBinding,
    current_generation: &mut dyn FnMut() -> std::result::Result<Option<String>, DeployError>,
    created_at: &str,
    lease: &LifecycleLease,
) -> Result<Vec<String>> {
    plan.validate_host(&admitted)?;
    let generation = current_generation()?;
    let locked = admitted
        .acquire_destination(generation.as_deref())
        .map_err(refused)?;
    let state = DeployState::open_bound(locked.settings())?;
    let _deployment = state.lock_deployment(&plan.home)?;
    let resources = plan
        .receipt
        .resources
        .iter()
        .map(|r| r.resource.clone())
        .collect::<Vec<_>>();
    let _resources = state.lock_destinations(&resources)?;
    super::super::ownership::refuse_foreign_ownership_bound(
        &state,
        &plan.home,
        &plan.target.id,
        &resources,
    )?;
    if state.read_receipt(&plan.home)?.as_ref() != Some(&plan.current)
        || state.read_lock_resources(&plan.home)? != plan.locks
        || state.read_inverse(&plan.home)? != plan.inverse
        || state.read_intent(&plan.home)?.is_some()
    {
        return Err(refused("cleanup ownership drifted after approval"));
    }
    let store = BoundVibeBinStore::from_locked_admission(&locked);
    let provider = BoundVibeBinProvider::for_retained(&store);
    let clients = super::super::model::ClientExecutables {
        claude: super::super::model::ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: super::super::model::ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: super::super::model::ClientExecutable::Missing {
            command: "opencode".into(),
        },
    };
    let request = DeployTargetRequest {
        target: &plan.target,
        profile: &plan.receipt.profile,
        project_root: locked.project().root_path(),
        settings_root: locked.settings().root_path(),
        user_home: locked.user_home().root_path(),
        clients: &clients,
        prior_receipt: Some(&plan.receipt),
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let transaction = Transaction {
        state: &state,
        home: &plan.home,
        identity: &plan.receipt.identity,
        provider_pin: &plan.receipt.provider.key,
        scope: EffectClass::User,
        created_at,
    };
    let removed = if cleaned(&plan.receipt, &plan.current)
        && plan.inverse.is_none()
        && plan.locks.is_none()
    {
        // The deletion transaction finished, but its immutable bridge result
        // may not have been published. Fresh authority independently verifies
        // absence; it never repeats an uncertain provider removal.
        let observed = provider
            .verify(&request, &resources)
            .map_err(DeployError::from)?;
        if observed.len() != resources.len() || observed.iter().any(|r| r.digest.is_some()) {
            return Err(refused(
                "cleanup finalization tail no longer observes exact absence",
            ));
        }
        Vec::new()
    } else {
        transaction.remove_bound(
            &provider,
            &request,
            &plan.receipt,
            &locked,
            current_generation,
        )?
    };
    let generation = current_generation()?;
    locked
        .reobserve_before_commit(generation.as_deref())
        .map_err(refused)?;
    let final_receipt = state
        .read_receipt(&plan.home)?
        .ok_or_else(|| refused("final cleanup receipt absent"))?;
    if !cleaned(&plan.receipt, &final_receipt)
        || state.read_inverse(&plan.home)?.is_some()
        || state.read_lock_resources(&plan.home)?.is_some()
        || state.read_intent(&plan.home)?.is_some()
    {
        return Err(refused("cleanup finalization ownership differs"));
    }
    let mut result = plan.result;
    result.receipt_sha256 = sha(&serde_json::to_vec_pretty(&final_receipt)?);
    match read::<ResultRecord>(lease, &result.run_id, "cleanup-result")? {
        Some(existing) if serde_json::to_vec(&existing)? == serde_json::to_vec(&result)? => {}
        Some(_) => return Err(refused("cleanup bridge result changed")),
        None => publish(lease, &result.run_id, "cleanup-result", &result)?,
    }
    Ok(removed)
}
