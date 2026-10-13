//! Strict owned document journal, using incumbent capability publication.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-RECOVERY-LAW");
use super::*;
use vibe_safefs::{DirectoryDurability, Project};
pub(super) use vibe_wire::generated::document_edit_journal::{
    DocumentEditJournal as Journal, DocumentEditJournalPhase as Phase, File as JournalFile,
    FileState,
};
const CAP: usize = 8 * 1024 * 1024;
pub(super) fn path(id: &str) -> Result<String, EditError> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(EditError::InvalidJournal);
    };
    Ok(format!(".vibe/document-edits/{id}.json"))
}
pub(super) fn binding(authority: &DocumentEditAuthority) -> Result<String, EditError> {
    authority
        .host
        .project
        .root_correlation_token()
        .map_err(|_| EditError::Unavailable)
}
pub(super) fn raw_hash(bytes: &[u8]) -> String {
    super::super::digest(bytes)
        .trim_start_matches("sha256:")
        .into()
}
/// Immutable intent framing excludes publication observations and phase.
pub(super) fn intent_id(journal: &Journal) -> String {
    fn field(out: &mut Vec<u8>, value: &[u8]) {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value);
    }
    let mut bytes = b"document-edit-intent/1".to_vec();
    for value in [&journal.recipe, &journal.root_binding, &journal.capture] {
        field(&mut bytes, value.as_bytes());
    }
    let mut files: Vec<_> = journal.files.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    bytes.extend_from_slice(&(files.len() as u64).to_be_bytes());
    for f in files {
        for value in [
            &f.path,
            &f.before_sha256,
            &f.after_sha256,
            &f.before_identity,
        ] {
            field(&mut bytes, value.as_bytes());
        }
        match f.unix_mode {
            None => bytes.push(0),
            Some(mode) => {
                bytes.push(1);
                bytes.extend_from_slice(&mode.to_be_bytes());
            }
        }
    }
    raw_hash(&bytes)
}
pub(super) fn decode(authority: &DocumentEditAuthority, raw: &[u8]) -> Result<Journal, EditError> {
    if raw.len() > CAP {
        return Err(EditError::InvalidJournal);
    };
    let journal: Journal = serde_json::from_slice(raw).map_err(|_| EditError::JournalShape)?;
    if journal.schema != 1 || journal.recipe != "document-edit-journal/1" {
        return Err(EditError::JournalEpoch);
    };
    path(&journal.id)?;
    if intent_id(&journal) != journal.id {
        return Err(EditError::JournalIntegrity);
    };
    if journal.root_binding != binding(authority)? {
        return Err(EditError::AuthorityRefused);
    };
    if journal.files.is_empty() || journal.files.len() > authority.resources.len() {
        return Err(EditError::JournalIntegrity);
    };
    let mut seen = std::collections::BTreeSet::new();
    for row in &journal.files {
        if !seen.insert(&row.path)
            || !authority
                .resources
                .iter()
                .any(|id| authority.host.declarations[id].path == row.path)
        {
            return Err(EditError::AuthorityRefused);
        };
        if row.before.len() > authority.host.limits.resource_bytes
            || row.after.len() > authority.host.limits.resource_bytes
            || row.before == row.after
            || raw_hash(&row.before) != row.before_sha256
            || raw_hash(&row.after) != row.after_sha256
            || row.before_identity.is_empty()
            || row.before_identity.len() > 256
            || row
                .after_identity
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256)
            || (row.state == FileState::Verified && row.after_identity.is_none())
        {
            return Err(EditError::JournalIntegrity);
        };
    }
    if journal.phase == Phase::Completed
        && journal.files.iter().any(|r| r.state != FileState::Verified)
    {
        return Err(EditError::JournalIntegrity);
    };
    Ok(journal)
}
pub(super) fn load(
    authority: &DocumentEditAuthority,
    id: &str,
) -> Result<Option<Journal>, EditError> {
    let snapshot = authority
        .host
        .project
        .read_file_snapshot_bounded(&path(id)?, CAP)
        .map_err(|_| EditError::InvalidJournal)?;
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    let journal = decode(authority, &snapshot.bytes)?;
    if journal.id != id {
        return Err(EditError::JournalIntegrity);
    };
    Ok(Some(journal))
}
pub(super) fn from_plan(plan: &EditPlan) -> Result<Journal, EditError> {
    Ok(Journal {
        schema: 1,
        recipe: "document-edit-journal/1".into(),
        id: plan.id.clone(),
        root_binding: binding(&plan.authority)?,
        capture: plan.capture.clone(),
        phase: Phase::Prepared,
        files: plan
            .files
            .values()
            .map(|f| JournalFile {
                path: f.path.clone(),
                before: f.before.bytes.clone(),
                after: f.desired.to_vec(),
                before_sha256: f.before.sha256.clone(),
                after_sha256: raw_hash(&f.desired),
                before_identity: f.before.identity.observation_correlation_token(),
                after_identity: None,
                unix_mode: f.before.unix_mode,
                state: FileState::Pending,
            })
            .collect(),
    })
}
pub(super) fn checkpoint(project: &Project, journal: &Journal) -> Result<(), EditError> {
    let raw = serde_json::to_vec(journal).map_err(|_| EditError::InvalidJournal)?;
    if raw.len() > CAP {
        return Err(EditError::InvalidJournal);
    };
    let relative = path(&journal.id)?;
    let prior = if let Some(prior) = project
        .read_file_snapshot_bounded(&relative, CAP)
        .map_err(|_| EditError::InvalidJournal)?
    {
        prior
    } else {
        let directory = project
            .dir(&[".vibe", "document-edits"], true)
            .map_err(|_| EditError::RecoveryRequired)?;
        project
            .publish_new_in(&directory, &format!("{}.json", journal.id), &raw)
            .map_err(|_| EditError::RecoveryRequired)?;
        project
            .read_file_snapshot_bounded(&relative, CAP)
            .map_err(|_| EditError::RecoveryRequired)?
            .ok_or(EditError::RecoveryRequired)?
    };
    let write = project
        .write_snapshot_guarded(&relative, &prior, &raw)
        .map_err(|_| EditError::RecoveryRequired)?;
    if !write.file_synced || matches!(write.parent, DirectoryDurability::Failed(_)) {
        return Err(EditError::RecoveryRequired);
    };
    let visible = project
        .read_file_snapshot_bounded(&relative, CAP)
        .map_err(|_| EditError::RecoveryRequired)?;
    if visible.as_ref().map(|s| s.bytes.as_slice()) != Some(raw.as_slice()) {
        return Err(EditError::RecoveryRequired);
    };
    Ok(())
}
impl DocumentEditAuthority {
    pub fn prepare_recovery(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        id: &str,
    ) -> Result<EditPlan, EditError> {
        let journal = load(self, id)?.ok_or(EditError::RecoveryRequired)?;
        let mut replacements = vec![];
        for row in &journal.files {
            let resource = *self
                .resources
                .iter()
                .find(|id| self.host.declarations[id].path == row.path)
                .ok_or(EditError::AuthorityRefused)?;
            let saved = capture
                .resource(resource)
                .and_then(|r| r.disk_observation())
                .ok_or(EditError::SaveRequired)?;
            if saved.unix_mode != row.unix_mode {
                return Err(EditError::ThirdState);
            };
            if saved.bytes == row.before {
                if saved.identity.observation_correlation_token() != row.before_identity {
                    return Err(EditError::ThirdState);
                };
            } else if saved.bytes == row.after {
                if row
                    .after_identity
                    .as_ref()
                    .is_some_and(|id| id != &saved.identity.observation_correlation_token())
                {
                    return Err(EditError::ThirdState);
                };
            } else {
                return Err(EditError::ThirdState);
            };
            replacements.push(DocumentReplacement {
                resource,
                bytes: Arc::from(row.after.as_slice()),
            });
        }
        let mut plan = self.prepare_inner(capture, replacements, true)?;
        plan.id = journal.id.clone();
        plan.recovery = Some(journal);
        Ok(plan)
    }
}
