//! Finite per-connection negotiation and private trusted context bindings.
//! A recognized request remains unvalidated and cannot dispatch from here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::machine::e1::session_frame::{
    Body, BodyInitializationRefused, BodyInitializationRefusedReason, BodyInitialized, BodyRefusal,
    BodyRefusalReason, BodyRequest, Capabilities, Context, ContextAvailability, ContextKind,
    Outcome, OutcomeCompleteness, OutcomeEffectState, SchemaOffer, SessionFrame,
};

use crate::{PROTOCOL_VERSION, ServerContext};

const API_VERSION: &str = "1";
const POSITION_ENCODING: &str = "raw-byte-offsets";
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Uninitialized,
    Initialized,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustedContextKind {
    Workspace,
    Member,
}

/// Only the composing surface supplies these values. Paths stay private and
/// never appear in the generated context descriptor or request body.
pub struct TrustedContextSource {
    pub kind: TrustedContextKind,
    pub context: ServerContext,
    pub target_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBuildError {
    IdsExhausted,
    MissingTrustedRoot,
}

struct BoundContext {
    descriptor: Context,
    _context: Option<ServerContext>,
    _target_roots: BTreeMap<String, PathBuf>,
}

/// The outer session/context IDs were checked. `envelope_json` has not been
/// decoded, admitted or executed by its domain owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnvalidatedRequest {
    pub session_id: String,
    pub request_id: String,
    pub context_id: String,
    pub target_id: Option<String>,
    pub envelope_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionDecision {
    Reply(SessionFrame),
    UnvalidatedRequest(UnvalidatedRequest),
}

pub struct MachineSession {
    phase: SessionPhase,
    session_id: String,
    contexts: BTreeMap<String, BoundContext>,
    request_ids: BTreeSet<String>,
}

fn supported_schemas() -> Vec<SchemaOffer> {
    [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
    ]
    .into_iter()
    .map(|format| SchemaOffer {
        format_id: format.id().to_owned(),
        epoch: u16::try_from(format.epoch())
            .expect("registered machine frame epoch fits its generated uint16 field"),
    })
    .collect()
}

fn unavailable_context(session: u64, ordinal: usize, kind: ContextKind) -> BoundContext {
    BoundContext {
        descriptor: Context {
            context_id: format!("machine-context-{session}-{ordinal}"),
            kind,
            availability: ContextAvailability::Unavailable,
            target_ids: Vec::new(),
            reason: Some("owner-not-bound".to_owned()),
        },
        _context: None,
        _target_roots: BTreeMap::new(),
    }
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && !id.chars().any(char::is_control)
}

impl MachineSession {
    /// A connection-local registry over already-composed, trusted contexts.
    /// This constructor does no HOME lookup, path read or root discovery.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
        r = 1
    )]
    pub fn new(sources: Vec<TrustedContextSource>) -> Result<Self, SessionBuildError> {
        let session = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| SessionBuildError::IdsExhausted)?;
        let mut contexts = BTreeMap::new();
        for (ordinal, source) in sources.into_iter().enumerate() {
            if source.context.project_root.as_os_str().is_empty() {
                return Err(SessionBuildError::MissingTrustedRoot);
            }
            let context_id = format!("machine-context-{session}-{ordinal}");
            let kind = match source.kind {
                TrustedContextKind::Workspace => ContextKind::Workspace,
                TrustedContextKind::Member => ContextKind::Member,
            };
            let mut target_roots = BTreeMap::new();
            for (target_ordinal, root) in source.target_roots.into_iter().enumerate() {
                if root.as_os_str().is_empty() {
                    return Err(SessionBuildError::MissingTrustedRoot);
                }
                target_roots.insert(
                    format!("machine-target-{session}-{ordinal}-{target_ordinal}"),
                    root,
                );
            }
            let descriptor = Context {
                context_id: context_id.clone(),
                kind,
                availability: ContextAvailability::Available,
                target_ids: target_roots.keys().cloned().collect(),
                reason: None,
            };
            contexts.insert(
                context_id,
                BoundContext {
                    descriptor,
                    _context: Some(source.context),
                    _target_roots: target_roots,
                },
            );
        }
        let next = contexts.len();
        for (ordinal, kind) in [ContextKind::Home, ContextKind::Container]
            .into_iter()
            .enumerate()
        {
            let bound = unavailable_context(session, next + ordinal, kind);
            contexts.insert(bound.descriptor.context_id.clone(), bound);
        }
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts,
            request_ids: BTreeSet::new(),
        })
    }

    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    /// EOF or the owning transport closes this connection without a daemon.
    pub fn close(&mut self) {
        self.phase = SessionPhase::Closed;
        self.contexts.clear();
        self.request_ids.clear();
    }

    fn reply(&self, request_id: Option<String>, body: Body) -> SessionDecision {
        SessionDecision::Reply(SessionFrame {
            schema: 1,
            session_id: (self.phase == SessionPhase::Initialized).then(|| self.session_id.clone()),
            request_id,
            body,
        })
    }

    fn refused(&self, request_id: Option<String>, reason: BodyRefusalReason) -> SessionDecision {
        self.reply(
            request_id,
            Body::Refusal(Box::new(BodyRefusal {
                reason,
                envelope_json: None,
                outcome: Outcome {
                    effect_state: OutcomeEffectState::NoEffects,
                    completeness: OutcomeCompleteness::Unavailable,
                    provenance_id: None,
                },
            })),
        )
    }

    fn initialization_refused(
        request_id: Option<String>,
        reason: BodyInitializationRefusedReason,
    ) -> SessionDecision {
        SessionDecision::Reply(SessionFrame {
            schema: 1,
            session_id: None,
            request_id,
            body: Body::InitializationRefused(Box::new(BodyInitializationRefused {
                reason,
                supported_api_versions: vec![API_VERSION.to_owned()],
                supported_schemas: supported_schemas(),
                supported_position_encodings: vec![POSITION_ENCODING.to_owned()],
            })),
        })
    }

    /// Initialize or check outer assigned IDs only. A successful request is
    /// explicitly unvalidated and must pass to its separate domain owner.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
        r = 1
    )]
    pub fn receive(&mut self, frame: SessionFrame) -> SessionDecision {
        let SessionFrame {
            schema,
            session_id,
            request_id,
            body,
        } = frame;
        if schema != 1 {
            return self.refused(request_id, BodyRefusalReason::UnsupportedVersion);
        }
        if self.phase == SessionPhase::Closed {
            return self.refused(request_id, BodyRefusalReason::Unavailable);
        }
        match body {
            Body::InitializeOffer(offer) => {
                if self.phase != SessionPhase::Uninitialized
                    || session_id.is_some()
                    || !request_id.as_deref().is_some_and(valid_id)
                {
                    return Self::initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::InvalidOffer,
                    );
                }
                if !offer.api_versions.iter().any(|value| value == API_VERSION) {
                    return Self::initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonApiVersion,
                    );
                }
                if supported_schemas()
                    .iter()
                    .any(|required| !offer.schema_offers.contains(required))
                {
                    return Self::initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonSchema,
                    );
                }
                if !offer
                    .position_encodings
                    .iter()
                    .any(|value| value == POSITION_ENCODING)
                {
                    return Self::initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonPositionEncoding,
                    );
                }
                let request_id = request_id.expect("validated nonblank request ID");
                self.request_ids.insert(request_id.clone());
                self.phase = SessionPhase::Initialized;
                self.reply(
                    Some(request_id),
                    Body::Initialized(Box::new(BodyInitialized {
                        api_version: API_VERSION.to_owned(),
                        selected_schemas: supported_schemas(),
                        position_encoding: POSITION_ENCODING.to_owned(),
                        actual_capabilities: Capabilities {
                            streaming: false,
                            replay: false,
                            control: false,
                        },
                        contexts: self
                            .contexts
                            .values()
                            .map(|bound| bound.descriptor.clone())
                            .collect(),
                        legacy_mcp_observed_version: PROTOCOL_VERSION.to_owned(),
                    })),
                )
            }
            Body::Request(request) => self.receive_request(session_id, request_id, *request),
            _ => self.refused(request_id, BodyRefusalReason::InvalidRequest),
        }
    }

    fn receive_request(
        &mut self,
        session_id: Option<String>,
        request_id: Option<String>,
        request: BodyRequest,
    ) -> SessionDecision {
        if self.phase != SessionPhase::Initialized {
            return self.refused(request_id, BodyRefusalReason::NotInitialized);
        }
        if session_id.as_deref() != Some(&self.session_id)
            || !request_id.as_deref().is_some_and(valid_id)
        {
            return self.refused(request_id, BodyRefusalReason::InvalidRequest);
        }
        let request_id = request_id.expect("validated nonblank request ID");
        if !self.request_ids.insert(request_id.clone()) {
            return self.refused(Some(request_id), BodyRefusalReason::InvalidRequest);
        }
        let Some(bound) = self.contexts.get(&request.context_id) else {
            return self.refused(Some(request_id), BodyRefusalReason::UnknownContext);
        };
        if bound.descriptor.availability != ContextAvailability::Available {
            return self.refused(Some(request_id), BodyRefusalReason::Unavailable);
        }
        if request
            .target_id
            .as_ref()
            .is_some_and(|id| !bound.descriptor.target_ids.contains(id))
        {
            return self.refused(Some(request_id), BodyRefusalReason::UnknownTarget);
        }
        SessionDecision::UnvalidatedRequest(UnvalidatedRequest {
            session_id: self.session_id.clone(),
            request_id,
            context_id: request.context_id,
            target_id: request.target_id,
            envelope_json: request.envelope_json,
        })
    }
}

#[cfg(test)]
mod tests;
