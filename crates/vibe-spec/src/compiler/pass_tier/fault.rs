//! Typed construction faults for the immutable pass plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use std::fmt;

use vibe_core::manifest::ExtensionKey;

use crate::compiler::transform::fault::TransformLoweringError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PassTierFault {
    #[error("enabled row {row} (`{key}`) is at non-compile point `{point}`")]
    NonCompilePoint {
        row: usize,
        key: String,
        point: String,
    },
    #[error("activated compile:pass row {row} (`{key}`) has no `pass` declaration")]
    MissingPass { row: usize, key: String },
    #[error(
        "dependency compile:pass row {row} (`{key}`) reached lowering without explicit host activation"
    )]
    DependencyNotActivated { row: usize, key: String },
    #[error("compile:pass row {row} (`{key}`) declares unsupported handler kind `{kind}`")]
    UnsupportedHandler {
        row: usize,
        key: String,
        kind: &'static str,
    },
    #[error("effective compile order contains more than u32::MAX rows")]
    OrderOverflow,
    #[error("pass plan contains duplicate extension key `{key}`")]
    DuplicateKey { key: ExtensionKey },
    #[error("staged transform lowering refused while partitioning compile rows: {source}")]
    Transform {
        #[source]
        source: TransformLoweringError,
    },
}

/// Public opaque error for the joint compile-plan lowering.
#[derive(Debug)]
pub struct CompilePlanLoweringError {
    inner: Box<PassTierFault>,
}

impl CompilePlanLoweringError {
    pub(crate) fn new(inner: PassTierFault) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    #[cfg(test)]
    pub(crate) fn inner(&self) -> &PassTierFault {
        &self.inner
    }
}

impl fmt::Display for CompilePlanLoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for CompilePlanLoweringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

impl From<PassTierFault> for CompilePlanLoweringError {
    fn from(inner: PassTierFault) -> Self {
        Self::new(inner)
    }
}
