use super::*;

/// Immutable in-process authority. No Deserialize, public fields, raw-root
/// issuer, persistent marker or global grant table exists.
#[derive(Debug)]
pub struct ExactGrant {
    plan: GenericPlan,
    host: u64,
}
impl GenericHost {
    /// Explicit host approval of this entire inspectable plan. Approval is a
    /// distinct call after payload admission and pure preparation.
    pub fn approve(&self, plan: &GenericPlan) -> Result<ExactGrant, GenericError> {
        self.check_state_identity(plan)?;
        self.check_plan(plan)?;
        Ok(ExactGrant {
            plan: plan.clone(),
            host: self.instance,
        })
    }
    pub(super) fn check_state_identity(&self, plan: &GenericPlan) -> Result<(), GenericError> {
        if self.state_identity()? != plan.state_identity {
            return Err(refuse("state home identity changed"));
        }
        Ok(())
    }
    pub(super) fn check_plan(&self, plan: &GenericPlan) -> Result<(), GenericError> {
        self.current(&plan.admitted.capture)?;
        if self.arena_identity()? != plan.coordination_identity {
            return Err(refuse("coordination arena changed"));
        }
        if self.roots != plan.roots
            || self.protected != plan.protected
            || self.authority.current_control().map_err(refuse)? != plan.control
        {
            return Err(refuse("root, principal, policy or protected-control drift"));
        }
        // Re-observe only the captured selection/destinations and incumbent
        // ownership. Never refresh a stale plan or partially merge it.
        if self.receipts()? != plan.receipts {
            return Err(refuse("ownership census changed"));
        }
        let state =
            super::super::state::DeployState::observe_bound(&self.settings).map_err(refuse)?;
        let observed = state
            .as_ref()
            .map(|s| s.read_generic(&self.home(&plan.target)))
            .transpose()
            .map_err(refuse)?
            .flatten();
        if observed != plan.retained_state {
            return Err(refuse("retained sidecar changed"));
        }
        for document in &plan.documents {
            let root = self.root(document.scope);
            root.preflight(&document.destination).map_err(refuse)?;
            if root
                .read_file_snapshot_bounded(&document.destination, 4 * 1024 * 1024)
                .map_err(refuse)?
                != document.before
            {
                return Err(refuse("destination changed since plan"));
            }
        }
        Ok(())
    }
    pub(super) fn check_grant(
        &self,
        plan: &GenericPlan,
        grant: Option<&ExactGrant>,
    ) -> Result<(), GenericError> {
        let grant = grant.ok_or_else(|| refuse("missing exact host grant"))?;
        if grant.host != self.instance || grant.plan != *plan {
            return Err(refuse("wrong, modified or broadened exact host grant"));
        }
        self.check_plan(plan)
    }
}
