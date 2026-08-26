//! Behavior-bearing emit backend registry.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::ir::{ArtifactTarget, LaneIr, PreEmissionWitness};
use super::pass::PassName;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BackendId(String);

impl BackendId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, BackendIdError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let valid = (1..=64).contains(&bytes.len())
            && valid_id_byte(bytes[0])
            && bytes
                .iter()
                .skip(1)
                .all(|byte| valid_id_byte(*byte) || b"._-".contains(byte));
        if valid {
            Ok(Self(value))
        } else {
            Err(BackendIdError { value })
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_id_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

#[derive(Debug, thiserror::Error)]
#[error("invalid emit backend id `{value}`: expected [a-z0-9][a-z0-9._-]{{0,63}}")]
pub(crate) struct BackendIdError {
    value: String,
}

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
        target: ArtifactTarget,
    ) -> Result<Arc<dyn EmitBackend>, BackendRegistryError> {
        let id = target.backend_id();
        self.implementations
            .get(&BackendId(id.to_string()))
            .cloned()
            .ok_or_else(|| BackendRegistryError::Missing {
                backend: id.to_string(),
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
