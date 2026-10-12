//! One generated source-query contract over the retained selected world.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#GENERATED-SOURCE-QUERY-CONTRACT");

use super::{CaptureOwner, SourceQueryUnavailable, StructuredView, WitnessedSourceWorld};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vibe_spec::{
    Authority, FileResolver, ResolveError, ResourceAddressError, SelfCoordinate, SpecAddress,
};
use vibe_specdoc::doc::{Block, SpecDoc, StatusEl, Vocabulary};
use vibe_specdoc::selection::SelectionError;
use vibe_wire::behaviour::source_query as wire;
use vibe_wire::generated::source_query::e1::{
    error::{Error as QueryError, ErrorCode},
    request::{Request, RequestFormat, RequestMode},
    result::Result as QueryResult,
};

mod chunk;
mod document;
mod preparation;
pub use document::{PreparedDocInstance, PreparedDocument};

#[derive(Debug)]
pub enum SourceQueryResponse {
    Success(QueryResult),
    Refused(QueryError),
}

impl SourceQueryResponse {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Refused(_))
    }
    pub fn into_value(self) -> Value {
        match self {
            Self::Success(result) => serde_json::to_value(result).expect("generated source result"),
            Self::Refused(error) => serde_json::to_value(error).expect("generated source refusal"),
        }
    }
}

#[derive(Debug)]
pub struct SourceQueryService {
    world: Arc<WitnessedSourceWorld>,
    resolver: FileResolver,
    own_identity: String,
}

impl SourceQueryService {
    /// Consumes existing sealed authority, never creates it from descriptors.
    pub fn from_world(world: Arc<WitnessedSourceWorld>) -> Result<Self, QueryError> {
        let prepared = &world.prepared;
        let own = prepared
            .captured
            .get(&CaptureOwner::SelfNode)
            .ok_or_else(|| unavailable("self capture"))?;
        let manifest = own.captured_manifest().map_err(world_error)?;
        let (coordinate, own_identity) = if let Some(package) = &manifest.package {
            (
                SelfCoordinate::new(Some(package.group.to_string()), package.name.clone()),
                format!("{}/{}", package.group, package.name),
            )
        } else if let Some(project) = &manifest.project {
            (
                SelfCoordinate::new(
                    project.group.as_ref().map(ToString::to_string),
                    project.name.clone(),
                ),
                project
                    .group
                    .as_ref()
                    .map(|group| format!("{group}/{}", project.name))
                    .unwrap_or_else(|| project.name.clone()),
            )
        } else {
            return Err(unavailable("node identity"));
        };
        let mut resolver = FileResolver::with_selected_world(
            prepared.node_root.clone(),
            world.workspace_root.clone(),
            prepared.selected.clone(),
            coordinate,
        );
        for capture in prepared.captured.values() {
            if let Some(map) = &capture.descriptor {
                resolver = resolver
                    .with_descriptor_source_map(map.spec_root().to_path_buf(), Some(map.clone()))
                    .map_err(|_| unavailable("prepared descriptor map"))?;
            }
        }
        Ok(Self {
            world,
            resolver,
            own_identity,
        })
    }

    /// Host composition prepares observations separately from request reading.
    pub fn prepare_host(root: &Path) -> Result<Self, QueryError> {
        preparation::prepare_host(root)
    }

    /// Invalid closed shape/epoch refuses before any host preparation/read.
    pub fn execute_json(root: &Path, value: &Value) -> SourceQueryResponse {
        let request = match wire::read_request(value) {
            Ok(request) => request,
            Err(boundary) => {
                return SourceQueryResponse::Refused(refusal(
                    if boundary == "epoch" {
                        ErrorCode::UnsupportedEpoch
                    } else {
                        ErrorCode::InvalidRequest
                    },
                    boundary,
                    "Source request does not match its registered contract",
                    vec![],
                ));
            }
        };
        match Self::prepare_host(root).and_then(|service| service.query(&request)) {
            Ok(result) => SourceQueryResponse::Success(result),
            Err(error) => SourceQueryResponse::Refused(error),
        }
    }

