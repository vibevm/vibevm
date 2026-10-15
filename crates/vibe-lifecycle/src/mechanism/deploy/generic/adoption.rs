//! Explicit donor-owned handoff. Only the named donor may validate and retire
//! its receipt; equal bytes and descriptor declarations never confer ownership.
use super::*;
use vibe_agent_projection::pkgskill::SkillDonor;
use vibe_wire::generated::deploy_generic_state as wire;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TransferPlan {
    pub family: wire::TransferFamily,
    pub donor_id: String,
    pub donor_root_identity: String,
    pub receipt: Vec<u8>,
    pub resources: Vec<GenericResource>,
}
impl GenericHost {
    pub fn prepare_skill_adoption(
        &self,
        admitted: &AdmittedClient,
        donor: &SkillDonor,
    ) -> Result<GenericPlan, GenericError> {
        donor.validate().map_err(refuse)?;
        let donor_scope = if donor.root() == self.project.root_path() {
            ClientScope::Project
        } else if donor.root() == self.user.root_path() {
            ClientScope::User
        } else {
            return Err(refuse("donor root differs from recipient capabilities"));
        };
        if donor.root_identity()
            != self
                .root(donor_scope)
                .root_correlation_token()
                .map_err(refuse)?
        {
            return Err(refuse("donor capability differs from recipient root"));
        }
        self.plan_mode(
            admitted,
            true,
            Some(TransferPlan {
                family: donor.family(),
                donor_id: donor.id().into(),
                donor_root_identity: donor.root_identity().into(),
                receipt: donor.receipt_bytes().to_vec(),
                resources: donor
                    .resources()
                    .iter()
                    .map(|(p, d)| GenericResource {
                        scope: donor_scope,
                        destination: p.clone(),
                        key: Vec::new(),
                        operation: vibe_core::manifest::ClientOperation::ProjectFile,
                        desired_digest: d.clone(),
                    })
                    .collect(),
            }),
        )
    }
    pub fn adopt_skill(
        &self,
        plan: &GenericPlan,
        grant: Option<&ExactGrant>,
        donor: &SkillDonor,
        created_at: &str,
    ) -> Result<GenericOutcome, GenericError> {
        let t = plan
            .transfer
            .as_ref()
            .ok_or_else(|| refuse("missing exact transfer plan"))?;
        if t.donor_id != donor.id()
            || t.donor_root_identity != donor.root_identity()
            || t.family != donor.family()
            || t.receipt != donor.receipt_bytes()
        {
            return Err(refuse("changed donor snapshot"));
        }
        donor.validate().map_err(refuse)?;
        self.apply_inner(plan, grant, created_at, Some(donor), None)
    }
}

