//! Behavior-bearing emit backend registry.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::ir::{ArtifactTarget, LaneIr, PreEmissionWitness};
use super::pass::PassName;

// The single backend/target identity lives below this registry, in
// `ir/target.rs`, so `ArtifactTarget` and `BackendRegistry` share one id law.
pub(crate) use super::ir::BackendId;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BackendError {
    #[error("backend `{backend}` cannot emit this Lane: {reason}")]
    Emit { backend: String, reason: String },
    #[error("backend `{backend}` transition differs at {field}")]
    Transition {
        backend: String,
        field: &'static str,
    },
    #[error("backend `{backend}` current bytes are invalid: {reason}")]
    Current { backend: String, reason: String },
}

pub(crate) trait EmitBackend: Send + Sync {
    fn id(&self) -> &BackendId;
    fn pass_name(&self) -> &PassName;
    fn emit(&self, lane: &LaneIr, witness: &PreEmissionWitness) -> Result<Vec<u8>, BackendError>;
}

#[derive(Default)]
pub(crate) struct BackendRegistry {
    implementations: BTreeMap<BackendId, Arc<dyn EmitBackend>>,
}

impl BackendRegistry {
    pub(crate) fn builtins() -> Self {
        let mut registry = Self::default();
        registry
            .register(Arc::new(
                super::emit::static_md::StaticMarkdownBackend::new(),
            ))
            .expect("static-md is the first valid built-in backend");
        registry
            .register(Arc::new(super::emit::static_xml::StaticXmlBackend::new()))
            .expect("static-xml is a distinct valid built-in backend");
        registry
    }

    pub(crate) fn register(
        &mut self,
        backend: Arc<dyn EmitBackend>,
    ) -> Result<(), BackendRegistryError> {
        let id = backend.id().clone();
        let expected_pass = format!("emit:{}", id.as_str());
        if backend.pass_name().as_str() != expected_pass {
            return Err(BackendRegistryError::PassIdentity {
                backend: id,
                actual: backend.pass_name().as_str().to_string(),
            });
        }
        if self.implementations.contains_key(&id) {
            return Err(BackendRegistryError::Collision { backend: id });
        }
        self.implementations.insert(id, backend);
        Ok(())
    }

    pub(crate) fn replace(
        &mut self,
        backend: Arc<dyn EmitBackend>,
    ) -> Result<Arc<dyn EmitBackend>, BackendRegistryError> {
        let id = backend.id().clone();
        let expected_pass = format!("emit:{}", id.as_str());
        if backend.pass_name().as_str() != expected_pass {
            return Err(BackendRegistryError::PassIdentity {
                backend: id,
                actual: backend.pass_name().as_str().to_string(),
            });
        }
        if !self.implementations.contains_key(&id) {
            return Err(BackendRegistryError::ReplacementMissing { backend: id });
        }
        self.implementations
            .insert(id, backend)
            .ok_or_else(|| BackendRegistryError::Missing {
                backend: "replacement race".to_string(),
            })
    }

    pub(crate) fn selected(
        &self,
        target: &ArtifactTarget,
    ) -> Result<Arc<dyn EmitBackend>, BackendRegistryError> {
        // A target's own backend id is a validated id by construction, so this
        // revalidation cannot fail; Missing keeps the honest refusal.
        let id =
            BackendId::new(target.backend_id()).map_err(|error| BackendRegistryError::Missing {
                backend: error.value,
            })?;
        self.implementations
            .get(&id)
            .cloned()
            .ok_or_else(|| BackendRegistryError::Missing {
                backend: id.as_str().to_string(),
            })
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn get(&self, id: &BackendId) -> Result<Arc<dyn EmitBackend>, BackendRegistryError> {
        self.implementations
            .get(id)
            .cloned()
            .ok_or_else(|| BackendRegistryError::Missing {
                backend: id.as_str().to_string(),
            })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BackendRegistryError {
    #[error("emit backend `{}` is already registered", .backend.as_str())]
    Collision { backend: BackendId },
    #[error(
        "emit backend `{}` declares pass `{actual}`, expected `emit:{}`",
        .backend.as_str(),
        .backend.as_str()
    )]
    PassIdentity { backend: BackendId, actual: String },
    #[error("required emit backend `{backend}` is not registered")]
    Missing { backend: String },
    #[error("emit backend `{}` cannot be replaced before registration", .backend.as_str())]
    ReplacementMissing { backend: BackendId },
}

#[cfg(test)]
mod tests;
