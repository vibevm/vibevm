//! Pure configuration/navigation/boot preview over one frozen capture.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-RAW-AND-PREVIEW");
use super::*;
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, select_source_rows};
use vibe_resolver::{DepSolver, FeatureExpansion, FeatureRequest, ResolvedGraph, ResolvoDepSolver};
use vibe_specdoc::captured::{
    CapturedInspection, CapturedSyntax, StructuredView, inspect_captured,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisDiagnostic {
    pub resource: ResourceId,
    pub capture: String,
    pub source_revision: Result<SourceRevision, LocationError>,
    pub code: &'static str,
    pub message: String,
    pub basis: vibe_specdoc::captured::PositionBasis,
}
#[derive(Debug)]
pub struct DocumentPreview<'a> {
    pub resource: ResourceId,
    pub capture: &'a str,
    pub source_revision: SourceRevision,
    pub inspection: CapturedInspection<'a>,
}
#[derive(Debug)]
pub struct CapturedOutline<'a> {
    pub source_revision: SourceRevision,
    pub projection_digest: String,
    pub resource: ResourceId,
    pub capture: &'a str,
    pub basis: vibe_specdoc::captured::PositionBasis,
    pub nodes: Vec<vibe_specdoc::selection::OutlineNode>,
}
#[derive(Debug)]
pub struct CapturedSelection<'a> {
    pub source_revision: SourceRevision,
    pub projection_digest: String,
    pub resource: ResourceId,
    pub capture: &'a str,
    pub basis: vibe_specdoc::captured::PositionBasis,
    pub node: vibe_specdoc::selection::NodeSelection,
}
#[derive(Debug)]
pub struct ConfigurationPreview {
    pub capture: String,
    pub coordinate: Option<String>,
    pub features: Option<FeatureExpansion>,
    pub dependencies: Option<ResolvedGraph>,
    pub boot: Option<vibe_workspace::boot::EffectiveBoot>,
    pub compiled_boot: Option<Vec<u8>>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub world_input_digests: Vec<(ResourceId, String)>,
}
impl AnalysisCapture {
    pub(super) fn diagnostic(
        &self,
        resource: ResourceId,
        code: &'static str,
        message: impl Into<String>,
    ) -> AnalysisDiagnostic {
        AnalysisDiagnostic {
            resource,
            capture: self.id.clone(),
            source_revision: self.source_revision(resource),
            code,
            message: message.into(),
            basis: vibe_specdoc::captured::PositionBasis::OriginalBytes,
        }
    }
    pub fn manifest(&self, id: ResourceId) -> Result<Manifest, AnalysisDiagnostic> {
        let resource = self
            .resources
            .get(&id)
            .ok_or_else(|| self.diagnostic(id, "unknown-resource", "resource not in capture"))?;
        if resource.declaration.role != ResourceRole::Manifest {
            return Err(self.diagnostic(id, "not-manifest", "resource has no manifest role"));
        }
        if resource.encoding == ByteEncoding::Opaque {
            return Err(self.diagnostic(
                id,
                "encoding-unavailable",
                "manifest encoding unavailable; raw retained",
            ));
        }
        let raw = resource
            .raw()
            .ok_or_else(|| self.diagnostic(id, "missing-manifest", "manifest absent in capture"))?;
        let text = std::str::from_utf8(raw)
            .map_err(|_| self.diagnostic(id, "invalid-encoding", "manifest is not UTF8"))?;
        Manifest::parse_str(text).map_err(|e| self.diagnostic(id, "manifest-syntax", e.to_string()))
    }
    pub(super) fn syntax(&self, id: ResourceId) -> Result<CapturedSyntax, AnalysisDiagnostic> {
        let resource = self
            .resources
            .get(&id)
            .ok_or_else(|| self.diagnostic(id, "unknown-resource", "resource not in capture"))?;
        let ResourceRole::Source { manifest } = &resource.declaration.role else {
            return Ok(CapturedSyntax::Opaque);
        };
        let owner = self
            .resources
            .iter()
            .find(|(_, r)| r.path() == manifest)
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                self.diagnostic(id, "missing-source-owner", "source owner not captured")
            })?;
        let parsed = self.manifest(owner)?;
        let coordinate = parsed
            .consumer_node()
            .map(|n| n.coordinate())
            .ok_or_else(|| {
                self.diagnostic(
                    owner,
                    "no-coordinate",
                    "source owner has no consumer identity",
                )
            })?;
        let rows = if let Some(p) = &parsed.package {
            &p.sources
        } else if let Some(p) = &parsed.project {
            &p.sources
        } else {
            return Ok(CapturedSyntax::Opaque);
        };
        let parent = Path::new(manifest).parent().unwrap_or(Path::new(""));
        let relative = Path::new(resource.path())
            .strip_prefix(parent)
            .ok()
            .and_then(Path::to_str)
            .ok_or_else(|| {
                self.diagnostic(
                    id,
                    "source-path",
                    "source does not belong to captured owner",
                )
            })?;
        let selected = select_source_rows(&coordinate, rows, relative)
            .map_err(|_| self.diagnostic(id, "source-path", "source path refused"))?;
        match selected {
            SelectedSource::Unclassified { .. } => Ok(CapturedSyntax::Opaque),
            SelectedSource::NeedsInput { .. } => Err(self.diagnostic(
                id,
                "ambiguous-descriptor",
                "source descriptor selection needs input",
            )),
            SelectedSource::Descriptor { descriptor, .. } => {
                Ok(match descriptor.interpretation() {
                    SourceInterpretation::VibeSpec if descriptor.media() == "text/markdown" => {
                        CapturedSyntax::VibeMarkdown
                    }
                    SourceInterpretation::VibeSpec => {
                        CapturedSyntax::VibeXml(vibe_specdoc::Vocabulary::Spec)
                    }
                    SourceInterpretation::VibeDoc => {
                        CapturedSyntax::VibeXml(vibe_specdoc::Vocabulary::Doc)
                    }
                    SourceInterpretation::Commonmark => CapturedSyntax::Commonmark,
                    SourceInterpretation::XmlData => CapturedSyntax::XmlData,
                    SourceInterpretation::Opaque => CapturedSyntax::Opaque,
                })
            }
        }
    }
    pub fn inspect(&self, id: ResourceId) -> Result<DocumentPreview<'_>, AnalysisDiagnostic> {
        let resource = self
            .resources
            .get(&id)
            .ok_or_else(|| self.diagnostic(id, "unknown-resource", "resource not in capture"))?;
        let raw = resource.raw().ok_or_else(|| {
            self.diagnostic(
                id,
                "missing-resource",
                "resource absent or moved in capture",
            )
        })?;
        if resource.encoding == ByteEncoding::Opaque {
            return Ok(DocumentPreview {
                resource: id,
                capture: &self.id,
                source_revision: self
                    .source_revision(id)
                    .expect("validated present resource"),
                inspection: inspect_captured(raw, CapturedSyntax::Opaque),
            });
        }
        let syntax = self.syntax(id)?;
        Ok(DocumentPreview {
            resource: id,
            capture: &self.id,
            source_revision: self
                .source_revision(id)
                .expect("validated present resource"),
            inspection: inspect_captured(raw, syntax),
        })
    }
    pub fn outline(&self, id: ResourceId) -> Result<CapturedOutline<'_>, AnalysisDiagnostic> {
        let preview = self.inspect(id)?;
        let Some(StructuredView::Vibe { ir, vocabulary }) = preview.inspection.view else {
            return Err(self.diagnostic(
                id,
                "structured-view-unavailable",
                "no selected Vibe view; raw remains available",
            ));
        };
        let nodes = vibe_specdoc::selection::outline(&ir, vocabulary)
            .map_err(|e| self.diagnostic(id, "node-selection", e.to_string()))?;
        Ok(CapturedOutline {
            source_revision: preview.source_revision,
            projection_digest: super::digest(vibe_specdoc::to_markdown(&ir).as_bytes()),
            resource: id,
            capture: &self.id,
            basis: vibe_specdoc::captured::PositionBasis::MarkdownProjection(
                "vibe-specdoc-to-markdown/1",
            ),
            nodes,
        })
    }
    pub fn select(
        &self,
        id: ResourceId,
        path: Option<&str>,
        children: bool,
    ) -> Result<CapturedSelection<'_>, AnalysisDiagnostic> {
        let preview = self.inspect(id)?;
        let Some(StructuredView::Vibe { ir, vocabulary }) = preview.inspection.view else {
            return Err(self.diagnostic(
                id,
                "structured-view-unavailable",
                "no selected Vibe view; raw remains available",
            ));
        };
        let projection_digest = super::digest(vibe_specdoc::to_markdown(&ir).as_bytes());
        let node = vibe_specdoc::selection::select_node(&ir, path, children, vocabulary)
            .map_err(|e| self.diagnostic(id, "node-selection", e.to_string()))?;
        Ok(CapturedSelection {
            source_revision: preview.source_revision,
            projection_digest,
            resource: id,
            capture: &self.id,
            basis: vibe_specdoc::captured::PositionBasis::MarkdownProjection(
                "vibe-specdoc-to-markdown/1",
            ),
            node,
        })
    }
    pub fn configuration_preview(&self, requested: &FeatureRequest) -> ConfigurationPreview {
        let mut result = ConfigurationPreview {
            capture: self.id.clone(),
            coordinate: None,
            features: None,
            dependencies: None,
            boot: None,
            compiled_boot: None,
            diagnostics: vec![],
            world_input_digests: vec![],
        };
        for (id, r) in &self.resources {
            if matches!(
                r.declaration.role,
                ResourceRole::Manifest | ResourceRole::Lockfile
            ) {
                if let Some(hash) = r.content_digest() {
                    result.world_input_digests.push((*id, hash));
                }
            }
            if r.version.is_some() && r.disk != r.underlying {
                result.diagnostics.push(self.diagnostic(
                    *id,
                    "underlying-disk-changed",
                    "buffer retains its original disk base; current captured disk differs",
                ));
            }
            if r.declaration.role == ResourceRole::Lockfile {
                if let Some(raw) = r.raw() {
                    let parsed = std::str::from_utf8(raw).ok().and_then(|text| {
                        toml::from_str::<vibe_core::manifest::Lockfile>(text).ok()
                    });
                    if parsed.as_ref().is_none_or(|l| {
                        l.meta.schema_version != vibe_core::manifest::CURRENT_SCHEMA_VERSION
                    }) {
                        result.diagnostics.push(self.diagnostic(
                            *id,
                            "world-lock-invalid",
                            "captured lock has unsupported syntax or epoch",
                        ));
                        return result;
                    }
                }
            }
        }
        let primary = match self.manifest(self.primary) {
            Ok(m) => m,
            Err(e) => {
                result.diagnostics.push(e);
                return result;
            }
        };
        result.coordinate = primary.consumer_node().map(|n| n.coordinate());
        result.features =
            match vibe_resolver::features::expand_manifest_features(&primary, requested) {
                Ok(f) => Some(f),
                Err(e) => {
                    result.diagnostics.push(self.diagnostic(
                        self.primary,
                        "feature-expansion",
                        e.to_string(),
                    ));
                    None
                }
            };
        if result.features.is_none() {
            return result;
        }
        let mut catalog = super::catalog::CapturedCatalog::default();
        for (id, resource) in &self.resources {
            if resource.declaration.role == ResourceRole::Manifest && *id != self.primary {
                if let Ok(manifest) = self.manifest(*id) {
                    if let Err(e) = catalog.insert(manifest) {
                        result
                            .diagnostics
                            .push(self.diagnostic(*id, "ambiguous-catalog", e));
                        return result;
                    }
                }
            }
        }
        if !primary.requires.path_packages.is_empty()
            || !primary.requires.git_packages.is_empty()
            || !primary.requires.var_packages.is_empty()
        {
            result.diagnostics.push(self.diagnostic(
                self.primary,
                "captured-resolution-unavailable",
                "path/git/variable source selection needs an explicit captured resolution",
            ));
            return result;
        }
        let roots = primary.requires.packages.clone();
        let graph = match ResolvoDepSolver::new(catalog).solve(&roots) {
            Ok(g) => g,
            Err(e) => {
                result.diagnostics.push(self.diagnostic(
                    self.primary,
                    "captured-resolution",
                    e.to_string(),
                ));
                return result;
            }
        };
        result.dependencies = Some(graph.clone());
        self.boot_preview(&primary, &graph, &mut result);
        result
    }
}

