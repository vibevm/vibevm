//! Library grant and owner dispatch; no reservation or result cache lives here.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE-WIRE-DECISION"
);
use super::{FixedPackageHost, FixedPackageHostError, LAW, contract::*};
use std::{path::Path, sync::Arc};
use vibe_lifecycle::StaticFileJournalInspection;

/// Owning failure channel. Original owner evidence survives failed projection or encoding.
///
/// ```
/// use vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute;
/// let e = FixedPackageRoute::dispatch(None, b"{}", std::path::Path::new("/unbound"));
/// assert!(!e.unwrap_err().effects_may_have_begun);
/// ```
#[derive(Debug)]
pub struct FixedPackageRouteError {
    pub error: err::FixedPackageActionError,
    pub effects_may_have_begun: bool,
    pub owner_observation: Option<StaticFileJournalInspection>,
    pub owner_failure: Option<FixedPackageHostError>,
}
impl std::fmt::Display for FixedPackageRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error.message)
    }
}
impl std::error::Error for FixedPackageRouteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.owner_failure
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}
/// Surface-neutral fixed package route retaining only the trusted embedding's host.
///
/// ```no_run
/// use std::{path::Path, sync::Arc};
/// use vibe_orchestrator::machine_operation::fixed_package::{FixedPackageHost, FixedPackageRoute};
/// fn inspect(host: Arc<FixedPackageHost>, root: &Path, input: &[u8]) {
///     let route = FixedPackageRoute::new(host);
///     let result = FixedPackageRoute::exchange_local(Some(&route), input, root);
///     let _ = result;
/// }
/// ```
pub struct FixedPackageRoute {
    pub(super) host: Arc<FixedPackageHost>,
}
impl FixedPackageRoute {
    /// Retain an existing authenticated host. See the type example.
    #[must_use]
    pub fn new(host: Arc<FixedPackageHost>) -> Self {
        Self { host }
    }
    /// Generated request dispatch under the embedding's physical root. See the error example.
    pub fn dispatch(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
    ) -> Result<rep::FixedPackageActionReply, FixedPackageRouteError> {
        Self::dispatch_bounded(grant, bytes, root, PUBLICATION_CAP)
    }
    pub(super) fn dispatch_bounded(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
    ) -> Result<rep::FixedPackageActionReply, FixedPackageRouteError> {
        Self::dispatch_owned(grant, bytes, root, cap).map(|(reply, _)| reply)
    }
    pub(super) fn dispatch_owned(
        grant: Option<&Self>,
        bytes: &[u8],
        root: &Path,
        cap: usize,
    ) -> Result<
        (
            rep::FixedPackageActionReply,
            Option<StaticFileJournalInspection>,
        ),
        FixedPackageRouteError,
    > {
        let request = decode(bytes).map_err(|s| {
            failure(
                grant,
                err::FixedPackageActionErrorCode::InvalidRequest,
                &s,
                None,
                false,
                None,
                None,
            )
        })?;
        let (action, key) = action_key(&request);
        let route = grant.ok_or_else(|| {
            failure(
                None,
                err::FixedPackageActionErrorCode::UnavailableHost,
                "explicit trusted host required",
                Some(action),
                false,
                None,
                None,
            )
        })?;
        route.compare_context_root(root).map_err(|e| {
            failure(
                grant,
                err::FixedPackageActionErrorCode::ContextRootMismatch,
                &e.to_string(),
                Some(action),
                false,
                None,
                Some(e),
            )
        })?;
        let observed = match &request.action {
            req::Action::Submit(_) => route.host.submit(key).map(Some),
            req::Action::Inspect(_) => route.host.inspect(key),
        };
        let observed = observed.map_err(|e| {
            let retained = route.host.inspect(key).ok().flatten();
            failure(
                grant,
                if action == "submit" {
                    err::FixedPackageActionErrorCode::SubmissionRefused
                } else {
                    err::FixedPackageActionErrorCode::InspectionUnavailable
                },
                &e.to_string(),
                Some(action),
                true,
                retained,
                Some(e),
            )
        })?;
        let projected = match &observed {
            Some(o) => project(o),
            None => Ok(rep::Reply::Absent(Box::new(rep::ReplyAbsent {
                key: key.into(),
            }))),
        };
        let reply = projected
            .map(|reply| rep::FixedPackageActionReply { schema: 1, reply })
            .and_then(|r| {
                roundtrip(&r, cap)?;
                Ok(r)
            });
        reply.map(|r| (r, observed.clone())).map_err(|s| {
            failure(
                grant,
                err::FixedPackageActionErrorCode::PublicationUnavailable,
                &s,
                Some(action),
                true,
                observed,
                None,
            )
        })
    }
    /// Compare a supplied physical root with this retained host without
    /// issuing a namespace or dispatching the package owner.
    pub fn compare_context_root(&self, root: &Path) -> Result<(), FixedPackageHostError> {
        self.host.check_root(None)?;
        let supplied = vibe_safefs::Project::open(root).map_err(super::root_error)?;
        if supplied
            .root_correlation_token()
            .map_err(super::root_error)?
            != self.host.root_token
        {
            return Err(FixedPackageHostError::ForeignRoot);
        }
        Ok(())
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<req::FixedPackageActionRequest, String> {
    if bytes.len() > REQUEST_CAP {
        return Err("request exceeds cap".into());
    }
    let r: req::FixedPackageActionRequest =
        serde_json::from_slice(bytes).map_err(|_| "generated request shape refused")?;
    if r.schema != 1 || !key(action_key(&r).1) {
        return Err("request epoch or key refused".into());
    }
    Ok(r)
}
pub(super) fn action_key(r: &req::FixedPackageActionRequest) -> (&'static str, &str) {
    match &r.action {
        req::Action::Submit(a) => ("submit", &a.key),
        req::Action::Inspect(a) => ("inspect", &a.key),
    }
}
pub(super) fn failure(
    grant: Option<&FixedPackageRoute>,
    code: err::FixedPackageActionErrorCode,
    reason: &str,
    action: Option<&str>,
    admitted: bool,
    observation: Option<StaticFileJournalInspection>,
    owner_failure: Option<FixedPackageHostError>,
) -> FixedPackageRouteError {
    let message = format!("violates REQ {LAW}: {reason}; fix surface: fixed package host/action");
    let mut end = message.len().min(4096);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    let projected = observation.as_ref().and_then(|o| project(o).ok());
    let view = projected.as_ref().and_then(|r| {
        let json = roundtrip(r, PUBLICATION_CAP).ok()?;
        serde_json::from_str::<err::Reply>(&json).ok()
    });
    let effect_observation = match projected.as_ref() {
        Some(rep::Reply::Completed(_)) => err::FixedPackageActionErrorEffectObservation::Landed,
        Some(_) => err::FixedPackageActionErrorEffectObservation::Uncertain,
        None if admitted => err::FixedPackageActionErrorEffectObservation::Unavailable,
        None => err::FixedPackageActionErrorEffectObservation::None,
    };
    FixedPackageRouteError {
        error: err::FixedPackageActionError {
            schema: 1,
            code,
            message: message[..end].into(),
            message_truncated: end < message.len(),
            requested_action: action.map(str::to_owned),
            requested_scope: grant.map(|r| {
                let s = &r.host.scope;
                err::Scope {
                    context: s.context.clone(),
                    target: s.target.clone(),
                    principal: s.principal.clone(),
                }
            }),
            scope_unavailable_reason: grant
                .is_none()
                .then(|| "no explicitly granted host scope".into()),
            known_run_id: observation.as_ref().and_then(owner_run).map(str::to_owned),
            view,
            effect_observation,
        },
        effects_may_have_begun: admitted,
        owner_observation: observation,
        owner_failure,
    }
}
