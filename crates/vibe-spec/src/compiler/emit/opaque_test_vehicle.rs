//! Opaque-byte test vehicle; never selected by a shipping ArtifactTarget.

use super::super::backend::{BackendError, BackendId, EmitBackend};
use super::super::ir::{LaneIr, PreEmissionWitness};
use super::super::pass::PassName;

pub(crate) struct OpaqueTestBackend {
    id: BackendId,
    pass: PassName,
    bytes: Vec<u8>,
}

impl OpaqueTestBackend {
    pub(crate) fn new() -> Self {
        Self {
            id: BackendId::new("opaque-test").unwrap(),
            pass: PassName::new("emit:opaque-test").unwrap(),
            bytes: vec![0x00, 0xff, b'\n'],
        }
    }

    pub(crate) fn replacement() -> Self {
        Self {
            id: BackendId::new("static-md").unwrap(),
            pass: PassName::new("emit:static-md").unwrap(),
            bytes: vec![0x10, 0xfe, b'R'],
        }
    }
}

impl EmitBackend for OpaqueTestBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(&self, _lane: &LaneIr, _witness: &PreEmissionWitness) -> Result<Vec<u8>, BackendError> {
        Ok(self.bytes.clone())
    }
}
