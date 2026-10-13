//! Authored change origin addresses joined to complete held metadata.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-COMPLETE-FACT-SCOPE");
use super::ParsedCapturedChange;
use crate::complete::{CapturedFactSet, FactSelection};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeFactSelectionError {
    Index,
    Pin,
    Unbound,
    Content,
    Manifest,
    Document(String),
}
impl ParsedCapturedChange {
    /// One explicit origin scope; other origins are not silently included.
    /// Completeness is carried by the actual selection, never authoring done.
    pub fn select_origin_facts<'a>(
        &self,
        index: usize,
        source: &'a CapturedFactSet,
    ) -> Result<FactSelection<'a>, ChangeFactSelectionError> {
        let origin = self
            .manifest
            .origins()
            .get(index)
            .ok_or(ChangeFactSelectionError::Index)?;
        if vibe_core::PackageRef::parse(&origin.package).ok().as_ref()
            != Some(source.inventory().pin())
        {
            return Err(ChangeFactSelectionError::Pin);
        }
        let content = origin
            .selected_content_hash
            .as_deref()
            .ok_or(ChangeFactSelectionError::Unbound)?;
        if vibe_core::ContentHash::parse(content).ok().as_ref()
            != Some(source.inventory().content_hash())
        {
            return Err(ChangeFactSelectionError::Content);
        }
        if origin.manifest_sha256 != format!("sha256:{}", source.inventory().manifest().sha256) {
            return Err(ChangeFactSelectionError::Manifest);
        }
        for requested in &origin.documents {
            for doc in source.documents().values().filter(|d| {
                requested.address.starts_with(&d.address_prefix)
                    && d.state == crate::complete::DocumentState::Native
            }) {
                if let Some(witness) = &doc.witness {
                    if requested.sha256 != witness.digest {
                        return Err(ChangeFactSelectionError::Document(
                            requested.address.clone(),
                        ));
                    }
                }
            }
        }
        Ok(source.select_exact(
            &origin
                .documents
                .iter()
                .map(|d| d.address.clone())
                .collect::<Vec<_>>(),
        ))
    }
}
