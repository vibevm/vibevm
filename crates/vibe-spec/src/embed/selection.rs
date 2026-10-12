//! Selected IR and explicit legacy section text at the incumbent facade.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#IR-SHARED-EXACT-SELECTION"
);

use super::*;

impl FsSectionSource {
    pub(super) fn classified_text(
        &self,
        addr: &SpecAddress,
        path: &Path,
    ) -> Result<Option<String>, String> {
        self.resolver
            .read_classified_node(
                addr,
                path,
                self.overlay.get(path).map(|bytes| bytes.as_ref()),
            )
            .map(|node| node.map(|node| vibe_specdoc::to_markdown(&node.ir)))
    }
}

pub(super) fn resolve_section(
    src: &str,
    file: &Path,
    addr: &SpecAddress,
) -> Result<String, String> {
    let tree = DocTree::parse(src);
    if tree
        .duplicate_anchors()
        .iter()
        .any(|id| addr.anchor.contains(id))
    {
        return Err(format!("ambiguous anchor in {}", file.display()));
    }
    let node = match tree.resolve_path(&addr.anchor) {
        Some(node) => node,
        None => {
            // B-011 §6.1: a missed short anchor reports qualified heirs.
            let candidates = addr
                .anchor
                .first()
                .map(|short| tree.qualified_candidates(short.as_str()))
                .filter(|c| !c.is_empty())
                .map(|c| {
                    format!(
                        " (qualified candidates for `{}`: {})",
                        addr.anchor.first().map(String::as_str).unwrap_or(""),
                        c.join(", ")
                    )
                })
                .unwrap_or_default();
            return Err(format!(
                "anchor not found in {}{candidates}",
                file.display()
            ));
        }
    };
    Ok(tree.text(node))
}

#[cfg(test)]
mod m_01_d_1_cases {
    use super::*;
    use crate::{DescriptorOutputRow, SelfCoordinate, ValidatedDescriptorMap};
    use sha2::{Digest, Sha256};
    use vibe_core::manifest::SourceInterpretation;

    fn fixture(
        suffix: &str,
        media: &str,
        interpretation: SourceInterpretation,
        text: &str,
        with_media: bool,
    ) -> (tempfile::TempDir, FsSectionSource) {
        let root = tempfile::tempdir().unwrap();
        let spec = root.path().join("spec");
        std::fs::create_dir(&spec).unwrap();
        let output = format!("spec/DOC.{suffix}");
        std::fs::write(root.path().join(&output), text).unwrap();
        let map = ValidatedDescriptorMap::from_witnessed_rows(
            root.path().to_path_buf(),
            spec.clone(),
            "observed-self".into(),
            "a".repeat(64),
            vec![DescriptorOutputRow {
                output: output.clone(),
                source: output.clone(),
                interpretation: Some(interpretation),
                output_sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
                document_address: Some("DOC".into()),
            }],
        )
        .unwrap();
        let map = if with_media {
            map.with_materialized_media(BTreeMap::from([(output, media.into())]))
                .unwrap()
        } else {
            map
        };
        let resolver = FileResolver::new(
            root.path(),
            SelfCoordinate::new(Some("org.demo".into()), "self".into()),
        )
        .with_descriptor_source_map(spec, Some(map))
        .unwrap();
        (root, FsSectionSource::new(resolver))
    }

    #[test]
    fn actual_fs_embed_table_and_list_match_shared_selected_ir_in_both_forms() {
        let markdown = "# D {#root}\n\n## Solution {#solution}\n\n- @fact:LIST List. @status:impl/done\n\n| Header | Value |\n| --- | --- |\n| @fact:CC-A-4-1 One. @status:impl/done | @fact:CC-A-4-2 Two. @status:impl/done |\n";
        let ir = vibe_specdoc::from_markdown(markdown).unwrap();
        let xml = vibe_specdoc::to_xml(&ir);
        for (suffix, media, source) in [
            ("md", "text/markdown", markdown),
            ("xml", "application/xml", xml.as_str()),
        ] {
            let (_root, reader) =
                fixture(suffix, media, SourceInterpretation::VibeSpec, source, true);
            for anchor in ["root.solution", "LIST", "CC-A-4-1", "CC-A-4-2"] {
                let address =
                    SpecAddress::parse(&format!("spec://org.demo/self/DOC#{anchor}")).unwrap();
                let selected = vibe_specdoc::selection::select_node(
                    &ir,
                    Some(anchor),
                    true,
                    vibe_specdoc::Vocabulary::Spec,
                )
                .unwrap();
                let read = reader.section_text(&address).unwrap();
                assert_eq!(read, vibe_specdoc::to_markdown(&selected.ir));
                let metadata = reader.source_metadata(&address, &[]).unwrap();
                assert_eq!(
                    reader
                        .read_resolved_source(&address, metadata)
                        .unwrap()
                        .into_parts()
                        .0,
                    read
                );
                let embedded =
                    crate::expand_embeds(&format!("#embed {address}\n"), &reader).unwrap();
                let key = address.without_pin();
                let body = embedded
                    .strip_prefix(&format!("<!-- embed: {key} -->\n"))
                    .unwrap()
                    .strip_suffix(&format!("<!-- /embed: {key} -->\n"))
                    .unwrap();
                assert_eq!(body, read);
                assert_eq!(vibe_specdoc::from_markdown(body).unwrap(), selected.ir);
            }
        }
    }

    #[test]
    fn declared_doc_xml_under_md_suffix_keeps_native_vocabulary_before_projection() {
        let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"solution\" title=\"Solution\"><p><p01 fact=\"true\">One.</p01></p><note kind=\"tip\">Tip.</note></section></spec>";
        let (_root, reader) = fixture(
            "md",
            "application/xml",
            SourceInterpretation::VibeDoc,
            xml,
            true,
        );
        let addr = SpecAddress::parse("spec://org.demo/self/DOC#root.solution").unwrap();
        let ir = vibe_specdoc::from_xml_with(xml, vibe_specdoc::Vocabulary::Doc).unwrap();
        let selected = vibe_specdoc::selection::select_node(
            &ir,
            Some("root.solution"),
            true,
            vibe_specdoc::Vocabulary::Doc,
        )
        .unwrap();
        assert_eq!(
            reader.section_text(&addr).unwrap(),
            vibe_specdoc::to_markdown(&selected.ir)
        );
        assert!(reader.section_text(&addr).unwrap().contains("Tip."));
    }

    #[test]
    fn missing_media_and_changed_output_refuse_before_semantic_selection() {
        let markdown = "# D {#root}\n\n@fact:ONE One. @status:impl/done\n";
        let (_root, reader) = fixture(
            "md",
            "text/markdown",
            SourceInterpretation::VibeSpec,
            markdown,
            false,
        );
        let addr = SpecAddress::parse("spec://org.demo/self/DOC#ONE").unwrap();
        assert!(reader.section_text(&addr).unwrap_err().contains("media"));
        let (root, reader) = fixture(
            "md",
            "text/markdown",
            SourceInterpretation::VibeSpec,
            markdown,
            true,
        );
        std::fs::write(
            root.path().join("spec/DOC.md"),
            markdown.replace("One.", "Changed."),
        )
        .unwrap();
        assert!(reader.section_text(&addr).unwrap_err().contains("changed"));
    }
}
