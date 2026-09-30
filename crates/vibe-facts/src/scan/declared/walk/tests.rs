use super::*;
use crate::scan::declared::{DeclaredReadProvenance, DeclaredSourceDisposition};

const XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"A\" status=\"impl/done\">one</fact></p></spec>";
const MARKDOWN: &str = "# Rule\n\n@fact:B Two. @status:impl/done\n";

fn package(rows: &str) -> PackageMeta {
    toml::from_str(&format!(
        "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n{rows}"
    ))
    .unwrap()
}

fn row<'a>(path: &'a str, bytes: Option<&'a [u8]>) -> CapturedDeclaredDocument<'a> {
    CapturedDeclaredDocument { path, bytes }
}

fn provenance<'o, 'raw>(
    observation: &'o DeclaredDocumentObservation<'raw>,
) -> &'o DeclaredReadProvenance {
    match observation {
        DeclaredDocumentObservation::Unavailable { provenance }
        | DeclaredDocumentObservation::Unclassified { provenance, .. }
        | DeclaredDocumentObservation::NeedsInput { provenance, .. }
        | DeclaredDocumentObservation::Raw { provenance, .. }
        | DeclaredDocumentObservation::Unsupported { provenance, .. }
        | DeclaredDocumentObservation::Invalid { provenance, .. }
        | DeclaredDocumentObservation::Structure { provenance, .. }
        | DeclaredDocumentObservation::Facts { provenance, .. } => provenance,
    }
}

#[test]
fn declared_xml_and_markdown_mint_only_from_exact_witnessed_bytes() {
    let meta = package(
        "[[sources]]\nfile = 'vibevm/vibespecs/A.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n\
         [[sources]]\nfile = 'vibevm/vibespecs/B.md'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    let observed = observe_declared_source(
        &meta,
        &[
            row("vibevm/vibespecs/B.md", Some(MARKDOWN.as_bytes())),
            row("vibevm/vibespecs/A.xml", Some(XML.as_bytes())),
        ],
    )
    .unwrap();
    assert_eq!(observed.documents.len(), 2);
    assert_eq!(observed.facts.len(), 2);
    assert!(observed.complete);
    assert_eq!(observed.facts[0].address, "spec://org.example/demo/A#A");
    assert_eq!(observed.facts[1].address, "spec://org.example/demo/B#B");
    let first = provenance(&observed.documents[0].1);
    assert_eq!(first.identity, "org.example/demo:vibevm/vibespecs/A.xml");
    assert_eq!(first.reader, Some("vibe-specdoc:xml/spec"));
    assert_eq!(
        first.witness,
        Some(crate::SourceFileWitness::of(
            "vibevm/vibespecs/A.xml",
            XML.as_bytes()
        ))
    );
}

#[test]
fn foreign_and_undeclared_vibe_shaped_xml_remain_raw_or_unclassified() {
    let meta = package(
        "bridge = true\n[[sources]]\nfile = 'foreign/a.xml'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n",
    );
    let observed = observe_declared_source(
        &meta,
        &[
            row("foreign/a.xml", Some(XML.as_bytes())),
            row("foreign/b.xml", Some(XML.as_bytes())),
        ],
    )
    .unwrap();
    assert!(observed.facts.is_empty());
    assert!(!observed.complete);
    assert!(
        matches!(&observed.documents[0].1, DeclaredDocumentObservation::Raw { bytes, provenance }
        if *bytes == XML.as_bytes() && provenance.reader.is_none())
    );
    assert!(
        matches!(&observed.documents[1].1, DeclaredDocumentObservation::Unclassified { bytes, provenance }
        if *bytes == XML.as_bytes() && provenance.reader.is_none())
    );
}

#[test]
fn ambiguous_rows_and_missing_exact_file_keep_distinct_incomplete_observations() {
    let meta = package(
        "[[sources]]\nfile = 'a.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n\
         [[sources]]\nfile = 'a.xml'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n\
         [[sources]]\nfile = 'missing.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'explicit'\n",
    );
    let observed = observe_declared_source(&meta, &[row("a.xml", Some(XML.as_bytes()))]).unwrap();
    assert!(observed.facts.is_empty());
    assert!(!observed.complete);
    assert!(
        matches!(&observed.documents[0].1, DeclaredDocumentObservation::NeedsInput { provenance, .. }
        if matches!(&provenance.disposition, DeclaredSourceDisposition::NeedsInput { declaration_indices } if declaration_indices == &vec![0,1]))
    );
    assert!(
        matches!(&observed.documents[1].1, DeclaredDocumentObservation::Unavailable { provenance }
        if provenance.witness.is_none() && provenance.identity == "org.example/demo:missing.xml")
    );
}

#[test]
fn simple_package_literal_use_does_not_mint_a_fact() {
    let meta = package(
        "format = 'simple'\n[[sources]]\nfile = 'literal.md'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    let observed =
        observe_declared_source(&meta, &[row("literal.md", Some(b"#use flow:other\n"))]).unwrap();
    assert!(observed.facts.is_empty());
    assert!(observed.complete);
    assert!(
        matches!(&observed.documents[0].1, DeclaredDocumentObservation::Facts { facts, provenance }
        if facts.is_empty() && provenance.witness.as_ref().is_some_and(|w| w.bytes == 16))
    );
}

#[test]
fn duplicate_captured_paths_refuse_before_any_fact_result() {
    let meta = package("");
    assert!(matches!(
        observe_declared_source(
            &meta,
            &[
                row("a.xml", Some(XML.as_bytes())),
                row("A.xml", Some(XML.as_bytes()))
            ]
        ),
        Err(DeclaredWalkError::DuplicatePath(_))
    ));
}

#[test]
fn declared_directory_media_not_suffix_selects_xml_reader() {
    let meta = package(
        "[[sources]]\ndirectory = 'vibevm/vibespecs'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    let observed =
        observe_declared_source(&meta, &[row("vibevm/vibespecs/A.md", Some(XML.as_bytes()))])
            .unwrap();
    assert!(!observed.complete);
    assert_eq!(observed.facts.len(), 1);
    assert_eq!(
        provenance(&observed.documents[0].1).reader,
        Some("vibe-specdoc:xml/spec")
    );
}

#[test]
fn exact_commonmark_structure_is_complete_without_facts_but_directory_default_is_not() {
    let exact = package(
        "[[sources]]\nfile = 'README.md'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
    );
    let raw = b"# Title\n\n#use package and @requires:plan stay prose.\n";
    let observed = observe_declared_source(&exact, &[row("README.md", Some(raw))]).unwrap();
    assert!(observed.complete);
    assert!(observed.facts.is_empty());
    assert!(
        matches!(&observed.documents[0].1, DeclaredDocumentObservation::Structure { provenance, document }
        if provenance.reader == Some("vibe-specdoc:commonmark") && document.source().as_bytes() == raw)
    );

    let directory = package(
        "[[sources]]\ndirectory = 'docs'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
    );
    let observed = observe_declared_source(&directory, &[row("docs/one.md", Some(raw))]).unwrap();
    assert!(!observed.complete);
    assert!(observed.facts.is_empty());
    assert!(matches!(
        &observed.documents[0].1,
        DeclaredDocumentObservation::Structure { .. }
    ));
}
