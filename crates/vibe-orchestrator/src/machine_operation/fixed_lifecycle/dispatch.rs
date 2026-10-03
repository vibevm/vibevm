//! Generated action dispatch delegates every domain decision to the retained host.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
use super::{
    FixedLifecycleRoute, FixedLifecycleRouteError,
    contract::{self, *},
    error::{failure, metadata},
};
use crate::machine_submission::{DecisionOutcome, FixedDecisionError};
use LifecycleActionErrorCode as Code;
use std::path::Path;

fn owner_code(e: &FixedDecisionError, fallback: Code) -> Code {
    match e {
        FixedDecisionError::Capacity => Code::Capacity,
        FixedDecisionError::ImplementationGap(_) => Code::ImplementationGap,
        _ => fallback,
    }
}
fn snapshot_reply(
    s: &crate::machine_submission::FixedOwnerEvidenceSnapshot,
    key: &str,
    submitted: bool,
) -> Result<Reply, (Code, String, bool)> {
    let (row, payload_json) = view(s).map_err(|e| (Code::PublicationUnavailable, e, true))?;
    use crate::machine_operation::lifecycle_view::Status;
    match &row.status {
        Status::RecoveryRequired(r) => {
            return Err((Code::InspectionUnavailable, r.reason.clone(), true));
        }
        Status::InspectionUnavailable(r) => {
            return Err((Code::InspectionUnavailable, r.reason.clone(), true));
        }
        _ => {}
    }
    Ok(if submitted {
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
    })
}
impl FixedLifecycleRoute {
    /// Decode bounded raw generated request bytes and dispatch under the embedding root.
    /// An absent grant returns the incumbent unavailable-host refusal and cannot
    /// initialize or recover authority. See the type example for canonical use.
    pub fn dispatch(
        grant: Option<&Self>,
        bytes: &[u8],
        trusted_root: &Path,
    ) -> Result<LifecycleActionReply, FixedLifecycleRouteError> {
        Self::dispatch_bounded(grant, bytes, trusted_root, PUBLICATION_CAP)
    }
    pub(super) fn dispatch_bounded(
        grant: Option<&Self>,
        bytes: &[u8],
        trusted_root: &Path,
        publication_cap: usize,
    ) -> Result<LifecycleActionReply, FixedLifecycleRouteError> {
        let host = grant.map(|b| b.host.as_ref());
        let request: LifecycleActionRequest = match decode(bytes) {
            Ok(r) => r,
            Err(m) => return failure(error(Code::InvalidRequest, &m, None, None, host), false),
        };
        let (action, phase, key) = metadata(&request);
        let refuse =
            |code, m: &str, admitted| failure(error(code, m, Some(action), phase, host), admitted);
        if request.schema != 1 {
            return refuse(Code::InvalidRequest, "schema must equal 1", false);
        }
        let Some(bridge) = grant else {
            return refuse(
                Code::UnavailableHost,
                "no explicitly granted lifecycle host",
                false,
            );
        };
        if bridge.host.compare_context_root(trusted_root).is_err() {
            return refuse(
                Code::ContextRootMismatch,
                "physical context root differs from retained host project",
                false,
            );
        }
        if key.is_some_and(|k| !contract::key(k)) {
            return refuse(
                Code::InvalidRequest,
                "key must be nonempty, trimmed, <=256 bytes and contain no controls",
                false,
            );
        }
        let result: Result<Reply, (Code, String, bool)> = match &request.action {
            Action::Prepare(p) => {
                if p.phase != "validate" || p.requested_options.deploy_profile.is_some() {
                    return refuse(
                        Code::ImplementationGap,
                        "fixed owner implements Validate without deploy_profile",
                        false,
                    );
                }
                bridge
                    .host
                    .prepare_requested_options(options(&p.requested_options))
                    .map_err(|e| (owner_code(&e, Code::DecisionRefused), e.to_string(), true))
                    .and_then(|q| question(&q).map_err(|e| (Code::PublicationUnavailable, e, true)))
            }
            Action::Answer(a) => {
                if a.message_json.len() > 65_536 {
                    return refuse(Code::InvalidRequest, "decision frame exceeds 64KiB", false);
                }
                let answer: crate::machine_operation::input_decision::InputDecisionMessage =
                    match serde_json::from_str(&a.message_json) {
                        Ok(a) => a,
                        Err(_) => {
                            return refuse(
                                Code::InvalidRequest,
                                "generated decision shape refused",
                                false,
                            );
                        }
                    };
                match bridge.host.answer_typed(&answer) {
                    Err(e) => Err((owner_code(&e, Code::DecisionRefused), e.to_string(), true)),
                    Ok(DecisionOutcome::Question(q)) => {
                        question(&q).map_err(|e| (Code::PublicationUnavailable, e, true))
                    }
                    Ok(DecisionOutcome::Validated(e)) => {
                        let Some(id) = answer.question_id.clone() else {
                            return refuse(
                                Code::PublicationUnavailable,
                                "owner validated without original question id",
                                true,
                            );
                        };
                        let plan_id = e.identity().to_owned();
                        bridge
                            .retain(id.clone(), e)
                            .map(|()| {
                                Reply::Validated(Box::new(ReplyValidated {
                                    accepted_ref: id,
                                    plan_id,
                                }))
                            })
                            .map_err(|e| {
                                (owner_code(&e, Code::DecisionRefused), e.to_string(), true)
                            })
                    }
                }
            }
            Action::Submit(s) => {
                let Some(e) = bridge.accepted(&s.accepted_ref) else {
                    let mut e = error(
                        Code::UnknownReference,
                        "accepted reference is not retained by this owner bridge",
                        Some(action),
                        phase,
                        host,
                    );
                    // A fresh route has no process-local accepted reference or
                    // job table, but C1 may still retain this key's run. Ask
                    // the domain owner before claiming the refusal had no
                    // prior effects. Failed inspection is also uncertain.
                    return match bridge.inspect(&s.key) {
                        Ok(Some(snapshot)) => {
                            enrich(&mut e, &snapshot);
                            failure(e, true)
                        }
                        Ok(None) => failure(e, false),
                        Err(_) => failure(e, true),
                    };
                };
                bridge
                    .submit(&e, &s.key)
                    .map_err(|e| (owner_code(&e, Code::SubmissionRefused), e.to_string(), true))
                    .and_then(|snapshot| snapshot_reply(&snapshot, &s.key, true))
            }
            Action::Attach(a) => bridge.attach_observer(&a.key, trusted_root),
            Action::Replay(r) => bridge.stream_observer(&r.observer_ref, &r.message_json, false),
            Action::Control(c) => bridge.stream_observer(&c.observer_ref, &c.message_json, true),
            Action::Detach(d) => bridge.detach_observer(&d.observer_ref),
            Action::Inspect(i) => bridge
                .inspect(&i.key)
                .map_err(|e| {
                    (
                        owner_code(&e, Code::InspectionUnavailable),
                        e.to_string(),
                        true,
                    )
                })
                .and_then(|snapshot| match snapshot {
                    None => Ok(Reply::Absent(Box::new(ReplyAbsent { key: i.key.clone() }))),
                    Some(s) => snapshot_reply(&s, &i.key, false),
                }),
        };
        match result {
            Ok(reply) => {
                let reply = LifecycleActionReply { schema: 1, reply };
                match size(&reply, publication_cap) {
                    Ok(_) => Ok(reply),
                    Err(m) => {
                        let mut e =
                            error(Code::PublicationUnavailable, &m, Some(action), phase, host);
                        if let Some(k) = key {
                            if let Ok(Some(s)) = bridge.inspect(k) {
                                enrich(&mut e, &s);
                            }
                        }
                        failure(e, true)
                    }
                }
            }
            Err((code, m, admitted)) => {
                let mut e = error(code, &m, Some(action), phase, host);
                if admitted {
                    if let Some(k) = key {
                        if let Ok(Some(s)) = bridge.inspect(k) {
                            enrich(&mut e, &s);
                        }
                    }
                }
                failure(e, admitted)
            }
        }
    }
}
