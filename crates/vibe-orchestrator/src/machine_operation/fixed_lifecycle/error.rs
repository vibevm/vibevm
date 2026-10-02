//! Refusals preserve generated owner evidence and the conservative effect boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
use super::{FixedLifecycleRoute, contract::*};
use LifecycleActionErrorCode as Code;

/// Generated refusal plus the conservative boundary formerly used by MCP.
/// `true` includes lease/preparation infrastructure as well as provider effects;
/// `false` never replaces retained effect observations for an existing run.
/// See [`FixedLifecycleRoute`] for canonical dispatch and handling. Adapter failures
/// use the same generated error without constructing a host:
/// ```
/// use vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRouteError;
/// let encoding = FixedLifecycleRouteError::invalid_request("request encoder cap", None);
/// assert!(!encoding.effects_may_have_begun);
/// assert!(encoding.error.requested_scope.is_none());
/// let request = br#"{"schema":1,"action":{"kind":"inspect","key":"once"}}"#;
/// let publication = FixedLifecycleRouteError::publication_unavailable("reply encoder cap", None, request);
/// assert!(publication.effects_may_have_begun);
/// assert_eq!(publication.error.requested_action.as_deref(), Some("inspect"));
/// ```
#[derive(Debug)]
pub struct FixedLifecycleRouteError {
    pub error: LifecycleActionError,
    pub effects_may_have_begun: bool,
}
impl std::fmt::Display for FixedLifecycleRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION: {}; fix surface: trusted fixed lifecycle action request/owner",
            self.error.message
        )
    }
}
impl std::error::Error for FixedLifecycleRouteError {}
impl FixedLifecycleRouteError {
    /// Surface publication refusal after a successful dispatch, with fresh owner truth.
    /// The original bounded request selects diagnostic metadata only, never authority.
    pub fn publication_unavailable(
        message: &str,
        grant: Option<&FixedLifecycleRoute>,
        bytes: &[u8],
    ) -> Self {
        let host = grant.map(|r| r.host.as_ref());
        let request: LifecycleActionRequest = match decode(bytes) {
            Ok(request) => request,
            Err(_) => {
                return Self {
                    error: error(Code::PublicationUnavailable, message, None, None, host),
                    effects_may_have_begun: true,
                };
            }
        };
        let (action, phase, key) = metadata(&request);
        let mut error = error(
            Code::PublicationUnavailable,
            message,
            Some(action),
            phase,
            host,
        );
        if let (Some(route), Some(key)) = (grant, key) {
            if let Ok(Some(snapshot)) = route.inspect(key) {
                enrich(&mut error, &snapshot);
            }
        }
        Self {
            error,
            effects_may_have_begun: true,
        }
    }
    /// Adapter encoding refusal, retaining the granted scope without dispatch.
    /// Used when the embedding's bounded request encoder fails before raw bytes exist.
    pub fn invalid_request(message: &str, grant: Option<&FixedLifecycleRoute>) -> Self {
        Self {
            error: error(
                Code::InvalidRequest,
                message,
                None,
                None,
                grant.map(|r| r.host.as_ref()),
            ),
            effects_may_have_begun: false,
        }
    }
}
pub(super) fn failure<T>(
    error: LifecycleActionError,
    effects_may_have_begun: bool,
) -> Result<T, FixedLifecycleRouteError> {
    Err(FixedLifecycleRouteError {
        error,
        effects_may_have_begun,
    })
}
pub(super) fn metadata(
    request: &LifecycleActionRequest,
) -> (&'static str, Option<&str>, Option<&str>) {
    match &request.action {
        Action::Prepare(p) => ("prepare", Some(p.phase.as_str()), None),
        Action::Answer(_) => ("answer", None, None),
        Action::Submit(s) => ("submit", Some("validate"), Some(s.key.as_str())),
        Action::Inspect(i) => ("inspect", Some("validate"), Some(i.key.as_str())),
        Action::Attach(a) => ("attach", Some("validate"), Some(a.key.as_str())),
        Action::Replay(_) => ("replay", None, None),
        Action::Control(_) => ("control", None, None),
        Action::Detach(_) => ("detach", None, None),
    }
}
