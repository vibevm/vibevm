//! Exact generated Input/Result/Error envelope and owner effect/provenance coherence.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE-WIRE-DECISION"
);
use super::{
    FixedPackageRoute, FixedPackageRouteError,
    contract::*,
    dispatch::{action_key, decode, failure},
};
use std::path::Path;
use vibe_wire::generated::{format_id::FormatId, machine::e1::operation_envelope as e};
pub const OPERATION_ID: &str = "rust:vibe-lifecycle:package-static-file";
pub const DOMAIN_OWNER: &str = "rust:vibe-lifecycle";
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Effect {
    None,
    Completed,
    Uncertain,
}
#[derive(Clone, Copy)]
pub(super) enum Complete {
    Complete,
    Partial,
    Unavailable,
}
pub(super) fn truth(r: &rep::Reply) -> (Effect, Complete, Option<String>) {
    let (effect, completeness) = match r {
        rep::Reply::Completed(_) => (Effect::Completed, Complete::Complete),
        rep::Reply::Uncertain(_) | rep::Reply::IntentReady(_) => {
            (Effect::Uncertain, Complete::Partial)
        }
        rep::Reply::RecoveryRequired(_) | rep::Reply::Retired(_) => {
            (Effect::Uncertain, Complete::Unavailable)
        }
        rep::Reply::Absent(_) => (Effect::None, Complete::Complete),
    };
    (effect, completeness, run(r).map(str::to_owned))
}
fn outer(body: e::Body) -> e::OperationEnvelope {
    e::OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION_ID.into()),
        domain_owner: Some(DOMAIN_OWNER.into()),
        source_id: None,
        evidence_id: None,
        body,
    }
}
fn result(reply: &rep::FixedPackageActionReply) -> Result<e::OperationEnvelope, String> {
    let (effect, completeness, provenance_id) = truth(&reply.reply);
    Ok(outer(e::Body::Result(Box::new(e::BodyResult {
        format_id: Some(FormatId::MachineFixedPackageActionReply.id().into()),
        payload_json: Some(roundtrip(reply, PUBLICATION_CAP)?),
        provenance_id,
        effect_state: match effect {
            Effect::None => e::BodyResultEffectState::NoEffects,
            Effect::Completed => e::BodyResultEffectState::Completed,
            Effect::Uncertain => e::BodyResultEffectState::Uncertain,
        },
        completeness: match completeness {
            Complete::Complete => e::BodyResultCompleteness::Complete,
            Complete::Partial => e::BodyResultCompleteness::Partial,
            Complete::Unavailable => e::BodyResultCompleteness::Unavailable,
        },
    }))))
}
pub(super) fn error(refusal: &FixedPackageRouteError) -> Result<e::OperationEnvelope, String> {
    let row = &refusal.error;
    if row.schema != 1 || row.message.len() > 4096 {
        return Err("invalid error epoch/diagnostic".into());
    }
    if row.known_run_id.as_deref().is_some_and(|r| !valid_run(r)) {
        return Err("invalid error run identity".into());
    }
    if refusal.owner_observation.is_some() && row.view.is_none() {
        return Err("original owner evidence cannot be published as a closed view".into());
    }
    let (effect, completeness, run) = if let Some(view) = &row.view {
        let json = roundtrip(view, PUBLICATION_CAP)?;
        let r: rep::Reply = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let expected = refusal
            .owner_observation
            .as_ref()
            .ok_or("error view lacks owner observation")?;
        if project(expected)? != r {
            return Err("error view differs from original owner observation".into());
        }
        truth(&r)
    } else {
        (
            if refusal.effects_may_have_begun {
                Effect::Uncertain
            } else {
                Effect::None
            },
            Complete::Unavailable,
            row.known_run_id.clone(),
        )
    };
    let expected_effect = match effect {
        Effect::None => err::FixedPackageActionErrorEffectObservation::None,
        Effect::Completed => err::FixedPackageActionErrorEffectObservation::Landed,
        Effect::Uncertain if row.view.is_none() => {
            err::FixedPackageActionErrorEffectObservation::Unavailable
        }
        Effect::Uncertain => err::FixedPackageActionErrorEffectObservation::Uncertain,
    };
    if row.known_run_id != run || row.effect_observation != expected_effect {
        return Err("error run/effect coherence refused".into());
    }
    Ok(outer(e::Body::Error(Box::new(e::BodyError {
        format_id: Some(FormatId::MachineFixedPackageActionError.id().into()),
        payload_json: Some(roundtrip(row, PUBLICATION_CAP)?),
        provenance_id: run,
        effect_state: match effect {
            Effect::None => e::BodyErrorEffectState::NoEffects,
            Effect::Completed => e::BodyErrorEffectState::Completed,
            Effect::Uncertain => e::BodyErrorEffectState::Uncertain,
        },
        completeness: match completeness {
            Complete::Complete => e::BodyErrorCompleteness::Complete,
            Complete::Partial => e::BodyErrorCompleteness::Partial,
            Complete::Unavailable => e::BodyErrorCompleteness::Unavailable,
        },
    }))))
}
impl FixedPackageRoute {
    /// Read the bounded generated envelope, then call the retained owner exactly once.
    /// Publication failure returns original evidence in the owning error channel.
    ///
    /// ```
    /// use vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute;
    /// use vibe_wire::generated::machine::e1::operation_envelope::Body;
    /// let e = FixedPackageRoute::exchange_local(None, b"{}", std::path::Path::new("/unbound")).unwrap();
    /// assert!(matches!(e.body, Body::Error(_)));
    /// ```
    pub fn exchange_local(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
    ) -> Result<e::OperationEnvelope, FixedPackageRouteError> {
        Self::exchange_bounded(grant, bytes, root, PUBLICATION_CAP)
    }
    pub(super) fn exchange_bounded(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
    ) -> Result<e::OperationEnvelope, FixedPackageRouteError> {
        let admitted = (|| -> Result<String, String> {
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
                return Err("expected Input stage".into());
            };
            if body.format_id != FormatId::MachineFixedPackageActionRequest.id() {
                return Err("exact registered request format required".into());
            }
            decode(body.payload_json.as_bytes())?;
            Ok(body.payload_json)
        })();
        let (request, response) = match admitted {
            Ok(bytes) => {
                let r = Self::dispatch_owned(grant, bytes.as_bytes(), root, PUBLICATION_CAP);
                (Some(bytes), r)
            }
            Err(s) => (
                None,
                Err(failure(
                    grant,
                    err::FixedPackageActionErrorCode::InvalidRequest,
                    &s,
                    None,
                    false,
                    None,
                    None,
                )),
            ),
        };
        match response {
            Ok((reply, observation)) => result(&reply)
                .and_then(|value| {
                    roundtrip(&value, cap)?;
                    Ok(value)
                })
                .map_err(|reason| {
                    failure(
                        grant,
                        err::FixedPackageActionErrorCode::PublicationUnavailable,
                        &reason,
                        request
                            .as_ref()
                            .and_then(|b| decode(b.as_bytes()).ok())
                            .as_ref()
                            .map(|r| action_key(r).0),
                        true,
                        observation,
                        None,
                    )
                }),
            Err(mut refusal) => match error(&refusal).and_then(|value| {
                roundtrip(&value, cap)?;
                Ok(value)
            }) {
                Ok(value) => Ok(value),
                Err(_) => {
                    refusal.error.code = err::FixedPackageActionErrorCode::PublicationUnavailable;
                    Err(refusal)
                }
            },
        }
    }
}
