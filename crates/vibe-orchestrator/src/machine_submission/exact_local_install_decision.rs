//! Owner-retained typed approval for the internal exact-local Install slice.
//! This prepares comparison evidence only. It neither reserves a C1 key nor
//! supplies the confirmation gate for an effectful Install.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use vibe_core::manifest::TargetOs;
use vibe_lifecycle::SubmissionScope;
use vibe_lifecycle::process::{SelectedFileWitness, propose_run_id};
use vibe_safefs::Project;

use super::lifecycle_adapter::preflight_one_package_local_install_under_lease;
use super::{PolicyOwner, PolicySnapshot};
use crate::command::lease_default_lifecycle;
use crate::machine_operation::input_decision::{self, ExpectedBinding};
use crate::machine_operation::input_decision::{
    Body, BodyApprovalQuestion, InputDecisionMessage, MAX_FRAME_BYTES, RequestedOptions,
    RequestedOptionsControl,
};
use crate::ports::BoundOnePackageSource;

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
    request_id: String,
    plan_id: String,
    source_id: String,
}
struct Question {
    message: InputDecisionMessage,
    base: Base,
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
    _holder: Arc<()>,
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
    source: Arc<dyn BoundOnePackageSource>,
    target_os: TargetOs,
    issuer: String,
    state: Mutex<State>,
    holder: Arc<()>,
}
impl ExactLocalInstallDecisionHost {
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
            source,
            target_os,
            issuer,
            state: Mutex::new(State {
                counter: 0,
                questions: Vec::new(),
            }),
            holder: Arc::new(()),
        })
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
        let captured = preflight_one_package_local_install_under_lease(
            &leased,
            self.source.as_ref(),
            self.target_os,
        )
        .map_err(|e| ExactLocalInstallDecisionError::SourceUnavailable(format!("{e:#}")))?;
        if captured.canonical_request.len() > CANONICAL_CAP
            || captured.canonical_plan.len() > CANONICAL_CAP
        {
            return Err(ExactLocalInstallDecisionError::Capacity);
        }
        let base = Base {
            request_id: digest(captured.canonical_request),
            plan_id: digest(captured.canonical_plan),
            source_id: format!("sha256:{}", captured.source.tree_sha256),
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
        Ok(PreparedExactLocalInstallEvidence {
            identity: expected_id,
            options: question.requested_options,
            _holder: self.holder.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
