//! One private root/capture primitive shared by both query families.
use crate::document_analysis::{
    AnalysisCapture, AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole,
};
use std::{path::Path, sync::Arc};
use vibe_workspace::source_query::SourceQueryService;

pub(super) struct CapturedHost {
    pub(super) host: Arc<DocumentAnalysisHost>,
    capture: AnalysisCapture,
    pub(super) context: String,
}
impl CapturedHost {
    pub(super) fn new(root: &Path) -> anyhow::Result<Self> {
        let host = DocumentAnalysisHost::new(
            root,
            "vibe.toml",
            vec![ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            }],
            AnalysisLimits {
                resource_bytes: 1024 * 1024,
                capture_bytes: 1024 * 1024,
                open_buffers: 1,
            },
        )?;
        let capture = host.client()?.freeze()?;
        let context = format!("adapter-context:{}", capture.context());
        Ok(Self {
            host,
            capture,
            context,
        })
    }
    pub(super) fn prepare(&self) -> anyhow::Result<SourceQueryService> {
        let source = SourceQueryService::prepare_host(self.host.root())
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let manifest = source
            .captured_manifest_at(self.host.root())
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let resource = self.host.resource("vibe.toml").expect("host declaration");
        anyhow::ensure!(
            self.capture.resource(resource).and_then(|r| r.raw()) == Some(manifest.as_bytes()),
            "host manifest changed before preparation"
        );
        source.revalidate().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(source)
    }
}
pub(super) fn selected_manifest(
    source: &SourceQueryService,
    group: &str,
    name: &str,
    root: &Path,
    version: &str,
    expected_self: bool,
) -> anyhow::Result<String> {
    source.revalidate().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let selected = source
        .selected_doc_instance(group, name)
        .ok_or_else(|| anyhow::anyhow!("provider outside selected source world"))?;
    anyhow::ensure!(
        selected.root == root
            && (expected_self || selected.version == version)
            && selected.is_self == expected_self,
        "selected provider mismatch"
    );
    let text = source
        .captured_manifest_at(root)
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let manifest = vibe_core::manifest::Manifest::parse_str(&text)?;
    let matches = manifest.package.as_ref().is_some_and(|p| {
        p.group.as_str() == group && p.name == name && p.version.to_string() == version
    }) || manifest.project.as_ref().is_some_and(|p| {
        p.group.as_ref().is_some_and(|g| g.as_str() == group)
            && p.name == name
            && p.version.to_string() == version
    });
    anyhow::ensure!(matches, "captured provider coordinate/version mismatch");
    source.revalidate().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(text)
}