    pub fn query(&self, request: &Request) -> Result<QueryResult, QueryError> {
        wire::validate_request(request).map_err(|boundary| {
            refusal(
                ErrorCode::InvalidRequest,
                boundary,
                "Invalid source request",
                vec![],
            )
        })?;
        if request.mode != RequestMode::Read
            && (request.cursor.is_some() || request.chunk_bytes.is_some())
        {
            return Err(refusal(
                ErrorCode::InvalidRequest,
                "mode",
                "Chunk controls apply only to read",
                vec![],
            ));
        }
        self.world.revalidate_current().map_err(world_error)?;
        let canonical = self
            .resolver
            .canonical_vibe_resource(&request.address)
            .map_err(|error| self.address_error(error))?;
        let parsed = SpecAddress::parse(canonical.as_str()).map_err(|_| {
            refusal(
                ErrorCode::InvalidRequest,
                "address",
                "Invalid canonical address",
                vec![],
            )
        })?;
        let physical = self
            .resolver
            .resolve_file(&parsed)
            .map_err(|error| self.address_error(error.into()))?;
        let (key, output) = self.capture_for_path(&physical)?;
        let owner = match &key {
            CaptureOwner::SelfNode => None,
            CaptureOwner::Dependency { group, name } => Some((group.as_str(), name.as_str())),
        };
        let source = self
            .world
            .extract_source(owner, &output)
            .map_err(extraction_error)?;
        let (document, vocabulary) = match &source.view {
            StructuredView::Vibe {
                document,
                vocabulary,
            } => (document, *vocabulary),
            StructuredView::Unavailable(reason) => {
                return Err(refusal(
                    ErrorCode::StructuredUnavailable,
                    "structured view",
                    reason,
                    vec![],
                ));
            }
            _ => {
                return Err(refusal(
                    ErrorCode::UnsupportedInterpretation,
                    "descriptor",
                    "Selected source is not a Vibe document",
                    vec![],
                ));
            }
        };
        let anchor = (!parsed.anchor.is_empty()).then(|| parsed.anchor.join("."));
        let selected = vibe_specdoc::selection::select_node(
            document,
            anchor.as_deref(),
            request.children,
            vocabulary,
        )
        .map_err(node_error)?;
        let descriptor = source
            .provenance
            .descriptor
            .as_ref()
            .ok_or_else(|| unavailable("selected descriptor"))?;
        let is_xml = source.provenance.materialized_media.as_deref() == Some("application/xml");
        let native = std::str::from_utf8(&source.capture.bytes)
            .map_err(|_| unavailable("captured UTF-8"))?;
        let range = match selected.path.last().filter(|_| anchor.is_some()) {
            None => Some((
                1,
                u32::try_from(native.lines().count().max(1))
                    .map_err(|_| unavailable("native line bounds"))?,
            )),
            Some(id) if is_xml => vibe_specdoc::native_anchor_range_with(native, vocabulary, id)
                .map_err(|_| {
                    refusal(
                        ErrorCode::LocationUnavailable,
                        "native XML range",
                        "Native selected range is unavailable",
                        vec![],
                    )
                })?,
            Some(id) => specmap_core::mdspec::native_anchor_range(native, id).map_err(|_| {
                refusal(
                    ErrorCode::LocationUnavailable,
                    "native Markdown range",
                    "Native selected range is unavailable",
                    vec![],
                )
            })?,
        }
        .ok_or_else(|| {
            refusal(
                ErrorCode::LocationUnavailable,
                "native range",
                "Selected node has no exact native range",
                vec![],
            )
        })?;
        let serialize = |doc: &SpecDoc| match request.format {
            RequestFormat::Md => vibe_specdoc::to_markdown(doc),
            RequestFormat::Xml => vibe_specdoc::to_xml(doc),
        };
        let full = serialize(document);
        let fragment = serialize(&selected.ir);
        if full.len() > wire::MAX_CONTENT_BYTES as usize
            || fragment.len() > wire::MAX_CONTENT_BYTES as usize
        {
            return Err(refusal(
                ErrorCode::LimitExceeded,
                "representation bytes",
                "Selected representation exceeds its bound",
                vec![],
            ));
        }
        let resolution_hash = self.resolution_hash()?;
        let document_hash = format!("sha256:{}", source.capture.sha256);
        let source_hash = owner
            .and_then(|(group, name)| {
                self.world
                    .lock_rows
                    .get(&(group.to_owned(), name.to_owned()))
            })
            .map(|row| row.content_hash.to_string());
        for (expected, actual, boundary) in [
            (
                request.expected_resolution_hash.as_deref(),
                Some(resolution_hash.as_str()),
                "resolution expectation",
            ),
            (
                request.expected_document_hash.as_deref(),
                Some(document_hash.as_str()),
                "document expectation",
            ),
            (
                request.expected_source_hash.as_deref(),
                source_hash.as_deref(),
                "source expectation",
            ),
        ] {
            if expected.is_some() && expected != actual {
                return Err(refusal(
                    ErrorCode::StaleExpectation,
                    boundary,
                    "Expected identity differs from the retained observation",
                    vec![],
                ));
            }
        }
        let semantic_hash = match selected.path.last() {
            Some(id) => {
                vibe_specdoc::canonical_node::canonical_node_hash(document, id).map_err(|_| {
                    node_error(SelectionError::Ambiguous {
                        requested: id.clone(),
                        candidates: vec![id.clone()],
                    })
                })?
            }
            None => None,
        };
        let (content, chunk) = if request.mode == RequestMode::Read {
            chunk::read(
                &fragment,
                request,
                &resolution_hash,
                &document_hash,
                &source.capture.identity.observation_correlation_token(),
            )?
        } else {
            (
                String::new(),
                json!({"offset":0,"end":0,"total":0,"next_cursor":null}),
            )
        };
        let outline = if request.mode == RequestMode::Outline {
            vibe_specdoc::selection::outline(document,vocabulary).map_err(node_error)?.into_iter()
                .filter(|row| row.path.starts_with(&selected.path))
                .map(|row| json!({"kind":match row.kind {vibe_specdoc::selection::SelectedNodeKind::Document=>"document",vibe_specdoc::selection::SelectedNodeKind::Section=>"section",vibe_specdoc::selection::SelectedNodeKind::Fact=>"fact",vibe_specdoc::selection::SelectedNodeKind::DocumentationBlock=>"documentation-block"},"path":row.path,"heading_chain":row.heading_chain,"bytes_with_children":row.bytes_with_children,"bytes_without_children":row.bytes_without_children,"size_recipe":"canonical-markdown+specdoc-selection/1"})).collect::<Vec<_>>()
        } else {
            vec![]
        };
        let path = physical
            .strip_prefix(self.world.workspace_root())
            .map_err(|_| unavailable("selected location root"))?
            .to_string_lossy()
            .replace('\\', "/");
        let result = json!({
            "schema":1,"address":canonical.as_str(),"mode":request.mode,"format":request.format,
            "dictionary":if vocabulary==Vocabulary::Doc {"vibe-doc"} else {"vibe-spec"},
            "status":selected_status(&selected),"heading_chain":selected.heading_chain,
            "selected_bytes":fragment.len(),"document_bytes":full.len(),"content":content,"outline":outline,"chunk":chunk,
            "location":{"path":path,"line_start":range.0,"line_end":range.1,"basis":if is_xml {"native-xml"} else {"native-markdown"}},
            "observation":{"resolution_hash":resolution_hash,"source_hash":source_hash,"document_hash":document_hash,"semantic_hash":semantic_hash,
                "descriptor_identity":descriptor.identity,"manifest_hash":format!("sha256:{}",source.provenance.manifest_sha256),"captured_bytes":source.capture.bytes.len(),"encoding":"utf8",
                "representation_recipe":format!("{}+{}+{}",vibe_specdoc::CONVERTER_RECIPE,vibe_specdoc::selection::SELECTION_RECIPE,if request.format==RequestFormat::Md {"md"} else {"xml"}),
                "strength":match source.provenance.strength {super::SourceStrength::Observed=>"observed",super::SourceStrength::RecordConsistent=>"record-consistent",super::SourceStrength::SourceProven=>return Err(unavailable("verified source proof owner"))}}});
        let result =
            wire::read_result(&result).map_err(|_| unavailable("generated result invariant"))?;
        self.world.revalidate_current().map_err(world_error)?;
        Ok(result)
    }

