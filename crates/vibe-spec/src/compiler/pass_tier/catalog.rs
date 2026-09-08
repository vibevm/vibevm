//! Immutable frontend/backend catalogs derived from one artifact pass plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-FRONTEND");

use std::fmt;

use vibe_core::manifest::{ExtensionKey, ExtensionPassKind};

use crate::compiler::ir::IrShape;
use crate::compiler::pass::{PassDescriptor, PassName};

use super::backend::BackendCatalog;
use super::execution::{PassTierCompileError, PassTierExecutionError};
use super::frontend::FrontendCatalog;
use super::plan::{PassEntry, PassPlan};

/// One catalog value retaining the pass's complete qualified identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogPass {
    entry: PassEntry,
    descriptor: PassDescriptor,
}

impl CatalogPass {
    pub(super) fn new(
        entry: &PassEntry,
        input: IrShape,
        output: IrShape,
    ) -> Result<Self, PassCatalogError> {
        let name = PassName::new(format!("pass:{}", entry.key().as_str())).map_err(|_| {
            PassCatalogError::InvalidIdentity {
                key: entry.key().clone(),
            }
        })?;
        Ok(Self {
            entry: entry.clone(),
            descriptor: PassDescriptor {
                name,
                input,
                output,
            },
        })
    }

    pub(crate) fn key(&self) -> &ExtensionKey {
        self.entry.key()
    }

    pub(crate) fn ordinal(&self) -> u32 {
        self.entry.ordinal()
    }

    pub(crate) fn descriptor(&self) -> &PassDescriptor {
        &self.descriptor
    }

    pub(crate) fn entry(&self) -> &PassEntry {
        &self.entry
    }
}

/// The two catalog projections and the remaining positionable pass plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassCatalogs {
    frontends: FrontendCatalog,
    backends: BackendCatalog,
    positioned: PassPlan,
    total_entries: usize,
}

impl PassCatalogs {
    pub(crate) fn resolve(plan: &PassPlan) -> Result<Self, PassCatalogCompileError> {
        let mut frontends = FrontendCatalog::default();
        let mut backends = BackendCatalog::default();
        let mut positioned = Vec::new();
        for entry in plan.entries() {
            match entry.declaration().kind {
                ExtensionPassKind::Frontend => frontends.register(entry)?,
                ExtensionPassKind::Backend => backends.register(entry)?,
                ExtensionPassKind::Transform | ExtensionPassKind::Lowering => {
                    positioned.push(entry.clone());
                }
            }
        }
        let positioned =
            PassPlan::build(positioned).map_err(|error| PassCatalogError::PositionedPlan {
                detail: error.to_string(),
            })?;
        Ok(Self {
            frontends,
            backends,
            positioned,
            total_entries: plan.len(),
        })
    }

    pub(crate) fn frontends(&self) -> &FrontendCatalog {
        &self.frontends
    }

    pub(crate) fn backends(&self) -> &BackendCatalog {
        &self.backends
    }

    pub(crate) fn positioned(&self) -> &PassPlan {
        &self.positioned
    }

    pub(crate) fn production_catalog_refusal(&self) -> Option<PassTierCompileError> {
        (self.total_entries != 0 && self.positioned.is_empty()).then(|| {
            PassTierExecutionError::VerifyEachRequired {
                entries: self.total_entries,
            }
            .into()
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PassCatalogError {
    #[error("pass `{key}` has no valid qualified catalog identity")]
    InvalidIdentity { key: ExtensionKey },
    #[error("frontend pass `{key}` must register at least one source format")]
    MissingFormats { key: ExtensionKey },
    #[error("frontend pass `{key}` registers invalid source format `{format}`")]
    InvalidFormat { key: ExtensionKey, format: String },
    #[error("frontend pass `{key}` cannot replace builtin source format `{format}` implicitly")]
    BuiltinFormat { key: ExtensionKey, format: String },
    #[error("source format `{format}` is registered by both `{first}` and `{second}`")]
    DuplicateFormat {
        format: String,
        first: ExtensionKey,
        second: ExtensionKey,
    },
    #[error("backend pass `{key}` must register one artifact/backend id")]
    MissingBackend { key: ExtensionKey },
    #[error("backend pass `{key}` registers invalid artifact/backend id `{backend}`")]
    InvalidBackend { key: ExtensionKey, backend: String },
    #[error("backend pass `{key}` cannot replace builtin backend `{backend}` implicitly")]
    BuiltinBackend { key: ExtensionKey, backend: String },
    #[error("backend `{backend}` is registered by both `{first}` and `{second}`")]
    DuplicateBackend {
        backend: String,
        first: ExtensionKey,
        second: ExtensionKey,
    },
    #[error("the positionable pass projection is invalid: {detail}")]
    PositionedPlan { detail: String },
}

/// Public opaque catalog refusal carried by artifact compilation.
#[derive(Debug)]
pub struct PassCatalogCompileError {
    inner: Box<PassCatalogError>,
}

impl PassCatalogCompileError {
    #[cfg(test)]
    pub(crate) fn inner(&self) -> &PassCatalogError {
        &self.inner
    }
}

impl fmt::Display for PassCatalogCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for PassCatalogCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

impl From<PassCatalogError> for PassCatalogCompileError {
    fn from(inner: PassCatalogError) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}
