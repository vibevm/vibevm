//! Trusted exact-local Install actions over the incumbent lifecycle owner.
//! References are process-local; C1/C2 and the lifecycle journal retain effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");

use crate::machine_operation::fixed_lifecycle::contract::{self, *};
use crate::machine_operation::input_decision::InputDecisionMessage;
use crate::machine_operation::lifecycle_view::{self, Status, StatusResultAvailableEffectState};
use crate::machine_submission::{
    ExactLocalInstallDecisionError, ExactLocalInstallDecisionHost, FixedOwnerEvidenceSnapshot,
    PreparedExactLocalInstallEvidence,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_lifecycle::SubmissionNamespace;
use vibe_wire::generated::{format_id::FormatId, machine::e1::operation_envelope as envelope};

pub const OPERATION_ID: &str = "rust:vibe-lifecycle:install-exact-local";
pub const DOMAIN_OWNER: &str = "rust:vibe-lifecycle";

/// One explicit host grant plus its owner-issued namespace. Neither the
/// generated action nor a copied accepted reference can construct this route.
pub struct ExactLocalInstallRoute {
    host: Arc<ExactLocalInstallDecisionHost>,
    namespace: SubmissionNamespace,
    accepted: Mutex<BTreeMap<String, PreparedExactLocalInstallEvidence>>,
}

#[derive(Debug)]
pub struct ExactLocalInstallRouteError {
    pub error: LifecycleActionError,
    pub effects_may_have_begun: bool,
}
impl std::fmt::Display for ExactLocalInstallRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exact local Install action refused: {}",
            self.error.message
        )
    }
}
impl std::error::Error for ExactLocalInstallRouteError {}

fn metadata(request: &LifecycleActionRequest) -> (&'static str, Option<&str>) {
    match &request.action {
        Action::Prepare(_) => ("prepare", None),
        Action::Answer(_) => ("answer", None),
        Action::Submit(s) => ("submit", Some(&s.key)),
        Action::Inspect(s) => ("inspect", Some(&s.key)),
        Action::Attach(s) => ("attach", Some(&s.key)),
        Action::Replay(_) => ("replay", None),
        Action::Control(_) => ("control", None),
        Action::Detach(_) => ("detach", None),
    }
}
fn owner_code(error: &ExactLocalInstallDecisionError) -> LifecycleActionErrorCode {
    use LifecycleActionErrorCode as C;
    match error {
        ExactLocalInstallDecisionError::Capacity => C::Capacity,
        ExactLocalInstallDecisionError::Infrastructure(_) => C::InspectionUnavailable,
        ExactLocalInstallDecisionError::InvalidData => C::InvalidRequest,
        _ => C::DecisionRefused,
    }
}
fn error(
    code: LifecycleActionErrorCode,
    message: &str,
    action: Option<&str>,
    host: Option<&ExactLocalInstallDecisionHost>,
) -> LifecycleActionError {
    let mut end = message.len().min(4096);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    LifecycleActionError {
        schema: 1,
        code,
        message: message[..end].into(),
        message_truncated: end < message.len(),
        requested_action: action.map(str::to_owned),
        requested_phase: Some("install".into()),
        requested_scope: host.map(|host| {
            let scope = host.readonly_scope();
            Scope {
                context: scope.context.clone(),
                target: scope.target.clone(),
                principal: scope.principal.clone(),
            }
        }),
        scope_unavailable_reason: host
            .is_none()
            .then(|| "no explicitly granted Install host scope".into()),
        known_run_id: None,
        view_json: None,
        effect_observation: LifecycleActionErrorEffectObservation::Unavailable,
    }
}
fn enrich(error: &mut LifecycleActionError, snapshot: &FixedOwnerEvidenceSnapshot) {
    error.known_run_id = snapshot.bound_run_id().map(str::to_owned);
    if let Ok((view, json)) = contract::view(snapshot) {
        if let Status::ResultAvailable(result) = view.status {
            error.effect_observation = match result.effect_state {
                StatusResultAvailableEffectState::None => {
                    LifecycleActionErrorEffectObservation::None
                }
                StatusResultAvailableEffectState::Landed => {
                    LifecycleActionErrorEffectObservation::Landed
                }
                StatusResultAvailableEffectState::Uncertain => {
                    LifecycleActionErrorEffectObservation::Uncertain
                }
            };
        }
        error.view_json = Some(json);
    }
}
fn publication_failure(
    grant: Option<&ExactLocalInstallRoute>,
    key: Option<&str>,
    message: &str,
) -> ExactLocalInstallRouteError {
    let mut refusal = ExactLocalInstallRouteError {
        error: error(
            LifecycleActionErrorCode::PublicationUnavailable,
            message,
            None,
            grant.map(|route| route.host.as_ref()),
        ),
        effects_may_have_begun: true,
    };
    if let (Some(route), Some(key)) = (grant, key) {
        if let Ok(Some(snapshot)) = route.host.inspect_existing(&route.namespace, key) {
            enrich(&mut refusal.error, &snapshot);
        }
    }
    refusal
}
fn snapshot_reply(
    snapshot: &FixedOwnerEvidenceSnapshot,
    key: &str,
    submitted: bool,
) -> Result<Reply, String> {
    let (view, payload_json) = contract::view(snapshot)?;
    match view.status {
        Status::RecoveryRequired(_) | Status::InspectionUnavailable(_) => {
            Err("retained Install inspection requires recovery".into())
        }
        _ => Ok(if submitted {
            Reply::Submitted(Box::new(ReplySubmitted {
                key: key.into(),
                format_id: ReplySubmittedFormatId::MachineLifecycleSubmissionView,
                payload_json,
            }))
        } else {
            Reply::View(Box::new(ReplyView {
                key: key.into(),
                format_id: ReplyViewFormatId::MachineLifecycleSubmissionView,
                payload_json,
            }))
        }),
    }
}

