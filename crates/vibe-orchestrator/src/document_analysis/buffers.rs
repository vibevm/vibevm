//! Versioned client memory only. No saves or domain admission here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-BUFFER-LAW");
use super::*;
#[derive(Debug, Clone)]
pub(super) struct Buffer {
    pub version: u64,
    pub encoding: ByteEncoding,
    pub bytes: Arc<[u8]>,
    pub intent: BufferIntent,
    pub underlying: Option<StableFileSnapshot>,
}
#[derive(Debug, Default)]
pub(super) struct ClientBuffers {
    pub open: BTreeMap<ResourceId, Buffer>,
    pub versions: BTreeMap<ResourceId, u64>,
    pub captures: u64,
    pub generation: u64,
    pub watch_sequence: u64,
    pub watch_gap: bool,
    pub fences: BTreeMap<ResourceId, u64>,
}
impl ClientBuffers {
    pub fn check_fences(
        &self,
        resource: ResourceId,
        intent: Option<&BufferIntent>,
    ) -> Result<(), DocumentAnalysisError> {
        if self.fences.contains_key(&resource)
            || intent.is_some_and(
                |i| matches!(i,BufferIntent::Move{target} if self.fences.contains_key(target)),
            )
        {
            return Err(DocumentAnalysisError::BarrierHeld);
        };
        Ok(())
    }
    pub fn advance(&mut self) -> Result<(), DocumentAnalysisError> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(DocumentAnalysisError::Unavailable)?;
        Ok(())
    }
}
impl DocumentClient {
    pub fn open(
        &self,
        epoch: u32,
        resource: ResourceId,
        version: u64,
        encoding: ByteEncoding,
        bytes: Arc<[u8]>,
        intent: BufferIntent,
    ) -> Result<(), DocumentAnalysisError> {
        if epoch != 1 {
            return Err(DocumentAnalysisError::UnknownEpoch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        state.check_fences(resource, Some(&intent))?;
        if state.open.contains_key(&resource) {
            return Err(DocumentAnalysisError::AlreadyOpen);
        }
        if version == 0 || state.versions.get(&resource).is_some_and(|v| version <= *v) {
            return Err(DocumentAnalysisError::StaleVersion);
        }
        if state.open.len() >= self.host.limits.open_buffers
            || bytes.len() > self.host.limits.resource_bytes
        {
            return Err(DocumentAnalysisError::ResourceLimit);
        }
        let underlying = self.host.disk(resource)?;
        self.validate_intent(resource, &intent, &underlying)?;
        state.advance()?;
        state.versions.insert(resource, version);
        state.open.insert(
            resource,
            Buffer {
                version,
                encoding,
                bytes,
                intent,
                underlying,
            },
        );
        Ok(())
    }
    pub fn change(
        &self,
        epoch: u32,
        resource: ResourceId,
        expected: u64,
        version: u64,
        encoding: ByteEncoding,
        bytes: Arc<[u8]>,
        intent: BufferIntent,
    ) -> Result<(), DocumentAnalysisError> {
        if epoch != 1 {
            return Err(DocumentAnalysisError::UnknownEpoch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        state.check_fences(resource, Some(&intent))?;
        let old = state
            .open
            .get(&resource)
            .ok_or(DocumentAnalysisError::NotOpen)?;
        if old.version != expected || version <= expected {
            return Err(DocumentAnalysisError::StaleVersion);
        }
        if bytes.len() > self.host.limits.resource_bytes {
            return Err(DocumentAnalysisError::ResourceLimit);
        }
        self.validate_intent(resource, &intent, &old.underlying)?;
        let underlying = old.underlying.clone();
        state.advance()?;
        state.open.insert(
            resource,
            Buffer {
                version,
                encoding,
                bytes,
                intent,
                underlying,
            },
        );
        state.versions.insert(resource, version);
        Ok(())
    }
    pub fn close(
        &self,
        epoch: u32,
        resource: ResourceId,
        expected: u64,
    ) -> Result<(), DocumentAnalysisError> {
        if epoch != 1 {
            return Err(DocumentAnalysisError::UnknownEpoch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        state.check_fences(resource, None)?;
        if state
            .open
            .get(&resource)
            .ok_or(DocumentAnalysisError::NotOpen)?
            .version
            != expected
        {
            return Err(DocumentAnalysisError::StaleVersion);
        }
        state.advance()?;
        state.open.remove(&resource);
        Ok(())
    }
    fn validate_intent(
        &self,
        id: ResourceId,
        intent: &BufferIntent,
        underlying: &Option<StableFileSnapshot>,
    ) -> Result<(), DocumentAnalysisError> {
        match intent {
            BufferIntent::Create if underlying.is_none() => Ok(()),
            BufferIntent::Replace | BufferIntent::Delete if underlying.is_some() => Ok(()),
            BufferIntent::Move { target } if underlying.is_some() && *target != id => {
                if self.host.disk(*target)?.is_some() {
                    Err(DocumentAnalysisError::InvalidIntent)
                } else {
                    Ok(())
                }
            }
            _ => Err(DocumentAnalysisError::InvalidIntent),
        }
    }
}
