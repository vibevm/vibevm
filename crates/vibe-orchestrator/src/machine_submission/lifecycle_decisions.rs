//! Owner-retained decisions for the incumbent fixed script profile.
//! This is preparation evidence only. Durable submission still needs fresh
//! policy checks and a new C1 canonical request before any early attachment.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");
// Owner-bound decisions and requested options: req r1.

use super::lifecycle_adapter::preflight_one_script_under_lease;
use crate::command::{DefaultLifecycleRequest, LeasedDefaultLifecycle, lease_default_lifecycle};
use crate::machine_operation::input_decision::{self, ExpectedBinding};
use input_decision::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::process::{SelectedFileWitness, propose_run_id};
use vibe_lifecycle::{SubmissionNamespace, SubmissionReservations, SubmissionScope};
use vibe_safefs::Project;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

mod policy;
mod questions;
pub use policy::HostPolicyFacts;
use policy::State;
use questions::*;
const MAX_SEQUENCES: usize = 4;
const MAX_RETAINED: usize = 8 * 1024 * 1024;
const CANONICAL_CAP: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedDecisionError {
    PolicyDenied,
    StalePolicy,
    ForeignScope,
    UnknownQuestion,
    SourceUnavailable(String),
    SourceDrift(String),
    Capacity,
    ImplementationGap(String),
    InvalidData,
    AnswerConflict,
    Infrastructure(String),
}
impl std::fmt::Display for FixedDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fixed lifecycle decision: {self:?}")
    }
}
impl std::error::Error for FixedDecisionError {}
fn infrastructure(e: impl std::fmt::Display) -> FixedDecisionError {
    FixedDecisionError::Infrastructure(format!("{e:#}"))
}
fn digest(bytes: Vec<u8>) -> String {
    SelectedFileWitness::from_bytes(bytes).sha256
}
fn capacity_available(count: usize, retained: usize, added: usize) -> bool {
    count < MAX_SEQUENCES
        && retained
            .checked_add(added)
            .is_some_and(|n| n <= MAX_RETAINED)
}
fn supported(options: &RequestedOptions) -> Result<(), FixedDecisionError> {
    let mut unsupported = Vec::new();
    if options.force {
        unsupported.push("force");
    }
    if options.trace {
        unsupported.push("trace");
    }
    if options.deploy_profile.is_some() {
        unsupported.push("deploy_profile");
    }
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(FixedDecisionError::ImplementationGap(format!(
            "fixed one-script decision profile does not implement requested option(s): {}",
            unsupported.join(", ")
        )))
    }
}

/// Acquisition may create `.vibe/` and its persistent lifecycle lock file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationEffects {
    LeaseInfrastructureMayBeCreated,
}

struct CapturedPlan {
    request: Vec<u8>,
    plan: Vec<u8>,
    request_id: String,
    plan_id: String,
    source_id: String,
}
impl CapturedPlan {
    fn retained_bytes(&self) -> usize {
        self.request.len() + self.plan.len()
    }
}
struct Sequence {
    base: Arc<CapturedPlan>,
    original: InputDecisionMessage,
    approval: Option<InputDecisionMessage>,
    selected: Option<String>,
    approved: Option<bool>,
}

/// Opaque, process-local preparation evidence. No execution, admission, grant,
/// control or journal methods. It expires as authority facts/source change;
/// any future durable submission MUST recheck those independently and associate
/// evidence with its private retained host/source/policy record. Comparing this
/// emitted identity alone cannot grant admission.
#[derive(Clone)]
pub struct ValidatedFixedPlanEvidence {
    identity: String,
    options: RequestedOptions,
}
impl ValidatedFixedPlanEvidence {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn options(&self) -> &RequestedOptions {
        &self.options
    }
}
pub enum DecisionOutcome {
    Question(InputDecisionMessage),
    Validated(ValidatedFixedPlanEvidence),
}

