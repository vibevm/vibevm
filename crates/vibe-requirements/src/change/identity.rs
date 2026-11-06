//! Identity analysis uses the accepted saved observation set; no new reads.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-INDEPENDENT-IDENTITIES");
use super::{SavedChange, SavedChangeError};
use vibe_core::manifest::change::{
    ChangeDocumentRole, ChangeError, ChangeErrorKind, IntentIdentity, NormativeInput, WorkGraph,
    WorkIdentity,
};
/// Separate normative and structural identities over retained bytes only.
/// ```
/// # use std::path::Path;
/// # use vibe_requirements::change::*;
/// # let root=tempfile::tempdir().unwrap();
/// # let id="chg-0123456789abcdef0123456789abcdef";
/// # std::fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n").unwrap();
/// # let directory=root.path().join("docs/custom-specs/changes").join(id);
/// # std::fs::create_dir_all(&directory).unwrap();
/// # let text=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../formats/corpora/change-manifest/e1/valid/representative.toml"));
/// # std::fs::write(directory.join("change.toml"),text).unwrap();
/// # std::fs::write(directory.join("intent.md"),"Intent").unwrap();
/// # std::fs::write(directory.join("acceptance.md"),"Accept").unwrap();
/// # let saved=read_saved_change(root.path(),".",Some(Path::new("docs/custom-specs")),id).unwrap();
/// let analysis=saved.identity_analysis().unwrap();
/// assert_eq!(analysis.intent.recipe(),vibe_core::manifest::change::INTENT_RECIPE);
/// assert_eq!(analysis.graph.items().len(),0);
/// std::fs::write(directory.join("intent.md"),"Changed after read").unwrap();
/// assert_eq!(saved.identity_analysis().unwrap().intent,analysis.intent);
/// ```
#[derive(Debug)]
pub struct ChangeIdentityAnalysis {
    pub intent: IntentIdentity,
    pub work: WorkIdentity,
    pub graph: WorkGraph,
}
impl SavedChange {
    /// Derive identity from retained bytes, never from current disk, evidence
    /// counts, closure output or authored work completion flags.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-INDEPENDENT-IDENTITIES"
    )]
    pub fn identity_analysis(&self) -> Result<ChangeIdentityAnalysis, SavedChangeError> {
        let observations: Vec<_> = self
            .input_observations
            .iter()
            .map(|(path, snapshot)| (path.as_path(), snapshot.as_slice()))
            .collect();
        identity_from_observations(&self.manifest, &observations)
    }
}
pub(super) fn identity_from_observations(
    manifest: &vibe_core::manifest::change::ChangeManifest,
    observations: &[(&std::path::Path, &[u8])],
) -> Result<ChangeIdentityAnalysis, SavedChangeError> {
    let mut normative = Vec::new();
    let mut work_texts = Vec::new();
    for doc in manifest
        .documents()
        .iter()
        .filter(|d| d.input && (d.normative || d.role == ChangeDocumentRole::Work))
    {
        let path = manifest.directory().join(&doc.path);
        let mut matches = observations.iter().filter(|(p, _)| *p == path.as_path());
        let (_, bytes) = matches
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
                bytes,
            });
        }
        if doc.role == ChangeDocumentRole::Work {
            work_texts.push(
                std::str::from_utf8(bytes)
                    .map_err(|_| SavedChangeError::Unavailable("work UTF8"))?,
            );
        }
    }
    let intent = manifest.intent_identity(&normative)?;
    let graph = WorkGraph::parse_documents(&work_texts)?;
    let work = graph.identity(manifest)?;
    Ok(ChangeIdentityAnalysis {
        intent,
        work,
        graph,
    })
}
