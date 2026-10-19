use super::super::model::{ClientExecutable, ClientExecutables};
use super::super::protocol::*;
use super::super::state::{CheckpointLedger, DeployState};
use super::super::transaction::Transaction;
use super::*;
use crate::mechanism::{
    DeployProvider, EffectClass, MechanismError, NetworkUse, PrivilegeNeed, ProviderDescriptor,
    ProviderOperation, Reversibility,
};
use std::cell::Cell;
use vibe_wire::generated::deploy_receipt::DeployIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericOutcome {
    pub generation: u32,
    pub resources: Vec<String>,
    pub effect: GenericEffect,
}
impl GenericHost {
    pub fn apply(
        &self,
        plan: &GenericPlan,
        grant: Option<&ExactGrant>,
        created_at: &str,
    ) -> Result<GenericOutcome, GenericError> {
        if plan.transfer.is_some() {
            return Err(refuse("transfer requires exact donor owner"));
        }
        self.apply_inner(plan, grant, created_at, None, None)
    }
    pub(super) fn apply_inner(
        &self,
        plan: &GenericPlan,
        grant: Option<&ExactGrant>,
        created_at: &str,
        donor: Option<&vibe_agent_projection::pkgskill::SkillDonor>,
        deployment: Option<&adoption::DeploymentDonor>,
    ) -> Result<GenericOutcome, GenericError> {
        // Even the clock is checked before lock/state infrastructure.
        self.check_grant(plan, grant)?;
        self.check_state_identity(plan)?;
        created_at
            .parse::<vibe_wire::generated::deploy_receipt::Rfc3339Timestamp>()
            .map_err(refuse)?;
        let boundary = Cell::new(GenericEffect::Infrastructure);
        let result = (|| {
            #[cfg(test)]
            BEFORE_DOCUMENT_LOCK_HOOK.with(|hook| {
                if let Some(h) = hook.borrow_mut().take() {
                    h();
                }
            });
            self.current_roots()?;
            let _arena = self
                .arena()
                .lock("deploy-physical-coordination.lock")
                .map_err(refuse)?;
            self.current_roots()?;
            let _deployment_donor_lease = deployment.map(|d| d.lock()).transpose()?;
            if let Some(d) = deployment {
                d.validate()?;
            }
            let _donor_lease = donor.map(|d| d.lock()).transpose().map_err(refuse)?;
            if let Some(d) = donor {
                d.validate().map_err(refuse)?;
            }
            let state = DeployState::open_bound(&self.settings).map_err(refuse)?;
            if let Some(expected) = &plan.state_identity {
                if state.generic_root_identity().map_err(refuse)? != *expected {
                    return Err(refuse("state home changed while opening apply"));
                }
            }
            let home = self.home(&plan.target);
            let _deployment = state.lock_deployment(&home).map_err(refuse)?;
            let _destinations = state.lock_destinations(&plan.locks(self)).map_err(refuse)?;
            #[cfg(test)]
            LOCKED_HOOK.with(|hook| {
                if let Some(h) = hook.borrow_mut().take() {
                    h();
                }
            });
            state.verify_bound_home(&self.settings).map_err(refuse)?;
            self.check_grant(plan, grant)?;
            if state.read_intent(&home).map_err(refuse)?.is_some() {
                return Err(refuse(
                    "interrupted generic transaction requires separately admitted C3 recovery",
                ));
            }
            let prior = state.read_receipt(&home).map_err(refuse)?;
            let planned_prior = plan
                .receipts
                .iter()
                .find(|(id, _)| id == home.id())
                .map(|(_, r)| r);
            if prior.as_ref() != planned_prior {
                return Err(refuse("ownership changed under deployment lock"));
            }
            super::super::ownership::refuse_generic_foreign(
                &state.receipts_bound().map_err(refuse)?.into_iter().filter(|(id,_)| !plan.transfer.as_ref().is_some_and(|t|t.family == vibe_wire::generated::deploy_generic_state::TransferFamily::Deployment && t.donor_id == *id)).collect::<Vec<_>>(),
                home.id(),
                self,
                &plan.resources,
            )
            .map_err(refuse)?;
            let target = vibe_core::manifest::DeployTarget {
                id: plan.target.clone(),
                artifact: String::new(),
                mechanism: "deploy:generic-host".parse().map_err(refuse)?,
                provider: None,
                when: None,
                depends_on: None,
                config: None,
            };
            let missing = || ClientExecutable::Missing {
                command: "not-admitted".into(),
            };
            let clients = ClientExecutables {
                claude: missing(),
                codex: missing(),
                opencode: missing(),
            };
            let request = DeployTargetRequest {
                target: &target,
                profile: "generic-host",
                project_root: self.project.root_path(),
                settings_root: self.settings.root_path(),
                user_home: self.user.root_path(),
                clients: &clients,
                prior_receipt: prior.as_ref(),
                recovery_intent: None,
                artifact: None,
                staging: None,
            };
            let identity = DeployIdentity {
                project: self.roots[0].clone(),
                package: None,
            };
            let provider = ClosedProvider {
                host: self,
                plan,
                boundary: &boundary,
            };
            let transaction = Transaction {
                state: &state,
                home: &home,
                identity: &identity,
                provider_pin: &plan.admitted.capture.observation.identity.provider,
                scope: EffectClass::User,
                created_at,
            };
            let deployment_plan = plan.deploy_plan(self);
            receipt::stage(
                self,
                &state,
                &home,
                plan,
                super::super::transaction::plan_hash(
                    &request,
                    &plan.admitted.capture.observation.identity.provider,
                    &deployment_plan,
                ),
            )?;
            if let Some(donor) = donor {
                let mut record = state
                    .read_generic(&home)
                    .map_err(refuse)?
                    .ok_or_else(|| refuse("missing transfer pending"))?;
                donor.hold(&record).map_err(refuse)?;
                donor.retire(&plan.identity()).map_err(refuse)?;
                record
                    .pending
                    .as_mut()
                    .unwrap()
                    .transfer
                    .as_mut()
                    .unwrap()
                    .phase = vibe_wire::generated::deploy_generic_state::TransferPhase::Retired;
                state.write_generic(&home, &record).map_err(refuse)?;
            }
            if let Some(donor) = deployment {
                let mut record = state.read_generic(&home).map_err(refuse)?.unwrap();
                donor.hold(&record)?;
                donor.retire(&plan.identity())?;
                record
                    .pending
                    .as_mut()
                    .unwrap()
                    .transfer
                    .as_mut()
                    .unwrap()
                    .phase = vibe_wire::generated::deploy_generic_state::TransferPhase::Retired;
                state.write_generic(&home, &record).map_err(refuse)?;
            }
            let applied = transaction
                .apply_generic(&provider, &request, &deployment_plan)
                .map_err(refuse)?;
            self.current(&plan.admitted.capture)?;
            if self.authority.current_control().map_err(refuse)? != plan.control {
                return Err(refuse("control drift after publication"));
            }
            state.verify_bound_home(&self.settings).map_err(refuse)?;
            Ok(GenericOutcome {
                generation: applied.generation,
                resources: applied.resources.into_iter().map(|r| r.resource).collect(),
                effect: boundary.get(),
            })
        })();
        result.map_err(|mut error: GenericError| {
            error.effect = boundary.get();
            error
        })
    }
}

