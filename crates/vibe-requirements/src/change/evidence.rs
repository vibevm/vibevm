//! Non-normative release evidence from the same retained native observations.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-BOOTSTRAP-EVIDENCE");
use super::{SavedChange, SavedChangeError};
use std::path::Path;
use vibe_core::manifest::change::{ChangeDocumentRole, ChangeManifest};
use vibe_wire::generated::preview::e1::release_impact::ReleaseImpact;

pub(super) fn from_observations(
    manifest: &ChangeManifest,
    inputs: &[(&Path, &[u8])],
) -> Result<Vec<ReleaseImpact>, SavedChangeError> {
    let mut records = vec![];
    for doc in manifest
        .documents()
        .iter()
        .filter(|d| d.role == ChangeDocumentRole::ReleaseImpact)
    {
        let path = manifest.directory().join(&doc.path);
        let mut found = inputs.iter().filter(|(p, _)| *p == path);
        let (_, bytes) = found
            .next()
            .ok_or_else(|| SavedChangeError::MissingDocument(doc.path.clone()))?;
        if found.next().is_some() {
            return Err(SavedChangeError::Unavailable(
                "duplicate release evidence observation",
            ));
        }
        records.push(ReleaseImpact::read_for_change(
            bytes,
            manifest.id().as_str(),
            &vibe_core::machine_json_path(&path),
        )?);
    }
    Ok(records)
}
impl SavedChange {
    pub fn release_impacts(&self) -> Result<Vec<ReleaseImpact>, SavedChangeError> {
        let inputs: Vec<_> = self
            .input_observations
            .iter()
            .map(|(p, s)| (p.as_path(), s.bytes.as_slice()))
            .collect();
        from_observations(&self.manifest, &inputs)
    }
}
