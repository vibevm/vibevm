//! Admitted client effects composed with the deployment transaction owner.
//! The embedding host supplies command authority and a complete, independent
//! scoped observer. Retained records are evidence, never process grants.
use super::{
    client_commands as commands, generic,
    state::{DeployState, DeploymentHome},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use vibe_safefs::Project;
use vibe_wire::generated::client_deploy::e1::reply::{
    Reply, ReplyAction, ReplyAuthority, ReplyStatus, Resource, ResourceState, Step, StepStatus,
};
mod journal;

pub trait Observer: Send + Sync {
    /// Current descriptor/source/principal/policy binding. No probes or refresh.
    fn subject(&self) -> Result<String, String>;
    /// Exactly the complete requested finite scope, including known absence.
    /// None is observed absence; Err is unavailable, never inferred absence.
    fn observe(&self, resources: &[String]) -> Result<BTreeMap<String, Option<String>>, String>;
}
pub struct Command {
    pub prepared: commands::Prepared,
    pub resource: String,
    pub desired: Option<String>,
    /// A trusted capability assertion, not a wire request.
    pub retry_safe: bool,
}
pub enum Files {
    Apply {
        host: Arc<generic::GenericHost>,
        plan: generic::GenericPlan,
    },
    Retained {
        host: Arc<generic::GenericHost>,
        plan: generic::RetainedPlan,
    },
}
enum FileGrant {
    Apply(generic::ExactGrant),
    Retained(generic::RetainedGrant),
}
impl Files {
    fn target(&self) -> &str {
        match self {
            Self::Apply { plan, .. } => plan.target(),
            Self::Retained { plan, .. } => plan.target(),
        }
    }
    fn retained_subject(&self) -> Result<String, String> {
        let (host, target) = match self {
            Self::Apply { host, plan } => (host, plan.target()),
            Self::Retained { host, plan } => (host, plan.target()),
        };
        let generic::ReceiptInspection::Retained { state, .. } =
            host.inspect_receipt(target).map_err(err)?
        else {
            return Err("missing retained C3 state".into());
        };
        state
            .pending
            .as_ref()
            .or(state.committed.as_ref())
            .map(|b| b.grant_subject.clone())
            .ok_or_else(|| "missing C3 operation".into())
    }
    fn resources(&self) -> Vec<(String, Option<String>)> {
        match self {
            Self::Apply { host, plan } => plan
                .resources()
                .iter()
                .map(|r| (r.owned(host), Some(r.desired_digest.clone())))
                .chain(
                    plan.removed_resources()
                        .iter()
                        .map(|r| (r.owned(host), None)),
                )
                .collect(),
            Self::Retained { host, plan } => plan
                .resources()
                .iter()
                .map(|r| {
                    use vibe_core::manifest::{ClientOperation, ClientScope};
                    use vibe_wire::generated::deploy_generic_state::{
                        ResourceOperation, ResourceScope, Value,
                    };
                    let desired = match &r.desired {
                        Value::Absent(_) => None,
                        Value::Present(v) => Some(v.blob.digest.clone()),
                    };
                    let resource = generic::GenericResource {
                        scope: match r.scope {
                            ResourceScope::Project => ClientScope::Project,
                            ResourceScope::User => ClientScope::User,
                        },
                        destination: r.destination.clone(),
                        key: r.key.clone(),
                        operation: match r.operation {
                            ResourceOperation::File => ClientOperation::ProjectFile,
                            ResourceOperation::JsonMember => ClientOperation::ReconcileJsonMember,
                            ResourceOperation::TomlMember => ClientOperation::ReconcileTomlMember,
                        },
                        desired_digest: desired.clone().unwrap_or_default(),
                    };
                    (resource.owned(host), desired)
                })
                .collect(),
        }
    }
    fn validate(&self, grant: &FileGrant) -> Result<(), String> {
        match (self, grant) {
            (Self::Apply { host, plan }, FileGrant::Apply(g)) => {
                host.validate_approval(plan, Some(g))
            }
            (Self::Retained { host, plan }, FileGrant::Retained(g)) => {
                host.validate_retained_approval(plan, Some(g))
            }
            _ => return Err("file approval mismatch".into()),
        }
        .map_err(err)
    }
    fn identity(&self) -> String {
        match self {
            Self::Apply { plan, .. } => plan.identity(),
            Self::Retained { plan, .. } => plan.identity(),
        }
    }
    fn approve(&self) -> Result<FileGrant, String> {
        match self {
            Self::Apply { host, plan } => host.approve(plan).map(FileGrant::Apply),
            Self::Retained { host, plan } => host.approve_retained(plan).map(FileGrant::Retained),
        }
        .map_err(|e| e.to_string())
    }
    fn execute(&self, grant: &FileGrant, clock: &str) -> Result<(), String> {
        match (self, grant) {
            (Self::Apply { host, plan }, FileGrant::Apply(g)) => host.apply(plan, Some(g), clock),
            (Self::Retained { host, plan }, FileGrant::Retained(g)) => {
                host.execute_retained(plan, Some(g), clock)
            }
            _ => return Err("file grant mismatch".into()),
        }
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}
pub struct Prepared {
    roots: [String; 3],
    target: String,
    scope: String,
    id: String,
    subject: String,
    commands: Vec<Command>,
    files: Option<Files>,
    resources: Vec<Resource>,
    action: ReplyAction,
}
impl Prepared {
    pub fn identity(&self) -> &str {
        &self.id
    }
}
pub struct Approval {
    used: AtomicBool,
    plan: String,
    commands: Vec<commands::ProcessGrant>,
    files: Option<FileGrant>,
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn request(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn pending(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
pub struct Host {
    project: Project,
    settings: Project,
    arena: Project,
    roots: [String; 3],
    target: String,
    scope: String,
    commands: Arc<commands::Host>,
    observer: Arc<dyn Observer>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn encoded(v: &impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(v).expect("closed generated deployment value")
}
fn value(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "absent".into())
}
impl Host {
    pub fn new(
        project: Project,
        settings: Project,
        arena: Project,
        target: String,
        scope: String,
        commands: Arc<commands::Host>,
        observer: Arc<dyn Observer>,
    ) -> Result<Self, String> {
        if target.is_empty() || scope.is_empty() {
            return Err("trusted target/scope required".into());
        }
        let roots = [
            project.root_correlation_token().map_err(err)?,
            settings.root_correlation_token().map_err(err)?,
            arena.root_correlation_token().map_err(err)?,
        ];
        Ok(Self {
            project,
            settings,
            arena,
            roots,
            target,
            scope,
            commands,
            observer,
        })
    }
    pub fn prepare_storage(&self) -> Result<(), String> {
        self.current()?;
        DeployState::open_bound(&self.settings)
            .map(|_| ())
            .map_err(err)
    }
    pub fn root(&self) -> &std::path::Path {
        self.project.root_path()
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
    fn current(&self) -> Result<(), String> {
        for (root, token) in [&self.project, &self.settings, &self.arena]
            .into_iter()
            .zip(&self.roots)
        {
            if root.root_correlation_token().map_err(err)? != *token
                || Project::open(root.root_path())
                    .map_err(err)?
                    .root_correlation_token()
                    .map_err(err)?
                    != *token
            {
                return Err("root or coordination arena replaced".into());
            }
        }
        Ok(())
    }
    fn home(&self) -> DeploymentHome {
        DeploymentHome::new(
            &self.settings.root_path().join("state/deployments"),
            &self.roots[0],
            None,
            &self.target,
        )
    }
    fn observe(&self, resources: &[Resource]) -> Result<Vec<Resource>, String> {
        let names: Vec<_> = resources.iter().map(|r| r.resource.clone()).collect();
        let found = self.observer.observe(&names)?;
        if found.len() != names.len() || names.iter().any(|name| !found.contains_key(name)) {
            return Err("observer did not return complete exact scope".into());
        }
        Ok(resources
            .iter()
            .map(|r| {
                let mut r = r.clone();
                r.observed = value(&found[&r.resource]);
                r.state = if r.observed == r.desired {
                    ResourceState::Desired
                } else if r.observed == r.prior {
                    ResourceState::Prior
                } else {
                    ResourceState::Third
                };
                r
            })
            .collect())
    }
    /// Commands must already have been explicitly prepared through D1. No probe
    /// is hidden here; descriptor and observations are independently supplied.
    pub fn prepare(
        &self,
        action: ReplyAction,
        commands: Vec<Command>,
        files: Option<Files>,
    ) -> Result<Prepared, String> {
        self.current()?;
        if !matches!(
            action,
            ReplyAction::Apply | ReplyAction::Update | ReplyAction::Remove
        ) {
            return Err("effect preparation action".into());
        }
        if commands.is_empty() && files.is_none() {
            return Err("empty effect plan".into());
        }
        let file_resources = files.as_ref().map(Files::resources).unwrap_or_default();
        let subject = self.observer.subject()?;
        if subject.is_empty() {
            return Err("descriptor/control subject unavailable".into());
        }
        let mut resources = Vec::new();
        for (resource, desired) in commands
            .iter()
            .map(|c| (c.resource.clone(), c.desired.clone()))
            .chain(file_resources)
        {
            if resource.is_empty() || resources.iter().any(|r: &Resource| r.resource == resource) {
                return Err("duplicate/empty resource".into());
            }
            resources.push(Resource {
                resource,
                prior: String::new(),
                desired: value(&desired),
                observed: String::new(),
                state: ResourceState::Unavailable,
            });
        }
        let names = resources
            .iter()
            .map(|r| r.resource.clone())
            .collect::<Vec<_>>();
        let before = self.observer.observe(&names)?;
        if before.len() != names.len() || names.iter().any(|r| !before.contains_key(r)) {
            return Err("incomplete initial observation".into());
        }
        for r in &mut resources {
            r.prior = value(&before[&r.resource]);
            r.observed = r.prior.clone();
            r.state = ResourceState::Prior;
        }
        for c in &commands {
            self.commands.check(&c.prepared).map_err(err)?;
        }
        let command_plans: Vec<_> = commands
            .iter()
            .map(|c| (c.prepared.inspect(), &c.resource, c.retry_safe))
            .collect();
        let id = digest(&encoded(&(
            &self.roots,
            &self.target,
            &self.scope,
            &subject,
            &action,
            &resources,
            command_plans,
            files.as_ref().map(Files::identity),
        )));
        Ok(Prepared {
            roots: self.roots.clone(),
            target: self.target.clone(),
            scope: self.scope.clone(),
            id,
            subject,
            commands,
            files,
            resources,
            action,
        })
    }
    fn check(&self, p: &Prepared) -> Result<(), String> {
        self.current()?;
        if p.roots != self.roots || p.target != self.target || p.scope != self.scope {
            return Err("foreign prepared deployment".into());
        }
        if self.observer.subject()? != p.subject {
            return Err("descriptor or control changed".into());
        }
        for c in &p.commands {
            self.commands.check(&c.prepared).map_err(err)?;
        }
        Ok(())
    }
    /// Rebind retained command subjects to newly prepared in-process values.
    /// This is explicit host admission after restart, never DTO resurrection.
    pub fn prepare_recovery(&self, run: &str, commands: Vec<Command>) -> Result<Prepared, String> {
        self.prepare_recovery_with_files(run, commands, None)
    }
    pub fn prepare_recovery_with_files(
        &self,
        run: &str,
        commands: Vec<Command>,
        files: Option<Files>,
    ) -> Result<Prepared, String> {
        self.current()?;
        let state = DeployState::observe_bound(&self.settings)
            .map_err(err)?
            .ok_or("missing state")?;
        let record = state
            .read_client(&self.home(), run)
            .map_err(err)?
            .ok_or("missing run")?;
        let (r, _) = self.inspect(run)?.ok_or("missing run")?;
        let file_count = files.as_ref().map_or(0, |f| f.resources().len());
        if self.observer.subject()? != record.subject
            || commands.len() + file_count != r.steps.len()
            || r.resources.len() != r.steps.len()
            || files.as_ref().map(|f| f.target()) != record.file_target.as_deref()
        {
            return Err("fresh admission does not cover retained subject".into());
        }
        if let Some(f) = &files {
            if record.file_subject.as_deref() != Some(f.retained_subject()?.as_str()) {
                return Err("C3 retained operation subject changed".into());
            }
            if !matches!(f, Files::Retained { .. }) {
                return Err("file recovery requires C3 retained admission".into());
            }
            for (name, desired) in f.resources() {
                if !r
                    .resources
                    .iter()
                    .any(|r| r.resource == name && r.desired == value(&desired))
                {
                    return Err("C3 retained resource differs".into());
                }
            }
        }
        for (command, step) in commands.iter().zip(&r.steps[file_count..]) {
            self.commands.check(&command.prepared).map_err(err)?;
            if command.resource != step.resource
                || digest(&encoded(command.prepared.inspect())) != step.subject
                || command.retry_safe != step.retry_safe
                || r.resources
                    .iter()
                    .find(|r| r.resource == command.resource)
                    .is_none_or(|r| r.desired != value(&command.desired))
            {
                return Err("fresh command differs from retained exact plan".into());
            }
        }
        Ok(Prepared {
            roots: self.roots.clone(),
            target: self.target.clone(),
            scope: self.scope.clone(),
            id: r.plan_id,
            subject: record.subject,
            commands,
            files,
            resources: r.resources,
            action: r.action,
        })
    }
    fn command_ownership(&self, p: &Prepared) -> Result<(), String> {
        let state = DeployState::observe_bound(&self.settings).map_err(err)?;
        let receipt = state
            .as_ref()
            .map(|s| s.read_receipt(&self.home()))
            .transpose()
            .map_err(err)?
            .flatten();
        if receipt.as_ref().is_some_and(|r| {
            r.resources
                .iter()
                .any(|owned| !p.commands.iter().any(|c| c.resource == owned.resource))
        }) {
            return Err("plan omits previously owned command resources".into());
        }
        for command in &p.commands {
            let row = p
                .resources
                .iter()
                .find(|r| r.resource == command.resource)
                .ok_or("command resource")?;
            if row.prior != "absent"
                && !receipt.as_ref().is_some_and(|r| {
                    r.resources.iter().any(|owned| {
                        owned.resource == row.resource && owned.post_digest == row.prior
                    })
                })
            {
                return Err("unowned or drifted command state".into());
            }
        }
        Ok(())
    }
    pub fn validate_approval(
        &self,
        p: &Prepared,
        approval: Option<&Approval>,
    ) -> Result<(), String> {
        let approval = approval.ok_or("missing exact approval")?;
        if approval.used.load(Ordering::SeqCst) {
            return Err("fresh exact approval required for another invocation".into());
        }
        if approval.plan != p.id || approval.commands.len() != p.commands.len() {
            return Err("wrong or expanded approval".into());
        }
        self.check(p)?;
        for (command, grant) in p.commands.iter().zip(&approval.commands) {
            self.commands
                .validate_process_grant(&command.prepared, Some(grant))
                .map_err(err)?;
        }
        match (&p.files, &approval.files) {
            (Some(files), Some(g)) => files.validate(g),
            (None, None) => Ok(()),
            _ => Err("file approval missing".into()),
        }
    }
    pub fn approve(&self, p: &Prepared) -> Result<Approval, String> {
        self.check(p)?;
        Ok(Approval {
            used: AtomicBool::new(false),
            plan: p.id.clone(),
            commands: p
                .commands
                .iter()
                .map(|c| self.commands.approve(&c.prepared).map_err(err))
                .collect::<Result<_, _>>()?,
            files: p.files.as_ref().map(Files::approve).transpose()?,
        })
    }
    /// Pure query: retained finite preparation plus independent read-only state.
    pub fn plan(&self, p: &Prepared) -> Reply {
        let mut r = self.initial(p, "");
        match self.check(p).and_then(|()| self.observe(&p.resources)) {
            Ok(rows) if rows.iter().all(|r| r.observed == r.prior) => {
                r.resources = rows;
            }
            Ok(rows) => {
                r.resources = rows;
                r.status = ReplyStatus::NeedsPreparation;
                r.message = "state changed since preparation".into();
            }
            Err(e) => {
                r.status = ReplyStatus::NeedsPreparation;
                r.message = e;
            }
        }
        r
    }
    fn initial(&self, p: &Prepared, run: &str) -> Reply {
        Reply {
            schema: 1,
            action: p.action.clone(),
            plan_id: p.id.clone(),
            run_id: run.into(),
            status: ReplyStatus::Planned,
            authority: ReplyAuthority::Required,
            resources: p.resources.clone(),
            steps: p
                .files
                .iter()
                .flat_map(|files| {
                    files.resources().into_iter().map(|(resource, _)| Step {
                        resource,
                        subject: files.identity(),
                        status: StepStatus::Pending,
                        retry_safe: true,
                    })
                })
                .chain(p.commands.iter().map(|c| Step {
                    resource: c.resource.clone(),
                    subject: digest(&encoded(c.prepared.inspect())),
                    status: StepStatus::Pending,
                    retry_safe: c.retry_safe,
                }))
                .collect(),
            cancel_pending: false,
            message: String::new(),
        }
    }
    pub fn proposed_run(&self, key: &str) -> String {
        format!(
            "client-{}",
            digest(&encoded(&(&self.scope, &self.roots, self.home().id(), key)))
        )
    }
    pub fn initialize(&self, p: &Prepared, run: &str) -> Result<(), String> {
        self.current()?;
        let state = DeployState::open_bound(&self.settings).map_err(err)?;
        let home = self.home();
        let _lock = state.lock_deployment(&home).map_err(err)?;
        if state.read_client(&home, run).map_err(err)?.is_some() {
            return Err("run already initialized".into());
        }
        self.save(&state, &home, p, &self.initial(p, run), false)
    }
    pub fn inspect(&self, run: &str) -> Result<Option<(Reply, bool)>, String> {
        self.current()?;
        let Some(state) = DeployState::observe_bound(&self.settings).map_err(err)? else {
            return Ok(None);
        };
        let Some(record) = state.read_client(&self.home(), run).map_err(err)? else {
            return Ok(None);
        };
        if record.scope != self.scope || record.deployment != self.home().id() {
            return Err("foreign deployment record".into());
        }
        Ok(Some((
            vibe_wire::behaviour::client_deploy::read_reply(&encoded(&record.reply))
                .map_err(err)?,
            record.published,
        )))
    }
    /// Closed historical command evidence for explicit trusted re-preparation.
    /// Reading this list issues no D1 Prepared or ProcessGrant.
    pub fn inspect_commands(
        &self,
        run: &str,
    ) -> Result<Vec<vibe_wire::generated::client_command::e1::plan::Plan>, String> {
        self.inspect(run)?.ok_or("unknown run")?;
        let state = DeployState::observe_bound(&self.settings)
            .map_err(err)?
            .ok_or("missing state")?;
        let record = state
            .read_client(&self.home(), run)
            .map_err(err)?
            .ok_or("missing run")?;
        record
            .commands
            .iter()
            .map(|c| {
                vibe_wire::behaviour::client_command::read_plan(&encoded(&c.plan)).map_err(err)
            })
            .collect()
    }
    /// Called under the incumbent outer lease after any active opaque call has
    /// reached its durable boundary. This never supplies a process kill handle.
    pub fn cancel_at_boundary(&self, run: &str) -> Result<Reply, String> {
        let (mut reply, _) = self.inspect(run)?.ok_or("unknown run")?;
        reply.action = ReplyAction::Cancel;
        if reply.status == ReplyStatus::Completed {
            reply.cancel_pending = false;
            reply.message = "operation already completed".into();
            return Ok(reply);
        }
        reply.cancel_pending = true;
        reply.status = ReplyStatus::Cancelled;
        reply.message = "cancelled at retained operation boundary".into();
        let state = DeployState::observe_bound(&self.settings)
            .map_err(err)?
            .ok_or("missing state")?;
        let mut record = state
            .read_client(&self.home(), run)
            .map_err(err)?
            .ok_or("missing run")?;
        if record.finalization_digest.is_some() {
            record.reply.cancel_pending = true;
            state.write_client(&self.home(), &record).map_err(err)?;
            reply.status = ReplyStatus::Partial;
            reply.message =
                "receipt finalization pending; explicit authorized recovery required".into();
            return Ok(reply);
        }
        // Retain the original effect verb for future explicit continuation.
        let mut retained = reply.clone();
        retained.action =
            serde_json::from_value(serde_json::to_value(&record.reply.action).map_err(err)?)
                .map_err(err)?;
        record.reply = serde_json::from_slice(&encoded(&retained)).map_err(err)?;
        record.published = true;
        state.write_client(&self.home(), &record).map_err(err)?;
        Ok(reply)
    }
    pub fn verify(&self, run: &str) -> Result<Reply, String> {
        let (mut r, _) = self.inspect(run)?.ok_or("unknown run")?;
        r.action = ReplyAction::Verify;
        match self.observe(&r.resources) {
            Ok(rows) => {
                r.resources = rows;
                if r.resources
                    .iter()
                    .any(|v| v.state != ResourceState::Desired)
                {
                    r.status = ReplyStatus::Partial;
                    r.message = "independent desired-state mismatch".into();
                }
            }
            Err(e) => {
                for row in &mut r.resources {
                    row.state = ResourceState::Unavailable;
                }
                r.status = ReplyStatus::Unknown;
                r.message = e;
            }
        }
        Ok(r)
    }
    pub fn execute(
        &self,
        p: &Prepared,
        approval: Option<&Approval>,
        run: &str,
        recovery: bool,
        cancel: &Cancellation,
        clock: &str,
    ) -> Result<Reply, String> {
        let (mut reply, published) = self.inspect(run)?.ok_or("uninitialized run")?;
        if reply.plan_id != p.id {
            reply.status = ReplyStatus::NeedsAuthority;
            reply.authority = ReplyAuthority::Stale;
            reply.message = "different retained plan".into();
            return Ok(reply);
        }
        if published && (!recovery || reply.status == ReplyStatus::Completed) {
            return Ok(reply);
        }
        let grant = match approval.filter(|g| g.plan == p.id) {
            Some(g) => g,
            None => {
                reply.status = ReplyStatus::NeedsAuthority;
                reply.authority = ReplyAuthority::Required;
                return Ok(reply);
            }
        };
        if let Err(e) = self.validate_approval(p, Some(grant)) {
            reply.status = ReplyStatus::NeedsAuthority;
            reply.authority = ReplyAuthority::Stale;
            reply.message = e;
            return Ok(reply);
        }
        if grant.used.swap(true, Ordering::SeqCst) {
            reply.status = ReplyStatus::NeedsAuthority;
            reply.authority = ReplyAuthority::Required;
            reply.message = "continuation requires a fresh exact approval".into();
            return Ok(reply);
        }
        reply.authority = ReplyAuthority::Admitted;
        reply.message.clear();
        reply.cancel_pending = false;
        let state = DeployState::open_bound(&self.settings).map_err(err)?;
        let home = self.home();
        let mut retained = state
            .read_client(&home, run)
            .map_err(err)?
            .ok_or("retained run disappeared")?;
        let receipt = state.read_receipt(&home).map_err(err)?;
        if recovery
            && retained.finalization_digest.is_some()
            && retained.finalization_digest == receipt.as_ref().map(|r| digest(&encoded(r)))
        {
            let _arena = self
                .arena
                .lock("deploy-physical-coordination.lock")
                .map_err(err)?;
            let _deployment = state.lock_deployment(&home).map_err(err)?;
            let _destinations = state
                .lock_destinations(
                    &reply
                        .resources
                        .iter()
                        .map(|r| r.resource.clone())
                        .collect::<Vec<_>>(),
                )
                .map_err(err)?;
            self.check(p)?;
            state.verify_bound_home(&self.settings).map_err(err)?;
            if let Err(e) = self.reconcile_finalization(&state, &home, p, &retained) {
                reply.status = ReplyStatus::Refused;
                reply.message = e;
                return Ok(reply);
            }
            let (finalized, _) = self.inspect(run)?.ok_or("missing finalized run")?;
            if finalized.status == ReplyStatus::Completed {
                return Ok(finalized);
            }
            reply = finalized;
            reply.message.clear();
            retained = state
                .read_client(&home, run)
                .map_err(err)?
                .ok_or("missing finalized record")?;
        }
        if retained.receipt_digest != receipt.as_ref().map(|r| digest(&encoded(r))) {
            reply.status = ReplyStatus::Refused;
            reply.message = "retained receipt join changed; fresh operation required".into();
            return Ok(reply);
        }
        if recovery {
            for row in &reply.resources {
                if p.commands.iter().any(|c| c.resource == row.resource)
                    && reply.steps.iter().any(|step| {
                        step.resource == row.resource && step.status == StepStatus::Pending
                    })
                    && row.prior != "absent"
                    && !receipt.as_ref().is_some_and(|r| {
                        r.resources.iter().any(|owned| {
                            owned.resource == row.resource && owned.post_digest == row.prior
                        })
                    })
                {
                    reply.status = ReplyStatus::Refused;
                    reply.message = "unattempted prefix lacks original ownership".into();
                    return Ok(reply);
                }
            }
        }
        if !recovery {
            if let Err(e) = self.command_ownership(p) {
                reply.status = ReplyStatus::Refused;
                reply.message = e;
                self.save(&state, &home, p, &reply, true)?;
                return Ok(reply);
            }
            match self.observe(&reply.resources) {
                Ok(rows) if rows.iter().all(|row| row.observed == row.prior) => {
                    reply.resources = rows
                }
                Ok(rows) => {
                    reply.resources = rows;
                    reply.status = ReplyStatus::NeedsPreparation;
                    reply.message = "full scoped state changed before dispatch".into();
                    self.save(&state, &home, p, &reply, true)?;
                    return Ok(reply);
                }
                Err(e) => {
                    for row in &mut reply.resources {
                        row.state = ResourceState::Unavailable;
                    }
                    reply.status = ReplyStatus::Unknown;
                    reply.message = e;
                    self.save(&state, &home, p, &reply, true)?;
                    return Ok(reply);
                }
            }
        }
        // C3 owns all file locking, private before images and retained inverse.
        // Its journal is durable before effects; command dispatch follows it.
        if cancel.pending() {
            reply.cancel_pending = true;
            reply.status = ReplyStatus::Cancelled;
            reply.message = "cancelled before first dispatch".into();
            self.save(&state, &home, p, &reply, true)?;
            return Ok(reply);
        }
        if let (Some(files), Some(g)) = (&p.files, &grant.files) {
            let count = files.resources().len();
            for step in &mut reply.steps[..count] {
                step.status = StepStatus::Attempted;
            }
            self.save(&state, &home, p, &reply, false)?;
            let result = files.execute(g, clock);
            match self.observe(&reply.resources) {
                Ok(rows) => {
                    reply.resources = rows;
                    for step in &mut reply.steps[..count] {
                        step.status = if reply.resources.iter().any(|r| {
                            r.resource == step.resource && r.state == ResourceState::Desired
                        }) {
                            StepStatus::Completed
                        } else {
                            StepStatus::Uncertain
                        };
                    }
                }
                Err(e) => {
                    reply.status = ReplyStatus::Unknown;
                    reply.message = e;
                    self.save(&state, &home, p, &reply, true)?;
                    return Ok(reply);
                }
            }
            if let Err(e) = result {
                reply.status = ReplyStatus::Partial;
                reply.message = e;
                self.save(&state, &home, p, &reply, true)?;
                return Ok(reply);
            }
            self.save(&state, &home, p, &reply, false)?;
        }
        self.current()?;
        let _arena = self
            .arena
            .lock("deploy-physical-coordination.lock")
            .map_err(err)?;
        self.current()?;
        let _deployment = state.lock_deployment(&home).map_err(err)?;
        let locks = reply
            .resources
            .iter()
            .map(|r| r.resource.clone())
            .collect::<Vec<_>>();
        let _destinations = state.lock_destinations(&locks).map_err(err)?;
        state.verify_bound_home(&self.settings).map_err(err)?;
        super::transaction::Transaction::client(
            self, &state, &home, p, grant, reply, recovery, cancel, clock,
        )
    }
}
#[cfg(test)]
mod m_08_d_2_cases;
