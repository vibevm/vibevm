//! Frozen analysis values. Underlying disk and active bytes remain separate.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use super::*;
use std::path::PathBuf;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceDisposition {
    Disk,
    Replaced,
    Created,
    Deleted,
    Moved { target: ResourceId },
    MoveDestination { source: ResourceId },
}
#[derive(Debug, Clone)]
pub struct CapturedResource {
    pub(super) declaration: ResourceDeclaration,
    pub(super) disk: Option<StableFileSnapshot>,
    pub(super) underlying: Option<StableFileSnapshot>,
    pub(super) bytes: Option<Arc<[u8]>>,
    pub(super) overlay: Option<Arc<[u8]>>,
    pub(super) version: Option<u64>,
    pub(super) encoding: ByteEncoding,
    pub(super) disposition: ResourceDisposition,
}
impl CapturedResource {
    pub fn path(&self) -> &str {
        &self.declaration.path
    }
    pub fn raw(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
    pub fn buffer_bytes(&self) -> Option<&[u8]> {
        self.overlay.as_deref()
    }
    pub fn buffer_version(&self) -> Option<u64> {
        self.version
    }
    pub fn encoding(&self) -> ByteEncoding {
        self.encoding
    }
    pub fn disposition(&self) -> &ResourceDisposition {
        &self.disposition
    }
    pub fn content_digest(&self) -> Option<String> {
        self.raw().map(super::digest)
    }
    pub fn disk_observation(&self) -> Option<&StableFileSnapshot> {
        self.disk.as_ref()
    }
    pub fn underlying_observation(&self) -> Option<&StableFileSnapshot> {
        self.underlying.as_ref()
    }
}
#[derive(Debug)]
pub struct AnalysisCapture {
    pub(super) id: String,
    pub(super) generation: u64,
    pub(super) context: u64,
    pub(super) client: u64,
    pub(super) primary: ResourceId,
    pub(super) root: PathBuf,
    pub(super) resources: BTreeMap<ResourceId, CapturedResource>,
}
impl AnalysisCapture {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn context(&self) -> u64 {
        self.context
    }
    pub fn client(&self) -> u64 {
        self.client
    }
    pub fn resource(&self, id: ResourceId) -> Option<&CapturedResource> {
        self.resources.get(&id)
    }
    pub fn resources(&self) -> impl Iterator<Item = (ResourceId, &CapturedResource)> {
        self.resources.iter().map(|(id, r)| (*id, r))
    }
}
impl DocumentClient {
    pub fn freeze(&self) -> Result<AnalysisCapture, DocumentAnalysisError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        let mut resources = BTreeMap::new();
        let mut total = 0usize;
        for (id, declaration) in &self.host.declarations {
            let disk = self.host.disk(*id)?;
            let bytes = disk.as_ref().map(|d| Arc::<[u8]>::from(d.bytes.as_slice()));
            let encoding = if bytes
                .as_ref()
                .is_none_or(|b| std::str::from_utf8(b).is_ok())
            {
                ByteEncoding::Utf8
            } else {
                ByteEncoding::Opaque
            };
            total = total
                .checked_add(bytes.as_ref().map_or(0, |b| b.len()))
                .ok_or(DocumentAnalysisError::ResourceLimit)?;
            if !self.host.boot_capture && total > self.host.limits.capture_bytes {
                return Err(DocumentAnalysisError::ResourceLimit);
            }
            resources.insert(
                *id,
                CapturedResource {
                    declaration: declaration.clone(),
                    disk: disk.clone(),
                    underlying: disk,
                    bytes,
                    overlay: None,
                    version: None,
                    encoding,
                    disposition: ResourceDisposition::Disk,
                },
            );
        }
        for (id, buffer) in &state.open {
            total = total
                .checked_add(buffer.bytes.len())
                .ok_or(DocumentAnalysisError::ResourceLimit)?;
            if !self.host.boot_capture && total > self.host.limits.capture_bytes {
                return Err(DocumentAnalysisError::ResourceLimit);
            }
            let resource = resources
                .get_mut(id)
                .ok_or(DocumentAnalysisError::UnknownResource)?;
            resource.underlying = buffer.underlying.clone();
            resource.version = Some(buffer.version);
            resource.encoding = buffer.encoding;
            resource.overlay = Some(buffer.bytes.clone());
            resource.bytes = Some(buffer.bytes.clone());
            resource.disposition = match &buffer.intent {
                BufferIntent::Replace => ResourceDisposition::Replaced,
                BufferIntent::Create => ResourceDisposition::Created,
                BufferIntent::Delete => {
                    resource.bytes = None;
                    ResourceDisposition::Deleted
                }
                BufferIntent::Move { target } => {
                    resource.bytes = None;
                    ResourceDisposition::Moved { target: *target }
                }
            };
        }
        let mut destinations = std::collections::BTreeSet::new();
        for (id, buffer) in &state.open {
            if let BufferIntent::Move { target } = buffer.intent {
                if !destinations.insert(target) {
                    return Err(DocumentAnalysisError::InvalidIntent);
                }
                if state.open.contains_key(&target) {
                    return Err(DocumentAnalysisError::InvalidIntent);
                }
                let destination = resources
                    .get_mut(&target)
                    .ok_or(DocumentAnalysisError::UnknownResource)?;
                if destination.disk.is_some() {
                    return Err(DocumentAnalysisError::InvalidIntent);
                }
                destination.bytes = Some(buffer.bytes.clone());
                destination.version = Some(buffer.version);
                destination.encoding = buffer.encoding;
                destination.overlay = Some(buffer.bytes.clone());
                destination.disposition = ResourceDisposition::MoveDestination { source: *id };
            }
        }
        if !self.host.boot_capture && total > self.host.limits.capture_bytes {
            return Err(DocumentAnalysisError::ResourceLimit);
        }
        state.captures = state
            .captures
            .checked_add(1)
            .ok_or(DocumentAnalysisError::Unavailable)?;
        let id = format!(
            "document-capture/1:{}:{}:{}",
            self.host.host, self.client, state.captures
        );
        Ok(AnalysisCapture {
            id,
            generation: state.generation,
            context: self.host.host,
            client: self.client,
            primary: self.host.primary,
            root: self.host.root().to_path_buf(),
            resources,
        })
    }
}
