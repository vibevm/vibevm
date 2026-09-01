//! Structured bridge from one compiler-native invoker to pending build facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER");

use std::collections::BTreeMap;
use std::fmt;

use specmark::spec;
use vibe_spec::{CompilerNativeInvoker, CompilerNativePolicy, CompilerPendingSet};

use super::{OwnerRuntimeView, PendingBuildFact};
use crate::WorkspaceError;

/// One invocation authority and the one-shot structured facts recorded by
/// that same object. The compiler borrows the pending set during the drain;
/// FINALIZE retains ownership afterwards.
///
/// ```
/// use vibe_spec::{
///     CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
///     CompilerNativeInvokerErrorKind, CompilerPendingSet,
/// };
/// use vibe_workspace::extension_world::{
///     CompilerNativeFactBinding, CompilerNativeFactError, PendingBuildFact,
/// };
///
/// struct Binding;
/// impl CompilerNativeInvoker for Binding {
///     fn invoke(
///         &self,
///         _call: CompilerNativeCall<'_>,
///     ) -> Result<Vec<u8>, CompilerNativeInvokerError> {
///         Err(CompilerNativeInvokerError::new(
///             CompilerNativeInvokerErrorKind::InvocationFailed,
///             "fixture",
///         ))
///     }
/// }
/// impl CompilerNativeFactBinding for Binding {
///     fn invoker(&self) -> &dyn CompilerNativeInvoker { self }
///     fn take_pending_build_facts(
///         &self,
///         _pending: &CompilerPendingSet,
///     ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
///         Err(CompilerNativeFactError::already_taken())
///     }
///     fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
///         Err(CompilerNativeFactError::already_taken())
///     }
/// }
/// let binding: &dyn CompilerNativeFactBinding = &Binding;
/// let _: &dyn CompilerNativeInvoker = binding.invoker();
/// ```
pub trait CompilerNativeFactBinding: Send + Sync {
    fn invoker(&self) -> &dyn CompilerNativeInvoker;

    fn take_pending_build_facts(
        &self,
        pending: &CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError>;

    /// Terminally drain a Ready compile's recorder, which must be empty.
    fn finish_ready(&self) -> Result<(), CompilerNativeFactError>;
}

/// One exact owner-borrowing binding plus its consumed manager policy.
pub struct OwnerNativeCompileBinding<B> {
    binding: B,
    policy: CompilerNativePolicy,
}

impl<B> OwnerNativeCompileBinding<B> {
    #[doc(hidden)]
    #[must_use]
    pub fn new(binding: B, policy: CompilerNativePolicy) -> Self {
        Self { binding, policy }
    }

    #[must_use]
    pub fn into_parts(self) -> (B, CompilerNativePolicy) {
        (self.binding, self.policy)
    }
}

/// Lazy provider of one concrete binding for the exact retained lane owner.
///
/// ```no_run
/// use vibe_workspace::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeView};
///
/// fn bind<'a, P: OwnerNativeCompileProvider>(
///     provider: &mut P,
///     owner: OwnerRuntimeView<'a>,
/// ) {
///     let _binding = provider.bind(owner);
/// }
/// ```
pub trait OwnerNativeCompileProvider {
    type Binding<'owner>: CompilerNativeFactBinding + 'owner;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError>;
}

/// Cross-crate factory for one replay's complete, workspace-owned policy map.
///
/// ```no_run
/// use std::collections::BTreeMap;
/// use vibe_workspace::extension_world::CompilerNativeReplayFactory;
///
/// fn replay_once<F: CompilerNativeReplayFactory>(factory: &mut F) {
///     if let Ok(provider) = factory.create(BTreeMap::new()) {
///         let _ = factory.finish(provider);
///     }
/// }
/// ```
#[doc(hidden)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER")]
pub trait CompilerNativeReplayFactory {
    type Provider: OwnerNativeCompileProvider;

    fn create(
        &mut self,
        policies: BTreeMap<super::OwnerRuntimeId, CompilerNativePolicy>,
    ) -> Result<Self::Provider, WorkspaceError>;

    fn finish(&mut self, provider: Self::Provider) -> Result<(), WorkspaceError>;
}

/// Opaque bounded refusal from the one-shot compiler-native fact recorder.
pub struct CompilerNativeFactError {
    fault: FactFault,
}

impl CompilerNativeFactError {
    #[doc(hidden)]
    #[must_use]
    pub const fn poisoned() -> Self {
        Self {
            fault: FactFault::Poisoned,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn already_taken() -> Self {
        Self {
            fault: FactFault::AlreadyTaken,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn missing(order: u32) -> Self {
        Self {
            fault: FactFault::Missing { order },
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn extra(order: u32) -> Self {
        Self {
            fault: FactFault::Extra { order },
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn conflict(order: u32) -> Self {
        Self {
            fault: FactFault::Conflict { order },
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn construction(order: u32) -> Self {
        Self {
            fault: FactFault::Construction { order },
        }
    }
}

impl fmt::Debug for CompilerNativeFactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompilerNativeFactError(..)")
    }
}

impl fmt::Display for CompilerNativeFactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fault.fmt(formatter)
    }
}

impl std::error::Error for CompilerNativeFactError {}

#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER")]
enum FactFault {
    #[error(
        "compiler-native pending fact state is poisoned \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: discard this invocation and retry from one fresh owner runtime)"
    )]
    Poisoned,
    #[error(
        "compiler-native pending facts were already taken \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: drain each compiler-native binding exactly once)"
    )]
    AlreadyTaken,
    #[error(
        "compiler-native pending fact for manager order {order} is missing \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: retain the fact from the same unavailable manager invocation)"
    )]
    Missing { order: u32 },
    #[error(
        "compiler-native pending fact for manager order {order} is extra \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: drain against the exact pending set from the same compile)"
    )]
    Extra { order: u32 },
    #[error(
        "compiler-native pending fact conflicts at manager order {order} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: preserve one exact retained row and semantic witness set)"
    )]
    Conflict { order: u32 },
    #[error(
        "compiler-native pending fact construction refused at manager order {order} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: restore the exact build:cargo route and pending reference)"
    )]
    Construction { order: u32 },
}
