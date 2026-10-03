//! Strict generated action dispatch; the lifecycle Cargo journal owns C1 and effects.
use super::{FixedCargoHost, FixedCargoHostError, contract::*};
use std::{path::Path, sync::Arc};
use vibe_lifecycle::retained_mechanism::BuildInspection;

#[derive(Debug)]
pub struct FixedCargoRouteError {
    pub error: err::FixedCargoActionError,
    pub effects_may_have_begun: bool,
    pub owner_observation: Option<BuildInspection>,
    pub owner_failure: Option<FixedCargoHostError>,
    pub requested_key: Option<String>,
}
impl std::fmt::Display for FixedCargoRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error.message)
    }
}
impl std::error::Error for FixedCargoRouteError {}

pub struct FixedCargoRoute {
    pub(super) host: Arc<FixedCargoHost>,
}
impl FixedCargoRoute {
    pub fn new(host: Arc<FixedCargoHost>) -> Self {
        Self { host }
    }
    pub fn compare_context_root(&self, root: &Path) -> Result<(), FixedCargoHostError> {
        self.host.check_root(None)?;
        let supplied = vibe_safefs::Project::open(root).map_err(super::root_error)?;
        if supplied
            .root_correlation_token()
            .map_err(super::root_error)?
            != self.host.root_token
        {
            return Err(FixedCargoHostError::ForeignRoot);
        }
        Ok(())
    }
    pub fn dispatch(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
    ) -> Result<rep::FixedCargoActionReply, FixedCargoRouteError> {
        Self::dispatch_owned(grant, bytes, root, PUBLICATION_CAP).map(|(reply, _)| reply)
    }
    pub(super) fn dispatch_owned(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
    ) -> Result<(rep::FixedCargoActionReply, BuildInspection), FixedCargoRouteError> {
        let request = decode(bytes).map_err(|reason| {
            failure(
                grant,
                err::FixedCargoActionErrorCode::InvalidRequest,
                &reason,
                None,
                false,
                None,
                None,
                None,
            )
        })?;
        let (action, key) = action_key(&request);
        let route = grant.ok_or_else(|| {
            failure(
                None,
                err::FixedCargoActionErrorCode::UnavailableHost,
                "explicit trusted fixed Cargo host required",
                Some(action),
                false,
                None,
                None,
                Some(key),
            )
        })?;
        route.compare_context_root(root).map_err(|error| {
            failure(
                grant,
                err::FixedCargoActionErrorCode::ContextRootMismatch,
                &error.to_string(),
                Some(action),
                false,
                None,
                Some(error),
                Some(key),
            )
        })?;
        let observed = match &request.action {
            req::Action::Submit(_) => route.host.submit(key),
            req::Action::Inspect(_) => route.host.inspect(key),
        };
        let observed = observed.map_err(|error| {
            let (may_effect, owner_observation) = owner_error_parts(&error);
            failure(
                grant,
                if action == "submit" {
                    err::FixedCargoActionErrorCode::SubmissionRefused
                } else {
                    err::FixedCargoActionErrorCode::InspectionUnavailable
                },
                &error.to_string(),
                Some(action),
                may_effect,
                owner_observation,
                Some(error),
                Some(key),
            )
        })?;
        let projected = project(&observed, key).and_then(|reply| {
            let value = rep::FixedCargoActionReply { schema: 1, reply };
            roundtrip(&value, cap)?;
            Ok(value)
        });
        projected
            .map(|reply| (reply, observed.clone()))
            .map_err(|reason| {
                failure(
                    grant,
                    err::FixedCargoActionErrorCode::PublicationUnavailable,
                    &reason,
                    Some(action),
                    true,
                    Some(observed),
                    None,
                    Some(key),
                )
            })
    }
}
pub(super) fn owner_error_parts(error: &FixedCargoHostError) -> (bool, Option<BuildInspection>) {
    match error {
        FixedCargoHostError::Submit(owner) => (
            owner.effects_may_have_begun,
            owner.owner_observation.clone(),
        ),
        FixedCargoHostError::PostOwner { observation, .. } => (true, Some(observation.clone())),
        FixedCargoHostError::Inspect(_) => (true, None),
        _ => (false, None),
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<req::FixedCargoActionRequest, String> {
    if bytes.len() > REQUEST_CAP {
        return Err("request exceeds cap".into());
    }
    let value: req::FixedCargoActionRequest =
        serde_json::from_slice(bytes).map_err(|_| "generated request shape refused".to_owned())?;
    if value.schema != 1 || !key(action_key(&value).1) {
        return Err("request epoch or key refused".into());
    }
    Ok(value)
}
pub(super) fn action_key(value: &req::FixedCargoActionRequest) -> (&'static str, &str) {
    match &value.action {
        req::Action::Submit(row) => ("submit", &row.key),
        req::Action::Inspect(row) => ("inspect", &row.key),
    }
}
pub(super) fn failure(
    grant: Option<&FixedCargoRoute>,
    code: err::FixedCargoActionErrorCode,
    reason: &str,
    action: Option<&str>,
    effects_may_have_begun: bool,
    owner_observation: Option<BuildInspection>,
    owner_failure: Option<FixedCargoHostError>,
    key: Option<&str>,
) -> FixedCargoRouteError {
    let mut end = reason.len().min(4096);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    let view = owner_observation
        .as_ref()
        .and_then(|observation| project(observation, key.unwrap_or_default()).ok())
        .map(|row| error_view(&row));
    let run = owner_observation
        .as_ref()
        .and_then(owner_run)
        .map(str::to_owned);
    let effect = match &view {
        Some(err::Reply::Completed(_)) => err::FixedCargoActionErrorEffectObservation::Landed,
        Some(err::Reply::Absent(_)) if !effects_may_have_begun => {
            err::FixedCargoActionErrorEffectObservation::None
        }
        Some(_) => err::FixedCargoActionErrorEffectObservation::Uncertain,
        None if effects_may_have_begun => err::FixedCargoActionErrorEffectObservation::Unavailable,
        None => err::FixedCargoActionErrorEffectObservation::None,
    };
    FixedCargoRouteError {
        error: err::FixedCargoActionError {
            schema: 1,
            code,
            message: reason[..end].into(),
            message_truncated: end < reason.len(),
            requested_action: action.map(str::to_owned),
            requested_scope: grant.map(|route| {
                let scope = &route.host.scope;
                err::Scope {
                    context: scope.context.clone(),
                    target: scope.target.clone(),
                    principal: scope.principal.clone(),
                }
            }),
            scope_unavailable_reason: grant
                .is_none()
                .then(|| "no explicitly granted host scope".into()),
            known_run_id: run,
            view,
            effect_observation: effect,
        },
        effects_may_have_begun,
        owner_observation,
        owner_failure,
        requested_key: key.map(str::to_owned),
    }
}
