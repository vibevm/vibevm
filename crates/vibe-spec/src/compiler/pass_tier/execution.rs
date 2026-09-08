//! Pass-tier execution authority and its opaque public refusal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use std::fmt;
use std::marker::PhantomData;

use vibe_core::manifest::ExtensionKey;

use crate::compiler::pass::{IrPayload, Pass, PassName, PassNameError};
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
    #[error("pass `{key}` cannot execute in the verified test lane: {detail}")]
    TestExecution { key: ExtensionKey, detail: String },
    #[error("the verified pass-tier lane has no installed pipeline verifier")]
    VerifierCapability,
    #[error("the pass-tier schedule is invalid: {source}")]
    Schedule {
        #[source]
        source: CompilerPipelineError,
    },
    #[error(
        "a nonempty pass plan ({entries} entries) cannot execute until R6.4 installs mandatory verify-each"
    )]
    VerifyEachRequired { entries: usize },
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

/// A typed schedule vehicle used only while production is fail-closed.
pub(super) struct CapabilityBlockedPass<Input, Output> {
    name: PassName,
    marker: PhantomData<fn(Input) -> Output>,
}

impl<Input, Output> CapabilityBlockedPass<Input, Output> {
    pub(super) fn new(name: PassName) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("mandatory pass-tier verify-each is unavailable")]
pub(super) struct CapabilityBlocked;

impl<Input, Output> Pass for CapabilityBlockedPass<Input, Output>
where
    Input: IrPayload,
    Output: IrPayload,
{
    type Input = Input;
    type Output = Output;
    type Error = CapabilityBlocked;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: Input) -> Result<Output, Self::Error> {
        Err(CapabilityBlocked)
    }
}