impl ExactLocalInstallRoute {
    #[must_use]
    pub fn new(host: Arc<ExactLocalInstallDecisionHost>, namespace: SubmissionNamespace) -> Self {
        Self {
            host,
            namespace,
            accepted: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn compare_context_root(&self, root: &Path) -> Result<(), ExactLocalInstallDecisionError> {
        self.host.compare_context_root(root)
    }

    /// Generated shape and owner admission are checked before any effect.
    pub fn dispatch(
        grant: Option<&Self>,
        bytes: &[u8],
        trusted_root: &Path,
    ) -> Result<LifecycleActionReply, ExactLocalInstallRouteError> {
        let host = grant.map(|route| route.host.as_ref());
        let request: LifecycleActionRequest =
            decode(bytes).map_err(|reason| ExactLocalInstallRouteError {
                error: error(
                    LifecycleActionErrorCode::InvalidRequest,
                    &reason,
                    None,
                    host,
                ),
                effects_may_have_begun: false,
            })?;
        let (action, key) = metadata(&request);
        let refuse = |code, message: &str, begun| ExactLocalInstallRouteError {
            error: error(code, message, Some(action), host),
            effects_may_have_begun: begun,
        };
        if request.schema != 1 {
            return Err(refuse(
                LifecycleActionErrorCode::InvalidRequest,
                "schema must equal 1",
                false,
            ));
        }
        let Some(route) = grant else {
            return Err(refuse(
                LifecycleActionErrorCode::UnavailableHost,
                "no explicitly granted Install host",
                false,
            ));
        };
        route
            .host
            .compare_context_root(trusted_root)
            .map_err(|reason| {
                refuse(
                    LifecycleActionErrorCode::ContextRootMismatch,
                    &reason.to_string(),
                    false,
                )
            })?;
        if key.is_some_and(|key| !contract::key(key)) {
            return Err(refuse(
                LifecycleActionErrorCode::InvalidRequest,
                "invalid submission key",
                false,
            ));
        }
        let result: Result<Reply, (LifecycleActionErrorCode, String, bool)> = match &request.action
        {
            Action::Prepare(row) => {
                if row.phase != "install" {
                    Err((
                        LifecycleActionErrorCode::ImplementationGap,
                        "exact local route admits only Install".into(),
                        false,
                    ))
                } else {
                    route
                        .host
                        .prepare_requested_options(contract::options(&row.requested_options))
                        .map_err(|reason| (owner_code(&reason), reason.to_string(), false))
                        .and_then(|question| {
                            contract::question(&question).map_err(|reason| {
                                (
                                    LifecycleActionErrorCode::PublicationUnavailable,
                                    reason,
                                    true,
                                )
                            })
                        })
                }
            }
            Action::Answer(row) => {
                let original: Result<InputDecisionMessage, _> =
                    serde_json::from_str(&row.message_json);
                match original {
                    Err(_) => Err((
                        LifecycleActionErrorCode::InvalidRequest,
                        "generated decision shape refused".into(),
                        false,
                    )),
                    Ok(original) => route
                        .host
                        .answer_bytes(row.message_json.as_bytes())
                        .map_err(|reason| (owner_code(&reason), reason.to_string(), false))
                        .and_then(|evidence| {
                            let reference = original.question_id.ok_or((
                                LifecycleActionErrorCode::InvalidRequest,
                                "missing question id".into(),
                                false,
                            ))?;
                            let mut accepted = route.accepted.lock().map_err(|_| {
                                (
                                    LifecycleActionErrorCode::Capacity,
                                    "accepted table unavailable".into(),
                                    false,
                                )
                            })?;
                            if !accepted.contains_key(&reference) && accepted.len() >= 4 {
                                return Err((
                                    LifecycleActionErrorCode::Capacity,
                                    "accepted table full".into(),
                                    false,
                                ));
                            }
                            let plan_id = evidence.identity().to_owned();
                            accepted.insert(reference.clone(), evidence);
                            Ok(Reply::Validated(Box::new(ReplyValidated {
                                accepted_ref: reference,
                                plan_id,
                            })))
                        }),
                }
            }
            Action::Submit(row) => {
                let evidence = route
                    .accepted
                    .lock()
                    .ok()
                    .and_then(|table| table.get(&row.accepted_ref).cloned());
                match evidence {
                    None => Err((
                        LifecycleActionErrorCode::UnknownReference,
                        "accepted reference is not retained by this route".into(),
                        true,
                    )),
                    Some(evidence) => {
                        match route.host.submit_approved(
                            &evidence,
                            &route.namespace,
                            &row.key,
                            chrono::Utc::now().to_rfc3339().parse().map_err(|_| {
                                refuse(
                                    LifecycleActionErrorCode::SubmissionRefused,
                                    "host clock unavailable",
                                    false,
                                )
                            })?,
                        ) {
                            Err(reason) => Err((owner_code(&reason), reason.to_string(), true)),
                            Ok(_) => route
                                .host
                                .inspect_existing(&route.namespace, &row.key)
                                .map_err(|reason| (owner_code(&reason), reason.to_string(), true))
                                .and_then(|snapshot| {
                                    snapshot.ok_or((
                                        LifecycleActionErrorCode::InspectionUnavailable,
                                        "C1 result absent after submit".into(),
                                        true,
                                    ))
                                })
                                .and_then(|snapshot| {
                                    snapshot_reply(&snapshot, &row.key, true).map_err(|reason| {
                                        (
                                            LifecycleActionErrorCode::InspectionUnavailable,
                                            reason,
                                            true,
                                        )
                                    })
                                }),
                        }
                    }
                }
            }
            Action::Inspect(row) => route
                .host
                .inspect_existing(&route.namespace, &row.key)
                .map_err(|reason| (owner_code(&reason), reason.to_string(), true))
                .and_then(|snapshot| match snapshot {
                    None => Ok(Reply::Absent(Box::new(ReplyAbsent {
                        key: row.key.clone(),
                    }))),
                    Some(snapshot) => {
                        snapshot_reply(&snapshot, &row.key, false).map_err(|reason| {
                            (
                                LifecycleActionErrorCode::InspectionUnavailable,
                                reason,
                                true,
                            )
                        })
                    }
                }),
            _ => Err((
                LifecycleActionErrorCode::ImplementationGap,
                "exact local Install supports prepare/answer/submit/inspect".into(),
                false,
            )),
        };
        match result {
            Ok(reply) => {
                let reply = LifecycleActionReply { schema: 1, reply };
                size(&reply, PUBLICATION_CAP)
                    .map_err(|reason| publication_failure(grant, key, &reason))?;
                Ok(reply)
            }
            Err((code, reason, begun)) => {
                let mut refusal = refuse(code, &reason, begun);
                if let Some(key) = key {
                    if let Ok(Some(snapshot)) = route.host.inspect_existing(&route.namespace, key) {
                        enrich(&mut refusal.error, &snapshot);
                    }
                }
                Err(refusal)
            }
        }
    }

    /// The local API accepts the registered action input inside the incumbent
    /// envelope. The outer stage preserves owner effect truth on failure.
    pub fn exchange_local(
        grant: Option<&Self>,
        bytes: &[u8],
        trusted_root: &Path,
    ) -> Result<envelope::OperationEnvelope, ExactLocalInstallRouteError> {
        let host = grant.map(|route| route.host.as_ref());
        let admitted = (|| -> Result<LifecycleActionRequest, String> {
            if bytes.len() > PUBLICATION_CAP {
                return Err("envelope exceeds cap".into());
            }
            let input: envelope::OperationEnvelope =
                serde_json::from_slice(bytes).map_err(|_| "generated envelope shape refused")?;
            if input.schema != 1
                || input.operation_id.as_deref() != Some(OPERATION_ID)
                || input.domain_owner.as_deref() != Some(DOMAIN_OWNER)
                || input.source_id.is_some()
                || input.evidence_id.is_some()
            {
                return Err("input epoch/operation/owner or caller evidence refused".into());
            }
            let envelope::Body::Input(body) = input.body else {
                return Err("expected input stage".into());
            };
            if body.format_id != FormatId::MachineLifecycleActionRequest.id() {
                return Err("exact registered action request format required".into());
            }
            let request: LifecycleActionRequest = decode(body.payload_json.as_bytes())?;
            if !matches!(
                request.action,
                Action::Prepare(_) | Action::Answer(_) | Action::Submit(_) | Action::Inspect(_)
            ) {
                return Err("local Install actions are prepare/answer/submit/inspect".into());
            }
            Ok(request)
        })();
        let key = admitted
            .as_ref()
            .ok()
            .and_then(|request| metadata(request).1)
            .map(str::to_owned);
        let response = match admitted {
            Ok(request) => match encode(&request, REQUEST_CAP) {
                Ok(raw) => Self::dispatch(grant, raw.as_bytes(), trusted_root),
                Err(reason) => Err(ExactLocalInstallRouteError {
                    error: error(
                        LifecycleActionErrorCode::InvalidRequest,
                        &reason,
                        None,
                        host,
                    ),
                    effects_may_have_begun: false,
                }),
            },
            Err(reason) => Err(ExactLocalInstallRouteError {
                error: error(
                    LifecycleActionErrorCode::InvalidRequest,
                    &reason,
                    None,
                    host,
                ),
                effects_may_have_begun: false,
            }),
        };
        let candidate = match response {
            Ok(reply) => {
                let truth = reply_truth(&reply)
                    .map_err(|reason| publication_failure(grant, key.as_deref(), &reason))?;
                let source = if let Reply::Question(question) = &reply.reply {
                    let question: InputDecisionMessage =
                        serde_json::from_str(&question.payload_json).map_err(|_| {
                            publication_failure(
                                grant,
                                key.as_deref(),
                                "generated question shape refused",
                            )
                        })?;
                    question.source_id
                } else {
                    None
                };
                envelope::OperationEnvelope {
                    schema: 1,
                    operation_id: Some(OPERATION_ID.into()),
                    domain_owner: Some(DOMAIN_OWNER.into()),
                    source_id: source,
                    evidence_id: None,
                    body: envelope::Body::Result(Box::new(envelope::BodyResult {
                        format_id: Some(FormatId::MachineLifecycleActionReply.id().into()),
                        payload_json: Some(encode(&reply, PUBLICATION_CAP).map_err(|reason| {
                            publication_failure(grant, key.as_deref(), &reason)
                        })?),
                        effect_state: truth.result_effect(),
                        completeness: truth.result_completeness(),
                        provenance_id: truth.run,
                    })),
                }
            }
            Err(refusal) => {
                let truth = refusal_truth(&refusal)
                    .map_err(|reason| publication_failure(grant, key.as_deref(), &reason))?;
                envelope::OperationEnvelope {
                    schema: 1,
                    operation_id: Some(OPERATION_ID.into()),
                    domain_owner: Some(DOMAIN_OWNER.into()),
                    source_id: None,
                    evidence_id: None,
                    body: envelope::Body::Error(Box::new(envelope::BodyError {
                        format_id: Some(FormatId::MachineLifecycleActionError.id().into()),
                        payload_json: Some(encode(&refusal.error, PUBLICATION_CAP).map_err(
                            |reason| publication_failure(grant, key.as_deref(), &reason),
                        )?),
                        effect_state: truth.error_effect(),
                        completeness: truth.error_completeness(),
                        provenance_id: truth.run,
                    })),
                }
            }
        };
        let serialized = encode(&candidate, PUBLICATION_CAP)
            .map_err(|reason| publication_failure(grant, key.as_deref(), &reason))?;
        let checked: envelope::OperationEnvelope =
            serde_json::from_str(&serialized).map_err(|_| {
                publication_failure(
                    grant,
                    key.as_deref(),
                    "generated envelope publication refused",
                )
            })?;
        if checked != candidate {
            return Err(publication_failure(
                grant,
                key.as_deref(),
                "publication roundtrip mismatch",
            ));
        }
        Ok(candidate)
    }
}

#[derive(Clone, Copy)]
enum Effect {
    None,
    Landed,
    Uncertain,
}
#[derive(Clone, Copy)]
enum Completeness {
    Complete,
    Partial,
    Unavailable,
}
struct Truth {
    effect: Effect,
    completeness: Completeness,
    run: Option<String>,
}
impl Truth {
    fn result_effect(&self) -> envelope::BodyResultEffectState {
        match self.effect {
            Effect::None => envelope::BodyResultEffectState::NoEffects,
            Effect::Landed => envelope::BodyResultEffectState::Completed,
            Effect::Uncertain => envelope::BodyResultEffectState::Uncertain,
        }
    }
    fn error_effect(&self) -> envelope::BodyErrorEffectState {
        match self.effect {
            Effect::None => envelope::BodyErrorEffectState::NoEffects,
            Effect::Landed => envelope::BodyErrorEffectState::Completed,
            Effect::Uncertain => envelope::BodyErrorEffectState::Uncertain,
        }
    }
    fn result_completeness(&self) -> envelope::BodyResultCompleteness {
        match self.completeness {
            Completeness::Complete => envelope::BodyResultCompleteness::Complete,
            Completeness::Partial => envelope::BodyResultCompleteness::Partial,
            Completeness::Unavailable => envelope::BodyResultCompleteness::Unavailable,
        }
    }
    fn error_completeness(&self) -> envelope::BodyErrorCompleteness {
        match self.completeness {
            Completeness::Complete => envelope::BodyErrorCompleteness::Complete,
            Completeness::Partial => envelope::BodyErrorCompleteness::Partial,
            Completeness::Unavailable => envelope::BodyErrorCompleteness::Unavailable,
        }
    }
}
fn view_truth(bytes: &str) -> Result<Truth, String> {
    let view: lifecycle_view::LifecycleSubmissionView =
        serde_json::from_str(bytes).map_err(|_| "owner view no longer decodes")?;
    if view.schema != 1 || view.operation_id != OPERATION_ID || view.requested_phase != "install" {
        return Err("owner view operation/phase mismatch".into());
    }
    let (effect, completeness) = match &view.status {
        Status::ResultAvailable(result) => (
            match result.effect_state {
                StatusResultAvailableEffectState::None => Effect::None,
                StatusResultAvailableEffectState::Landed => Effect::Landed,
                StatusResultAvailableEffectState::Uncertain => Effect::Uncertain,
            },
            if matches!(result.report, lifecycle_view::Report::Unavailable(_)) {
                Completeness::Unavailable
            } else if view.partial
                || matches!(
                    result.outcome,
                    lifecycle_view::StatusResultAvailableOutcome::Parked
                        | lifecycle_view::StatusResultAvailableOutcome::Uncertain
                )
            {
                Completeness::Partial
            } else {
                Completeness::Complete
            },
        ),
        Status::Starting(_) | Status::Running(_) => (Effect::Uncertain, Completeness::Partial),
        Status::RecoveryRequired(_) | Status::InspectionUnavailable(_) => {
            (Effect::Uncertain, Completeness::Unavailable)
        }
    };
    Ok(Truth {
        effect,
        completeness,
        run: view.run_id,
    })
}
fn reply_truth(reply: &LifecycleActionReply) -> Result<Truth, String> {
    match &reply.reply {
        Reply::Submitted(row) => view_truth(&row.payload_json),
        Reply::View(row) => view_truth(&row.payload_json),
        Reply::Question(_) | Reply::Validated(_) => Ok(Truth {
            effect: Effect::Uncertain,
            completeness: Completeness::Partial,
            run: None,
        }),
        Reply::Absent(_) => Ok(Truth {
            effect: Effect::Uncertain,
            completeness: Completeness::Complete,
            run: None,
        }),
        _ => Err("unsupported local Install reply".into()),
    }
}
fn refusal_truth(refusal: &ExactLocalInstallRouteError) -> Result<Truth, String> {
    let error = &refusal.error;
    let mut truth = if let Some(view) = &error.view_json {
        view_truth(view)?
    } else {
        Truth {
            effect: if refusal.effects_may_have_begun {
                Effect::Uncertain
            } else {
                Effect::None
            },
            completeness: Completeness::Unavailable,
            run: None,
        }
    };
    if truth.run.is_none() {
        truth.run = error.known_run_id.clone();
    }
    if error.view_json.is_none() {
        truth.effect = match error.effect_observation {
            LifecycleActionErrorEffectObservation::None => Effect::None,
            LifecycleActionErrorEffectObservation::Landed => Effect::Landed,
            LifecycleActionErrorEffectObservation::Uncertain => Effect::Uncertain,
            LifecycleActionErrorEffectObservation::Unavailable => truth.effect,
        };
    }
    Ok(truth)
}