/// Trusted host owns the original physical project, immutable coordinates,
/// policy and bounded retained questions. No provider or transport is coupled
/// to its cell; the cell is never held during filesystem I/O.
pub struct FixedLifecycleDecisionHost {
    root: PathBuf,
    project: Project,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    target_os: TargetOs,
    issuer: String,
    state: Mutex<State>,
}
impl FixedLifecycleDecisionHost {
    /// Trusted embedding boundary. Wire bodies cannot construct these facts.
    /// This only looks up a known namespace under the lease; it neither issues
    /// one nor checks expiry for effect admission. Lease infrastructure may be
    /// created, so errors/success are not a zero-write guarantee.
    pub fn from_host(
        root: PathBuf,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        target_os: TargetOs,
        policy: HostPolicyFacts,
    ) -> Result<Self, FixedDecisionError> {
        let project = Project::open(&root).map_err(infrastructure)?;
        let issuer = propose_run_id(&root).map_err(infrastructure)?; // nonce only, never a domain run
        let host = Self {
            root,
            project,
            namespace,
            scope,
            target_os,
            issuer,
            state: Mutex::new(State {
                facts: policy,
                revision: 1,
                counter: 0,
                sequences: vec![],
            }),
        };
        let _ = host.scope_lease()?;
        Ok(host)
    }
    pub fn preparation_effects(&self) -> PreparationEffects {
        PreparationEffects::LeaseInfrastructureMayBeCreated
    }
    pub fn set_host_policy(&self, facts: HostPolicyFacts) -> Result<(), FixedDecisionError> {
        let mut state = self.state.lock().map_err(infrastructure)?;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(FixedDecisionError::Capacity)?;
        state.facts = facts;
        state.sequences.clear();
        Ok(())
    }
    fn scope_lease(&self) -> Result<LeasedDefaultLifecycle, FixedDecisionError> {
        let leased = lease_default_lifecycle(&self.root).map_err(infrastructure)?;
        let lease = leased.retain_lease();
        // Compare freshly leased physical root with the original retained handle.
        let original = self
            .project
            .root_correlation_token()
            .map_err(|_| FixedDecisionError::ForeignScope)?;
        let current = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(|_| FixedDecisionError::ForeignScope)?;
        if original != current {
            return Err(FixedDecisionError::ForeignScope);
        }
        SubmissionReservations::inspect_existing_key(
            &lease,
            &self.namespace,
            &self.scope,
            "fixed-decision-namespace-lookup",
        )
        .map_err(|e| match e {
            vibe_lifecycle::SubmissionError::ForeignScope => FixedDecisionError::ForeignScope,
            _ => infrastructure(e),
        })?;
        Ok(leased)
    }
    fn capture(&self) -> Result<(CapturedPlan, LeasedDefaultLifecycle), FixedDecisionError> {
        let leased = self.scope_lease()?;
        let request = DefaultLifecycleRequest {
            requested: vibe_lifecycle::Phase::Validate,
            force: false,
            agent_mode: RunAgentMode::Cli,
            assume_yes: true,
            trace_flag: false,
            install_inputs: Default::default(),
            policy: Default::default(),
        };
        let admitted = preflight_one_script_under_lease(&leased, &request, self.target_os)
            .map_err(|e| FixedDecisionError::SourceUnavailable(format!("{e:#}")))?;
        if admitted.canonical_request.len() > CANONICAL_CAP
            || admitted.canonical_plan.len() > CANONICAL_CAP
        {
            return Err(FixedDecisionError::Capacity);
        }
        vibe_lifecycle::process::recheck_script_source_witness(&admitted.source)
            .map_err(|e| FixedDecisionError::SourceUnavailable(format!("{e:#}")))?;
        // Source helpers use incumbent path reads. Reopen and correlate after
        // those reads too, while the original outer lease is still held.
        let observed = Project::open(&self.root).map_err(|_| FixedDecisionError::ForeignScope)?;
        if observed
            .root_correlation_token()
            .map_err(|_| FixedDecisionError::ForeignScope)?
            != self
                .project
                .root_correlation_token()
                .map_err(|_| FixedDecisionError::ForeignScope)?
        {
            return Err(FixedDecisionError::ForeignScope);
        }
        let plan_id = digest(admitted.canonical_plan.clone());
        let request_id = digest(admitted.canonical_request.clone());
        let source_id = format!("sha256:{}", admitted.source.source.sha256);
        Ok((
            CapturedPlan {
                request: admitted.canonical_request,
                plan: admitted.canonical_plan,
                request_id,
                plan_id,
                source_id,
            },
            leased,
        ))
    }
    fn message(
        &self,
        id: String,
        generation: String,
        base: &CapturedPlan,
        plan_id: String,
        options: RequestedOptions,
        body: Body,
    ) -> InputDecisionMessage {
        InputDecisionMessage {
            schema: 1,
            question_id: Some(id),
            operation_id: Some(OPERATION.into()),
            domain_owner: Some(DOMAIN.into()),
            context_id: Some(self.scope.context.clone()),
            target_id: Some(self.scope.target.clone()),
            principal_id: Some(self.scope.principal.clone()),
            plan_id: Some(plan_id),
            source_id: Some(base.source_id.clone()),
            policy_generation: Some(generation),
            requested_options: options,
            body,
        }
    }
    /// `control=None` asks an actual supported control-policy selection, including
    /// the legitimate no-control choice. Explicit permitted controls ask approval.
    pub fn prepare_requested_options(
        &self,
        options: RequestedOptions,
    ) -> Result<InputDecisionMessage, FixedDecisionError> {
        supported(&options)?; // before any source/runtime calls
        let (revision, facts) = {
            let state = self.state.lock().map_err(infrastructure)?;
            if !state.facts.permits(&options.control) {
                return Err(FixedDecisionError::PolicyDenied);
            }
            if state.sequences.len() >= MAX_SEQUENCES {
                return Err(FixedDecisionError::Capacity);
            }
            (state.revision, state.facts.clone())
        };
        let (base, _held_lease) = self.capture()?;
        let base = Arc::new(base);
        let mut state = self.state.lock().map_err(infrastructure)?;
        if revision != state.revision {
            return Err(FixedDecisionError::StalePolicy);
        }
        let total: usize = state
            .sequences
            .iter()
            .map(|s| s.base.retained_bytes())
            .sum();
        if !capacity_available(state.sequences.len(), total, base.retained_bytes()) {
            return Err(FixedDecisionError::Capacity);
        }
        let generation = state.generation(&self.issuer);
        let (body, plan_id) = if options.control == RequestedOptionsControl::None {
            (
                Body::SelectionQuestion(Box::new(BodySelectionQuestion {
                    prompt: "Select a supported lifecycle control policy.".into(),
                    min_choices: 1,
                    max_choices: 1,
                    choices: choices(&facts),
                })),
                base.plan_id.clone(),
            )
        } else {
            (
                approval_body(),
                resulting_id(&base, &options, &generation, &facts),
            )
        };
        let question = self.message(
            state.question_id(&self.issuer)?,
            generation,
            &base,
            plan_id,
            options,
            body,
        );
        input_decision::validate(&question, expected(&question))
            .map_err(|_| FixedDecisionError::InvalidData)?;
        state.sequences.push(Sequence {
            base,
            original: question.clone(),
            approval: None,
            selected: None,
            approved: None,
        });
        Ok(question)
    }
    /// Enforces the 64KiB frame bound before the canonical generated reader.
    pub fn answer(&self, bytes: &[u8]) -> Result<DecisionOutcome, FixedDecisionError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(FixedDecisionError::InvalidData);
        }
        let answer: InputDecisionMessage =
            serde_json::from_slice(bytes).map_err(|_| FixedDecisionError::InvalidData)?;
        self.answer_typed(&answer)
    }
    pub fn answer_typed(
        &self,
        answer: &InputDecisionMessage,
    ) -> Result<DecisionOutcome, FixedDecisionError> {
        supported(&answer.requested_options)?;
        let (revision, base, question) = {
            let state = self.state.lock().map_err(infrastructure)?;
            if answer.policy_generation.as_deref() != Some(&state.generation(&self.issuer)) {
                return Err(FixedDecisionError::StalePolicy);
            }
            if answer.context_id.as_deref() != Some(&self.scope.context)
                || answer.target_id.as_deref() != Some(&self.scope.target)
                || answer.principal_id.as_deref() != Some(&self.scope.principal)
            {
                return Err(FixedDecisionError::ForeignScope);
            }
            let sequence = state
                .sequences
                .iter()
                .find(|s| {
                    s.original.question_id == answer.question_id
                        || s.approval
                            .as_ref()
                            .is_some_and(|q| q.question_id == answer.question_id)
                })
                .ok_or(FixedDecisionError::UnknownQuestion)?;
            let q = sequence
                .approval
                .as_ref()
                .filter(|q| q.question_id == answer.question_id)
                .unwrap_or(&sequence.original);
            input_decision::validate_answer(answer, q, expected(q))
                .map_err(|_| FixedDecisionError::InvalidData)?;
            if !state.facts.permits(&q.requested_options.control) {
                return Err(FixedDecisionError::PolicyDenied);
            }
            (state.revision, sequence.base.clone(), q.clone())
        };
        let (fresh, _held_lease) = self.capture().map_err(|error| match error {
            FixedDecisionError::SourceUnavailable(reason) => {
                FixedDecisionError::SourceDrift(format!("fresh source preflight failed: {reason}"))
            }
            other => other,
        })?;
        if fresh.plan_id != base.plan_id
            || fresh.request_id != base.request_id
            || fresh.source_id != base.source_id
        {
            return Err(FixedDecisionError::SourceDrift(format!(
                "captured source identity changed: original plan={}, request={}, source={}; current plan={}, request={}, source={}",
                base.plan_id,
                base.request_id,
                base.source_id,
                fresh.plan_id,
                fresh.request_id,
                fresh.source_id,
            )));
        }
        let mut state = self.state.lock().map_err(infrastructure)?;
        if state.revision != revision {
            return Err(FixedDecisionError::StalePolicy);
        }
        let index = state
            .sequences
            .iter()
            .position(|s| {
                s.original.question_id == answer.question_id
                    || s.approval
                        .as_ref()
                        .is_some_and(|q| q.question_id == answer.question_id)
            })
            .ok_or(FixedDecisionError::UnknownQuestion)?;
        match &answer.body {
            Body::SelectionAnswer(a) => {
                let choice = &a.choice_ids[0]; // validated cardinality
                if let Some(previous) = &state.sequences[index].selected {
                    if previous != choice {
                        return Err(FixedDecisionError::AnswerConflict);
                    }
                    return Ok(DecisionOutcome::Question(
                        state.sequences[index]
                            .approval
                            .clone()
                            .expect("selected approval"),
                    ));
                }
                let mut options = question.requested_options.clone();
                options.control = selected(choice)?;
                if !state.facts.permits(&options.control) {
                    return Err(FixedDecisionError::PolicyDenied);
                }
                let generation = state.generation(&self.issuer);
                let plan_id = resulting_id(&base, &options, &generation, &state.facts);
                let q = self.message(
                    state.question_id(&self.issuer)?,
                    generation,
                    &base,
                    plan_id,
                    options,
                    approval_body(),
                );
                state.sequences[index].selected = Some(choice.clone());
                state.sequences[index].approval = Some(q.clone());
                Ok(DecisionOutcome::Question(q))
            }
            Body::ApprovalAnswer(a) => {
                if state.sequences[index]
                    .approved
                    .is_some_and(|previous| previous != a.approved)
                {
                    return Err(FixedDecisionError::AnswerConflict);
                }
                state.sequences[index].approved = Some(a.approved);
                if !a.approved || !state.facts.approve_captured_script {
                    return Err(FixedDecisionError::PolicyDenied);
                }
                Ok(DecisionOutcome::Validated(ValidatedFixedPlanEvidence {
                    identity: question.plan_id.expect("owned identity"),
                    options: question.requested_options,
                }))
            }
            _ => Err(FixedDecisionError::InvalidData),
        }
    }
}
#[cfg(test)]
mod native_tests;
#[cfg(test)]
mod tests;
