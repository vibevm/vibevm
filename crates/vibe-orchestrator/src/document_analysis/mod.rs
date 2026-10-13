//! Client memory and immutable source analysis; no disk apply authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-BUFFER-LAW");
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use vibe_safefs::{Project, StableFileSnapshot};
mod boot_preview;
mod buffers;
mod capture;
mod catalog;
mod compile_plan;
pub mod edit;
mod invalidation;
mod locations;
mod preview;
mod views;
pub use capture::{AnalysisCapture, CapturedResource, ResourceDisposition};
pub use invalidation::*;
pub use locations::*;
pub use preview::{
    AnalysisDiagnostic, CapturedOutline, CapturedSelection, ConfigurationPreview, DocumentPreview,
};
pub use views::*;
static HOSTS: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId {
    host: u64,
    index: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteEncoding {
    Utf8,
    Opaque,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferIntent {
    Replace,
    Create,
    Delete,
    Move { target: ResourceId },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceRole {
    Manifest,
    Lockfile,
    Source { manifest: String },
    Opaque,
}
#[derive(Debug, Clone)]
pub struct ResourceDeclaration {
    pub path: String,
    pub role: ResourceRole,
}
#[derive(Debug, Clone, Copy)]
pub struct AnalysisLimits {
    pub resource_bytes: usize,
    pub capture_bytes: usize,
    pub open_buffers: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentAnalysisError {
    InvalidDeclaration,
    UnknownResource,
    UnknownEpoch,
    StaleVersion,
    AlreadyOpen,
    NotOpen,
    InvalidIntent,
    ResourceLimit,
    Unavailable,
    BarrierHeld,
}
impl std::fmt::Display for DocumentAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "document analysis refused: {self:?}")
    }
}
impl std::error::Error for DocumentAnalysisError {}
#[derive(Debug)]
pub struct DocumentAnalysisHost {
    project: Project,
    host: u64,
    primary: ResourceId,
    declarations: BTreeMap<ResourceId, ResourceDeclaration>,
    limits: AnalysisLimits,
    clients: AtomicU64,
    participants: Mutex<edit::ParticipantRegistry>,
}
impl DocumentAnalysisHost {
    /// Trusted host composition only. Requests can name only minted resources,
    /// not roots or paths. Source affiliation must name an admitted manifest.
    pub fn new(
        root: &Path,
        primary_manifest: &str,
        declarations: Vec<ResourceDeclaration>,
        limits: AnalysisLimits,
    ) -> Result<Arc<Self>, DocumentAnalysisError> {
        if limits.resource_bytes == 0
            || limits.resource_bytes == usize::MAX
            || limits.capture_bytes < limits.resource_bytes
            || limits.open_buffers == 0
        {
            return Err(DocumentAnalysisError::InvalidDeclaration);
        }
        let project = Project::open(root).map_err(|_| DocumentAnalysisError::Unavailable)?;
        let host = HOSTS
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        let mut rows = BTreeMap::new();
        let mut paths = BTreeMap::new();
        for (index, row) in declarations.into_iter().enumerate() {
            vibe_core::manifest::declarant_path(Path::new(&row.path))
                .map_err(|_| DocumentAnalysisError::InvalidDeclaration)?;
            if paths
                .insert(vibe_safefs::path_identity_key(&row.path), row.path.clone())
                .is_some()
            {
                return Err(DocumentAnalysisError::InvalidDeclaration);
            }
            rows.insert(
                ResourceId {
                    host,
                    index: index as u64,
                },
                row,
            );
        }
        let primary = rows
            .iter()
            .find(|(_, r)| r.path == primary_manifest && r.role == ResourceRole::Manifest)
            .map(|(id, _)| *id)
            .ok_or(DocumentAnalysisError::InvalidDeclaration)?;
        for row in rows.values() {
            if let ResourceRole::Source { manifest } = &row.role {
                let owner = rows
                    .values()
                    .find(|r| r.path == *manifest && r.role == ResourceRole::Manifest)
                    .ok_or(DocumentAnalysisError::InvalidDeclaration)?;
                let parent = Path::new(&owner.path).parent().unwrap_or(Path::new(""));
                if Path::new(&row.path).strip_prefix(parent).is_err() {
                    return Err(DocumentAnalysisError::InvalidDeclaration);
                }
            }
        }
        Ok(Arc::new(Self {
            project,
            host,
            primary,
            declarations: rows,
            limits,
            clients: AtomicU64::new(1),
            participants: Mutex::new(edit::ParticipantRegistry::default()),
        }))
    }
    pub fn root(&self) -> &Path {
        self.project.root_path()
    }
    pub fn resource(&self, path: &str) -> Option<ResourceId> {
        self.declarations
            .iter()
            .find(|(_, d)| d.path == path)
            .map(|(id, _)| *id)
    }
    pub fn client(self: &Arc<Self>) -> Result<DocumentClient, DocumentAnalysisError> {
        let id = self
            .clients
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        let mut registry = self
            .participants
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        if registry.barrier.is_some() {
            return Err(DocumentAnalysisError::BarrierHeld);
        };
        let state = Arc::new(RegisteredClient {
            buffers: Mutex::new(buffers::ClientBuffers::default()),
            connected: AtomicBool::new(true),
        });
        registry.clients.insert(id, Arc::downgrade(&state));
        Ok(DocumentClient {
            host: self.clone(),
            client: id,
            state,
        })
    }
    fn disk(&self, id: ResourceId) -> Result<Option<StableFileSnapshot>, DocumentAnalysisError> {
        let declaration = self
            .declarations
            .get(&id)
            .ok_or(DocumentAnalysisError::UnknownResource)?;
        self.project
            .read_file_snapshot_bounded(&declaration.path, self.limits.resource_bytes)
            .map_err(|_| DocumentAnalysisError::Unavailable)
    }
}
#[derive(Debug)]
pub struct DocumentClient {
    host: Arc<DocumentAnalysisHost>,
    client: u64,
    state: Arc<RegisteredClient>,
}
#[derive(Debug)]
struct RegisteredClient {
    buffers: Mutex<buffers::ClientBuffers>,
    connected: AtomicBool,
}
impl RegisteredClient {
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, buffers::ClientBuffers>> {
        self.buffers.lock()
    }
}
impl Drop for DocumentClient {
    fn drop(&mut self) {
        // Lock-free disconnect remains visible even while apply holds fences.
        self.state.connected.store(false, Ordering::Release);
    }
}
fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
#[cfg(test)]
mod m_16_d_1_cases;

#[cfg(test)]
mod m_16_d_2_cases;
