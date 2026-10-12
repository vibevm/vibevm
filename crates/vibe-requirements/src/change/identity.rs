//! Identity analysis uses the accepted saved observation set; no new reads.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-INDEPENDENT-IDENTITIES");
use super::{SavedChange, SavedChangeError};
use vibe_core::manifest::change::{
    ChangeDocumentRole, ChangeError, ChangeErrorKind, IntentIdentity, NormativeInput, WorkGraph,
    WorkIdentity,
};
#[derive(Debug)]
pub struct ChangeIdentityAnalysis {
    pub intent: IntentIdentity,
    pub work: WorkIdentity,
    pub graph: WorkGraph,
}
impl SavedChange {
    /// Derive identity from retained bytes, never from current disk, evidence
    /// counts, closure output or authored work completion flags.
    pub fn identity_analysis(&self) -> Result<ChangeIdentityAnalysis, SavedChangeError> {
        let mut normative = Vec::new();
        let mut work_texts = Vec::new();
        for doc in self
            .manifest
            .documents()
            .iter()
            .filter(|d| d.input && (d.normative || d.role == ChangeDocumentRole::Work))
        {
            let path = self.manifest.directory().join(&doc.path);
            let mut matches = self.input_observations.iter().filter(|(p, _)| p == &path);
            let (_, snapshot) = matches
                .next()
                .ok_or_else(|| SavedChangeError::MissingDocument(doc.path.clone()))?;
            if matches.next().is_some() {
                return Err(SavedChangeError::Declaration(ChangeError {
                    kind: ChangeErrorKind::Input,
                    field: "saved.inputs".into(),
                    reason: "duplicate retained role observation",
                }));
            }
            if doc.normative {
                normative.push(NormativeInput {
                    path: &doc.path,
                    bytes: &snapshot.bytes,
                });
            }
            if doc.role == ChangeDocumentRole::Work {
                work_texts.push(
                    std::str::from_utf8(&snapshot.bytes)
                        .map_err(|_| SavedChangeError::Unavailable("work UTF8"))?,
                );
            }
        }
        let intent = self.manifest.intent_identity(&normative)?;
        let graph = WorkGraph::parse_documents(&work_texts)?;
        let work = graph.identity(&self.manifest);
        Ok(ChangeIdentityAnalysis {
            intent,
            work,
            graph,
        })
    }
}
