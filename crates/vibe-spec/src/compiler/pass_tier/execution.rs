//! Pass-tier execution authority and its opaque public refusal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use std::fmt;

use vibe_core::manifest::ExtensionKey;

use crate::compiler::pass::PassNameError;
use crate::compiler::pipeline::CompilerPipelineError;

/// Internal construction/execution faults for one resolved pass plan.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PassTierExecutionError {
    #[error("pass `{key}` has no named placement; frontend/backend catalogs land in R6.3-D")]
    IntrinsicPlacement { key: ExtensionKey },
    #[error("pass `{key}` has no valid attributed name: {source}")]
    Name {
        key: ExtensionKey,
        #[source]
        source: PassNameError,
    },
    #[error("pass `{key}` declares an invalid compiler IR shape: {detail}")]
    Shape {
        key: ExtensionKey,
        detail: &'static str,
    },
    #[error("pass `{key}` replacement anchor `{anchor}` is absent from the complete schedule")]
    ReplacementAnchor { key: ExtensionKey, anchor: String },
    #[error("pass `{key}` cannot enter native execution: {detail}")]
    NativeExecution { key: ExtensionKey, detail: String },
    #[error("pass `{key}` requires the compiler-native invoker before source execution")]
    MissingInvoker { key: ExtensionKey },
    #[error("the verified pass-tier lane has no installed pipeline verifier")]
    VerifierCapability,
    #[error("the pass-tier schedule is invalid: {source}")]
    Schedule {
        #[source]
        source: CompilerPipelineError,
    },
    #[error(
        "pass plan contains {entries} frontend/backend catalog row(s); catalog execution is deferred to R6.5"
    )]
    CatalogDeferred { entries: usize },
}

/// Public opaque pass-tier construction or capability refusal.
#[derive(Debug)]
pub struct PassTierCompileError {
    inner: Box<PassTierExecutionError>,
}

impl PassTierCompileError {
    pub(crate) fn new(inner: PassTierExecutionError) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    #[cfg(test)]
    pub(crate) fn inner(&self) -> &PassTierExecutionError {
        &self.inner
    }
}

impl fmt::Display for PassTierCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for PassTierCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

impl From<PassTierExecutionError> for PassTierCompileError {
    fn from(inner: PassTierExecutionError) -> Self {
        Self::new(inner)
    }
}
