use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::Workspace;
use crate::extension_world::ExtensionWorldEpoch;
use crate::source_query::{PreparedOwnerCapture, PreparedWorldWitness};
use vibe_core::manifest::Manifest;
use vibe_safefs::Project;

fn fixture(rows: &[(&str, &str, &str, &[u8])]) -> (tempfile::TempDir, WitnessedSourceWorld) {
    fixture_with_extra(rows, "")
}

fn fixture_with_extra(
    rows: &[(&str, &str, &str, &[u8])],
    extra: &str,
) -> (tempfile::TempDir, WitnessedSourceWorld) {
    let root = tempfile::tempdir().unwrap();
    let mut authored =
        "[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n".to_owned();
    for (file, media, interpretation, bytes) in rows {
        let role = if *interpretation == "vibe-spec" {
            "spec"
        } else if *interpretation == "vibe-doc" {
            "documentation"
        } else {
            "resource"
        };
        authored.push_str(&format!("\n[[package.sources]]\nfile='{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"));
        let path = root.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    authored.push_str(extra);
    fs::write(root.path().join("vibe.toml"), &authored).unwrap();
    let project = Project::open(root.path()).unwrap();
    let snapshot = |file: &str| {
        project
            .read_file_snapshot_bounded(file, 1024 * 1024)
            .unwrap()
            .unwrap()
    };
    let manifest = snapshot("vibe.toml");
    let capture = PreparedOwnerCapture::self_node(
        root.path().to_path_buf(),
        manifest.clone(),
        rows.iter()
            .map(|(file, ..)| ((*file).into(), snapshot(file)))
            .collect(),
        None,
    );
    let workspace = Workspace {
        root: root.path().to_path_buf(),
        root_manifest: Manifest::parse_str(&authored).unwrap(),
        members: vec![],
    };
    let world = WitnessedSourceWorld::from_prepared_node(
        &workspace,
        ".",
        &ExtensionWorldEpoch::empty(),
        &BTreeSet::new(),
        vec![capture],
        PreparedWorldWitness::new(
            None,
            BTreeMap::from([(".".into(), manifest)]),
            BTreeMap::new(),
        ),
    )
    .unwrap();
    (root, world)
}

#[test]
fn actual_compiler_embed_and_read_spec_share_section_list_and_table_ir() {
    use vibe_core::manifest::SourceInterpretation;
    use vibe_spec::{
        DescriptorOutputRow, FileResolver, FsSectionSource, SelfCoordinate, SpecAddress,
        ValidatedDescriptorMap,
    };
    let md = "# D {#root}\n\n## Solution {#solution}\n\n- @fact:LIST List. @status:impl/done\n\n| Header | Value |\n| --- | --- |\n| @fact:CC-A-4-1 One. @status:impl/done | @fact:CC-A-4-2 Two. @status:impl/done |\n";
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap());
    for (suffix, media, body) in [
        ("md", "text/markdown", md),
        ("xml", "application/xml", xml.as_str()),
    ] {
        for anchor in ["root.solution", "LIST", "CC-A-4-1", "CC-A-4-2"] {
            let path = format!("spec/DOC.{suffix}");
            let entry = format!("# Entry {{#entry}}\n\n#embed spec://org.demo/self/DOC#{anchor}\n");
            let (root, world) = fixture(&[
                (&path, media, "vibe-spec", body.as_bytes()),
                (
                    "spec/ENTRY.md",
                    "text/markdown",
                    "vibe-spec",
                    entry.as_bytes(),
                ),
            ]);
            let mut rows = Vec::new();
            let mut media_rows = BTreeMap::new();
            let mut manifest_sha = String::new();
            for output in [path.as_str(), "spec/ENTRY.md"] {
                let observed = world.extract_source(None, output).unwrap();
                manifest_sha = observed.provenance.manifest_sha256.clone();
                rows.push(DescriptorOutputRow {
                    output: output.into(),
                    source: output.into(),
                    interpretation: Some(SourceInterpretation::VibeSpec),
                    output_sha256: observed.capture.sha256.clone(),
                    document_address: Some(vibe_spec::canonical_doc_path(output)),
                });
                media_rows.insert(
                    output.into(),
                    observed.provenance.materialized_media.unwrap(),
                );
            }
            // Self capture is Observed, never original package proof. These
            // rows bind actual captured bytes to the same observed self root.
            let map = ValidatedDescriptorMap::from_witnessed_rows(
                root.path().to_path_buf(),
                root.path().join("spec"),
                "observed-self-not-source-proof".into(),
                manifest_sha,
                rows,
            )
            .unwrap()
            .with_materialized_media(media_rows)
            .unwrap();
            let reader = FsSectionSource::new(
                FileResolver::new(
                    root.path(),
                    SelfCoordinate::new(Some("org.demo".into()), "self".into()),
                )
                .with_descriptor_source_map(root.path().join("spec"), Some(map))
                .unwrap(),
            );
            let expected = world.read_spec(None, &path, Some(anchor), true).unwrap();
            let compiled = vibe_spec::compile_static(
                &SpecAddress::parse("spec://org.demo/self/ENTRY#entry").unwrap(),
                &reader,
            )
            .unwrap();
            let key = format!("spec://org.demo/self/DOC#{anchor}");
            let opening = format!("<!-- embed: {key} -->\n");
            let closing = format!("<!-- /embed: {key} -->");
            assert_eq!(compiled.matches(&opening).count(), 1);
            assert_eq!(compiled.matches(&closing).count(), 1);
            let payload = compiled
                .split_once(&opening)
                .unwrap()
                .1
                .split_once(&closing)
                .unwrap()
                .0;
            let selected_ir = vibe_specdoc::from_markdown(payload).unwrap();
            assert_eq!(selected_ir, expected.selection.ir, "{suffix}/{anchor}");
            assert_eq!(
                vibe_specdoc::to_markdown(&selected_ir),
                expected.markdown,
                "{suffix}/{anchor}"
            );
            assert!(!compiled.contains("#embed"));
            assert_eq!(expected.provenance.strength, SourceStrength::Observed);
        }
    }
}

