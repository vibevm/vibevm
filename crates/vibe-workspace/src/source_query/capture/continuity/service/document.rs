//! The actual documentation locator consumes this same captured resolver.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-DOC-COMPATIBILITY");

use super::*;
use vibe_core::{ContentHash, manifest::Materialization};

#[derive(Debug, Clone)]
pub struct PreparedDocInstance {
    pub root: PathBuf,
    pub version: String,
    pub source_hash: Option<ContentHash>,
    pub in_place: bool,
    pub is_self: bool,
    pub lang: String,
}

#[derive(Debug, Clone)]
pub struct PreparedDocument {
    pub canonical_address: String,
    pub path: PathBuf,
    pub text: String,
    pub xml: bool,
    pub vocabulary: Vocabulary,
    pub instance: PreparedDocInstance,
}

impl SourceQueryService {
    pub fn revalidate(&self) -> Result<(), QueryError> {
        self.world
            .revalidate_current()
            .map(|_| ())
            .map_err(world_error)
    }

    /// Exact whole-document raw bytes from this retained selected capture.
    /// No semantic reconstruction or new source authority is implied.
    pub fn captured_bytes(&self, address: &str) -> Result<Vec<u8>, QueryError> {
        self.revalidate()?;
        let canonical = self
            .resolver
            .canonical_vibe_resource(address)
            .map_err(|error| self.address_error(error))?;
        let parsed = SpecAddress::parse(canonical.as_str())
            .map_err(|_| unavailable("canonical raw document address"))?;
        if !parsed.anchor.is_empty() {
            return Err(refusal(
                ErrorCode::InvalidRequest,
                "raw document selection",
                "Raw capture reads require a whole-document address",
                vec![],
            ));
        }
        let path = self
            .resolver
            .resolve_file(&parsed)
            .map_err(|error| self.address_error(error.into()))?;
        let (owner, output) = self.capture_for_path(&path)?;
        let capture = &self.world.prepared.captured[&owner];
        let bytes = capture
            .outputs
            .get(&output)
            .ok_or_else(|| unavailable("selected raw capture"))?
            .bytes
            .clone();
        self.revalidate()?;
        Ok(bytes)
    }

    /// Only a retained visible owner may supply manifest metadata. The path
    /// is compared as identity data; this does not read or admit another root.
    pub fn captured_manifest_at(&self, root: &Path) -> Result<String, QueryError> {
        self.revalidate()?;
        let capture = self
            .world
            .prepared
            .captured
            .values()
            .find(|capture| capture.root == root)
            .ok_or_else(|| unavailable("selected documentation manifest"))?;
        let text = std::str::from_utf8(&capture.manifest.bytes)
            .map_err(|_| unavailable("captured manifest UTF-8"))?
            .to_owned();
        self.revalidate()?;
        Ok(text)
    }

    /// This is already retained metadata, never in-tree/store/newest lookup.
    pub fn selected_doc_instance(&self, group: &str, name: &str) -> Option<PreparedDocInstance> {
        for (owner, capture) in &self.world.prepared.captured {
            let manifest = capture.captured_manifest().ok()?;
            let is_self = matches!(owner, CaptureOwner::SelfNode);
            let matches = match owner {
                CaptureOwner::SelfNode => {
                    manifest.package.as_ref().is_some_and(|package| {
                        package.group.as_str() == group && package.name == name
                    }) || manifest.project.as_ref().is_some_and(|project| {
                        project
                            .group
                            .as_ref()
                            .is_some_and(|own| own.as_str() == group)
                            && project.name == name
                    })
                }
                CaptureOwner::Dependency {
                    group: own,
                    name: own_name,
                } => own == group && own_name == name,
            };
            if !matches {
                continue;
            }
            let selected = self
                .world
                .lock_rows
                .get(&(group.to_owned(), name.to_owned()));
            let value = serde_json::to_value(&manifest).ok()?;
            let lang = value
                .get("i18n")
                .and_then(|table| table.get("canonical"))
                .and_then(Value::as_str)
                .unwrap_or("en")
                .to_owned();
            return Some(PreparedDocInstance {
                root: capture.root.clone(),
                version: if is_self {
                    String::new()
                } else {
                    selected?.version.to_string()
                },
                source_hash: if is_self {
                    None
                } else {
                    Some(selected?.content_hash.clone())
                },
                in_place: selected
                    .is_some_and(|row| row.materialization == Materialization::InPlace),
                is_self,
                lang,
            });
        }
        None
    }

    /// Whole-page compatibility ignores an anchor, never a revision/version.
    /// Held bytes/media/vocabulary survive through the existing page pipeline.
    pub fn captured_document(&self, address: &str) -> Result<PreparedDocument, QueryError> {
        self.revalidate()?;
        let parsed = SpecAddress::parse(address).map_err(|_| {
            refusal(
                ErrorCode::InvalidRequest,
                "documentation address",
                "Invalid documentation address",
                vec![],
            )
        })?;
        if parsed.pinned_r.is_some() {
            return Err(refusal(
                ErrorCode::InvalidRequest,
                "unsupported revision",
                "Revision-pinned captured source is unavailable",
                vec![],
            ));
        }
        let (group, name) = match &parsed.authority {
            Authority::Package { group, name, .. } => (group.as_str(), name.as_str()),
            _ => {
                return Err(refusal(
                    ErrorCode::InvalidRequest,
                    "documentation authority",
                    "Qualified selected authority required",
                    vec![],
                ));
            }
        };
        let whole = address.split('#').next().unwrap_or(address);
        let canonical = self
            .resolver
            .canonical_vibe_resource(whole)
            .map_err(|error| self.address_error(error))?;
        let whole = SpecAddress::parse(canonical.as_str())
            .map_err(|_| unavailable("canonical documentation address"))?;
        let path = self
            .resolver
            .resolve_file(&whole)
            .map_err(|error| self.address_error(error.into()))?;
        let (key, output) = self.capture_for_path(&path)?;
        let owner = match &key {
            CaptureOwner::SelfNode => None,
            CaptureOwner::Dependency { group, name } => Some((group.as_str(), name.as_str())),
        };
        let source = self
            .world
            .extract_source(owner, &output)
            .map_err(extraction_error)?;
        let vocabulary = match source.view {
            StructuredView::Vibe { vocabulary, .. } => vocabulary,
            StructuredView::Unavailable(reason) => {
                return Err(refusal(
                    ErrorCode::StructuredUnavailable,
                    "documentation view",
                    &reason,
                    vec![],
                ));
            }
            _ => {
                return Err(refusal(
                    ErrorCode::UnsupportedInterpretation,
                    "documentation descriptor",
                    "Source has no Vibe documentation view",
                    vec![],
                ));
            }
        };
        let text = std::str::from_utf8(&source.capture.bytes)
            .map_err(|_| unavailable("captured documentation UTF-8"))?
            .to_owned();
        let instance = self
            .selected_doc_instance(group, name)
            .ok_or_else(|| unavailable("selected documentation instance"))?;
        let xml = source.provenance.materialized_media.as_deref() == Some("application/xml");
        self.revalidate()?;
        Ok(PreparedDocument {
            canonical_address: canonical.as_str().to_owned(),
            path,
            text,
            xml,
            vocabulary,
            instance,
        })
    }
}
