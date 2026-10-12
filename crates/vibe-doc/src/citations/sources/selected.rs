//! Explicit selected composition; legacy four-source callers stay separate.
use super::*;
use std::sync::Arc;
use vibe_workspace::source_query::SourceQueryService;

impl SpecSources {
    pub fn with_selected_service(mut self, service: Arc<SourceQueryService>) -> Self {
        self.selected = Some(service);
        self
    }

    pub(crate) fn has_selected_service(&self) -> bool {
        self.selected.is_some()
    }

    pub(crate) fn selected_manifest_at(&self, root: &Path) -> Result<Option<String>, String> {
        self.selected
            .as_ref()
            .map(|service| {
                service
                    .captured_manifest_at(root)
                    .map_err(|error| format!("{} [{}]", error.message, error.boundary))
            })
            .transpose()
    }

    pub(super) fn selected_instance(&self, group: &str, name: &str) -> Option<Instance> {
        let retained = self.selected.as_ref()?.selected_doc_instance(group, name)?;
        Some(Instance {
            source: if retained.is_self {
                Source::Checkout
            } else {
                Source::Lock
            },
            version: retained.version,
            root: retained.root,
            source_hash: retained.source_hash,
            in_place: retained.in_place,
        })
    }

    pub(super) fn selected_locate(&self, address: &SpecAddress) -> Result<Located, String> {
        let selected = self.selected.as_ref().expect("selected composition");
        let document = selected.captured_document(&address.raw).map_err(|error| {
            format!(
                "selected documentation: {} [{}]",
                error.message, error.boundary
            )
        })?;
        Ok(Located {
            path: document.path,
            source: if document.instance.is_self {
                Source::Checkout
            } else {
                Source::Lock
            },
            lang: document.instance.lang,
            captured: Some(CapturedSource {
                text: document.text,
                media: if document.xml {
                    CapturedMedia::Xml
                } else {
                    CapturedMedia::Markdown
                },
                vocabulary: document.vocabulary,
            }),
        })
    }
}
