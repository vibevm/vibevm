//! Registered artifact/backend identities at the compiler's emit seat.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-BACKEND");

use std::collections::BTreeMap;

use crate::compiler::backend::{BackendId, is_builtin_backend_id};
use crate::compiler::ir::{ArtifactTarget, EmittedIr, LaneIr};
use crate::compiler::pass::{IrPayload, PassName};
use crate::compiler::transform::native_manager::CompilerNativeInvoker;

use super::catalog::{CatalogPass, PassCatalogError};
use super::native::NativeBackendPass;
use super::plan::PassEntry;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BackendCatalog {
    backends: BTreeMap<BackendId, CatalogPass>,
}

impl BackendCatalog {
    pub(super) fn register(&mut self, entry: &PassEntry) -> Result<(), PassCatalogError> {
        let authored = entry.declaration().artifact.as_deref().ok_or_else(|| {
            PassCatalogError::MissingBackend {
                key: entry.key().clone(),
            }
        })?;
        let backend =
            BackendId::new(authored.to_owned()).map_err(|_| PassCatalogError::InvalidBackend {
                key: entry.key().clone(),
                backend: bounded(authored),
            })?;
        if is_builtin_backend_id(backend.as_str()) {
            return Err(PassCatalogError::BuiltinBackend {
                key: entry.key().clone(),
                backend: backend.as_str().to_owned(),
            });
        }
        if let Some(first) = self.backends.get(&backend) {
            return Err(PassCatalogError::DuplicateBackend {
                backend: backend.as_str().to_owned(),
                first: first.key().clone(),
                second: entry.key().clone(),
            });
        }
        self.backends.insert(
            backend,
            CatalogPass::new(entry, LaneIr::SHAPE, EmittedIr::SHAPE)?,
        );
        Ok(())
    }

    pub(crate) fn get(&self, backend: &str) -> Option<&CatalogPass> {
        BackendId::new(backend.to_owned())
            .ok()
            .and_then(|id| self.backends.get(&id))
    }

    pub(crate) fn len(&self) -> usize {
        self.backends.len()
    }

    pub(crate) fn selected_native<'invoke>(
        &self,
        target: &ArtifactTarget,
        invoker: Option<&'invoke dyn CompilerNativeInvoker>,
    ) -> Result<Option<NativeBackendPass<'invoke>>, BackendAdmissionError> {
        if !target.is_custom() {
            return Ok(None);
        }
        let backend = target.backend_id();
        let binding = self.get(backend).ok_or_else(|| BackendAdmissionError {
            pass: PassName::new(format!("backend:{backend}"))
                .expect("a validated backend id forms a pass name"),
            reason: format!("custom backend `{backend}` has no selected catalog entry"),
        })?;
        let invoker = invoker.ok_or_else(|| BackendAdmissionError {
            pass: binding.descriptor().name.clone(),
            reason: "backend provider pre-admission requires a compiler-native invoker".to_owned(),
        })?;
        let pass = NativeBackendPass::new(
            binding.entry(),
            invoker,
            backend,
            binding.descriptor().name.clone(),
        )
        .map_err(|error| BackendAdmissionError {
            pass: binding.descriptor().name.clone(),
            reason: error.to_string(),
        })?;
        pass.admit()?;
        Ok(Some(pass))
    }
}

#[derive(Debug)]
pub(crate) struct BackendAdmissionError {
    pub(crate) pass: PassName,
    pub(crate) reason: String,
}

fn bounded(value: &str) -> String {
    let mut characters = value.chars();
    let mut result = characters.by_ref().take(80).collect::<String>();
    if characters.next().is_some() {
        result.push('…');
    }
    result
}