    fn capture_for_path(&self, path: &Path) -> Result<(CaptureOwner, String), QueryError> {
        for (owner, capture) in &self.world.prepared.captured {
            if let Ok(relative) = path.strip_prefix(&capture.root) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                if capture.outputs.contains_key(&relative) {
                    return Ok((owner.clone(), relative));
                }
            }
        }
        Err(unavailable("selected output capture"))
    }

    fn resolution_hash(&self) -> Result<String, QueryError> {
        let rows = self.world.lock_rows.values().collect::<Vec<_>>();
        let membership = self
            .world
            .membership
            .iter()
            .map(|(id, _, snapshot)| (id, &snapshot.sha256))
            .collect::<Vec<_>>();
        let closure = self
            .world
            .closure_manifests
            .iter()
            .map(|(id, (_, snapshot))| (id, &snapshot.sha256))
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&(rows, membership, closure))
            .map_err(|_| unavailable("resolution witness encoding"))?;
        Ok(format!(
            "sha256-query-resolution/1:{:x}",
            Sha256::digest(bytes)
        ))
    }

    fn address_error(&self, error: ResourceAddressError) -> QueryError {
        use ResourceAddressError as R;
        let (code, message) = match error {
            R::UnselectedPackage(_)
            | R::Lookup(
                ResolveError::UnselectedPackage { .. } | ResolveError::PackageSlotNotFound(_),
            ) => (
                ErrorCode::UnknownPackage,
                "Package is outside the selected visible world",
            ),
            R::WrongSelectedVersion { .. }
            | R::Lookup(
                ResolveError::SelectedVersionMismatch { .. }
                | ResolveError::SelfCoordinateVersioned { .. },
            ) => (
                ErrorCode::WrongVersion,
                "Version differs from exact selected authority",
            ),
            R::Lookup(ResolveError::DocNotFound { .. }) => (
                ErrorCode::UnknownDocument,
                "No captured document has that exact identity",
            ),
            R::Lookup(
                ResolveError::AmbiguousDoc { .. }
                | ResolveError::PairCollision { .. }
                | ResolveError::ActiveFormatCollision { .. },
            ) => (
                ErrorCode::AmbiguousDocument,
                "Document identity is ambiguous",
            ),
            R::VibeSourceRequired => (
                ErrorCode::UnsupportedInterpretation,
                "Selected descriptor does not admit Vibe reading",
            ),
            R::Lookup(ResolveError::SourceUnstable { .. }) => (
                ErrorCode::StaleWorld,
                "Source changed during address resolution",
            ),
            R::Lookup(_) => (
                ErrorCode::UnavailableWorld,
                "Selected address observation is unavailable",
            ),
            _ => (
                ErrorCode::InvalidRequest,
                "Address does not match strict resource grammar",
            ),
        };
        let candidates = self
            .world
            .prepared
            .captured
            .iter()
            .flat_map(|(key, capture)| {
                let coordinate = match key {
                    CaptureOwner::SelfNode => self.own_identity.clone(),
                    CaptureOwner::Dependency { group, name } => format!("{group}/{name}"),
                };
                if code == ErrorCode::UnknownPackage {
                    vec![coordinate]
                } else {
                    capture
                        .outputs
                        .keys()
                        .filter(|file| capture.descriptor.as_ref().and_then(|map| map.output_row(file))
                            .is_some_and(|row| matches!(row.interpretation,
                                Some(vibe_core::manifest::SourceInterpretation::VibeSpec | vibe_core::manifest::SourceInterpretation::VibeDoc))))
                        .map(|file| format!("{coordinate}/{}", vibe_spec::canonical_doc_path(file)))
                        .collect()
                }
            })
            .collect();
        refusal(code, "selected address", message, candidates)
    }
}

