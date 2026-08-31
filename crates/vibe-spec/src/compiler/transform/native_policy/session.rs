//! Private capture validation and thread-safe native pending session.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;
use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::ExtensionKey;

use super::{
    CompilerInvocationReceipts, CompilerNativePolicy, CompilerPendingRef, CompilerPendingSet,
    PolicyMode,
};
use crate::compiler::transform::config::config_digest;
use crate::compiler::transform::native_identity::CompilerNativeImplementationDigest;
use crate::compiler::transform::plan::{
    ImplementationComponents, TransformConfig, TransformPlan, TransformStage,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PendingCapture {
    pub(crate) reference: CompilerPendingRef,
    pub(crate) point: CompilePoint,
    pub(crate) config: Option<[u8; 32]>,
    pub(crate) implementation: CompilerNativeImplementationDigest,
}

#[derive(Debug)]
pub(crate) struct Receipt {
    pub(crate) capture: PendingCapture,
    pub(crate) count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability {
    Available,
    Unavailable,
}

#[derive(Debug)]
struct Seen {
    capture: PendingCapture,
    availability: Availability,
}

enum ManagedState {
    Collect {
        plan_digest: Option<[u8; 32]>,
        seen: BTreeMap<u32, Seen>,
    },
    Resolve {
        expected: BTreeMap<u32, Receipt>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnavailableDisposition {
    Hard,
    ContinueOriginal,
}

pub(crate) enum NativePolicyResult {
    Fail,
    Collected(CompilerPendingSet),
    Resolved(CompilerInvocationReceipts),
}

pub(crate) enum NativePolicySession {
    Fail,
    Managed(ManagedSession),
}

pub(crate) struct ManagedSession {
    plan_digest: Option<[u8; 32]>,
    state: Mutex<ManagedState>,
}

impl NativePolicySession {
    pub(crate) fn new(
        plan: &TransformPlan,
        policy: CompilerNativePolicy,
    ) -> Result<Self, CompilerNativePolicyError> {
        let (plan_digest, state) = match policy.mode {
            PolicyMode::Fail => return Ok(Self::Fail),
            PolicyMode::Collect => {
                let digest = plan.digest().map(|value| *value.as_bytes());
                (
                    digest,
                    ManagedState::Collect {
                        plan_digest: digest,
                        seen: BTreeMap::new(),
                    },
                )
            }
            PolicyMode::Resolve(expected) => {
                validate_expected(plan, &expected)?;
                let digest = plan.digest().map(|value| *value.as_bytes());
                (
                    digest,
                    ManagedState::Resolve {
                        expected: expected
                            .entries
                            .into_vec()
                            .into_iter()
                            .map(|capture| (capture.reference.order, Receipt { capture, count: 0 }))
                            .collect(),
                    },
                )
            }
        };
        Ok(Self::Managed(ManagedSession {
            plan_digest,
            state: Mutex::new(state),
        }))
    }

    pub(crate) fn success(
        &self,
        order: u32,
        key: &ExtensionKey,
        point: CompilePoint,
        config: Option<&TransformConfig>,
        implementation: CompilerNativeImplementationDigest,
    ) -> Result<(), CompilerNativePolicyError> {
        let Self::Managed(managed) = self else {
            return Ok(());
        };
        managed
            .observe(
                capture(
                    managed.plan_digest,
                    order,
                    key,
                    point,
                    config,
                    implementation,
                )?,
                Availability::Available,
            )
            .map(|_| ())
    }

    pub(crate) fn unavailable(
        &self,
        order: u32,
        key: &ExtensionKey,
        point: CompilePoint,
        config: Option<&TransformConfig>,
        implementation: CompilerNativeImplementationDigest,
    ) -> Result<UnavailableDisposition, CompilerNativePolicyError> {
        let Self::Managed(managed) = self else {
            return Ok(UnavailableDisposition::Hard);
        };
        managed.observe(
            capture(
                managed.plan_digest,
                order,
                key,
                point,
                config,
                implementation,
            )?,
            Availability::Unavailable,
        )
    }

    pub(crate) fn finish(self) -> Result<NativePolicyResult, CompilerNativePolicyError> {
        match self {
            Self::Fail => Ok(NativePolicyResult::Fail),
            Self::Managed(managed) => managed.finish(),
        }
    }

    #[cfg(test)]
    pub(crate) fn observe_capture_for_test(
        &self,
        capture: PendingCapture,
        availability: Availability,
    ) -> Result<UnavailableDisposition, CompilerNativePolicyError> {
        match self {
            Self::Fail => Ok(UnavailableDisposition::Hard),
            Self::Managed(managed) => managed.observe(capture, availability),
        }
    }

    #[cfg(test)]
    pub(crate) fn hold_state_for_test(&self, action: impl FnOnce()) {
        if let Self::Managed(managed) = self {
            managed.hold_state_for_test(action);
        }
    }
}

impl ManagedSession {
    fn observe(
        &self,
        capture: PendingCapture,
        availability: Availability,
    ) -> Result<UnavailableDisposition, CompilerNativePolicyError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error(PolicyFault::Poisoned))?;
        match &mut *state {
            ManagedState::Collect { seen, .. } => {
                observe_collect(seen, capture, availability)?;
                Ok(if availability == Availability::Unavailable {
                    UnavailableDisposition::ContinueOriginal
                } else {
                    UnavailableDisposition::Hard
                })
            }
            ManagedState::Resolve { expected } => {
                observe_resolve(expected, capture, availability)?;
                Ok(UnavailableDisposition::Hard)
            }
        }
    }

    fn finish(self) -> Result<NativePolicyResult, CompilerNativePolicyError> {
        match self
            .state
            .into_inner()
            .map_err(|_| error(PolicyFault::Poisoned))?
        {
            ManagedState::Collect { plan_digest, seen } => {
                let entries = seen
                    .into_values()
                    .filter_map(|seen| {
                        (seen.availability == Availability::Unavailable).then_some(seen.capture)
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                Ok(NativePolicyResult::Collected(CompilerPendingSet {
                    plan_digest,
                    entries,
                }))
            }
            ManagedState::Resolve { expected } => {
                let missing = expected
                    .values()
                    .filter(|receipt| receipt.count == 0)
                    .count();
                if missing != 0 {
                    return Err(error(PolicyFault::MissingExpected { count: missing }));
                }
                Ok(NativePolicyResult::Resolved(CompilerInvocationReceipts {
                    entries: expected
                        .into_values()
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                }))
            }
        }
    }

    #[cfg(test)]
    fn hold_state_for_test(&self, action: impl FnOnce()) {
        let _guard = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        action();
    }
}

fn observe_collect(
    seen: &mut BTreeMap<u32, Seen>,
    capture: PendingCapture,
    availability: Availability,
) -> Result<(), CompilerNativePolicyError> {
    match seen.entry(capture.reference.order) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(Seen {
                capture,
                availability,
            });
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) => {
            let prior = entry.get();
            same_capture(&prior.capture, &capture)?;
            if prior.availability != availability {
                return Err(error(PolicyFault::MixedAvailability {
                    order: capture.reference.order,
                    first: prior.availability,
                    next: availability,
                }));
            }
            Ok(())
        }
    }
}

fn observe_resolve(
    expected: &mut BTreeMap<u32, Receipt>,
    capture: PendingCapture,
    availability: Availability,
) -> Result<(), CompilerNativePolicyError> {
    let order = capture.reference.order;
    let Some(receipt) = expected.get_mut(&order) else {
        return if availability == Availability::Available {
            Ok(())
        } else {
            Err(error(PolicyFault::UnexpectedUnavailable { order }))
        };
    };
    same_capture(&receipt.capture, &capture)?;
    if availability == Availability::Unavailable {
        return Err(error(PolicyFault::ResidualUnavailable { order }));
    }
    receipt.count = receipt
        .count
        .checked_add(1)
        .ok_or_else(|| error(PolicyFault::InvocationCountOverflow { order }))?;
    Ok(())
}

fn same_capture(
    expected: &PendingCapture,
    actual: &PendingCapture,
) -> Result<(), CompilerNativePolicyError> {
    if expected == actual {
        return Ok(());
    }
    let member = if expected.reference.plan_digest != actual.reference.plan_digest {
        "plan digest"
    } else if expected.reference.key != actual.reference.key {
        "qualified key"
    } else if expected.point != actual.point {
        "compile point"
    } else if expected.config != actual.config {
        "semantic config"
    } else {
        "native implementation"
    };
    Err(error(PolicyFault::CaptureConflict {
        order: actual.reference.order,
        member,
    }))
}

fn validate_expected(
    plan: &TransformPlan,
    expected: &CompilerPendingSet,
) -> Result<(), CompilerNativePolicyError> {
    let digest = plan.digest().map(|value| *value.as_bytes());
    if digest != expected.plan_digest {
        return Err(error(PolicyFault::ExpectedPlanMismatch));
    }
    for capture in &expected.entries {
        let order = capture.reference.order;
        let Some(entry) = plan.entries().get(order as usize) else {
            return Err(error(PolicyFault::ExpectedOrderMissing { order }));
        };
        if entry.order() != order || entry.seed().key() != &capture.reference.key {
            return Err(error(PolicyFault::ExpectedKeyMismatch { order }));
        }
        let ImplementationComponents::Native {
            digest: implementation,
        } = entry.seed().implementation().components()
        else {
            return Err(error(PolicyFault::ExpectedNotNative { order }));
        };
        let actual = PendingCapture {
            reference: CompilerPendingRef {
                plan_digest: digest.ok_or_else(|| error(PolicyFault::ExpectedPlanMismatch))?,
                order,
                key: entry.seed().key().clone(),
            },
            point: point(entry.seed().stage()),
            config: entry.config_digest().map(|value| *value.as_bytes()),
            implementation,
        };
        same_capture(capture, &actual)?;
    }
    Ok(())
}

pub(crate) fn validate_pending_set(
    plan: &TransformPlan,
    pending: &CompilerPendingSet,
) -> Result<(), CompilerNativePolicyError> {
    validate_expected(plan, pending)
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn pending_set_for_test(
    plan: &TransformPlan,
    mut entries: Vec<(u32, ExtensionKey)>,
) -> Result<CompilerPendingSet, CompilerNativePolicyError> {
    let digest = plan
        .digest()
        .map(|value| *value.as_bytes())
        .ok_or_else(|| error(PolicyFault::PlanIdentityMissing))?;
    entries.sort_by_key(|(order, _)| *order);
    let entries = entries
        .into_iter()
        .map(|(order, key)| {
            let entry = plan
                .entries()
                .get(order as usize)
                .ok_or_else(|| error(PolicyFault::ExpectedOrderMissing { order }))?;
            let ImplementationComponents::Native {
                digest: implementation,
            } = entry.seed().implementation().components()
            else {
                return Err(error(PolicyFault::ExpectedNotNative { order }));
            };
            Ok(PendingCapture {
                reference: CompilerPendingRef {
                    plan_digest: digest,
                    order,
                    key,
                },
                point: point(entry.seed().stage()),
                config: entry.config_digest().map(|value| *value.as_bytes()),
                implementation,
            })
        })
        .collect::<Result<Vec<_>, CompilerNativePolicyError>>()?
        .into_boxed_slice();
    Ok(CompilerPendingSet {
        plan_digest: Some(digest),
        entries,
    })
}

fn capture(
    plan_digest: Option<[u8; 32]>,
    order: u32,
    key: &ExtensionKey,
    point: CompilePoint,
    config: Option<&TransformConfig>,
    implementation: CompilerNativeImplementationDigest,
) -> Result<PendingCapture, CompilerNativePolicyError> {
    Ok(PendingCapture {
        reference: CompilerPendingRef {
            plan_digest: plan_digest.ok_or_else(|| error(PolicyFault::PlanIdentityMissing))?,
            order,
            key: key.clone(),
        },
        point,
        config: config.map(|value| *config_digest(value.as_table()).as_bytes()),
        implementation,
    })
}

fn point(stage: &TransformStage) -> CompilePoint {
    match stage {
        TransformStage::Source => CompilePoint::Source,
        TransformStage::Document => CompilePoint::Document,
        TransformStage::Lane => CompilePoint::Lane,
        TransformStage::Emitted => CompilePoint::Emitted,
    }
}

#[derive(Debug)]
pub struct CompilerNativePolicyError {
    inner: Box<PolicyFault>,
}

impl fmt::Display for CompilerNativePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for CompilerNativePolicyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

#[derive(Debug, thiserror::Error)]
enum PolicyFault {
    #[error("a native pending capture has no nonempty transform-plan identity")]
    PlanIdentityMissing,
    #[error("the consumed expected pending set belongs to a different transform plan")]
    ExpectedPlanMismatch,
    #[error("expected pending order {order} is absent from the current transform plan")]
    ExpectedOrderMissing { order: u32 },
    #[error("expected pending order {order} names a different current transform key")]
    ExpectedKeyMismatch { order: u32 },
    #[error("expected pending order {order} is no longer a native transform")]
    ExpectedNotNative { order: u32 },
    #[error("pending order {order} changed its {member} across repeated calls")]
    CaptureConflict { order: u32, member: &'static str },
    #[error("pending order {order} mixed {first:?} then {next:?} across repeated calls")]
    MixedAvailability {
        order: u32,
        first: Availability,
        next: Availability,
    },
    #[error("expected pending order {order} remained unavailable during Resolve")]
    ResidualUnavailable { order: u32 },
    #[error("nonexpected order {order} became unavailable during Resolve")]
    UnexpectedUnavailable { order: u32 },
    #[error("Resolve completed without invocation receipts for {count} expected entries")]
    MissingExpected { count: usize },
    #[error("pending order {order} exceeded the invocation receipt count bound")]
    InvocationCountOverflow { order: u32 },
    #[error("the compiler-native pending state lock was poisoned")]
    Poisoned,
}

fn error(inner: PolicyFault) -> CompilerNativePolicyError {
    CompilerNativePolicyError {
        inner: Box::new(inner),
    }
}
