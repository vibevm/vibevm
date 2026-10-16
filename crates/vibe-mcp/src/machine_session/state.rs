//! Finite per-connection negotiation and private trusted context bindings.
//! A recognized request remains unvalidated and cannot dispatch from here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::machine::e1::operation_envelope::{
    Body as EnvelopeBody, OperationEnvelope,
};
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
    EntropyUnavailable,
    MissingTrustedRoot,
    ObserverUnavailable,
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
    request_id_bytes: usize,
    correlation_exhausted: bool,
    schemas: Vec<SchemaOffer>,
    selected_schemas: Vec<SchemaOffer>,
    capabilities: Capabilities,
    require_target: bool,
}

fn supported_schemas() -> Vec<SchemaOffer> {
    [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MigrationPlanRequest,
        FormatId::MigrationPlanReport,
        FormatId::MigrationPlanError,
    ]
    .into_iter()
    .map(|format| SchemaOffer {
        format_id: format.id().to_owned(),
        epoch: u16::try_from(format.epoch())
            .expect("registered machine frame epoch fits its generated uint16 field"),
    })
    .collect()
}

fn unavailable_context(session: &str, ordinal: usize, kind: ContextKind) -> BoundContext {
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

pub const MAX_REQUEST_IDS: usize = 256;
pub const MAX_REQUEST_ID_BYTES: usize = 256;
pub const MAX_CORRELATION_BYTES: usize = 64 * 1024;

fn allocate_session() -> Result<String, SessionBuildError> {
    let sequence = NEXT_SESSION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| SessionBuildError::IdsExhausted)?;
    // The counter distinguishes connections in this process. A fresh nonce
    // keeps their public session/context/target IDs distinct after restart,
    // so a frame from an old process cannot match by the same spelling.
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| SessionBuildError::EntropyUnavailable)?;
    let random = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("{random}-{sequence}"))
}

fn valid_id(id: &str) -> bool {
    id.len() <= MAX_REQUEST_ID_BYTES && !id.trim().is_empty() && !id.chars().any(char::is_control)
}

impl MachineSession {
    /// A connection-local registry over already-composed, trusted contexts.
    /// This constructor does no HOME lookup, path read or root discovery.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
        r = 1
    )]
    pub fn new(sources: Vec<TrustedContextSource>) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
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
            let bound = unavailable_context(&session, next + ordinal, kind);
            contexts.insert(bound.descriptor.context_id.clone(), bound);
        }
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts,
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: supported_schemas(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: false,
        })
    }

    /// Available only from a sealed real observation handle. These assigned IDs
    /// route to that handle; the descriptor is not a root or attachment grant.
    pub(super) fn bound_observer(
        reader: &vibe_orchestrator::machine_submission::ObservationHandle,
        control: bool,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let target_ids = reader
            .summary()
            .target_id()
            .map(|_| format!("machine-target-{session}-0-0"))
            .into_iter()
            .collect();
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids,
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationStreamMessage,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: true,
                replay: true,
                control,
            },
            require_target: reader.summary().target_id().is_some(),
        })
    }

    /// Only the fixed runner calls this after retaining an explicit host grant.
    pub(super) fn bound_fixed(
        _route: &vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::MachineLifecycleActionRequest,
                FormatId::MachineLifecycleActionReply,
                FormatId::MachineLifecycleActionError,
                FormatId::MachineInputDecisionMessage,
                FormatId::MachineLifecycleSubmissionView,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    /// The exact-local Install runner has the same outer framing and schema
    /// offer as fixed Validate, with its own retained owner route.
    pub(super) fn bound_exact_local_install(
        _route: &vibe_orchestrator::machine_operation::exact_local_install::ExactLocalInstallRoute,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::MachineLifecycleActionRequest,
                FormatId::MachineLifecycleActionReply,
                FormatId::MachineLifecycleActionError,
                FormatId::MachineInputDecisionMessage,
                FormatId::MachineLifecycleSubmissionView,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    /// Only the package runner calls this after retaining an explicit route grant.
    pub(super) fn bound_fixed_package(
        _route: &std::sync::Arc<
            vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute,
        >,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::MachineFixedPackageActionRequest,
                FormatId::MachineFixedPackageActionReply,
                FormatId::MachineFixedPackageActionError,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    /// Only the Cargo runner calls this after retaining an explicit route grant.
    pub(super) fn bound_fixed_cargo(
        _route: &std::sync::Arc<vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute>,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::MachineFixedCargoActionRequest,
                FormatId::MachineFixedCargoActionReply,
                FormatId::MachineFixedCargoActionError,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    /// Only the Deploy runner calls this after retaining an explicit route grant.
    /// Active and recovery-only routes advertise the same inspect/submit wire
    /// format; the owner refuses submission through a recovery-only grant.
    pub(super) fn bound_fixed_deploy(
        _route: &std::sync::Arc<
            vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute,
        >,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::MachineFixedDeployActionRequest,
                FormatId::MachineProfiledDeployActionRequest,
                FormatId::MachineFixedDeployActionReply,
                FormatId::MachineFixedDeployActionError,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    // Root-approved narrow registration; no new session or frame owner.
    pub(super) fn bound_client_deploy(
        _route: &std::sync::Arc<
            vibe_orchestrator::machine_operation::client_deploy::ClientDeployRoute,
        >,
    ) -> Result<Self, SessionBuildError> {
        let session = allocate_session()?;
        let context_id = format!("machine-context-{session}-0");
        let descriptor = Context {
            context_id: context_id.clone(),
            kind: ContextKind::Workspace,
            availability: ContextAvailability::Available,
            target_ids: vec![format!("machine-target-{session}-0-0")],
            reason: None,
        };
        Ok(Self {
            phase: SessionPhase::Uninitialized,
            session_id: format!("machine-session-{session}"),
            contexts: BTreeMap::from([(
                context_id,
                BoundContext {
                    descriptor,
                    _context: None,
                    _target_roots: BTreeMap::new(),
                },
            )]),
            request_ids: BTreeSet::new(),
            request_id_bytes: 0,
            correlation_exhausted: false,
            schemas: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationEnvelope,
                FormatId::ClientDeployRequest,
                FormatId::ClientDeployReply,
                FormatId::ClientDeployError,
            ]
            .into_iter()
            .map(|format| SchemaOffer {
                format_id: format.id().to_owned(),
                epoch: 1,
            })
            .collect(),
            selected_schemas: Vec::new(),
            capabilities: Capabilities {
                streaming: false,
                replay: false,
                control: false,
            },
            require_target: true,
        })
    }

    pub(super) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(super) fn correlation_exhausted(&self) -> bool {
        self.correlation_exhausted
    }

    fn reserve_request_id(&mut self, id: &str) -> bool {
        if self.request_ids.contains(id) {
            return false;
        }
        if self.request_ids.len() >= MAX_REQUEST_IDS
            || self.request_id_bytes.saturating_add(id.len()) > MAX_CORRELATION_BYTES
        {
            self.correlation_exhausted = true;
            return false;
        }
        self.request_id_bytes += id.len();
        self.request_ids.insert(id.to_owned());
        true
    }

    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    /// EOF or the owning transport closes this connection without a daemon.
    pub fn close(&mut self) {
        self.phase = SessionPhase::Closed;
        self.contexts.clear();
        self.request_ids.clear();
        self.request_id_bytes = 0;
    }
}

mod negotiation;

#[cfg(test)]
mod tests;
