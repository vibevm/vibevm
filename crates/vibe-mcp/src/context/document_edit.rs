//! Trusted finite edit grant; read contexts stay read-only by default.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::ServerContext;
use std::sync::Arc;
use vibe_orchestrator::document_analysis::edit::{DocumentEditAuthority, EditError};
impl ServerContext {
    pub fn with_document_edit_authority(
        mut self,
        authority: Arc<DocumentEditAuthority>,
    ) -> Result<Self, EditError> {
        let read = self
            .document_analysis
            .as_ref()
            .ok_or(EditError::AuthorityRefused)?;
        if authority.root() != self.project_root || !authority.belongs_to(read) {
            return Err(EditError::AuthorityRefused);
        };
        self.document_edit_authority = Some(authority);
        Ok(self)
    }
    pub fn document_edits(&self) -> Result<Arc<DocumentEditAuthority>, EditError> {
        let authority = self
            .document_edit_authority
            .as_ref()
            .ok_or(EditError::AuthorityRefused)?;
        let read = self
            .document_analysis
            .as_ref()
            .ok_or(EditError::AuthorityRefused)?;
        if authority.root() != self.project_root || !authority.belongs_to(read) {
            return Err(EditError::AuthorityRefused);
        };
        Ok(authority.clone())
    }
}
#[cfg(test)]
mod m_16_d_3_cases {
    use super::*;
    use std::time::Duration;
    use vibe_orchestrator::document_analysis::edit::*;
    use vibe_orchestrator::document_analysis::*;
    #[test]
    fn explicit_context_grant_is_required_for_acknowledged_edit() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname='demo'\nversion='1.0.0'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("a.md"), b"before").unwrap();
        let host = DocumentAnalysisHost::new(
            root.path(),
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "a.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 4096,
                capture_bytes: 16384,
                open_buffers: 4,
            },
        )
        .unwrap();
        let id = host.resource("a.md").unwrap();
        let authority = DocumentEditAuthority::new(host.clone(), vec![id]).unwrap();
        let context = ServerContext::with_store_root(root.path(), root.path().join("unused-store"))
            .with_document_analysis(host)
            .unwrap();
        assert_eq!(
            context.document_edits().unwrap_err(),
            EditError::AuthorityRefused
        );
        assert!(!root.path().join(".vibe").exists());
        let context = context.with_document_edit_authority(authority).unwrap();
        let a = context.document_client().unwrap();
        let b = context.document_client().unwrap();
        let capture = a.freeze().unwrap();
        let plan = context
            .document_edits()
            .unwrap()
            .prepare(
                &capture,
                vec![DocumentReplacement {
                    resource: id,
                    bytes: Arc::from(b"after".as_slice()),
                }],
            )
            .unwrap();
        let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
        a.acknowledge_edit(&barrier, true).unwrap();
        b.acknowledge_edit(&barrier, true).unwrap();
        let outcome = barrier.apply().unwrap();
        assert_eq!(outcome.state, EditOutcomeState::Completed);
        assert_eq!(outcome.verified, vec![id]);
        assert_eq!(std::fs::read(root.path().join("a.md")).unwrap(), b"after");
        assert!(!root.path().join("unused-store").exists());
    }
}
