//! Owner-retained typed approval for the internal exact-local Install slice.
//! This prepares comparison evidence only. It neither reserves a C1 key nor
//! supplies the confirmation gate for an effectful Install.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use vibe_core::manifest::TargetOs;
use vibe_lifecycle::process::{SelectedFileWitness, propose_run_id};
use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_safefs::Project;

use super::lifecycle_adapter::{
    FixedOwnerEvidenceSnapshot, RetainedLifecycleResult, cold_snapshot_for_phase,
    inspect_exact_local_install_existing, preflight_one_package_local_install_under_lease,
    submit_approved_one_package_local_install,
};
use super::{PolicyOwner, PolicySnapshot, SubmissionOutcome};
use crate::command::{DefaultLifecycleRequest, LeasedDefaultLifecycle, lease_default_lifecycle};
use crate::install::InstallPolicy;
use crate::machine_operation::input_decision::{self, ExpectedBinding};
use crate::machine_operation::input_decision::{
    Body, BodyApprovalQuestion, InputDecisionMessage, MAX_FRAME_BYTES, RequestedOptions,
    RequestedOptionsControl,
};
use crate::ports::BoundOnePackageSource;
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::Phase;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::shared::Timestamp;

const OPERATION: &str = "rust:vibe-lifecycle:install-exact-local";
const DOMAIN: &str = "rust:vibe-lifecycle";
const MAX_QUESTIONS: usize = 4;
const CANONICAL_CAP: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactLocalInstallDecisionError {
    PolicyDenied,
    StalePolicy,
    ForeignScope,
    UnknownQuestion,
    SourceDrift,
    SourceUnavailable(String),
    RecoveryOnly,
    UnsupportedOptions,
    InvalidData,
    Capacity,
    Infrastructure(String),
}
impl std::fmt::Display for ExactLocalInstallDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exact local Install decision: {self:?}")
    }
}
impl std::error::Error for ExactLocalInstallDecisionError {}
fn infrastructure(error: impl std::fmt::Display) -> ExactLocalInstallDecisionError {
    ExactLocalInstallDecisionError::Infrastructure(format!("{error:#}"))
}
fn digest(bytes: Vec<u8>) -> String {
    SelectedFileWitness::from_bytes(bytes).sha256
}
fn supported(options: &RequestedOptions) -> Result<(), ExactLocalInstallDecisionError> {
    if options.force
        || options.trace
        || options.deploy_profile.is_some()
        || options.control != RequestedOptionsControl::None
    {
        Err(ExactLocalInstallDecisionError::UnsupportedOptions)
    } else {
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Base {
    request: Vec<u8>,
    plan: Vec<u8>,
    request_id: String,
    plan_id: String,
    source_id: String,
}
struct Question {
    message: InputDecisionMessage,
    base: Base,
    approved: bool,
}
struct State {
    counter: u64,
    questions: Vec<Question>,
}

/// Opaque process-local comparison evidence. It has no submission method and
/// cannot approve the incumbent install confirmation gate by itself.
#[derive(Clone)]
pub struct PreparedExactLocalInstallEvidence {
    identity: String,
    options: RequestedOptions,
    holder: Arc<()>,
    question_id: String,
    base: Base,
}
impl PreparedExactLocalInstallEvidence {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn options(&self) -> &RequestedOptions {
        &self.options
    }
}

/// The trusted embedding owns the physical project, current policy owner and
/// bound source port. No wire value can construct or replace those owners.
pub struct ExactLocalInstallDecisionHost {
    root: PathBuf,
    project: Arc<Project>,
    root_token: String,
    scope: SubmissionScope,
    policy: Arc<PolicyOwner>,
    source: Option<Arc<dyn BoundOnePackageSource>>,
    target_os: TargetOs,
    issuer: String,
    state: Mutex<State>,
    holder: Arc<()>,
}
impl ExactLocalInstallDecisionHost {
    pub fn readonly_scope(&self) -> &SubmissionScope {
        &self.scope
    }

    /// Recheck the physical root supplied by a trusted embedding.
    pub fn compare_context_root(
        &self,
        root: &std::path::Path,
    ) -> Result<(), ExactLocalInstallDecisionError> {
        let observed = Project::open(root)
            .map_err(infrastructure)?
            .root_correlation_token()
            .map_err(infrastructure)?;
        if observed != self.root_token {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        Ok(())
    }

    /// Cold C1/C2 observation; a missing capsule is recovery-required, never
    /// absence of effects. This method grants no fresh submission.
    pub fn inspect_existing(
        &self,
        namespace: &SubmissionNamespace,
        key: &str,
    ) -> Result<Option<FixedOwnerEvidenceSnapshot>, ExactLocalInstallDecisionError> {
        let leased = lease_default_lifecycle(&self.root).map_err(infrastructure)?;
        self.compare_context_root(leased.retain_lease().root())?;
        let lease = leased.retain_lease();
        let status = inspect_exact_local_install_existing(&lease, namespace, &self.scope, key)
            .map_err(infrastructure)?;
        status
            .map(|status| {
                cold_snapshot_for_phase(&self.scope, Phase::Install, OPERATION, status)
                    .map_err(infrastructure)
            })
            .transpose()
    }

    /// Read-only host composition. Policy initialization is a separate,
    /// explicit trusted action; question preparation never initializes it.
    pub fn from_host(
        root: PathBuf,
        policy: Arc<PolicyOwner>,
        source: Arc<dyn BoundOnePackageSource>,
        target_os: TargetOs,
    ) -> Result<Self, ExactLocalInstallDecisionError> {
        let project = Arc::new(Project::open(&root).map_err(infrastructure)?);
        let root_token = project.root_correlation_token().map_err(infrastructure)?;
        let issuer = propose_run_id(&root).map_err(infrastructure)?;
        let scope = policy.scope().clone();
        policy.observe_readonly().map_err(infrastructure)?;
        Ok(Self {
            root,
            project,
            root_token,
            scope,
            policy,
            source: Some(source),
            target_os,
            issuer,
            state: Mutex::new(State {
                counter: 0,
                questions: Vec::new(),
            }),
            holder: Arc::new(()),
        })
    }

    /// A cold read-only owner view. It retains no source port, questions or
    /// approval evidence and cannot create a new Install submission.
    pub fn from_recovery_host(
        root: PathBuf,
        policy: Arc<PolicyOwner>,
        target_os: TargetOs,
    ) -> Result<Self, ExactLocalInstallDecisionError> {
        let project = Arc::new(Project::open(&root).map_err(infrastructure)?);
        let root_token = project.root_correlation_token().map_err(infrastructure)?;
        let issuer = propose_run_id(&root).map_err(infrastructure)?;
        let scope = policy.scope().clone();
        policy.observe_readonly().map_err(infrastructure)?;
        Ok(Self {
            root,
            project,
            root_token,
            scope,
            policy,
            source: None,
            target_os,
            issuer,
            state: Mutex::new(State {
                counter: 0,
                questions: Vec::new(),
            }),
            holder: Arc::new(()),
        })
    }

    pub fn recovery_only(&self) -> bool {
        self.source.is_none()
    }

    fn expected<'a>(question: &'a InputDecisionMessage) -> ExpectedBinding<'a> {
        ExpectedBinding {
            question_id: question.question_id.as_deref(),
            operation_id: Some(OPERATION),
            domain_owner: Some(DOMAIN),
            context_id: question.context_id.as_deref(),
            target_id: question.target_id.as_deref(),
            principal_id: question.principal_id.as_deref(),
            plan_id: question.plan_id.as_deref(),
            source_id: question.source_id.as_deref(),
            policy_generation: question.policy_generation.as_deref(),
            requested_options: &question.requested_options,
        }
    }

    fn leased_policy_and_base(
        &self,
    ) -> Result<(PolicySnapshot, Base), ExactLocalInstallDecisionError> {
        let leased = lease_default_lifecycle(&self.root).map_err(infrastructure)?;
        let lease = leased.retain_lease();
        let current = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(infrastructure)?;
        let reopened = Project::open(&self.root)
            .map_err(infrastructure)?
            .root_correlation_token()
            .map_err(infrastructure)?;
        if current != self.root_token
            || reopened != self.root_token
            || self
                .project
                .root_correlation_token()
                .map_err(infrastructure)?
                != self.root_token
        {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        let snapshot = self
            .policy
            .current_under_lease(&lease)
            .map_err(infrastructure)?;
        if !snapshot.facts().approve_exact_local_install {
            return Err(ExactLocalInstallDecisionError::PolicyDenied);
        }
        let source = self
            .source
            .as_ref()
            .ok_or(ExactLocalInstallDecisionError::RecoveryOnly)?;
        let captured = preflight_one_package_local_install_under_lease(
            &leased,
            source.as_ref(),
            self.target_os,
        )
        .map_err(|e| ExactLocalInstallDecisionError::SourceUnavailable(format!("{e:#}")))?;
        if captured.canonical_request.len() > CANONICAL_CAP
            || captured.canonical_plan.len() > CANONICAL_CAP
        {
            return Err(ExactLocalInstallDecisionError::Capacity);
        }
        let base = Base {
            request_id: digest(captured.canonical_request.clone()),
            plan_id: digest(captured.canonical_plan.clone()),
            source_id: captured.source.tree_sha256,
            request: captured.canonical_request,
            plan: captured.canonical_plan,
        };
        Ok((snapshot, base))
    }

    fn resulting_id(
        &self,
        base: &Base,
        options: &RequestedOptions,
        generation: &str,
    ) -> Result<String, ExactLocalInstallDecisionError> {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "format": "vibevm.lifecycle.exact-local-install-decision/1",
            "operation": OPERATION,
            "context": self.scope.context,
            "target": self.scope.target,
            "principal": self.scope.principal,
            "request": base.request_id,
            "base_plan": base.plan_id,
            "source": base.source_id,
            "options": options,
            "policy_generation": generation,
            "approve_exact_local_install": true,
        }))
        .map_err(infrastructure)?;
        Ok(digest(bytes))
    }

    pub fn prepare_requested_options(
        &self,
        options: RequestedOptions,
    ) -> Result<InputDecisionMessage, ExactLocalInstallDecisionError> {
        supported(&options)?;
        let (snapshot, base) = self.leased_policy_and_base()?;
        let mut state = self.state.lock().map_err(infrastructure)?;
        if state.questions.len() >= MAX_QUESTIONS {
            return Err(ExactLocalInstallDecisionError::Capacity);
        }
        state.counter = state
            .counter
            .checked_add(1)
            .ok_or(ExactLocalInstallDecisionError::Capacity)?;
        let question = InputDecisionMessage {
            schema: 1,
            question_id: Some(format!("{}:install-question:{}", self.issuer, state.counter)),
            operation_id: Some(OPERATION.into()),
            domain_owner: Some(DOMAIN.into()),
            context_id: Some(self.scope.context.clone()),
            target_id: Some(self.scope.target.clone()),
            principal_id: Some(self.scope.principal.clone()),
            plan_id: Some(self.resulting_id(&base, &options, snapshot.generation())?),
            source_id: Some(base.source_id.clone()),
            policy_generation: Some(snapshot.generation().into()),
            requested_options: options,
            body: Body::ApprovalQuestion(Box::new(BodyApprovalQuestion {
                prompt: "Approve this exact local package Install plan?".into(),
                effect_summary: "A later admitted submission may install the bound local package. This answer does not submit it.".into(),
            })),
        };
        input_decision::validate(&question, Self::expected(&question))
            .map_err(|_| ExactLocalInstallDecisionError::InvalidData)?;
        state.questions.push(Question {
            message: question.clone(),
            base,
            approved: false,
        });
        Ok(question)
    }

    /// The generated reader validates the bounded frame and exact question.
    /// A true answer remains data until the incumbent owner accepts it later.
    pub fn answer_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<PreparedExactLocalInstallEvidence, ExactLocalInstallDecisionError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(ExactLocalInstallDecisionError::InvalidData);
        }
        let lookup: InputDecisionMessage = serde_json::from_slice(bytes)
            .map_err(|_| ExactLocalInstallDecisionError::InvalidData)?;
        let message = {
            let state = self.state.lock().map_err(infrastructure)?;
            let retained = state
                .questions
                .iter()
                .find(|q| q.message.question_id == lookup.question_id)
                .ok_or(ExactLocalInstallDecisionError::UnknownQuestion)?;
            input_decision::parse(bytes, Self::expected(&retained.message))
                .map_err(|_| ExactLocalInstallDecisionError::InvalidData)?
        };
        self.answer_typed(&message)
    }

    pub fn answer_typed(
        &self,
        answer: &InputDecisionMessage,
    ) -> Result<PreparedExactLocalInstallEvidence, ExactLocalInstallDecisionError> {
        supported(&answer.requested_options)?;
        if answer.context_id.as_deref() != Some(&self.scope.context)
            || answer.target_id.as_deref() != Some(&self.scope.target)
            || answer.principal_id.as_deref() != Some(&self.scope.principal)
        {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        let (question, base) = {
            let state = self.state.lock().map_err(infrastructure)?;
            let retained = state
                .questions
                .iter()
                .find(|q| q.message.question_id == answer.question_id)
                .ok_or(ExactLocalInstallDecisionError::UnknownQuestion)?;
            input_decision::validate_answer(
                answer,
                &retained.message,
                Self::expected(&retained.message),
            )
            .map_err(|_| ExactLocalInstallDecisionError::InvalidData)?;
            (retained.message.clone(), retained.base.clone())
        };
        let Body::ApprovalAnswer(approval) = &answer.body else {
            return Err(ExactLocalInstallDecisionError::InvalidData);
        };
        if !approval.approved {
            return Err(ExactLocalInstallDecisionError::PolicyDenied);
        }
        let (snapshot, current) = self.leased_policy_and_base()?;
        if question.policy_generation.as_deref() != Some(snapshot.generation()) {
            return Err(ExactLocalInstallDecisionError::StalePolicy);
        }
        if current != base {
            return Err(ExactLocalInstallDecisionError::SourceDrift);
        }
        let expected_id =
            self.resulting_id(&current, &question.requested_options, snapshot.generation())?;
        if question.plan_id.as_deref() != Some(expected_id.as_str()) {
            return Err(ExactLocalInstallDecisionError::SourceDrift);
        }
        let question_id = question
            .question_id
            .clone()
            .ok_or(ExactLocalInstallDecisionError::InvalidData)?;
        let mut state = self.state.lock().map_err(infrastructure)?;
        let retained = state
            .questions
            .iter_mut()
            .find(|q| q.message.question_id.as_deref() == Some(&question_id))
            .ok_or(ExactLocalInstallDecisionError::UnknownQuestion)?;
        if retained.base != current || retained.message != question {
            return Err(ExactLocalInstallDecisionError::SourceDrift);
        }
        retained.approved = true;
        Ok(PreparedExactLocalInstallEvidence {
            identity: expected_id,
            options: question.requested_options,
            holder: self.holder.clone(),
            question_id,
            base: current,
        })
    }

    /// Internal owner route only. The typed answer and returned evidence are
    /// insufficient without this host's retained approval record and C1 lease.
    /// This seal is process-local: a new host cannot reconstruct an effectful
    /// retry after restart. Cold C1/C2 result inspection remains separate.
    pub(crate) fn submit_approved(
        &self,
        evidence: &PreparedExactLocalInstallEvidence,
        namespace: &SubmissionNamespace,
        key: &str,
        observed_at: Timestamp,
    ) -> Result<SubmissionOutcome<RetainedLifecycleResult>, ExactLocalInstallDecisionError> {
        if key.is_empty()
            || key.len() > 256
            || key.trim() != key
            || key.chars().any(char::is_control)
        {
            return Err(ExactLocalInstallDecisionError::InvalidData);
        }
        let source = self
            .source
            .as_ref()
            .ok_or(ExactLocalInstallDecisionError::RecoveryOnly)?;
        let profile = AdmittedExactLocalInstallProfile::from_owned(self, evidence)?;
        let request = DefaultLifecycleRequest {
            requested: Phase::Install,
            force: false,
            agent_mode: RunAgentMode::Cli,
            assume_yes: true,
            trace_flag: false,
            install_inputs: Default::default(),
            policy: InstallPolicy {
                offline: true,
                slot_integrity: SlotIntegrity::Verify,
                ..Default::default()
            },
        };
        submit_approved_one_package_local_install(
            &self.root,
            source.as_ref(),
            namespace,
            &self.scope,
            key,
            request,
            self.target_os,
            observed_at,
            &profile,
        )
        .map_err(
            |error| match error.downcast::<ExactLocalInstallDecisionError>() {
                Ok(typed) => typed,
                Err(other) => infrastructure(other),
            },
        )
    }
}

