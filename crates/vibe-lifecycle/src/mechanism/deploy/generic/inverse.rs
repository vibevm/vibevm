//! Current host authority over retained, closed operations. Source selection is
//! deliberately not consulted for inspection, cleanup, inverse or revocation.
use super::super::state::DeployState;
use super::*;
use vibe_wire::generated::deploy_generic_state as wire;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptInspection {
    Absent,
    Legacy {
        receipt: DeployReceipt,
        limitation: &'static str,
    },
    Retained {
        receipt: Option<DeployReceipt>,
        state: wire::DeployGenericState,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RetainedOperation {
    Cleanup,
    Inverse,
    Revocation,
    RestoreDonor,
    Recovery,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedPlan {
    pub(super) target: String,
    pub(super) operation: RetainedOperation,
    pub(super) state: wire::DeployGenericState,
    pub(super) receipt: Option<DeployReceipt>,
    pub(super) desired: Vec<wire::Resource>,
    pub(super) control: ControlObservation,
    pub(super) roots: [String; 3],
    pub(super) protected: ProtectedControls,
    pub(super) host: u64,
    pub(super) recovery: bool,
    pub(super) forward: Option<CapturedClient>,
}
impl RetainedPlan {
    pub fn operation(&self) -> RetainedOperation {
        self.operation
    }
    pub fn resources(&self) -> &[wire::Resource] {
        &self.desired
    }
    pub fn identity(&self) -> String {
        digest(
            &encoded(&(
                "generic-retained-plan/1",
                &self.target,
                self.operation,
                &self.state,
                &self.receipt,
                &self.desired,
                &self.control,
                &self.roots,
                &self.protected,
                self.recovery,
            ))
            .expect("closed retained plan"),
        )
    }
}
#[derive(Debug)]
pub struct RetainedGrant {
    plan: RetainedPlan,
}

impl GenericHost {
    /// Resume retained bytes, never re-plan from a changed descriptor. Forward
    /// recovery additionally requires the exact currently selected capture.
    pub fn prepare_recovery(
        &self,
        target: &str,
        forward: Option<&AdmittedClient>,
    ) -> Result<RetainedPlan, GenericError> {
        let ReceiptInspection::Retained { receipt, state } = self.inspect_receipt(target)? else {
            return Err(refuse("recovery requires retained receipt basis"));
        };
        let pending = state
            .pending
            .as_ref()
            .or(state.committed.as_ref())
            .ok_or_else(|| refuse("no retained generic operation"))?;
        let operation = match pending.operation {
            wire::BindingOperation::Cleanup => RetainedOperation::Cleanup,
            wire::BindingOperation::Inverse if pending.transfer.is_some() => {
                RetainedOperation::RestoreDonor
            }
            wire::BindingOperation::Inverse => RetainedOperation::Inverse,
            wire::BindingOperation::Revocation => RetainedOperation::Revocation,
            wire::BindingOperation::Apply
            | wire::BindingOperation::Upgrade
            | wire::BindingOperation::Recovery => RetainedOperation::Recovery,
            wire::BindingOperation::Adoption => RetainedOperation::Recovery,
        };
        let forward = if matches!(
            pending.operation,
            wire::BindingOperation::Apply
                | wire::BindingOperation::Upgrade
                | wire::BindingOperation::Recovery
                | wire::BindingOperation::Adoption
        ) {
            if pending.revoked || state.committed.as_ref().is_some_and(|b| b.revoked) {
                return Err(refuse("revoked forward recovery"));
            }
            let admitted = forward
                .ok_or_else(|| refuse("forward recovery requires current selected source"))?;
            self.current(&admitted.capture)?;
            if encoded(&admitted.capture.observation.identity)? != encoded(&pending.pins)? {
                return Err(refuse("recovery source pins changed"));
            }
            Some(admitted.capture.clone())
        } else {
            None
        };
        let store = DeployState::observe_bound(&self.settings)
            .map_err(refuse)?
            .ok_or_else(|| refuse("state disappeared"))?;
        let home = self.home(target);
        for row in &pending.resources {
            self.protect(scope(row), &row.destination)?;
            let snapshot = self
                .root(scope(row))
                .read_file_snapshot_bounded(&row.destination, 4 * 1024 * 1024)
                .map_err(refuse)?;
            let found = plan::observe_value(
                snapshot.as_ref().map(|s| s.bytes.as_slice()),
                &resource(row),
            )?;
            let before = match &row.before {
                wire::Before::Absent(_) => None,
                wire::Before::Retained(v) => {
                    Some(store.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                }
                wire::Before::Unavailable(_) => {
                    if pending.operation == wire::BindingOperation::Adoption {
                        match &row.desired {
                            wire::Value::Present(v) => {
                                Some(store.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                            }
                            wire::Value::Absent(_) => None,
                        }
                    } else {
                        return Err(refuse("recovery before-state unavailable"));
                    }
                }
            };
            let desired = match &row.desired {
                wire::Value::Absent(_) => None,
                wire::Value::Present(v) => {
                    Some(store.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                }
            };
            if (found != before && found != desired)
                || !retained_mode_matches(
                    row,
                    snapshot.as_ref(),
                    found.as_deref(),
                    before.as_deref(),
                    desired.as_deref(),
                    true,
                )
            {
                return Err(refuse("third digest in generic recovery"));
            }
        }
        let control = self.authority.current_control().map_err(refuse)?;
        if control.principal.is_empty() || control.policy_generation.is_empty() {
            return Err(refuse("current control required"));
        }
        let desired = pending.resources.clone();
        let plan = RetainedPlan {
            target: target.into(),
            operation,
            state,
            receipt,
            desired,
            control,
            roots: self.roots.clone(),
            protected: self.protected.clone(),
            host: self.instance,
            recovery: true,
            forward,
        };
        self.check_retained(&plan)?;
        Ok(plan)
    }
    pub fn execute_retained(
        &self,
        plan: &RetainedPlan,
        grant: Option<&RetainedGrant>,
        created_at: &str,
    ) -> Result<super::GenericOutcome, GenericError> {
        if plan
            .state
            .pending
            .as_ref()
            .or(plan.state.committed.as_ref())
            .is_some_and(|b| b.transfer.is_some())
        {
            return Err(refuse("recorded transfer requires donor owner"));
        }
        self.execute_retained_inner(plan, grant, created_at, None, None)
    }
    pub fn execute_skill_retained(
        &self,
        plan: &RetainedPlan,
        grant: Option<&RetainedGrant>,
        donor: &vibe_agent_projection::pkgskill::SkillDonor,
        created_at: &str,
    ) -> Result<super::GenericOutcome, GenericError> {
        let b = plan
            .state
            .pending
            .as_ref()
            .or(plan.state.committed.as_ref())
            .ok_or_else(|| refuse("missing transfer"))?;
        let t = b
            .transfer
            .as_ref()
            .ok_or_else(|| refuse("missing transfer"))?;
        if (donor.root() != self.project.root_path() && donor.root() != self.user.root_path())
            || t.family != donor.family()
            || t.donor_id != donor.id()
            || t.donor_root_identity != donor.root_identity()
            || t.receipt_digest != digest(donor.receipt_bytes())
        {
            return Err(refuse("wrong donor"));
        }
        donor.matches_transfer(t).map_err(refuse)?;
        self.execute_retained_inner(plan, grant, created_at, Some(donor), None)
    }
    pub fn execute_deployment_retained(
        &self,
        plan: &RetainedPlan,
        grant: Option<&RetainedGrant>,
        donor: &adoption::DeploymentDonor,
        created_at: &str,
    ) -> Result<super::GenericOutcome, GenericError> {
        let b = plan
            .state
            .pending
            .as_ref()
            .or(plan.state.committed.as_ref())
            .ok_or_else(|| refuse("missing transfer"))?;
        let t = b
            .transfer
            .as_ref()
            .ok_or_else(|| refuse("missing transfer"))?;
        donor.matches(t)?;
        self.execute_retained_inner(plan, grant, created_at, None, Some(donor))
    }
    fn execute_retained_inner(
        &self,
        plan: &RetainedPlan,
        grant: Option<&RetainedGrant>,
        created_at: &str,
        donor: Option<&vibe_agent_projection::pkgskill::SkillDonor>,
        deployment: Option<&adoption::DeploymentDonor>,
    ) -> Result<super::GenericOutcome, GenericError> {
        if grant.is_none_or(|g| g.plan != *plan) {
            return Err(refuse("fresh exact retained operation grant required"));
        }
        self.check_retained(plan)?;
        created_at
            .parse::<vibe_wire::generated::deploy_receipt::Rfc3339Timestamp>()
            .map_err(refuse)?;
        let boundary = std::cell::Cell::new(GenericEffect::Infrastructure);
        let result = (|| {
            self.current_roots()?;
            let _arena = self
                .arena()
                .lock("deploy-physical-coordination.lock")
                .map_err(refuse)?;
            self.current_roots()?;
            let _deployment_donor_lease = deployment.map(|d| d.lock()).transpose()?;
            let _donor_lease = donor.map(|d| d.lock()).transpose().map_err(refuse)?;
            let state = DeployState::open_bound(&self.settings).map_err(refuse)?;
            let home = self.home(&plan.target);
            let _deployment = state.lock_deployment(&home).map_err(refuse)?;
            let locks = plan
                .desired
                .iter()
                .flat_map(|r| resource(r).locks(self))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let _destinations = state.lock_destinations(&locks).map_err(refuse)?;
            self.check_retained(plan)?;
            // Re-prepare under locks: current foreign neighbors may change,
            // owned literal members and the exact approval may not.
            if !plan.recovery && self.prepare_retained(&plan.target, plan.operation)? != *plan {
                return Err(refuse("retained plan changed under lock"));
            }
            if plan.recovery && plan.state.pending.is_none() {
                let c = plan
                    .state
                    .committed
                    .as_ref()
                    .ok_or_else(|| refuse("missing committed state"))?;
                if let Some(intent) = state.read_intent(&home).map_err(refuse)? {
                    if intent.plan_hash != c.plan_hash || intent.target.generation != c.generation {
                        return Err(refuse("changed committed intent"));
                    }
                    state.retire_intent(&home).map_err(refuse)?;
                }
                if let Some(donor) = donor {
                    let t = c
                        .transfer
                        .as_ref()
                        .ok_or_else(|| refuse("missing transfer"))?;
                    if t.phase == wire::TransferPhase::Restoring {
                        donor.restore(&t.hold_id).map_err(refuse)?;
                        let mut finished = plan.state.clone();
                        finished
                            .committed
                            .as_mut()
                            .unwrap()
                            .transfer
                            .as_mut()
                            .unwrap()
                            .phase = wire::TransferPhase::Restored;
                        state.write_generic(&home, &finished).map_err(refuse)?;
                    }
                }
                if let Some(donor) = deployment {
                    let t = c
                        .transfer
                        .as_ref()
                        .ok_or_else(|| refuse("missing transfer"))?;
                    if t.phase == wire::TransferPhase::Restoring {
                        donor.restore(&t.hold_id)?;
                        let mut finished = plan.state.clone();
                        finished
                            .committed
                            .as_mut()
                            .unwrap()
                            .transfer
                            .as_mut()
                            .unwrap()
                            .phase = wire::TransferPhase::Restored;
                        state.write_generic(&home, &finished).map_err(refuse)?;
                    }
                }
                return Ok(super::GenericOutcome {
                    generation: c.generation,
                    resources: plan
                        .receipt
                        .as_ref()
                        .unwrap()
                        .resources
                        .iter()
                        .map(|r| r.resource.clone())
                        .collect(),
                    effect: GenericEffect::Infrastructure,
                });
            }
            if plan.recovery {
                let admitted = plan.forward.as_ref().map(|capture| AdmittedClient {
                    capture: capture.clone(),
                    payloads: Vec::new(),
                });
                if self.prepare_recovery(&plan.target, admitted.as_ref())? != *plan {
                    return Err(refuse("recovery changed under locks"));
                }
            }
            let mut record = plan.state.clone();
            let mut pending = if plan.recovery {
                record.pending.as_ref()
            } else {
                record.committed.as_ref()
            }
            .ok_or_else(|| refuse("missing retained semantics"))?
            .clone();
            if !plan.recovery {
                pending.generation = pending
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| refuse("generation exhausted"))?;
                pending.plan_hash = plan.identity();
                pending.grant_subject = plan.identity();
                pending.operation = match plan.operation {
                    RetainedOperation::Cleanup => wire::BindingOperation::Cleanup,
                    RetainedOperation::Inverse | RetainedOperation::RestoreDonor => {
                        wire::BindingOperation::Inverse
                    }
                    RetainedOperation::Revocation => wire::BindingOperation::Revocation,
                    RetainedOperation::Recovery => wire::BindingOperation::Recovery,
                };
                if plan.operation == RetainedOperation::RestoreDonor {
                    pending
                        .transfer
                        .as_mut()
                        .ok_or_else(|| refuse("missing donor transfer"))?
                        .phase = wire::TransferPhase::Restoring;
                }
                pending.revoked |= plan.operation == RetainedOperation::Revocation;
                pending.resources = plan.desired.clone();
                pending.phase = wire::BindingPhase::Prepared;
                pending.receipt_digest = None;
            }
            pending.semantics_digest =
                vibe_wire::behaviour::deploy_generic_state::semantics_digest(&pending)
                    .map_err(refuse)?;
            record.pending = Some(pending);
            vibe_wire::behaviour::deploy_generic_state::validate(&record).map_err(refuse)?;
            let owned = plan
                .desired
                .iter()
                .filter(|_| plan.operation != RetainedOperation::RestoreDonor)
                .filter_map(|r| match &r.desired {
                    wire::Value::Present(v) => {
                        Some(vibe_wire::generated::deploy_receipt::OwnedResource {
                            resource: resource(r).owned(self),
                            post_digest: v.blob.digest.clone(),
                        })
                    }
                    wire::Value::Absent(_) => None,
                })
                .collect::<Vec<_>>();
            let basis = match &plan.receipt {
                Some(r) => r.clone(),
                None => {
                    let p = record
                        .pending
                        .as_ref()
                        .ok_or_else(|| refuse("missing initial pending"))?;
                    use vibe_wire::generated::deploy_receipt::*;
                    DeployReceipt {
                        operation_binding: None,
                        applied_at: created_at.parse().map_err(refuse)?,
                        artifact_digest: "0".repeat(64),
                        desired_config_digest: p.grant_subject.clone(),
                        generation: p.generation,
                        identity: DeployIdentity {
                            project: self.roots[0].clone(),
                            package: None,
                        },
                        profile: "generic-host".into(),
                        provider: ProviderIdentity {
                            key: p.pins.provider.clone(),
                            version: None,
                            content_hash: None,
                        },
                        resources: Vec::new(),
                        reversible: false,
                        schema: 1,
                        scope: DestinationScope::User,
                        status: ReceiptStatus::Verified,
                        target: plan.target.clone(),
                        evidence: Some("initial retained recovery".into()),
                        finalized_at: None,
                        prior_state_handle: None,
                    }
                }
            };
            let transaction = super::super::transaction::Transaction {
                state: &state,
                home: &home,
                identity: &basis.identity,
                provider_pin: &basis.provider.key,
                scope: crate::mechanism::EffectClass::User,
                created_at,
            };
            let mut execute = |ledger: &mut super::super::state::CheckpointLedger<'_>| {
                let operation = (|| -> Result<(), GenericError> {
                    for row in &plan.desired {
                        self.current_roots()?;
                        if let Some(capture) = &plan.forward {
                            self.current(capture)?;
                        }
                        if self.authority.current_control().map_err(refuse)? != plan.control {
                            return Err(refuse("current control changed"));
                        }
                        let root = self.root(scope(row));
                        self.protect(scope(row), &row.destination)?;
                        let before = root
                            .read_file_snapshot_bounded(&row.destination, 4 * 1024 * 1024)
                            .map_err(refuse)?;
                        let expected = match &row.before {
                            wire::Before::Absent(_) => None,
                            wire::Before::Retained(v) => {
                                Some(state.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                            }
                            wire::Before::Unavailable(_) => {
                                if plan.recovery && plan.forward.is_some() {
                                    match &row.desired {
                                        wire::Value::Present(v) => Some(
                                            state
                                                .read_generic_blob(&home, &v.blob)
                                                .map_err(refuse)?,
                                        ),
                                        wire::Value::Absent(_) => None,
                                    }
                                } else {
                                    return Err(refuse("unavailable before state"));
                                }
                            }
                        };
                        let desired = match &row.desired {
                            wire::Value::Absent(_) => None,
                            wire::Value::Present(v) => {
                                Some(state.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                            }
                        };
                        let found = plan::observe_value(
                            before.as_ref().map(|b| b.bytes.as_slice()),
                            &resource(row),
                        )?;
                        if (found != expected && !(plan.recovery && found == desired))
                            || !retained_mode_matches(
                                row,
                                before.as_ref(),
                                found.as_deref(),
                                expected.as_deref(),
                                desired.as_deref(),
                                plan.recovery,
                            )
                        {
                            return Err(refuse("third digest before retained publication"));
                        }
                        let after = plan::restore_value(
                            before.as_ref().map(|b| b.bytes.as_slice()),
                            &resource(row),
                            desired.as_deref(),
                        )?;
                        receipt::progress_member(
                            self,
                            &plan.target,
                            &row.destination,
                            scope(row),
                            Some(&row.key),
                            false,
                        )?;
                        if before.as_ref().map(|b| &b.bytes) != after.as_ref() {
                            boundary.set(GenericEffect::Destinations);
                            match (&before, &after) {
                                (Some(snapshot), Some(bytes)) => {
                                    root.write_snapshot_guarded(&row.destination, snapshot, bytes)
                                        .map_err(|e| {
                                            refuse(format!("guarded retained publication: {e:?}"))
                                        })?;
                                }
                                (Some(snapshot), None) => {
                                    if root
                                        .read_file_snapshot_bounded(
                                            &row.destination,
                                            4 * 1024 * 1024,
                                        )
                                        .map_err(refuse)?
                                        .as_ref()
                                        != Some(snapshot)
                                    {
                                        return Err(refuse("removal snapshot changed"));
                                    }
                                    root.remove_file(&row.destination).map_err(refuse)?;
                                }
                                (None, Some(bytes)) => {
                                    let (parents, name) =
                                        vibe_safefs::split_relative(&row.destination)
                                            .map_err(refuse)?;
                                    let parts =
                                        parents.iter().map(String::as_str).collect::<Vec<_>>();
                                    let directory = root.dir(&parts, true).map_err(refuse)?;
                                    root.publish_new_in_with_mode(
                                        &directory,
                                        &name,
                                        bytes,
                                        if row.key.is_empty() {
                                            match &row.desired {
                                                wire::Value::Present(v) => Some(v.blob.mode),
                                                wire::Value::Absent(_) => None,
                                            }
                                        } else {
                                            None
                                        },
                                    )
                                    .map_err(|e| refuse(e.into_report()))?;
                                }
                                (None, None) => {}
                            }
                        }
                        let observed = root
                            .read_file_bounded(&row.destination, 4 * 1024 * 1024)
                            .map_err(refuse)?;
                        if plan::observe_value(observed.as_deref(), &resource(row))? != desired {
                            return Err(refuse("retained independent verify failed"));
                        }
                        if desired.is_some() {
                            ledger
                                .completed(&resource(row).owned(self))
                                .map_err(refuse)?;
                        }
                        receipt::progress_member(
                            self,
                            &plan.target,
                            &row.destination,
                            scope(row),
                            Some(&row.key),
                            true,
                        )?;
                    }
                    self.current_roots()?;
                    if let Some(capture) = &plan.forward {
                        self.current(capture)?;
                    }
                    if self.authority.current_control().map_err(refuse)? != plan.control {
                        return Err(refuse("current control changed after publication"));
                    }
                    Ok(())
                })();
                operation.map_err(|e| super::super::error::DeployError::RecordInvalid {
                    record: "generic retained operation",
                    reason: e.to_string(),
                })
            };
            if let Some(donor) = donor {
                donor.ensure_hold(&record).map_err(refuse)?;
                let t = record.pending.as_ref().unwrap().transfer.as_ref().unwrap();
                if record.pending.as_ref().unwrap().operation == wire::BindingOperation::Adoption
                    && (t.phase == wire::TransferPhase::Held
                        || t.phase == wire::TransferPhase::Retired)
                {
                    donor.retire(&t.hold_id).map_err(refuse)?;
                }
            }
            if let Some(donor) = deployment {
                donor.ensure_hold(&record)?;
                let t = record.pending.as_ref().unwrap().transfer.as_ref().unwrap();
                if record.pending.as_ref().unwrap().operation == wire::BindingOperation::Adoption
                    && matches!(
                        t.phase,
                        wire::TransferPhase::Held | wire::TransferPhase::Retired
                    )
                {
                    donor.retire(&t.hold_id)?;
                }
            }
            let applied = transaction
                .retained(
                    plan.receipt.as_ref(),
                    &basis,
                    record,
                    &locks,
                    &owned,
                    &mut execute,
                )
                .map_err(refuse)?;
            if plan.operation == RetainedOperation::RestoreDonor {
                if donor.is_none() && deployment.is_none() {
                    return Err(refuse("missing donor restoration owner"));
                }
                let mut finished = state.read_generic(&home).map_err(refuse)?.unwrap();
                let t = finished
                    .committed
                    .as_mut()
                    .unwrap()
                    .transfer
                    .as_mut()
                    .unwrap();
                if let Some(donor) = donor {
                    donor.restore(&t.hold_id).map_err(refuse)?;
                }
                if let Some(donor) = deployment {
                    donor.restore(&t.hold_id)?;
                }
                t.phase = wire::TransferPhase::Restored;
                state.write_generic(&home, &finished).map_err(refuse)?;
            }
            Ok(super::GenericOutcome {
                generation: applied.generation,
                resources: applied.resources.into_iter().map(|r| r.resource).collect(),
                effect: boundary.get(),
            })
        })();
        result.map_err(|mut e: GenericError| {
            e.effect = boundary.get();
            e
        })
    }
    pub fn inspect_receipt(&self, target: &str) -> Result<ReceiptInspection, GenericError> {
        self.current_roots()?;
        let Some(state) = DeployState::observe_bound(&self.settings).map_err(refuse)? else {
            return Ok(ReceiptInspection::Absent);
        };
        let home = self.home(target);
        let receipt = state.read_receipt(&home).map_err(refuse)?;
        match state.read_generic(&home).map_err(refuse)? {
            Some(record) => {
                for b in [record.committed.as_ref(), record.pending.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    if b.coordination_identity != self.arena_identity()? {
                        return Err(refuse("coordination arena identity mismatch"));
                    }
                }
                if let Some(p) = &record.pending {
                    match (&receipt, &record.committed) {
                        (None, None) if p.generation == 0 => {}
                        (Some(r), _) if r.generation == p.generation => {
                            if p.receipt_digest
                                .as_ref()
                                .is_some_and(|d| *d != digest(&encoded(r).unwrap_or_default()))
                                || r.desired_config_digest != p.grant_subject
                            {
                                return Err(refuse("changed published receipt join"));
                            }
                            let expected = p
                                .resources
                                .iter()
                                .filter(|_| {
                                    !p.transfer.as_ref().is_some_and(|t| {
                                        matches!(
                                            t.phase,
                                            wire::TransferPhase::Restoring
                                                | wire::TransferPhase::Restored
                                        )
                                    })
                                })
                                .filter_map(|row| match &row.desired {
                                    wire::Value::Present(v) => {
                                        Some((resource(row).owned(self), v.blob.digest.clone()))
                                    }
                                    wire::Value::Absent(_) => None,
                                })
                                .collect::<Vec<_>>();
                            if r.resources
                                .iter()
                                .map(|r| (r.resource.clone(), r.post_digest.clone()))
                                .collect::<Vec<_>>()
                                != expected
                            {
                                return Err(refuse("changed published ownership"));
                            }
                        }
                        (Some(r), Some(c))
                            if c.generation == r.generation
                                && c.receipt_digest == Some(digest(&encoded(r)?)) => {}
                        (Some(r), None)
                            if r.generation.checked_add(1) == Some(p.generation)
                                && p.operation == wire::BindingOperation::Adoption => {}
                        _ => return Err(refuse("changed pending receipt join")),
                    }
                }
                if record.pending.is_none() {
                    let c = record
                        .committed
                        .as_ref()
                        .ok_or_else(|| refuse("missing committed semantics"))?;
                    let r = receipt
                        .as_ref()
                        .ok_or_else(|| refuse("missing joined receipt"))?;
                    if c.generation != r.generation
                        || c.receipt_digest != Some(digest(&encoded(r)?))
                    {
                        return Err(refuse("changed generic receipt join"));
                    }
                }
                Ok(ReceiptInspection::Retained {
                    receipt,
                    state: record,
                })
            }
            None => Ok(match receipt {
                Some(receipt) => ReceiptInspection::Legacy {
                    receipt,
                    limitation: "no retained operations or before-images; explicit closed-semantics adoption required",
                },
                None => ReceiptInspection::Absent,
            }),
        }
    }
    pub fn prepare_retained(
        &self,
        target: &str,
        operation: RetainedOperation,
    ) -> Result<RetainedPlan, GenericError> {
        if operation == RetainedOperation::Recovery {
            return Err(refuse(
                "forward recovery requires prepare_recovery and current source",
            ));
        }
        let ReceiptInspection::Retained {
            receipt: Some(receipt),
            state,
        } = self.inspect_receipt(target)?
        else {
            return Err(refuse(
                "retained closed semantics unavailable; legacy is not reversible",
            ));
        };
        if state.pending.is_some() {
            return Err(refuse("pending operation requires recovery"));
        }
        let c = state
            .committed
            .as_ref()
            .ok_or_else(|| refuse("missing committed semantics"))?;
        if let Some(t) = &c.transfer {
            if t.phase == wire::TransferPhase::Restored || operation == RetainedOperation::Inverse {
                return Err(refuse("transfer requires exact donor restoration"));
            }
        } else if operation == RetainedOperation::RestoreDonor {
            return Err(refuse("no recorded donor"));
        }
        let control = self.authority.current_control().map_err(refuse)?;
        if control.principal.is_empty() || control.policy_generation.is_empty() {
            return Err(refuse("current control required"));
        }
        let store = DeployState::observe_bound(&self.settings)
            .map_err(refuse)?
            .ok_or_else(|| refuse("state disappeared"))?;
        let home = self.home(target);
        let mut desired = Vec::new();
        let mut rows = c.resources.clone();
        if operation == RetainedOperation::RestoreDonor {
            let t = c
                .transfer
                .as_ref()
                .ok_or_else(|| refuse("missing donor transfer"))?;
            for original in &t.resources {
                if !rows.iter().any(|r| {
                    r.scope == original.scope
                        && r.destination == original.destination
                        && r.key == original.key
                }) {
                    let mut absent = original.clone();
                    absent.desired = wire::Value::Absent(Box::new(wire::ValueAbsent {}));
                    rows.push(absent);
                }
            }
        }
        for row in &rows {
            let scope = scope(row);
            self.protect(scope, &row.destination)?;
            let root = self.root(scope);
            let snapshot = root
                .read_file_snapshot_bounded(&row.destination, 4 * 1024 * 1024)
                .map_err(refuse)?;
            if row.key.is_empty() {
                if let (Some(found), wire::Value::Present(v)) = (&snapshot, &row.desired) {
                    if found.unix_mode.is_some_and(|mode| mode != v.blob.mode) {
                        return Err(refuse("owned file mode changed"));
                    }
                }
            }
            let bytes = snapshot.map(|s| s.bytes);
            let resource = resource(row);
            let found = plan::observe_value(bytes.as_deref(), &resource)?;
            let expected = match &row.desired {
                wire::Value::Absent(_) => None,
                wire::Value::Present(v) => {
                    Some(store.read_generic_blob(&home, &v.blob).map_err(refuse)?)
                }
            };
            if found != expected {
                return Err(refuse("owned destination changed"));
            }
            let mut next = row.clone();
            next.desired = match operation {
                RetainedOperation::Recovery => return Err(refuse("use prepare_recovery")),
                RetainedOperation::Cleanup => wire::Value::Absent(Box::new(wire::ValueAbsent {})),
                RetainedOperation::Revocation => row.desired.clone(),
                RetainedOperation::RestoreDonor => c
                    .transfer
                    .as_ref()
                    .unwrap()
                    .resources
                    .iter()
                    .find(|r| {
                        r.scope == row.scope && r.destination == row.destination && r.key == row.key
                    })
                    .map(|r| r.desired.clone())
                    .unwrap_or_else(|| wire::Value::Absent(Box::new(wire::ValueAbsent {}))),
                RetainedOperation::Inverse => match &row.before {
                    wire::Before::Absent(_) => wire::Value::Absent(Box::new(wire::ValueAbsent {})),
                    wire::Before::Retained(b) => {
                        store.read_generic_blob(&home, &b.blob).map_err(refuse)?;
                        wire::Value::Present(Box::new(wire::ValuePresent {
                            blob: b.blob.clone(),
                        }))
                    }
                    wire::Before::Unavailable(_) => {
                        return Err(refuse("legacy before-state unavailable; non-reversible"));
                    }
                },
            };
            next.before = match &row.desired {
                wire::Value::Absent(_) => wire::Before::Absent(Box::new(wire::BeforeAbsent {})),
                wire::Value::Present(v) => wire::Before::Retained(Box::new(wire::BeforeRetained {
                    blob: v.blob.clone(),
                })),
            };
            next.progress = wire::ResourceProgress::NotStarted;
            desired.push(next);
        }
        let plan = RetainedPlan {
            target: target.into(),
            operation,
            state,
            receipt: Some(receipt),
            desired,
            control,
            roots: self.roots.clone(),
            protected: self.protected.clone(),
            host: self.instance,
            recovery: false,
            forward: None,
        };
        self.check_retained(&plan)?;
        Ok(plan)
    }
    fn check_retained(&self, plan: &RetainedPlan) -> Result<(), GenericError> {
        self.current_roots()?;
        if let Some(capture) = &plan.forward {
            self.current(capture)?;
        }
        if plan.host != self.instance
            || plan.roots != self.roots
            || plan.protected != self.protected
            || self.authority.current_control().map_err(refuse)? != plan.control
        {
            return Err(refuse("current retained operation authority changed"));
        }
        let observed = self.inspect_receipt(&plan.target)?;
        if observed
            != (ReceiptInspection::Retained {
                receipt: plan.receipt.clone(),
                state: plan.state.clone(),
            })
        {
            return Err(refuse("retained records changed"));
        }
        Ok(())
    }
    pub fn approve_retained(&self, plan: &RetainedPlan) -> Result<RetainedGrant, GenericError> {
        self.check_retained(plan)?;
        Ok(RetainedGrant { plan: plan.clone() })
    }
}
pub(super) fn scope(row: &wire::Resource) -> ClientScope {
    match row.scope {
        wire::ResourceScope::Project => ClientScope::Project,
        wire::ResourceScope::User => ClientScope::User,
    }
}
pub(super) fn resource(row: &wire::Resource) -> plan::GenericFootprint {
    plan::GenericFootprint {
        scope: scope(row),
        destination: row.destination.clone(),
        key: row.key.clone(),
        operation: match row.operation {
            wire::ResourceOperation::File => vibe_core::manifest::ClientOperation::ProjectFile,
            wire::ResourceOperation::JsonMember => {
                vibe_core::manifest::ClientOperation::ReconcileJsonMember
            }
            wire::ResourceOperation::TomlMember => {
                vibe_core::manifest::ClientOperation::ReconcileTomlMember
            }
        },
    }
}

fn retained_mode_matches(
    row: &wire::Resource,
    snapshot: Option<&vibe_safefs::StableFileSnapshot>,
    found: Option<&[u8]>,
    expected: Option<&[u8]>,
    desired: Option<&[u8]>,
    recovery: bool,
) -> bool {
    if !row.key.is_empty() || snapshot.is_none() {
        return true;
    }
    let actual = snapshot.and_then(|s| s.unix_mode);
    let desired_mode = match &row.desired {
        wire::Value::Present(v) => Some(v.blob.mode),
        wire::Value::Absent(_) => None,
    };
    let expected_mode = match &row.before {
        wire::Before::Retained(v) => Some(v.blob.mode),
        wire::Before::Unavailable(_) => desired_mode,
        wire::Before::Absent(_) => None,
    };
    (found == expected && actual == expected_mode)
        || (recovery && found == desired && actual == desired_mode)
}
