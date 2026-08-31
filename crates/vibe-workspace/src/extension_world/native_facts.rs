//! Structured bridge from one compiler-native invoker to pending build facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER");

use std::fmt;

use specmark::spec;
use vibe_spec::{CompilerNativeInvoker, CompilerPendingSet};

use super::PendingBuildFact;

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
