//! Strict fixed Deploy action dispatch; host callback is private and test-only.
use super::{FixedDeployHost, FixedDeployHostError, FixedDeployRecoveryHost, contract::*};
use std::{path::Path, sync::Arc};
use vibe_lifecycle::retained_mechanism::DeployInspection;

/// Typed preflight or owner-publication refusal. A missing grant cannot begin
/// a Deploy effect.
///
/// ```
/// use vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute;
/// let refusal = FixedDeployRoute::dispatch(None, b"{}", std::path::Path::new("/unbound"))
///     .unwrap_err();
/// assert!(!refusal.effects_may_have_begun);
/// ```
#[derive(Debug)]
pub struct FixedDeployRouteError {
    pub error: err::FixedDeployActionError,
    pub effects_may_have_begun: bool,
    pub owner_observation: Option<DeployInspection>,
    pub owner_failure: Option<FixedDeployHostError>,
    pub requested_key: Option<String>,
}
impl std::fmt::Display for FixedDeployRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error.message)
    }
}
impl std::error::Error for FixedDeployRouteError {}

impl FixedDeployRouteError {
    /// Refuse an adapter encoding before owner dispatch. Only the retained
    /// host may supply scope; request bytes never supply one.
    pub fn invalid_request(message: &str, grant: Option<&FixedDeployRoute>) -> Self {
        failure(
            grant,
            err::FixedDeployActionErrorCode::InvalidRequest,
            message,
            None,
            false,
            None,
            None,
            None,
            None,
        )
    }
}