pub struct DeploymentDonor {
    settings: Project,
    state: super::super::state::DeployState,
    home: super::super::state::DeploymentHome,
    receipt: vibe_wire::generated::deploy_receipt::DeployReceipt,
    bytes: Vec<u8>,
}
impl GenericHost {
    /// Address the incumbent receipt by its logical execution identity. This
    /// observes its existing home; it never relocates a legacy receipt into a
    /// home derived from the generic host's physical root correlation.
    pub fn deployment_donor(
        &self,
        project: &str,
        target: &str,
        package: Option<&str>,
    ) -> Result<DeploymentDonor, GenericError> {
        let state = super::super::state::DeployState::observe_bound(&self.settings)
            .map_err(refuse)?
            .ok_or_else(|| refuse("missing donor state"))?;
        let home = super::super::state::DeploymentHome::new(
            &self
                .settings
                .root_path()
                .join(super::super::model::DEPLOY_STATE_DIR),
            project,
            package,
            target,
        );
        if state.read_generic(&home).map_err(refuse)?.is_some() {
            return Err(refuse("unrelated generic export unsupported"));
        }
        let receipt = if let Some(hold) = state.read_transfer_hold(&home).map_err(refuse)? {
            let t = hold
                .pending
                .as_ref()
                .and_then(|b| b.transfer.as_ref())
                .ok_or_else(|| refuse("missing donor transfer"))?;
            if t.family != wire::TransferFamily::Deployment
                || t.donor_id != home.id()
                || t.donor_root_identity != state.generic_root_identity().map_err(refuse)?
            {
                return Err(refuse("retained deployment donor capability mismatch"));
            }
            serde_json::from_slice(
                &state
                    .read_generic_blob(&home, &t.receipt_blob)
                    .map_err(refuse)?,
            )
            .map_err(refuse)?
        } else {
            state
                .read_receipt(&home)
                .map_err(refuse)?
                .ok_or_else(|| refuse("missing donor receipt"))?
        };
        if receipt.identity.project != project
            || receipt.identity.package.as_deref() != package
            || receipt.target != target
        {
            return Err(refuse("deployment donor identity mismatch"));
        }
        let bytes = encoded(&receipt)?;
        Ok(DeploymentDonor {
            settings: Project::open(self.settings.root_path()).map_err(refuse)?,
            state,
            home,
            receipt,
            bytes,
        })
    }
    pub fn prepare_deployment_adoption(
        &self,
        admitted: &AdmittedClient,
        donor: &DeploymentDonor,
    ) -> Result<GenericPlan, GenericError> {
        donor
            .state
            .verify_bound_home(&self.settings)
            .map_err(refuse)?;
        donor.validate()?;
        let mut rows = Vec::new();
        for old in &donor.receipt.resources {
            let row = if let Some(json) = old.resource.strip_prefix("generic:") {
                let (root, path, key): (String, String, Vec<String>) =
                    serde_json::from_str(json).map_err(refuse)?;
                let scope = if std::path::Path::new(&root) == self.project.root_path() {
                    ClientScope::Project
                } else if std::path::Path::new(&root) == self.user.root_path() {
                    ClientScope::User
                } else {
                    return Err(refuse("foreign donor root"));
                };
                let binding = admitted
                    .capture
                    .observation
                    .request
                    .bindings
                    .iter()
                    .find(|b| {
                        b.scope == scope
                            && (b.destination == path
                                || (b.shape == vibe_core::manifest::ClientShape::Tree
                                    && path.starts_with(&(b.destination.clone() + "/"))))
                            && b.key == key
                    })
                    .ok_or_else(|| refuse("missing explicit donor semantics"))?;
                GenericResource {
                    scope,
                    destination: path,
                    key,
                    operation: binding.operations[0],
                    desired_digest: old.post_digest.clone(),
                }
            } else {
                let (scope, path) = if let Some(p) = old.resource.strip_prefix("home:") {
                    (ClientScope::User, p.to_string())
                } else {
                    (
                        ClientScope::Project,
                        std::path::Path::new(&old.resource)
                            .strip_prefix(self.project.root_path())
                            .map_err(refuse)?
                            .to_str()
                            .ok_or_else(|| refuse("donor path encoding"))?
                            .to_string(),
                    )
                };
                let (destination, key) = if let Some((p, k)) = path.split_once("#mcp/") {
                    (p.to_string(), vec!["mcp".into(), k.into()])
                } else {
                    (path, Vec::new())
                };
                GenericResource {
                    scope,
                    destination,
                    key: key.clone(),
                    operation: if key.is_empty() {
                        vibe_core::manifest::ClientOperation::ProjectFile
                    } else {
                        vibe_core::manifest::ClientOperation::ReconcileJsonMember
                    },
                    desired_digest: old.post_digest.clone(),
                }
            };
            rows.push(row);
        }
        self.plan_mode(
            admitted,
            true,
            Some(TransferPlan {
                family: wire::TransferFamily::Deployment,
                donor_id: donor.home.id().into(),
                donor_root_identity: donor.state.generic_root_identity().map_err(refuse)?,
                receipt: donor.bytes.clone(),
                resources: rows,
            }),
        )
    }
    pub fn adopt_deployment(
        &self,
        plan: &GenericPlan,
        grant: Option<&ExactGrant>,
        donor: &DeploymentDonor,
        created_at: &str,
    ) -> Result<GenericOutcome, GenericError> {
        let t = plan
            .transfer
            .as_ref()
            .ok_or_else(|| refuse("missing transfer"))?;
        donor.current_root()?;
        if t.family != wire::TransferFamily::Deployment
            || t.donor_root_identity != donor.state.generic_root_identity().map_err(refuse)?
            || t.donor_id != donor.home.id()
            || t.receipt != donor.bytes
        {
            return Err(refuse("changed deployment donor"));
        }
        self.apply_inner(plan, grant, created_at, None, Some(donor))
    }
}
impl DeploymentDonor {
    fn current_root(&self) -> Result<(), GenericError> {
        let named = Project::open(self.settings.root_path()).map_err(refuse)?;
        if named.root_correlation_token().map_err(refuse)?
            != self.settings.root_correlation_token().map_err(refuse)?
        {
            return Err(refuse("deployment donor settings changed"));
        }
        self.state.verify_bound_home(&named).map_err(refuse)
    }
    pub(super) fn matches(&self, t: &wire::Transfer) -> Result<(), GenericError> {
        self.current_root()?;
        if t.family != wire::TransferFamily::Deployment
            || t.donor_root_identity != self.state.generic_root_identity().map_err(refuse)?
            || t.donor_id != self.home.id()
            || t.receipt_digest != digest(&self.bytes)
        {
            return Err(refuse("wrong deployment donor"));
        }
        Ok(())
    }
    fn retired(&self) -> vibe_wire::generated::deploy_receipt::DeployReceipt {
        let mut r = self.receipt.clone();
        r.resources.clear();
        r.prior_state_handle = None;
        r.reversible = false;
        r
    }
    pub(super) fn lock(&self) -> Result<vibe_safefs::LockGuard, GenericError> {
        self.current_root()?;
        let guard = self.state.lock_deployment(&self.home).map_err(refuse)?;
        self.current_root()?;
        Ok(guard)
    }
    pub(super) fn validate(&self) -> Result<(), GenericError> {
        self.current_root()?;
        if self.receipt.status == vibe_wire::generated::deploy_receipt::ReceiptStatus::RolledBack
            || self.receipt.resources.is_empty()
        {
            return Err(refuse("inactive donor receipt proves no ownership"));
        }
        if self
            .state
            .read_receipt(&self.home)
            .map_err(refuse)?
            .as_ref()
            != Some(&self.receipt)
            || self
                .state
                .read_intent(&self.home)
                .map_err(refuse)?
                .is_some()
            || self
                .state
                .read_transfer_hold(&self.home)
                .map_err(refuse)?
                .is_some()
        {
            return Err(refuse("changed or held deployment donor"));
        }
        Ok(())
    }
    pub(super) fn hold(&self, record: &wire::DeployGenericState) -> Result<(), GenericError> {
        self.validate()?;
        let t = record
            .pending
            .as_ref()
            .and_then(|b| b.transfer.as_ref())
            .ok_or_else(|| refuse("missing transfer"))?;
        self.matches(t)?;
        if t.family != wire::TransferFamily::Deployment
            || t.donor_id != self.home.id()
            || t.receipt_digest != digest(&self.bytes)
        {
            return Err(refuse("wrong donor hold"));
        }
        self.state
            .retain_generic_blob(&self.home, &self.bytes, 0o600)
            .map_err(refuse)?;
        self.state
            .write_transfer_hold(&self.home, record)
            .map_err(refuse)
    }
    pub(super) fn ensure_hold(
        &self,
        record: &wire::DeployGenericState,
    ) -> Result<(), GenericError> {
        let b = record
            .pending
            .as_ref()
            .or(record.committed.as_ref())
            .ok_or_else(|| refuse("missing recipient binding"))?;
        self.matches(
            b.transfer
                .as_ref()
                .ok_or_else(|| refuse("missing transfer"))?,
        )?;
        if let Some(old) = self.state.read_transfer_hold(&self.home).map_err(refuse)? {
            let held = old.pending.as_ref().ok_or_else(|| refuse("missing hold"))?;
            if held.coordination_identity != b.coordination_identity
                || held.transfer.as_ref().map(|t| &t.hold_id)
                    != b.transfer.as_ref().map(|t| &t.hold_id)
            {
                return Err(refuse("donor arena/hold mismatch"));
            }
            let current = self.state.read_receipt(&self.home).map_err(refuse)?;
            if current.as_ref() != Some(&self.receipt) && current.as_ref() != Some(&self.retired())
            {
                return Err(refuse("changed donor receipt under hold"));
            }
            Ok(())
        } else {
            if b.operation != wire::BindingOperation::Adoption {
                return Err(refuse("missing retained donor hold"));
            }
            self.hold(record)
        }
    }
    pub(super) fn retire(&self, hold_id: &str) -> Result<(), GenericError> {
        let mut hold = self
            .state
            .read_transfer_hold(&self.home)
            .map_err(refuse)?
            .ok_or_else(|| refuse("missing donor hold"))?;
        let t = hold
            .pending
            .as_mut()
            .and_then(|b| b.transfer.as_mut())
            .ok_or_else(|| refuse("missing transfer"))?;
        if t.hold_id != hold_id {
            return Err(refuse("changed donor hold"));
        }
        self.matches(t)?;
        let found = self.state.read_receipt(&self.home).map_err(refuse)?;
        if found.as_ref() != Some(&self.receipt) && found.as_ref() != Some(&self.retired()) {
            return Err(refuse("changed donor receipt"));
        }
        self.state
            .write_receipt(&self.home, &self.retired())
            .map_err(refuse)?;
        t.phase = wire::TransferPhase::Retired;
        self.state
            .write_transfer_hold(&self.home, &hold)
            .map_err(refuse)
    }
    pub(super) fn restore(&self, hold_id: &str) -> Result<(), GenericError> {
        let mut hold = self
            .state
            .read_transfer_hold(&self.home)
            .map_err(refuse)?
            .ok_or_else(|| refuse("missing donor hold"))?;
        let t = hold
            .pending
            .as_mut()
            .and_then(|b| b.transfer.as_mut())
            .ok_or_else(|| refuse("missing transfer"))?;
        if t.hold_id != hold_id {
            return Err(refuse("changed donor hold"));
        }
        self.matches(t)?;
        let found = self.state.read_receipt(&self.home).map_err(refuse)?;
        if found.as_ref() != Some(&self.receipt) && found.as_ref() != Some(&self.retired()) {
            return Err(refuse("changed donor receipt"));
        }
        self.state
            .write_receipt(&self.home, &self.receipt)
            .map_err(refuse)?;
        t.phase = wire::TransferPhase::Restored;
        self.state
            .write_transfer_hold(&self.home, &hold)
            .map_err(refuse)
    }
}
