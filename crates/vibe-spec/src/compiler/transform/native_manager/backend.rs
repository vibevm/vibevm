//! Bytes-only native backend invocation under manager-owned reconstruction.

use std::collections::BTreeMap;

use serde_json::Value;
use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::ExtensionKey;
use vibe_wire::behaviour::native_backend::{AdmittedBackendReply, BackendReplyError, decode_reply};

use super::{
    BoundedMessage, CompilerNativeCall, CompilerNativeImplementationDigest, NativeManagerError,
    NativeRuntime, bounded, returned_request, transition, wire,
};
use crate::compiler::backend::BackendId;
use crate::compiler::ir::{EmittedArtifact, LaneIr};
use crate::compiler::pass::{AnyIr, PassName};
use crate::compiler::verify::IrVerifier;

pub(crate) struct NativeBackendEntry<'entry> {
    runtime: NativeRuntime<'entry>,
    key: &'entry ExtensionKey,
    order: u32,
    config: &'entry BTreeMap<String, Option<Value>>,
    implementation: CompilerNativeImplementationDigest,
    pass: &'entry PassName,
    backend: &'entry str,
}

impl<'entry> NativeBackendEntry<'entry> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        runtime: NativeRuntime<'entry>,
        key: &'entry ExtensionKey,
        order: u32,
        config: &'entry BTreeMap<String, Option<Value>>,
        implementation: CompilerNativeImplementationDigest,
        pass: &'entry PassName,
        backend: &'entry str,
    ) -> Self {
        Self {
            runtime,
            key,
            order,
            config,
            implementation,
            pass,
            backend,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum NativeBackendError {
    #[error(transparent)]
    Manager(#[from] NativeManagerError),
    #[error("the strict native backend reply reader refused: {0}")]
    Reply(#[from] BackendReplyError),
}

pub(crate) fn execute_backend(
    entry: NativeBackendEntry<'_>,
    lane: LaneIr,
) -> Result<EmittedArtifact, NativeBackendError> {
    let payload = wire::encode_generated(&AnyIr::Lane(lane.clone())).map_err(returned_request)?;
    let raw = entry
        .runtime
        .invoker
        .invoke(CompilerNativeCall {
            key: entry.key,
            point: CompilePoint::Pass,
            order: entry.order,
            config: entry.config,
            implementation: entry.implementation,
            frontend_physical_stem: None,
            backend: Some(entry.backend),
            payload,
        })
        .map_err(NativeManagerError::Invoker)?;
    let bytes = match decode_reply(&raw)? {
        AdmittedBackendReply::Ok { bytes, .. } => bytes,
        AdmittedBackendReply::Fail { message } => {
            return Err(NativeManagerError::Fail {
                message: BoundedMessage::new(Some(&message)),
            }
            .into());
        }
    };
    let backend = BackendId::new(entry.backend.to_owned()).map_err(|error| {
        NativeManagerError::ReturnedIr {
            detail: bounded(&error.to_string()),
        }
    })?;
    let emitted = EmittedArtifact {
        provenance: crate::compiler::emit::build_native_provenance(
            &lane,
            backend,
            entry.pass.clone(),
            &bytes,
        ),
        bytes,
    };
    IrVerifier
        .verify(&AnyIr::Emitted(emitted.clone()))
        .map_err(transition)?;
    Ok(emitted)
}