/// One trusted host route; the generated envelope remains an error without it.
///
/// ```
/// use vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute;
/// use vibe_wire::generated::machine::e1::operation_envelope::Body;
/// let envelope = FixedDeployRoute::exchange_local(
///     None, b"{}", std::path::Path::new("/unbound")
/// ).unwrap();
/// assert!(matches!(envelope.body, Body::Error(_)));
/// ```
pub struct FixedDeployRoute {
    pub(super) host: RouteHost,
}
pub(super) enum RouteHost {
    Active(Arc<FixedDeployHost>),
    RecoveryOnly(Arc<FixedDeployRecoveryHost>),
}
impl RouteHost {
    fn scope(&self) -> &vibe_lifecycle::SubmissionScope {
        match self {
            Self::Active(host) => &host.scope,
            Self::RecoveryOnly(host) => &host.scope,
        }
    }
}
impl FixedDeployRoute {
    pub fn new(host: Arc<FixedDeployHost>) -> Self {
        Self {
            host: RouteHost::Active(host),
        }
    }
    /// An old pointer can inspect retained keys, but cannot submit effects.
    pub fn recovery_only(host: Arc<FixedDeployRecoveryHost>) -> Self {
        Self {
            host: RouteHost::RecoveryOnly(host),
        }
    }
    pub fn compare_context_root(&self, root: &Path) -> Result<(), FixedDeployHostError> {
        let token = match &self.host {
            RouteHost::Active(host) => {
                host.check_roots(None)?;
                &host.root_tokens[0]
            }
            RouteHost::RecoveryOnly(host) => {
                host.check_roots(None)?;
                &host.root_tokens[0]
            }
        };
        let supplied = vibe_safefs::Project::open(root).map_err(super::root_error)?;
        if supplied
            .root_correlation_token()
            .map_err(super::root_error)?
            != *token
        {
            return Err(FixedDeployHostError::ForeignRoot);
        }
        Ok(())
    }
    pub fn dispatch(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
    ) -> Result<rep::FixedDeployActionReply, FixedDeployRouteError> {
        Self::dispatch_owned(grant, bytes, root, PUBLICATION_CAP).map(|(reply, _)| reply)
    }
    pub(super) fn dispatch_owned(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
    ) -> Result<(rep::FixedDeployActionReply, DeployInspection), FixedDeployRouteError> {
        Self::dispatch_owned_with_owner_call(grant, bytes, root, cap, |host, action, key| {
            match action {
                req::Action::Submit(_) => host.submit(key),
                req::Action::Inspect(_) => host.inspect(key),
            }
        })
    }
    pub(super) fn dispatch_owned_with_owner_call(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
        owner_call: impl FnOnce(
            &FixedDeployHost,
            &req::Action,
            &str,
        ) -> Result<DeployInspection, FixedDeployHostError>,
    ) -> Result<(rep::FixedDeployActionReply, DeployInspection), FixedDeployRouteError> {
        let request = decode(bytes).map_err(|reason| {
            failure(
                grant,
                err::FixedDeployActionErrorCode::InvalidRequest,
                &reason,
                None,
                false,
                None,
                None,
                None,
                None,
            )
        })?;
        let (action, key) = action_key(&request);
        let route = grant.ok_or_else(|| {
            failure(
                None,
                err::FixedDeployActionErrorCode::UnavailableHost,
                "explicit trusted fixed Deploy host required",
                Some(action),
                false,
                None,
                None,
                Some(key),
                None,
            )
        })?;
        route.compare_context_root(root).map_err(|error| {
            failure(
                grant,
                err::FixedDeployActionErrorCode::ContextRootMismatch,
                &error.to_string(),
                Some(action),
                false,
                None,
                Some(error),
                Some(key),
                None,
            )
        })?;
        let observed = match &route.host {
            RouteHost::Active(host) => owner_call(host, &request.action, key),
            RouteHost::RecoveryOnly(host) => match &request.action {
                req::Action::Inspect(_) => host.inspect(key),
                req::Action::Submit(_) => {
                    let inspection = host.inspect(key);
                    return Err(match inspection {
                        Ok(observation) => failure(
                            grant,
                            err::FixedDeployActionErrorCode::SubmissionRefused,
                            "old Deploy grant is inspection-only; issue a new trusted host for effects",
                            Some(action),
                            !matches!(&observation, DeployInspection::Absent),
                            Some(observation),
                            None,
                            Some(key),
                            None,
                        ),
                        Err(error) => failure(
                            grant,
                            err::FixedDeployActionErrorCode::SubmissionRefused,
                            "old Deploy grant is inspection-only; retained key inspection unavailable",
                            Some(action),
                            true,
                            None,
                            Some(error),
                            Some(key),
                            None,
                        ),
                    });
                }
            },
        }
        .map_err(|error| {
            let (may_effect, owner_observation, run) = owner_error_parts(&error);
            failure(
                grant,
                if action == "submit" {
                    err::FixedDeployActionErrorCode::SubmissionRefused
                } else {
                    err::FixedDeployActionErrorCode::InspectionUnavailable
                },
                &error.to_string(),
                Some(action),
                may_effect,
                owner_observation,
                Some(error),
                Some(key),
                run,
            )
        })?;
        let projected = project(&observed, key).and_then(|reply| {
            let value = rep::FixedDeployActionReply { schema: 1, reply };
            roundtrip(&value, cap)?;
            Ok(value)
        });
        projected
            .map(|reply| (reply, observed.clone()))
            .map_err(|reason| {
                failure(
                    grant,
                    err::FixedDeployActionErrorCode::PublicationUnavailable,
                    &reason,
                    Some(action),
                    true,
                    Some(observed),
                    None,
                    Some(key),
                    None,
                )
            })
    }
}
pub(super) fn owner_error_parts(
    error: &FixedDeployHostError,
) -> (bool, Option<DeployInspection>, Option<String>) {
    match error {
        FixedDeployHostError::Submit(owner) => (
            owner.effects_may_have_begun,
            owner.owner_observation.clone(),
            owner.known_run_id.clone(),
        ),
        FixedDeployHostError::PostOwner { observation, .. } => (
            true,
            Some(observation.clone()),
            owner_run(observation).map(str::to_owned),
        ),
        FixedDeployHostError::Inspect(_) => (true, None, None),
        // A host failure can race or follow an already reserved key. Only the
        // outer generated-shape/root preflight above proves no dispatch here.
        _ => (true, None, None),
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<req::FixedDeployActionRequest, String> {
    if bytes.len() > REQUEST_CAP {
        return Err("request exceeds cap".into());
    }
    let value: req::FixedDeployActionRequest =
        serde_json::from_slice(bytes).map_err(|_| "generated request shape refused".to_owned())?;
    if value.schema != 1 || !key(action_key(&value).1) {
        return Err("request epoch or key refused".into());
    }
    Ok(value)
}
pub(super) fn action_key(value: &req::FixedDeployActionRequest) -> (&'static str, &str) {
    match &value.action {
        req::Action::Submit(row) => ("submit", &row.key),
        req::Action::Inspect(row) => ("inspect", &row.key),
    }
}
pub(super) fn failure(
    grant: Option<&FixedDeployRoute>,
    code: err::FixedDeployActionErrorCode,
    reason: &str,
    action: Option<&str>,
    effects_may_have_begun: bool,
    owner_observation: Option<DeployInspection>,
    owner_failure: Option<FixedDeployHostError>,
    key: Option<&str>,
    known_run: Option<String>,
) -> FixedDeployRouteError {
    let mut end = reason.len().min(4096);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    let view = owner_observation
        .as_ref()
        .and_then(|observation| project(observation, key.unwrap_or_default()).ok())
        .map(|row| error_view(&row));
    let run = known_run.or_else(|| {
        owner_observation
            .as_ref()
            .and_then(owner_run)
            .map(str::to_owned)
    });
    let effect = match &view {
        Some(err::Reply::Completed(_)) => err::FixedDeployActionErrorEffectObservation::Landed,
        Some(err::Reply::Absent(_)) if !effects_may_have_begun => {
            err::FixedDeployActionErrorEffectObservation::None
        }
        Some(_) => err::FixedDeployActionErrorEffectObservation::Uncertain,
        None if effects_may_have_begun => err::FixedDeployActionErrorEffectObservation::Unavailable,
        None => err::FixedDeployActionErrorEffectObservation::None,
    };
    FixedDeployRouteError {
        error: err::FixedDeployActionError {
            schema: 1,
            code,
            message: reason[..end].into(),
            message_truncated: end < reason.len(),
            requested_action: action.map(str::to_owned),
            requested_phase: action.map(|_| "deploy".into()),
            requested_scope: grant.map(|route| {
                let scope = route.host.scope();
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