pub(super) fn refusal(
    code: ErrorCode,
    boundary: &str,
    message: &str,
    mut candidates: Vec<String>,
) -> QueryError {
    candidates.sort();
    candidates.dedup();
    candidates.truncate(wire::MAX_CANDIDATES);
    let error = QueryError {
        schema: 1,
        code,
        boundary: boundary.chars().take(128).collect(),
        message: message.chars().take(1024).collect(),
        candidates,
    };
    debug_assert!(wire::validate_error(&error).is_ok());
    error
}
pub(super) fn unavailable(boundary: &str) -> QueryError {
    refusal(
        ErrorCode::UnavailableWorld,
        boundary,
        "Prepared selected source is unavailable",
        vec![],
    )
}
pub(super) fn world_error(error: SourceQueryUnavailable) -> QueryError {
    match error {
        SourceQueryUnavailable::StaleWorld(boundary) => refusal(
            ErrorCode::StaleWorld,
            boundary,
            "Prepared source world changed",
            vec![],
        ),
        SourceQueryUnavailable::WorldUnavailable(boundary) => unavailable(boundary),
        _ => unavailable("selected owner/capture continuity"),
    }
}
fn node_error(error: SelectionError) -> QueryError {
    match error {
        SelectionError::Unknown { candidates, .. } => refusal(
            ErrorCode::UnknownAnchor,
            "node selection",
            "Unknown exact node",
            candidates,
        ),
        SelectionError::Ambiguous { candidates, .. } => refusal(
            ErrorCode::AmbiguousAnchor,
            "node selection",
            "Ambiguous exact node",
            candidates,
        ),
        SelectionError::LimitExceeded(boundary) => refusal(
            ErrorCode::LimitExceeded,
            boundary,
            "Node selection exceeds its bound",
            vec![],
        ),
        SelectionError::InvalidPath(_) => refusal(
            ErrorCode::InvalidRequest,
            "node path",
            "Invalid node path",
            vec![],
        ),
    }
}
fn extraction_error(error: super::ExtractionError) -> QueryError {
    match error {
        super::ExtractionError::World(error) => world_error(error),
        super::ExtractionError::Node(error) => node_error(error),
        super::ExtractionError::VibeUnavailable(reason) => refusal(
            ErrorCode::StructuredUnavailable,
            "structured view",
            &reason,
            vec![],
        ),
        _ => unavailable("selected descriptor/extraction"),
    }
}
fn selected_status(selected: &vibe_specdoc::selection::NodeSelection) -> Option<String> {
    use vibe_specdoc::selection::SelectedNodeKind;
    let doc = &selected.ir;
    let status = match selected.kind {
        SelectedNodeKind::Document => doc.status.as_ref(),
        SelectedNodeKind::Section => doc
            .sections
            .first()
            .and_then(|section| section.status.as_ref()),
        SelectedNodeKind::Fact | SelectedNodeKind::DocumentationBlock => {
            doc.preamble.first().and_then(|node| match &node.block {
                Block::Paragraph(unit)
                | Block::Quote(unit)
                | Block::Note { body: unit, .. }
                | Block::Figure { caption: unit, .. } => {
                    unit.fact.as_ref().and_then(|fact| fact.status.as_ref())
                }
                Block::List { items, .. } => items
                    .first()
                    .and_then(|unit| unit.fact.as_ref())
                    .and_then(|fact| fact.status.as_ref()),
                Block::Table { rows } => rows
                    .first()
                    .and_then(|row| row.first())
                    .and_then(|unit| unit.fact.as_ref())
                    .and_then(|fact| fact.status.as_ref()),
                _ => None,
            })
        }
    };
    status.map(|status: &StatusEl| format!("{}/{}", status.stage, status.state))
}

#[cfg(test)]
mod m_01_d_2_cases;

#[cfg(test)]
mod m_01_e_2_cases;