/// A sealed local profile retains the owner and the exact accepted question.
/// Only the owner can construct it; adapters can inspect and guard it.
pub(in crate::machine_submission) struct AdmittedExactLocalInstallProfile<'a> {
    host: &'a ExactLocalInstallDecisionHost,
    evidence: &'a PreparedExactLocalInstallEvidence,
    generation: String,
}
impl<'a> AdmittedExactLocalInstallProfile<'a> {
    fn from_owned(
        host: &'a ExactLocalInstallDecisionHost,
        evidence: &'a PreparedExactLocalInstallEvidence,
    ) -> Result<Self, ExactLocalInstallDecisionError> {
        if !Arc::ptr_eq(&host.holder, &evidence.holder) {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        supported(&evidence.options)?;
        let state = host.state.lock().map_err(infrastructure)?;
        let retained = state
            .questions
            .iter()
            .find(|q| q.message.question_id.as_deref() == Some(&evidence.question_id))
            .ok_or(ExactLocalInstallDecisionError::UnknownQuestion)?;
        if !retained.approved
            || retained.base != evidence.base
            || retained.message.plan_id.as_deref() != Some(&evidence.identity)
            || retained.message.requested_options != evidence.options
        {
            return Err(ExactLocalInstallDecisionError::InvalidData);
        }
        let generation = retained
            .message
            .policy_generation
            .clone()
            .ok_or(ExactLocalInstallDecisionError::InvalidData)?;
        Ok(Self {
            host,
            evidence,
            generation,
        })
    }

    pub(in crate::machine_submission) fn guard_before_attach(
        &self,
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        base_request: &[u8],
    ) -> Result<(), ExactLocalInstallDecisionError> {
        if scope != &self.host.scope {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        let current = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(infrastructure)?;
        let reopened = Project::open(lease.root())
            .map_err(infrastructure)?
            .root_correlation_token()
            .map_err(infrastructure)?;
        if current != self.host.root_token
            || reopened != self.host.root_token
            || self
                .host
                .project
                .root_correlation_token()
                .map_err(infrastructure)?
                != self.host.root_token
        {
            return Err(ExactLocalInstallDecisionError::ForeignScope);
        }
        if base_request != self.evidence.base.request {
            return Err(ExactLocalInstallDecisionError::SourceDrift);
        }
        // Keep the original true-answer record alive across replay; no wire
        // approval or copied evidence can become a new owner.
        Self::from_owned(self.host, self.evidence)?;
        Ok(())
    }

    pub(in crate::machine_submission) fn guard_fresh(
        &self,
        leased: &LeasedDefaultLifecycle,
        base_request: &[u8],
        base_plan: &[u8],
    ) -> Result<(), ExactLocalInstallDecisionError> {
        let lease = leased.retain_lease();
        self.guard_before_attach(&lease, &self.host.scope, base_request)?;
        let snapshot = self
            .host
            .policy
            .current_under_lease(&lease)
            .map_err(infrastructure)?;
        if snapshot.generation() != self.generation {
            return Err(ExactLocalInstallDecisionError::StalePolicy);
        }
        if !snapshot.facts().approve_exact_local_install {
            return Err(ExactLocalInstallDecisionError::PolicyDenied);
        }
        let source = self
            .host
            .source
            .as_ref()
            .ok_or(ExactLocalInstallDecisionError::RecoveryOnly)?;
        source
            .recheck_source_binding(source.source_witness())
            .map_err(|e| ExactLocalInstallDecisionError::SourceUnavailable(format!("{e:#}")))?;
        if base_plan != self.evidence.base.plan {
            return Err(ExactLocalInstallDecisionError::SourceDrift);
        }
        Ok(())
    }

    fn wrap(&self, kind: &str, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        let base: serde_json::Value = serde_json::from_slice(base)?;
        let value = serde_json::json!({
            "format": kind,
            "base": base,
            "base_plan_sha256": self.evidence.base.plan_id,
            "accepted_plan": self.evidence.identity,
            "requested_options": self.evidence.options,
            "policy_generation": self.generation,
            "source": self.evidence.base.source_id,
            "scope": {
                "context": self.host.scope.context,
                "target": self.host.scope.target,
                "principal": self.host.scope.principal,
            },
        });
        let mut out = Vec::new();
        super::lifecycle_plan::write_canonical(&value, &mut out)?;
        anyhow::ensure!(
            out.len() <= CANONICAL_CAP,
            "admitted Install canonical capacity"
        );
        Ok(out)
    }
    pub(in crate::machine_submission) fn request(&self, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.wrap(
            "vibevm.lifecycle.admitted-exact-local-install-request/1",
            base,
        )
    }
    pub(in crate::machine_submission) fn plan(&self, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.wrap("vibevm.lifecycle.admitted-exact-local-install-plan/1", base)
    }
}

#[cfg(test)]
mod tests;
