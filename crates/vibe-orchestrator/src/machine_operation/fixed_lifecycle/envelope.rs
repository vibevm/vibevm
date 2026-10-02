//! Exact local Input/Result/Error exchange. Generated values never supply a grant.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-LIFECYCLE-EXCHANGE");
use super::{FixedLifecycleRoute, FixedLifecycleRouteError, contract::*};
use crate::machine_operation::lifecycle_view as v;
use std::path::Path;
use vibe_wire::generated::{format_id::FormatId, machine::e1::operation_envelope as e};

pub const OPERATION_ID: &str = crate::machine_submission::FIXED_OPERATION;
pub const DOMAIN_OWNER: &str = crate::machine_submission::FIXED_DOMAIN_OWNER;

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
    fn unknown(admitted: bool) -> Self {
        Self {
            effect: if admitted {
                Effect::Uncertain
            } else {
                Effect::None
            },
            completeness: Completeness::Unavailable,
            run: None,
        }
    }
}
fn view_truth(bytes: &str) -> Result<Truth, String> {
    // These bytes were projected and semantically checked against the opaque
    // owner sample by dispatch::snapshot_reply or contract::enrich.
    let row: v::LifecycleSubmissionView = serde_json::from_str(bytes)
        .map_err(|_| "owner view no longer decodes as the generated root")?;
    if row.schema != 1 || row.operation_id != OPERATION_ID {
        return Err("owner view epoch/operation mismatch".into());
    }
    let (effect, completeness) = match &row.status {
        v::Status::ResultAvailable(r) => (
            match r.effect_state {
                v::StatusResultAvailableEffectState::None => Effect::None,
                v::StatusResultAvailableEffectState::Landed => Effect::Landed,
                v::StatusResultAvailableEffectState::Uncertain => Effect::Uncertain,
            },
            if matches!(r.report, v::Report::Unavailable(_)) {
                Completeness::Unavailable
            } else if row.partial
                || matches!(
                    r.outcome,
                    v::StatusResultAvailableOutcome::Parked
                        | v::StatusResultAvailableOutcome::Uncertain
                )
            {
                Completeness::Partial
            } else {
                Completeness::Complete
            },
        ),
        v::Status::Starting(_) | v::Status::Running(_) => {
            (Effect::Uncertain, Completeness::Partial)
        }
        v::Status::RecoveryRequired(_) | v::Status::InspectionUnavailable(_) => {
            (Effect::Uncertain, Completeness::Unavailable)
        }
    };
    Ok(Truth {
        effect,
        completeness,
        run: row.run_id,
    })
}
fn result_truth(reply: &LifecycleActionReply) -> Result<Truth, String> {
    if reply.schema != 1 {
        return Err("reply epoch mismatch".into());
    }
    match &reply.reply {
        Reply::Submitted(r) => view_truth(&r.payload_json),
        Reply::View(r) => view_truth(&r.payload_json),
        Reply::Question(_) | Reply::Validated(_) => Ok(Truth {
            // Preparation/answer can create lease infrastructure. No terminal
            // provider evidence exists yet, even after exact approval.
            effect: Effect::Uncertain,
            completeness: Completeness::Partial,
            run: None,
        }),
        Reply::Absent(_) => Ok(Truth {
            effect: Effect::Uncertain,
            completeness: Completeness::Complete,
            run: None,
        }),
        _ => Err("local action exchange cannot publish observer replies".into()),
    }
}
fn outer(body: e::Body, source: Option<String>) -> e::OperationEnvelope {
    e::OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION_ID.into()),
        domain_owner: Some(DOMAIN_OWNER.into()),
        source_id: source,
        evidence_id: None,
        body,
    }
}
fn error_envelope(refusal: &FixedLifecycleRouteError) -> Result<e::OperationEnvelope, String> {
    let error = &refusal.error;
    let mut truth = match &error.view_json {
        Some(bytes) => view_truth(bytes)?,
        None => Truth::unknown(refusal.effects_may_have_begun),
    };
    if truth.run.is_none() {
        truth.run = error.known_run_id.clone();
    }
    // Owner effect evidence wins even for a pre-dispatch refusal of an
    // existing run. An Err alone never means NoEffects.
    if error.view_json.is_none() {
        truth.effect = match error.effect_observation {
            LifecycleActionErrorEffectObservation::None => Effect::None,
            LifecycleActionErrorEffectObservation::Landed => Effect::Landed,
            LifecycleActionErrorEffectObservation::Uncertain => Effect::Uncertain,
            LifecycleActionErrorEffectObservation::Unavailable => truth.effect,
        };
    }
    Ok(outer(
        e::Body::Error(Box::new(e::BodyError {
            format_id: Some(FormatId::MachineLifecycleActionError.id().into()),
            payload_json: Some(encode(error, PUBLICATION_CAP)?),
            effect_state: match truth.effect {
                Effect::None => e::BodyErrorEffectState::NoEffects,
                Effect::Landed => e::BodyErrorEffectState::Completed,
                Effect::Uncertain => e::BodyErrorEffectState::Uncertain,
            },
            completeness: match truth.completeness {
                Completeness::Complete => e::BodyErrorCompleteness::Complete,
                Completeness::Partial => e::BodyErrorCompleteness::Partial,
                Completeness::Unavailable => e::BodyErrorCompleteness::Unavailable,
            },
            provenance_id: truth.run,
        })),
        None,
    ))
}
impl FixedLifecycleRoute {
    /// Decode the exact generated envelope and lifecycle request before dispatch.
    /// Only the embedding's retained host/root grant supplies authority. The
    /// local subset implements prepare, answer, submit and inspect.
    ///
    /// ```
    /// use std::path::Path;
    /// use vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute;
    /// use vibe_wire::generated::machine::e1::operation_envelope::Body;
    /// let refusal = FixedLifecycleRoute::exchange_local(None, b"{}", Path::new("/unbound"));
    /// assert!(matches!(refusal.unwrap().body, Body::Error(_)));
    /// ```
    pub fn exchange_local(
        grant: Option<&Self>,
        bytes: &[u8],
        trusted_root: &Path,
    ) -> Result<e::OperationEnvelope, FixedLifecycleRouteError> {
        let admission = (|| -> Result<LifecycleActionRequest, String> {
            // The outer has two escaping layers, so its own cap is independent
            // from the decoded action request cap.
            if bytes.len() > PUBLICATION_CAP {
                return Err("envelope exceeds cap".into());
            }
            let input: e::OperationEnvelope =
                serde_json::from_slice(bytes).map_err(|_| "generated envelope shape refused")?;
            if input.schema != 1
                || input.operation_id.as_deref() != Some(OPERATION_ID)
                || input.domain_owner.as_deref() != Some(DOMAIN_OWNER)
                || input.source_id.is_some()
                || input.evidence_id.is_some()
            {
                return Err("input epoch/operation/owner or caller evidence refused".into());
            }
            let e::Body::Input(body) = input.body else {
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
                return Err("local action subset is prepare/answer/submit/inspect".into());
            }
            Ok(request)
        })();
        let request_bytes = admission
            .as_ref()
            .ok()
            .and_then(|r| encode(r, REQUEST_CAP).ok());
        let response = match (admission, request_bytes.as_deref()) {
            (Ok(_), Some(bytes)) => Self::dispatch(grant, bytes.as_bytes(), trusted_root),
            (Err(reason), _) => Err(FixedLifecycleRouteError::invalid_request(&reason, grant)),
            _ => Err(FixedLifecycleRouteError::invalid_request(
                "action encoder cap",
                grant,
            )),
        };
        let candidate = match &response {
            Ok(reply) => result_truth(reply).and_then(|truth| {
                let source = match &reply.reply {
                    Reply::Question(q) => {
                        let q: crate::machine_operation::input_decision::InputDecisionMessage =
                            serde_json::from_str(&q.payload_json)
                                .map_err(|_| "generated question shape refused")?;
                        q.source_id
                    }
                    _ => None,
                };
                Ok(outer(
                    e::Body::Result(Box::new(e::BodyResult {
                        format_id: Some(FormatId::MachineLifecycleActionReply.id().into()),
                        payload_json: Some(encode(reply, PUBLICATION_CAP)?),
                        effect_state: match truth.effect {
                            Effect::None => e::BodyResultEffectState::NoEffects,
                            Effect::Landed => e::BodyResultEffectState::Completed,
                            Effect::Uncertain => e::BodyResultEffectState::Uncertain,
                        },
                        completeness: match truth.completeness {
                            Completeness::Complete => e::BodyResultCompleteness::Complete,
                            Completeness::Partial => e::BodyResultCompleteness::Partial,
                            Completeness::Unavailable => e::BodyResultCompleteness::Unavailable,
                        },
                        provenance_id: truth.run,
                    })),
                    source,
                ))
            }),
            Err(refusal) => error_envelope(refusal),
        };
        let publication = candidate.and_then(|value| {
            let bytes = encode(&value, PUBLICATION_CAP)?;
            let decoded: e::OperationEnvelope =
                serde_json::from_str(&bytes).map_err(|_| "generated publication reader refused")?;
            if decoded != value {
                return Err("publication roundtrip mismatch".into());
            }
            Ok(value)
        });
        publication
            .map_err(|reason| Self::publication_failure(grant, request_bytes.as_deref(), &reason))
    }
    fn publication_failure(
        grant: Option<&Self>,
        bytes: Option<&str>,
        reason: &str,
    ) -> FixedLifecycleRouteError {
        FixedLifecycleRouteError::publication_unavailable(
            reason,
            grant,
            bytes.unwrap_or("").as_bytes(),
        )
    }
}
