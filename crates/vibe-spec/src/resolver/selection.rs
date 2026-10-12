//! Held descriptor-selected source to the shared SpecDoc node selector.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SHARED-SPECDOC-SELECTION");

use super::FileResolver;
use crate::SpecAddress;
use std::path::Path;
use vibe_core::manifest::SourceInterpretation;
use vibe_specdoc::doc::Vocabulary;
use vibe_specdoc::selection::NodeSelection;

impl FileResolver {
    /// None names only the explicit legacy no-descriptor route. A descriptor
    /// lacking media or a changed current output refuses before parsing.
    pub(crate) fn read_classified_node(
        &self,
        addr: &SpecAddress,
        path: &Path,
        overlay: Option<&[u8]>,
    ) -> Result<Option<NodeSelection>, String> {
        let root = self
            .spec_root(&addr.authority)
            .map_err(|error| error.to_string())?;
        let map = match self.descriptor_maps.get(&root) {
            None => return Ok(None),
            Some(Some(map)) => map,
            Some(None) => return Err("selected descriptor map is unavailable".into()),
        };
        let (interpretation, media, digest) = map
            .source_context(path)
            .map_err(|error| error.to_string())?;
        let held;
        let bytes = match overlay {
            Some(bytes) => bytes,
            None => {
                held = self
                    .read_observed_source(addr, path)
                    .map_err(|error| error.to_string())?;
                if held.sha256 != digest {
                    return Err("selected descriptor output changed before node selection".into());
                }
                &held.bytes
            }
        };
        let text = std::str::from_utf8(bytes)
            .map_err(|_| "selected Vibe source is not UTF-8".to_owned())?;
        let (doc, vocabulary) = match (interpretation, media) {
            (SourceInterpretation::VibeSpec, "text/markdown") => {
                (vibe_specdoc::from_markdown(text), Vocabulary::Spec)
            }
            (SourceInterpretation::VibeSpec, "application/xml") => {
                (vibe_specdoc::from_xml(text), Vocabulary::Spec)
            }
            (SourceInterpretation::VibeDoc, "application/xml") => (
                vibe_specdoc::from_xml_with(text, Vocabulary::Doc),
                Vocabulary::Doc,
            ),
            _ => return Err("selected Vibe media has no compatible node reader".into()),
        };
        let doc = doc.map_err(|error| format!("{}: {error}", path.display()))?;
        let anchor = (!addr.anchor.is_empty()).then(|| addr.anchor.join("."));
        vibe_specdoc::selection::select_node(&doc, anchor.as_deref(), true, vocabulary)
            .map(Some)
            .map_err(|error| error.to_string())
    }
}