#[test]
fn authored_export_returns_exact_literal_section_and_missing_name_candidates() {
    let bytes =
        b"# Guide\n\n## Solution\n\n@fact:LITERAL remains ordinary.\n\n## Other\n\nNot selected.\n";
    let (_root, world) = fixture_with_extra(
        &[("docs/guide.md", "text/markdown", "commonmark", bytes)],
        "\n[[package.source_exports]]\nname='solution'\nfile='docs/guide.md'\nheadings=['Guide','Solution']\n",
    );
    let selected = world.read_export(None, "solution").unwrap();
    assert_eq!(selected.export_name, "solution");
    assert_eq!(selected.heading_chain, ["Guide", "Solution"]);
    assert_eq!(selected.representation.basis, SpanBasis::CapturedBytes);
    assert!(
        std::str::from_utf8(selected.content())
            .unwrap()
            .contains("@fact:LITERAL")
    );
    assert!(
        !std::str::from_utf8(selected.content())
            .unwrap()
            .contains("Not selected")
    );
    assert_eq!(
        &selected.capture.bytes[selected.representation.span.clone()],
        selected.content()
    );
    let error = world.read_export(None, "missing").unwrap_err();
    assert!(matches!(error,ExtractionError::Export {candidates,..} if candidates==["solution"]));
}

#[test]
fn ambiguous_export_heading_refuses_without_hiding_raw_content() {
    let bytes = b"# Guide\n\n## Solution\n\nOne.\n\n## Solution\n\nTwo.\n";
    let (_root, world) = fixture_with_extra(
        &[("docs/guide.md", "text/markdown", "commonmark", bytes)],
        "\n[[package.source_exports]]\nname='solution'\nfile='docs/guide.md'\nheadings=['Guide','Solution']\n",
    );
    let error = world.read_export(None, "solution").unwrap_err();
    assert!(matches!(error,ExtractionError::Export {reason,..} if reason.contains("Ambiguous")));
    assert_eq!(
        world.extract_source(None, "docs/guide.md").unwrap().raw(),
        bytes
    );
}

#[test]
fn captured_ir_outline_matches_selected_heading_chain_and_byte_sizes() {
    let bytes=b"# D {#root}\n\n## Solution {#solution}\n\n@fact:ONE One. @status:impl/done\n\n### Child {#child}\n\n@fact:TWO Two. @status:impl/done\n";
    let (_root, world) = fixture(&[("spec/DOC.md", "text/markdown", "vibe-spec", bytes)]);
    let full = world
        .read_spec(None, "spec/DOC.md", Some("root.solution"), true)
        .unwrap();
    let direct = world
        .read_spec(None, "spec/DOC.md", Some("root.solution"), false)
        .unwrap();
    let outline = world.outline_spec(None, "spec/DOC.md").unwrap();
    let section = outline
        .nodes
        .iter()
        .find(|node| node.path.join(".") == "root.solution")
        .unwrap();
    assert_eq!(section.heading_chain, full.selection.heading_chain);
    assert_eq!(section.bytes_with_children, full.markdown.len());
    assert_eq!(section.bytes_without_children, direct.markdown.len());
    assert_eq!(outline.representation.basis, SpanBasis::EmittedMarkdown);
}

