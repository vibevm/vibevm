//! Preview-only domain adapter over the incumbent submission service.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#PREVIEW-PREPARATION");
use super::*;
use crate::machine_submission::{self, JournalState, SubmissionJournal, SubmissionOutcome};
use std::sync::atomic::{AtomicU64, Ordering};
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{DirectoryDurability, Project};
pub use vibe_workspace::boot_debugger::preparation::*;

/// Host-selected producer. Transport can select only a previously registered plan.
/// Implementations must confine effects to the admitted cache and selected inputs.
pub trait PreparationProvider: Send + Sync {
    fn prepare(&self, stage: &PreparationStage, cache: &Path) -> StageOutcome;
}
struct Capability {
    namespace: SubmissionNamespace,
    provider: Arc<dyn PreparationProvider>,
}
pub struct PreparationService {
    plan: PreparationPlan,
    id: String,
    cache: Project,
    project_root: std::path::PathBuf,
    scope: SubmissionScope,
    capability: Mutex<Option<Capability>>,
    provider_invocations: AtomicU64,
}
impl PreparationService {
    /// Read-only construction never acquires a lease or creates a directory.
    pub fn open(
        project_root: &Path,
        plan: PreparationPlan,
        principal: String,
    ) -> Result<Self, String> {
        let root = std::fs::canonicalize(project_root).map_err(|e| e.to_string())?;
        let cache = Project::open(Path::new(&plan.cache)).map_err(|e| e.to_string())?;
        let cache_root = std::fs::canonicalize(cache.root_path()).map_err(|e| e.to_string())?;
        if cache_root.starts_with(&root)
            || root.starts_with(&cache_root)
            || cache_root != Path::new(&plan.cache)
            || plan.stages.is_empty()
            || plan.context.is_empty()
            || plan.capture.is_empty()
            || principal.is_empty()
        {
            return Err("preview requires a separate exact cache, capture and principal".into());
        }
        let id = hash(
            serde_json::to_vec(&plan)
                .map_err(|e| e.to_string())?
                .as_slice(),
        );
        let scope = SubmissionScope {
            context: plan.context.clone(),
            target: id.clone(),
            principal,
        };
        Ok(Self {
            plan,
            id,
            cache,
            project_root: root,
            scope,
            capability: Mutex::new(None),
            provider_invocations: AtomicU64::new(0),
        })
    }
    pub fn plan(&self) -> &PreparationPlan {
        &self.plan
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Session-local attempted provider calls, including failed/unavailable calls.
    /// Retained reads and inspection/compare never increment this counter.
    pub fn provider_invocations(&self) -> u64 {
        self.provider_invocations.load(Ordering::Relaxed)
    }
    /// Explicit host grant. No Install/Deploy authority or transport token is used.
    pub fn admit(&self, provider: Arc<dyn PreparationProvider>, ttl_ms: u64) -> Result<(), String> {
        let lease = LifecycleLease::acquire(self.cache.root_path()).map_err(|e| e.to_string())?;
        let namespace = SubmissionReservations::issue_namespace(&lease, &self.scope, ttl_ms)
            .map_err(|e| e.to_string())?;
        *self.capability.lock().map_err(|_| "capability poisoned")? = Some(Capability {
            namespace,
            provider,
        });
        Ok(())
    }
    pub fn revoke(&self) -> Result<(), String> {
        *self.capability.lock().map_err(|_| "capability poisoned")? = None;
        Ok(())
    }
    fn path(&self) -> String {
        format!(".vibe/boot-preparation/{}.json", &self.id[7..])
    }
    /// Existing pinned read state only; works without namespace/provider authority.
    pub fn read(&self) -> Result<Option<PreparationOutcome>, String> {
        let bytes = self
            .cache
            .read_file_bounded(&self.path(), 64 * 1024 * 1024)
            .map_err(|e| e.to_string())?;
        bytes
            .map(|bytes| {
                let outcome: PreparationOutcome =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                outcome.validate(&self.plan)?;
                Ok(outcome)
            })
            .transpose()
    }
    fn publish(&self, lease: &LifecycleLease, outcome: &PreparationOutcome) -> Result<(), String> {
        outcome.validate(&self.plan)?;
        if lease.root() != self.cache.root_path() {
            return Err("foreign preview lease".into());
        }
        let project = lease.publication_journal_project();
        if project
            .root_correlation_token()
            .map_err(|e| e.to_string())?
            != self
                .cache
                .root_correlation_token()
                .map_err(|e| e.to_string())?
        {
            return Err("preview cache replaced".into());
        }
        let bytes = serde_json::to_vec(outcome).map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("preparation evidence exceeds retention bound".into());
        }
        let pinned = project.root_dir().map_err(|e| e.to_string())?;
        let write = project
            .write_atomic_durable_in(&pinned, &self.path(), &bytes)
            .map_err(|e| e.into_report().to_string())?;
        if !write.file_synced
            || write.parent != DirectoryDurability::Synced
            || write
                .directory_syncs
                .iter()
                .any(|s| s.durability != DirectoryDurability::Synced)
            || self.read()?.as_ref() != Some(outcome)
        {
            return Err("uncertain preparation evidence publication".into());
        }
        Ok(())
    }
    fn execute(&self) -> Result<Option<PreparationOutcome>, String> {
        // A retained result, including interrupted valid stages, never needs a provider.
        if let Some(outcome) = self.read()? {
            return Ok(Some(outcome));
        }
        let capability = self.capability.lock().map_err(|_| "capability poisoned")?;
        let Some(capability) = capability.as_ref() else {
            return Ok(None);
        };
        let lease =
            Arc::new(LifecycleLease::acquire(self.cache.root_path()).map_err(|e| e.to_string())?);
        let mut journal = Journal {
            service: self,
            provider: capability.provider.as_ref(),
            uncertain: None,
        };
        let plan = serde_json::to_vec(&self.plan).map_err(|e| e.to_string())?;
        match machine_submission::submit(
            lease,
            &capability.namespace,
            &self.scope,
            &self.id,
            self.id.as_bytes(),
            &plan,
            &mut journal,
        )
        .map_err(|e| e.to_string())?
        {
            SubmissionOutcome::Invoked { outcome, .. }
            | SubmissionOutcome::Replayed { outcome, .. } => Ok(Some(outcome)),
            _ => self.read(),
        }
    }
}
struct Journal<'a> {
    service: &'a PreparationService,
    provider: &'a dyn PreparationProvider,
    uncertain: Option<String>,
}
impl SubmissionJournal for Journal<'_> {
    type Outcome = PreparationOutcome;
    fn proposed_run_id(&self) -> String {
        format!("preview-{}", &self.service.id[7..])
    }
    fn inspect(
        &mut self,
        _: &LifecycleLease,
        run_id: &str,
    ) -> Result<JournalState<Self::Outcome>, String> {
        if run_id != self.proposed_run_id() {
            return Err("foreign preview run".into());
        }
        if let Some(reason) = &self.uncertain {
            return Ok(JournalState::Uncertain(reason.clone()));
        }
        Ok(match self.service.read()? {
            None => JournalState::Absent,
            Some(o) if o.terminal => JournalState::Complete(o),
            Some(_) => JournalState::Prepared,
        })
    }
    fn initialize(&mut self, lease: Arc<LifecycleLease>, _: &str) -> Result<(), String> {
        self.service.publish(
            &lease,
            &PreparationOutcome {
                plan: self.service.plan.clone(),
                stages: vec![],
                terminal: false,
            },
        )
    }
    fn invoke(&mut self, lease: Arc<LifecycleLease>, _: &str) -> Result<Self::Outcome, String> {
        let mut outcome = PreparationOutcome {
            plan: self.service.plan.clone(),
            stages: vec![],
            terminal: false,
        };
        for stage in &self.service.plan.stages {
            self.service
                .provider_invocations
                .fetch_add(1, Ordering::Relaxed);
            let result = self.provider.prepare(stage, self.service.cache.root_path());
            let failed = !matches!(result, StageOutcome::Captured { .. });
            outcome.stages.push(result);
            outcome.terminal = failed || outcome.stages.len() == outcome.plan.stages.len();
            if let Err(reason) = self.service.publish(&lease, &outcome) {
                self.uncertain = Some(reason.clone());
                return Err(reason);
            }
            if failed {
                break;
            }
        }
        Ok(outcome)
    }
}
impl BootDebuggerRoute {
    pub fn register_preparation(
        &self,
        service: Arc<PreparationService>,
    ) -> Result<q::Request, String> {
        let mut state = self.state.lock().map_err(|_| "debugger unavailable")?;
        if service.project_root != std::fs::canonicalize(self.root()).map_err(|e| e.to_string())?
            || service.plan.context != self.context
            || !state.snapshots.contains_key(&service.plan.capture)
        {
            return Err("foreign or missing preparation capture".into());
        }
        let id = service.id.clone();
        if state.preparations.contains_key(&id) {
            return Err("preparation already registered".into());
        }
        state.preparations.insert(id.clone(), service);
        Ok(q::Request {
            schema: 1,
            context_id: self.context.clone(),
            command: q::Command::PreparationPlan(Box::new(q::CommandPreparationPlan {
                plan_id: id,
            })),
        })
    }
    pub(super) fn preparation(
        &self,
        state: &State,
        id: &str,
        execute: bool,
        planned: bool,
    ) -> Answer<r::Result> {
        let service = state
            .preparations
            .get(id)
            .ok_or_else(|| fault(e::ErrorCode::Unavailable, "preparation plan unavailable"))?;
        let outcome = if planned {
            Ok(None)
        } else if execute {
            service.execute()
        } else {
            service.read()
        }
        .map_err(|reason| fault(e::ErrorCode::Unavailable, &reason))?;
        let status = match &outcome {
            _ if planned => r::BodyPreparationStatus::Planned,
            None => r::BodyPreparationStatus::NeedsPreparation,
            Some(o) if !o.terminal => r::BodyPreparationStatus::RecoveryRequired,
            Some(o) if o.complete() => r::BodyPreparationStatus::Complete,
            Some(o) if matches!(o.stages.first(), Some(StageOutcome::Unavailable { .. })) => {
                r::BodyPreparationStatus::Unavailable
            }
            Some(_) => r::BodyPreparationStatus::Partial,
        };
        self.result(
            Some(service.plan.capture.clone()),
            r::Body::Preparation(Box::new(r::BodyPreparation {
                plan_id: id.into(),
                plan_json: json(&service.plan)?,
                outcome_json: outcome.as_ref().map(json).transpose()?,
                status,
            })),
        )
    }
}
