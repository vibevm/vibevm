use super::*;
use vibe_wire::generated::{
    deploy_client_state::DeployClientState,
    deploy_receipt::{
        DeployIdentity, DeployReceipt, DestinationScope, OwnedResource, ProviderIdentity,
        ReceiptStatus,
    },
};
impl Host {
    pub(super) fn save(
        &self,
        state: &DeployState,
        home: &DeploymentHome,
        p: &Prepared,
        r: &Reply,
        published: bool,
    ) -> Result<(), String> {
        vibe_wire::behaviour::client_deploy::validate_reply(r).map_err(err)?;
        let previous = state.read_client(home, &r.run_id).map_err(err)?;
        let file_subject = match previous {
            Some(ref old) => {
                if old.plan_id != p.id
                    || old.file_target.as_deref() != p.files.as_ref().map(Files::target)
                {
                    return Err("retained file/plan join changed".into());
                }
                old.file_subject.clone()
            }
            None => p.files.as_ref().map(Files::identity),
        };
        let record:DeployClientState=serde_json::from_value(serde_json::json!({"schema":1,"deployment":home.id(),"scope":self.scope,"plan_id":p.id,"run_id":r.run_id,"subject":p.subject,"reply":r,"published":published,"file_target":p.files.as_ref().map(Files::target),"file_subject":file_subject,"commands":p.commands.iter().map(|c|serde_json::json!({"resource":c.resource,"plan":c.prepared.inspect()})).collect::<Vec<_>>(),"receipt_digest":state.read_receipt(home).map_err(err)?.as_ref().map(|receipt|digest(&encoded(receipt)))})).map_err(err)?;
        state.write_client(home, &record).map_err(err)
    }
    /// Reconcile only an exact receipt whose publication basis was durable first.
    /// Caller holds the incumbent arena, deployment and destination locks.
    pub(super) fn reconcile_finalization(
        &self,
        state: &DeployState,
        home: &DeploymentHome,
        p: &Prepared,
        record: &DeployClientState,
    ) -> Result<bool, String> {
        let Some(expected) = &record.finalization_digest else {
            return Ok(false);
        };
        let receipt = state
            .read_receipt(home)
            .map_err(err)?
            .ok_or("finalization receipt absent")?;
        if digest(&encoded(&receipt)) != *expected {
            return Err("foreign finalization receipt".into());
        }
        let generation = record
            .finalization_generation
            .ok_or("missing finalization generation")?;
        if receipt.generation != generation
            || receipt.desired_config_digest != p.id
            || receipt.target != self.target
            || receipt.identity.project != self.roots[0]
            || record.plan_id != p.id
        {
            return Err("foreign finalization identity".into());
        }
        let locks = state
            .read_lock_resources(home)
            .map_err(err)?
            .ok_or("missing finalization sidecar")?;
        let binding = locks
            .pending
            .as_ref()
            .or(locks.committed.as_ref())
            .ok_or("missing finalization binding")?;
        let resources: Vec<_> = record
            .reply
            .resources
            .iter()
            .map(|r| r.resource.clone())
            .collect();
        if !binding.matches(generation, &p.id)
            || binding.resources != resources
            || binding.operation_binding.is_some()
            || binding.native.is_some()
        {
            return Err("foreign finalization sidecar".into());
        }
        let intent = state.read_intent(home).map_err(err)?;
        if let Some(intent) = &intent {
            let resources: Vec<_> = record
                .reply
                .resources
                .iter()
                .filter(|r| r.desired != "absent")
                .map(|r| {
                    (
                        r.resource.clone(),
                        r.desired.clone(),
                        (r.prior != "absent").then(|| r.prior.clone()),
                    )
                })
                .collect();
            let intended: Vec<_> = intent
                .resources
                .iter()
                .map(|r| {
                    (
                        r.resource.clone(),
                        r.desired_digest.clone(),
                        r.prior_digest.clone(),
                    )
                })
                .collect();
            if intent.plan_hash != p.id
                || intent.target.generation != generation
                || intent.target.project != self.roots[0]
                || intent.target.target != self.target
                || intent.target.profile != "client-deploy"
                || intent.target.package.is_some()
                || intent.operation_binding.is_some()
                || intended != resources
            {
                return Err("foreign finalization intent".into());
            }
        } else if record.reply.status
            != vibe_wire::generated::deploy_client_state::ReplyStatus::Completed
            || locks.pending.is_some()
        {
            return Err("missing finalization intent".into());
        }
        if locks.pending.is_some() {
            super::super::sidecar::promote(state, home, generation, &p.id).map_err(err)?;
        }
        let reply = vibe_wire::behaviour::client_deploy::read_reply(&encoded(&record.reply))
            .map_err(err)?;
        if reply.status == ReplyStatus::Completed {
            state.retire_intent(home).map_err(err)?;
        }
        self.save(state, home, p, &reply, true)?;
        Ok(true)
    }
    pub(in crate::mechanism::deploy) fn transact(
        &self,
        state: &DeployState,
        home: &DeploymentHome,
        p: &Prepared,
        grant: &Approval,
        mut r: Reply,
        recovery: bool,
        cancel: &Cancellation,
        clock: &str,
    ) -> Result<Reply, String> {
        let prior = state.read_receipt(home).map_err(err)?;
        for (id, receipt) in state.receipts_bound().map_err(err)? {
            if id != home.id()
                && receipt.resources.iter().any(|owned| {
                    p.commands
                        .iter()
                        .any(|command| command.resource == owned.resource)
                })
            {
                return Err("foreign deployment owns resource".into());
            }
        }
        if !recovery && let Err(reason) = self.command_ownership(p) {
            r.status = ReplyStatus::Refused;
            r.message = reason;
            self.save(state, home, p, &r, true)?;
            return Ok(r);
        }
        let outstanding = state.read_intent(home).map_err(err)?;
        if outstanding.as_ref().is_some_and(|v| v.plan_hash != p.id) {
            r.status = ReplyStatus::Refused;
            r.message = "existing deployment intent requires its own recovery".into();
            self.save(state, home, p, &r, true)?;
            return Ok(r);
        }
        let generation = outstanding.as_ref().map_or_else(
            || prior.as_ref().map_or(1, |v| v.generation.saturating_add(1)),
            |v| v.target.generation,
        );
        let stamp = clock.parse().map_err(err)?;
        super::super::sidecar::stage_pending(
            state,
            home,
            generation,
            &p.id,
            &r.resources
                .iter()
                .map(|r| r.resource.clone())
                .collect::<Vec<_>>(),
            None,
        )
        .map_err(err)?;
        let intent: vibe_wire::generated::deploy_intent::DeployIntent=serde_json::from_value(serde_json::json!({
            "schema":1,"plan_hash":p.id,"resources":r.resources.iter().filter(|v|v.desired!="absent").map(|v|serde_json::json!({"resource":v.resource,"desired_digest":v.desired,"prior_digest":if v.prior=="absent" {None}else{Some(&v.prior)}})).collect::<Vec<_>>(),
            "target":{"project":self.roots[0],"profile":"client-deploy","target":self.target,"generation":generation},"started_at":clock
        })).map_err(err)?;
        state.write_intent(home, &intent).map_err(err)?;
        let mut ledger =
            super::super::state::CheckpointLedger::open(state, home, &p.id).map_err(err)?;
        r.status = ReplyStatus::Partial;
        r.message.clear();
        for command in 0..p.commands.len() {
            let i = p.files.as_ref().map_or(0, |files| files.resources().len()) + command;
            match self.observe(&r.resources) {
                Ok(rows) => r.resources = rows,
                Err(e) => {
                    for row in &mut r.resources {
                        row.state = ResourceState::Unavailable;
                    }
                    r.status = ReplyStatus::Unknown;
                    r.message = e;
                    break;
                }
            }
            if r.resources
                .iter()
                .any(|row| row.state == ResourceState::Third)
            {
                r.status = ReplyStatus::Refused;
                r.message = "third scoped state; prefix retained".into();
                break;
            }
            let row = r
                .resources
                .iter()
                .find(|v| v.resource == r.steps[i].resource)
                .ok_or("step resource")?;
            if r.steps[i].status == StepStatus::Completed {
                if row.state != ResourceState::Desired {
                    r.status = ReplyStatus::Refused;
                    r.message = "completed prefix drifted".into();
                    break;
                }
                continue;
            }
            if matches!(
                r.steps[i].status,
                StepStatus::Attempted | StepStatus::Uncertain
            ) {
                if !recovery {
                    for row in &mut r.resources {
                        row.state = ResourceState::Unavailable;
                    }
                    r.status = ReplyStatus::Unknown;
                    r.message = "explicit recovery required".into();
                    break;
                }
                if row.state == ResourceState::Desired {
                    r.steps[i].status = StepStatus::Completed;
                    ledger.completed(&r.steps[i].resource).map_err(err)?;
                    self.save(state, home, p, &r, false)?;
                    continue;
                }
                if !r.steps[i].retry_safe {
                    r.status = ReplyStatus::Refused;
                    r.message = "uncertain command has no safe retry capability".into();
                    break;
                }
            } else if row.observed != row.prior {
                r.status = ReplyStatus::NeedsPreparation;
                r.message = "unattempted resource changed".into();
                break;
            }
            if cancel.pending() {
                r.cancel_pending = true;
                r.status = ReplyStatus::Cancelled;
                r.message = "stopped at durable pre-dispatch boundary".into();
                break;
            }
            if let Err(e) = self.check(p) {
                r.status = ReplyStatus::NeedsAuthority;
                r.authority = ReplyAuthority::Stale;
                r.message = e;
                break;
            }
            r.steps[i].status = StepStatus::Attempted;
            self.save(state, home, p, &r, false)?;
            let result = self.commands.execute(
                &p.commands[command].prepared,
                Some(&grant.commands[command]),
            );
            r.cancel_pending = cancel.pending();
            let observation = self.observe(&r.resources);
            match observation {
                Ok(rows) => r.resources = rows,
                Err(e) => {
                    r.steps[i].status = StepStatus::Uncertain;
                    for row in &mut r.resources {
                        row.state = ResourceState::Unavailable;
                    }
                    r.status = ReplyStatus::Unknown;
                    r.message = e;
                    break;
                }
            }
            let landed = r
                .resources
                .iter()
                .find(|row| row.resource == r.steps[i].resource)
                .is_some_and(|row| row.state == ResourceState::Desired);
            if landed {
                r.steps[i].status = StepStatus::Completed;
                ledger.completed(&r.steps[i].resource).map_err(err)?;
            } else {
                r.steps[i].status = StepStatus::Uncertain;
            }
            self.save(state, home, p, &r, false)?;
            if !result.as_ref().is_ok_and(|output| output.code == Some(0)) || !landed {
                r.status = if landed {
                    ReplyStatus::Partial
                } else {
                    ReplyStatus::Unknown
                };
                r.message =
                    "process failure or independent mismatch; observed prefix retained".into();
                break;
            }
            if r.cancel_pending {
                r.status = ReplyStatus::Cancelled;
                r.message = "stopped after observed command boundary".into();
                break;
            }
        }
        if r.steps.iter().all(|s| s.status == StepStatus::Completed)
            && !matches!(
                r.status,
                ReplyStatus::Unknown
                    | ReplyStatus::Cancelled
                    | ReplyStatus::NeedsAuthority
                    | ReplyStatus::Refused
            )
            && r.message.is_empty()
        {
            match self.observe(&r.resources) {
                Ok(rows) => {
                    r.resources = rows;
                    if r.resources
                        .iter()
                        .all(|row| row.state == ResourceState::Desired)
                    {
                        r.status = ReplyStatus::Completed;
                    } else {
                        r.status = ReplyStatus::Partial;
                        r.message = "complete independent verification failed".into()
                    }
                }
                Err(e) => {
                    for row in &mut r.resources {
                        row.state = ResourceState::Unavailable;
                    }
                    r.status = ReplyStatus::Unknown;
                    r.message = e
                }
            }
        }
        // The incumbent receipt owns observed effects even on partial failure.
        let receipt = DeployReceipt {
            schema: 1,
            operation_binding: None,
            applied_at: stamp,
            finalized_at: Some(stamp),
            artifact_digest: "0".repeat(64),
            desired_config_digest: p.id.clone(),
            generation,
            identity: DeployIdentity {
                project: self.roots[0].clone(),
                package: None,
            },
            profile: "client-deploy".into(),
            provider: ProviderIdentity {
                key: "org.vibevm/vibe#client-deploy".into(),
                version: None,
                content_hash: None,
            },
            resources: {
                let mut owned = prior
                    .as_ref()
                    .map(|p| p.resources.clone())
                    .unwrap_or_default();
                for row in &r.resources {
                    if p.commands.iter().any(|c| c.resource == row.resource)
                        && row.state == ResourceState::Desired
                        && r.steps.iter().any(|s| {
                            s.resource == row.resource && s.status == StepStatus::Completed
                        })
                    {
                        owned.retain(|v| v.resource != row.resource);
                        if row.observed != "absent" {
                            owned.push(OwnedResource {
                                resource: row.resource.clone(),
                                post_digest: row.observed.clone(),
                            });
                        }
                    }
                }
                owned
            },
            reversible: false,
            scope: DestinationScope::User,
            status: if r.status == ReplyStatus::Completed {
                ReceiptStatus::Verified
            } else {
                ReceiptStatus::Failed
            },
            target: self.target.clone(),
            evidence: Some(format!("client run {}; plan {}", r.run_id, p.id)),
            prior_state_handle: None,
        };
        // The exact future receipt and final reply are durable before publication.
        self.save(state, home, p, &r, false)?;
        let mut basis = state
            .read_client(home, &r.run_id)
            .map_err(err)?
            .ok_or("missing finalization basis")?;
        basis.finalization_digest = Some(digest(&encoded(&receipt)));
        basis.finalization_generation = Some(generation);
        state.write_client(home, &basis).map_err(err)?;
        state.write_receipt(home, &receipt).map_err(err)?;
        #[cfg(test)]
        finalization_fault(1)?;
        super::super::sidecar::promote(state, home, generation, &p.id).map_err(err)?;
        #[cfg(test)]
        finalization_fault(2)?;
        if r.status == ReplyStatus::Completed {
            state.retire_intent(home).map_err(err)?;
        }
        #[cfg(test)]
        finalization_fault(3)?;
        self.save(state, home, p, &r, true)?;
        Ok(r)
    }
}

#[cfg(test)]
thread_local! { pub(super) static FINALIZATION_FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
fn finalization_fault(stage: u8) -> Result<(), String> {
    FINALIZATION_FAULT.with(|f| {
        if f.get() == stage {
            f.set(0);
            Err(format!("interrupted finalization stage {stage}"))
        } else {
            Ok(())
        }
    })
}