struct ClosedProvider<'a> {
    host: &'a GenericHost,
    plan: &'a GenericPlan,
    boundary: &'a Cell<GenericEffect>,
}
fn mechanism(error: impl ToString) -> MechanismError {
    MechanismError::GenericHost {
        reason: error.to_string(),
    }
}
impl ClosedProvider<'_> {
    fn current(&self) -> Result<(), MechanismError> {
        self.host
            .current(&self.plan.admitted.capture)
            .map_err(mechanism)?;
        if self.host.authority.current_control().map_err(mechanism)? != self.plan.control {
            return Err(mechanism("current policy/principal drift"));
        }
        Ok(())
    }
}
impl DeployProvider for ClosedProvider<'_> {
    fn descriptor(&self) -> DeployDescriptor<'_> {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: &self.plan.admitted.capture.observation.identity.provider,
                kinds: &[],
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: Reversibility::Irreversible,
                operations: &[
                    ProviderOperation::Plan,
                    ProviderOperation::Apply,
                    ProviderOperation::Verify,
                    ProviderOperation::Fingerprint,
                ],
            },
            atomic_replacement: true,
            reference_ownership: true,
        }
    }
    fn plan(&self, _: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        Ok(self.plan.deploy_plan(self.host))
    }
    fn fingerprint(
        &self,
        _: &DeployTargetRequest<'_>,
        _: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.current()?;
        Ok(DeployFingerprint {
            digest: self.plan.identity(),
            summary: "captured source and separately admitted payload".into(),
        })
    }
    fn apply(
        &self,
        _: &DeployTargetRequest<'_>,
        _: &DeployPlan,
        ledger: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        // Recheck ALL destinations before the first external write. Planning
        // merged distinct members into one whole-document publication.
        for d in &self.plan.documents {
            let now = self
                .host
                .root(d.scope)
                .read_file_snapshot_bounded(&d.destination, 4 * 1024 * 1024)
                .map_err(mechanism)?;
            if now != d.before {
                return Err(mechanism("destination changed after locked validation"));
            }
        }
        for d in &self.plan.documents {
            self.current()?;
            receipt::progress(self.host, &self.plan.target, &d.destination, d.scope, false)
                .map_err(mechanism)?;
            let root = self.host.root(d.scope);
            if d.after.is_none() {
                let expected = d
                    .before
                    .as_ref()
                    .ok_or_else(|| mechanism("missing removal preimage"))?;
                if root
                    .read_file_snapshot_bounded(&d.destination, 4 * 1024 * 1024)
                    .map_err(mechanism)?
                    .as_ref()
                    != Some(expected)
                {
                    return Err(mechanism("removal snapshot changed"));
                }
                self.boundary.set(GenericEffect::Destinations);
                root.remove_file(&d.destination).map_err(mechanism)?;
                receipt::progress(self.host, &self.plan.target, &d.destination, d.scope, true)
                    .map_err(mechanism)?;
                continue;
            }
            let after = d.after.as_ref().expect("present document");
            let publication = if let Some(expected) = &d.before {
                // Existing documents retain their mode and must still name
                // the exact snapshot under the cooperating document lock.
                match root.write_snapshot_guarded(&d.destination, expected, after) {
                    Ok(_) => Ok(()),
                    Err(vibe_safefs::GuardedPublishError::Publication(error)) => Err(error),
                    Err(error) => {
                        return Err(mechanism(format!("guarded destination changed: {error:?}")));
                    }
                }
            } else {
                // Absence is not replacement authority. The incumbent
                // create-new publication preserves a racing foreign occupant.
                let (parents, name) =
                    vibe_safefs::split_relative(&d.destination).map_err(mechanism)?;
                let components = parents.iter().map(String::as_str).collect::<Vec<_>>();
                if root
                    .dir_if_present(&components)
                    .map_err(mechanism)?
                    .is_none()
                {
                    // A failed directory walk may have created a prefix.
                    self.boundary.set(GenericEffect::Destinations);
                }
                let directory = root.dir(&components, true).map_err(mechanism)?;
                // Retained semantics stage absent-file desired blobs as 0600.
                // Publish that exact mode too; ambient umask must not make a
                // just-created file immediately fail retained cleanup.
                root.publish_new_in_with_mode(&directory, &name, after, Some(0o600))
            };
            match publication {
                Ok(_) => self.boundary.set(GenericEffect::Destinations),
                Err(error) => {
                    if error.stage == vibe_safefs::PublishStage::PossiblyPublished
                        || !error.created_directories.is_empty()
                    {
                        self.boundary.set(GenericEffect::Destinations);
                    }
                    return Err(mechanism(format!(
                        "{}: {:?}; created directories {:?}; {}",
                        d.destination, error.stage, error.created_directories, error.source
                    )));
                }
            }
            for r in self
                .plan
                .resources
                .iter()
                .filter(|r| r.scope == d.scope && r.destination == d.destination)
            {
                ledger.completed(&r.owned(self.host)).map_err(mechanism)?;
            }
            receipt::progress(self.host, &self.plan.target, &d.destination, d.scope, true)
                .map_err(mechanism)?;
        }
        for r in &self.plan.removed {
            let bytes = self
                .host
                .root(r.scope)
                .read_file_bounded(&r.destination, 4 * 1024 * 1024)
                .map_err(mechanism)?;
            if plan::observe_value(bytes.as_deref(), r)
                .map_err(mechanism)?
                .is_some()
            {
                return Err(mechanism("removed resource independently remains"));
            }
        }
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: "closed host writes through retained SafeFS; no process capability".into(),
        })
    }
    fn verify(
        &self,
        _: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.current()?;
        resources
            .iter()
            .map(|name| {
                let r = self
                    .plan
                    .resources
                    .iter()
                    .find(|r| r.owned(self.host) == *name)
                    .ok_or_else(|| mechanism("verify resource outside exact grant"))?;
                let bytes = self
                    .host
                    .root(r.scope)
                    .read_file_bounded(&r.destination, 4 * 1024 * 1024)
                    .map_err(mechanism)?;
                let value = plan::observe_value(bytes.as_deref(), r).map_err(mechanism)?;
                Ok(ObservedResource {
                    resource: name.clone(),
                    digest: value.map(|b| digest(&b)),
                })
            })
            .collect()
    }
    fn remove(
        &self,
        _: &DeployTargetRequest<'_>,
        _: &[String],
        _: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        Err(mechanism("generic inverse requires C3 authority"))
    }
    fn recover(
        &self,
        _: &DeployTargetRequest<'_>,
        _: &DeployPlan,
        _: &[ObservedResource],
        _: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(mechanism("generic recovery requires C3 authority"))
    }
}
#[cfg(test)]
thread_local! {pub(super) static LOCKED_HOOK:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}

#[cfg(test)]
thread_local! {pub(super) static BEFORE_DOCUMENT_LOCK_HOOK:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}