#[test]
fn commonmark_preserves_literal_annotations_without_fact_facets() {
    let bytes = b"# Guide\n\n## Solution\n\n@fact:TEXT literal @status:impl/done\n#use spec://org.demo/pkg/DOC\n";
    let (_root, world) = fixture(&[("docs/guide.md", "text/markdown", "commonmark", bytes)]);
    let source = world.extract_source(None, "docs/guide.md").unwrap();
    assert_eq!(source.raw(), bytes);
    let StructuredView::CommonMark(document) = source.view else {
        panic!("ordinary view")
    };
    let section = document.heading("Solution").unwrap();
    let selected = &document.source()[document.node(section).span.clone()];
    assert!(selected.contains("@fact:TEXT") && selected.contains("#use"));
    assert!(matches!(
        world.read_spec(None, "docs/guide.md", Some("TEXT"), true),
        Err(ExtractionError::VibeUnavailable(_))
    ));
    assert_eq!(source.provenance.strength, SourceStrength::Observed);
}

#[test]
fn malformed_xml_data_and_same_stem_commonmark_remain_raw_resources() {
    let xml = b"<broken>";
    let md = b"# Literal\n\n@fact:NOT-A-SPEC remains text.\n";
    let (_root, world) = fixture(&[
        ("docs/raw.md", "text/markdown", "commonmark", md),
        ("docs/raw.xml", "application/xml", "xml-data", xml),
    ]);
    let source = world.extract_source(None, "docs/raw.xml").unwrap();
    assert_eq!(source.raw(), xml);
    assert!(matches!(
        source.view,
        StructuredView::XmlData(vibe_specdoc::xml_data::DataInspection::Unavailable(_))
    ));
    assert_eq!(world.extract_source(None, "docs/raw.md").unwrap().raw(), md);
    assert!(world.read_spec(None, "docs/raw.xml", None, true).is_err());
    assert_eq!(source.representation.basis, SpanBasis::CapturedBytes);
    assert_eq!(source.representation.span, 0..xml.len());
}

#[test]
fn opaque_invalid_encoding_is_available_without_inspection_or_facts() {
    let bytes = &[0xff, 0, 0x80, 1];
    let (_root, world) = fixture(&[(
        "assets/raw.xml",
        "application/octet-stream",
        "opaque",
        bytes,
    )]);
    let source = world.extract_source(None, "assets/raw.xml").unwrap();
    assert_eq!(source.raw(), bytes);
    assert_eq!(source.representation.encoding, SourceEncoding::Bytes);
    assert!(matches!(source.view, StructuredView::NotRequested));
}

#[test]
fn declared_xml_ignores_suffix_and_emitted_positions_are_not_raw() {
    let xml=b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"solution\" title=\"Solution\"><p><p01 fact=\"true\">One.</p01></p><note kind=\"tip\">Tip.</note></section></spec>";
    let (_root, world) = fixture(&[("spec/DOC.md", "application/xml", "vibe-doc", xml)]);
    let selected = world
        .read_spec(None, "spec/DOC.md", Some("root.solution.p01"), true)
        .unwrap();
    assert_eq!(selected.selection.heading_chain, ["D", "Solution"]);
    assert_eq!(selected.capture.bytes, xml);
    assert_eq!(
        selected.provenance.materialized_media.as_deref(),
        Some("application/xml")
    );
    assert_eq!(selected.representation.basis, SpanBasis::EmittedMarkdown);
    assert_eq!(selected.representation.span, 0..selected.markdown.len());
    assert_ne!(selected.representation.span, 0..xml.len());
    assert!(selected.markdown.contains("One."));
    assert!(!selected.markdown.contains("Tip."));
    assert!(selected.provenance.descriptor.is_some());
}

#[test]
fn nested_selection_unknown_candidates_and_invalid_vibe_raw_are_truthful() {
    let good=b"# D {#root}\n\n## Solution {#solution}\n\n@fact:ONE One. @status:impl/done\n\n### Child {#child}\n\n@fact:TWO Two. @status:impl/done\n";
    let bad = b"# D {#root}\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";
    let (_root, world) = fixture(&[
        ("spec/GOOD.md", "text/markdown", "vibe-spec", good),
        ("spec/BAD.md", "text/markdown", "vibe-spec", bad),
    ]);
    let selected = world
        .read_spec(None, "spec/GOOD.md", Some("root.solution"), false)
        .unwrap();
    assert!(selected.markdown.contains("ONE"));
    assert!(!selected.markdown.contains("TWO"));
    let error = world
        .read_spec(None, "spec/GOOD.md", Some("root.solution.MISSING"), true)
        .unwrap_err();
    let ExtractionError::Node(SelectionError::Unknown { candidates, .. }) = error else {
        panic!("candidate refusal")
    };
    assert!(candidates.contains(&"root.solution.ONE".into()));
    let raw = world.extract_source(None, "spec/BAD.md").unwrap();
    assert_eq!(raw.raw(), bad);
    assert!(matches!(raw.view, StructuredView::Unavailable(_)));
    assert!(world.read_spec(None, "spec/BAD.md", None, true).is_err());
}
