//! Buffer-only proposals do not grant disk edits or save authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-PROPOSAL-LAW");
use super::super::*;
#[derive(Debug)]
pub struct BufferProposal {
    context: u64,
    client: u64,
    resource: ResourceId,
    expected_version: u64,
    expected_digest: String,
    replacement: Arc<[u8]>,
}
impl AnalysisCapture {
    pub fn propose_buffer_edit(
        &self,
        id: ResourceId,
        replacement: Arc<[u8]>,
    ) -> Result<BufferProposal, DocumentAnalysisError> {
        let resource = self
            .resource(id)
            .ok_or(DocumentAnalysisError::UnknownResource)?;
        let expected_version = resource
            .buffer_version()
            .ok_or(DocumentAnalysisError::NotOpen)?;
        let raw = resource
            .buffer_bytes()
            .ok_or(DocumentAnalysisError::NotOpen)?;
        Ok(BufferProposal {
            context: self.context,
            client: self.client,
            resource: id,
            expected_version,
            expected_digest: super::super::digest(raw),
            replacement,
        })
    }
}
impl DocumentClient {
    pub fn accept_buffer_proposal(
        &self,
        proposal: &BufferProposal,
        new_version: u64,
    ) -> Result<(), DocumentAnalysisError> {
        if proposal.context != self.host.host || proposal.client != self.client {
            return Err(DocumentAnalysisError::InvalidDeclaration);
        };
        if new_version <= proposal.expected_version {
            return Err(DocumentAnalysisError::StaleVersion);
        };
        if proposal.replacement.len() > self.host.limits.resource_bytes {
            return Err(DocumentAnalysisError::ResourceLimit);
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        state.check_fences(proposal.resource, None)?;
        let old = state
            .open
            .get(&proposal.resource)
            .ok_or(DocumentAnalysisError::NotOpen)?;
        if old.version != proposal.expected_version
            || super::super::digest(&old.bytes) != proposal.expected_digest
        {
            return Err(DocumentAnalysisError::StaleVersion);
        };
        state.check_fences(proposal.resource, Some(&old.intent))?;
        let mut changed = old.clone();
        changed.bytes = proposal.replacement.clone();
        changed.version = new_version;
        state.advance()?;
        state.versions.insert(proposal.resource, new_version);
        state.open.insert(proposal.resource, changed);
        Ok(())
    }
}