#[cfg(test)]
mod m_05_b_1_correction_cases {
    use super::*;

    fn preview(body: &str) -> ConfigurationPreview {
        let root = tempfile::tempdir().unwrap();
        let text = format!(
            "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n{body}"
        );
        std::fs::write(root.path().join("vibe.toml"), &text).unwrap();
        let host = DocumentAnalysisHost::new(
            root.path(),
            "vibe.toml",
            vec![ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            }],
            AnalysisLimits {
                resource_bytes: 1024 * 1024,
                capture_bytes: 2 * 1024 * 1024,
                open_buffers: 1,
            },
        )
        .unwrap();
        let client = host.client().unwrap();
        let capture = client.freeze().unwrap();
        // Prove the preview consumes frozen input, not a new manifest read.
        std::fs::write(root.path().join("vibe.toml"), "poison after capture").unwrap();
        let result = capture.configuration_preview(&FeatureRequest::default());
        assert_eq!(
            std::fs::read_to_string(root.path().join("vibe.toml")).unwrap(),
            "poison after capture"
        );
        result
    }

    #[test]
    fn undeclared_default_activation_refuses() {
        let result = preview("[features]\ndefault=['dep:org.example/missing']");
        assert!(result.features.is_none());
        assert!(result.dependencies.is_none());
        assert!(result.diagnostics.iter().any(|d| {
            d.code == "feature-expansion"
                && d.message
                    .contains("undeclared dependency `org.example/missing`")
        }));
    }

    #[test]
    fn dormant_undeclared_weak_reference_refuses() {
        let result = preview("[features]\n_private=['org.example/missing?/fast']");
        assert!(result.features.is_none());
        assert!(result.dependencies.is_none());
        assert!(result.diagnostics.iter().any(|d| {
            d.code == "feature-expansion"
                && d.message
                    .contains("undeclared dependency `org.example/missing`")
        }));
    }

    #[test]
    fn required_dependency_enables_weak_request() {
        let result = preview(
            "[requires.packages]\n'org.example/dep'='^1'\n[features]\ndefault=['org.example/dep?/fast']",
        );
        let expansion = result.features.unwrap();
        assert!(expansion.active_deps.contains("org.example/dep"));
        assert!(expansion.dep_features["org.example/dep"].contains("fast"));
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code == "feature-expansion")
        );
        // The bounded capture has no dependency catalog; feature evaluation
        // succeeds before the separate resolution stage reports that absence.
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "captured-resolution")
        );
    }
    #[test]
    fn unavailable_export_catalogue_preserves_legacy_subskill_selector() {
        let result = preview("[features]\ndefault=['a']\na=['subskill:stack/rust']");
        assert!(
            result
                .features
                .unwrap()
                .active_subskills
                .contains("stack/rust")
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code == "feature-expansion")
        );
    }
}
