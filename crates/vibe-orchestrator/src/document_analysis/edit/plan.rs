//! Saved captured inputs, finite host grant and participant commitments.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::*;
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    AuthorityRefused,
    InvalidPlan,
    SaveRequired,
    Stale,
    Disconnected,
    Refused,
    Expired,
    Busy,
    AcknowledgementRequired,
    Unavailable,
    AlreadyApplied,
    RecoveryRequired,
    InvalidJournal,
    JournalShape,
    JournalEpoch,
    JournalIntegrity,
    ThirdState,
}
impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "document edit refused: {self:?}")
    }
}
impl std::error::Error for EditError {}
#[derive(Debug, Clone)]
pub struct DocumentReplacement {
    pub resource: ResourceId,
    pub bytes: Arc<[u8]>,
}
#[derive(Debug)]
pub struct DocumentEditAuthority {
    pub(super) host: Arc<DocumentAnalysisHost>,
    pub(super) resources: BTreeSet<ResourceId>,
}
impl DocumentEditAuthority {
    /// Explicit trusted composition, separate from the read host/context.
    pub fn new(
        host: Arc<DocumentAnalysisHost>,
        resources: Vec<ResourceId>,
    ) -> Result<Arc<Self>, EditError> {
        let selected: BTreeSet<_> = resources.iter().copied().collect();
        if selected.is_empty() || selected.len() != resources.len() {
            return Err(EditError::AuthorityRefused);
        };
        for id in &selected {
            let row = host
                .declarations
                .get(id)
                .ok_or(EditError::AuthorityRefused)?;
            if row.role == ResourceRole::Lockfile
                || row.path.starts_with(".vibe/")
                || row.path.starts_with(".zap-state/")
            {
                return Err(EditError::AuthorityRefused);
            };
        }
        Ok(Arc::new(Self {
            host,
            resources: selected,
        }))
    }
    pub fn belongs_to(&self, host: &Arc<DocumentAnalysisHost>) -> bool {
        Arc::ptr_eq(&self.host, host)
    }
    pub fn root(&self) -> &std::path::Path {
        self.host.root()
    }
    pub fn prepare(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        replacements: Vec<DocumentReplacement>,
    ) -> Result<EditPlan, EditError> {
        self.prepare_inner(capture, replacements, false)
    }
    pub(super) fn prepare_inner(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        replacements: Vec<DocumentReplacement>,
        allow_unchanged: bool,
    ) -> Result<EditPlan, EditError> {
        if capture.context != self.host.host {
            return Err(EditError::AuthorityRefused);
        };
        let mut rows = BTreeMap::new();
        for replacement in replacements {
            let id = replacement.resource;
            if !self.resources.contains(&id) {
                return Err(EditError::AuthorityRefused);
            };
            if replacement.bytes.len() > self.host.limits.resource_bytes || rows.contains_key(&id) {
                return Err(EditError::InvalidPlan);
            };
            let resource = capture.resource(id).ok_or(EditError::InvalidPlan)?;
            let disk = resource.disk_observation().ok_or(EditError::SaveRequired)?;
            if resource.raw() != Some(disk.bytes.as_slice())
                || !matches!(
                    resource.disposition(),
                    ResourceDisposition::Disk | ResourceDisposition::Replaced
                )
            {
                return Err(EditError::SaveRequired);
            };
            if !allow_unchanged && replacement.bytes.as_ref() == disk.bytes.as_slice() {
                return Err(EditError::InvalidPlan);
            };
            rows.insert(
                id,
                PlannedFile {
                    path: resource.path().into(),
                    before: disk.clone(),
                    desired: replacement.bytes,
                },
            );
        }
        if rows.is_empty() {
            return Err(EditError::InvalidPlan);
        };
        let expected = rows
            .iter()
            .map(|(id, file)| (*id, Some(Arc::from(file.before.bytes.as_slice()))))
            .collect();
        let participants = super::fence::collect(self, capture, &expected)?;
        let mut plan = EditPlan {
            authority: self.clone(),
            id: String::new(),
            capture: capture.id().into(),
            files: rows,
            participants,
            recovery: None,
        };
        plan.id = super::journal::intent_id(&super::journal::from_plan(&plan)?);
        Ok(plan)
    }
}
pub struct EditPlan {
    pub(super) authority: Arc<DocumentEditAuthority>,
    pub(super) id: String,
    pub(super) capture: String,
    pub(super) files: BTreeMap<ResourceId, PlannedFile>,
    pub(super) participants: Vec<Participant>,
    pub(super) recovery: Option<super::journal::Journal>,
}
impl std::fmt::Debug for EditPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditPlan")
            .field("id", &self.id)
            .field("capture", &self.capture)
            .field("resources", &self.files.keys().collect::<Vec<_>>())
            .field("participants", &self.participants.len())
            .finish()
    }
}
impl EditPlan {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn capture(&self) -> &str {
        &self.capture
    }
    pub fn resources(&self) -> impl Iterator<Item = ResourceId> + '_ {
        self.files.keys().copied()
    }
}
pub(super) struct PlannedFile {
    pub path: String,
    pub before: StableFileSnapshot,
    pub desired: Arc<[u8]>,
}
pub(super) struct Participant {
    pub client_id: u64,
    pub client: Arc<RegisteredClient>,
    pub versions: BTreeMap<ResourceId, Option<u64>>,
    pub generation: u64,
}
