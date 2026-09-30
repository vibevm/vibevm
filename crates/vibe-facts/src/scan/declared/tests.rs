use super::*;

const XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"A\" status=\"impl/done\">one</fact></p></spec>";

fn package(rows: &str) -> PackageMeta {
    toml::from_str(&format!(
        "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n{rows}"
    ))
    .unwrap()
}

#[test]
fn declared_native_xml_mints_only_from_witnessed_bytes() {
    let meta = package(
        "[[sources]]\nfile = 'vibevm/vibespecs/A.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    let DeclaredDocumentObservation::Facts { provenance, facts } =
        observe_declared_document(&meta, "vibevm/vibespecs/A.xml", Some(XML.as_bytes())).unwrap()
    else {
        panic!("declared source must parse")
    };
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].address, "spec://org.example/demo/A#A");
    assert_eq!(provenance.reader, Some("vibe-specdoc:xml/spec"));
    assert_eq!(
        provenance.witness,
        Some(SourceFileWitness::of(
            "vibevm/vibespecs/A.xml",
            XML.as_bytes()
        ))
    );
    assert!(matches!(
        provenance.disposition,
        DeclaredSourceDisposition::Selected {
            declaration_index: 0,
            ..
        }
    ));
}

#[test]
fn foreign_or_undeclared_vibe_shaped_xml_never_mints_facts() {
    let meta = package(
        "bridge = true\n[[sources]]\nfile = 'foreign/a.xml'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n",
    );
    let DeclaredDocumentObservation::Raw { provenance, bytes } =
        observe_declared_document(&meta, "foreign/a.xml", Some(XML.as_bytes())).unwrap()
    else {
        panic!("opaque source must remain raw")
    };
    assert_eq!(bytes, XML.as_bytes());
    assert_eq!(provenance.reader, None);
    assert_eq!(provenance.witness.as_ref().unwrap().bytes, XML.len() as u64);
    let DeclaredDocumentObservation::Unclassified { provenance, bytes } =
        observe_declared_document(&meta, "foreign/b.xml", Some(XML.as_bytes())).unwrap()
    else {
        panic!("undeclared source must remain unclassified")
    };
    assert_eq!(bytes, XML.as_bytes());
    assert!(matches!(
        provenance.disposition,
        DeclaredSourceDisposition::Unclassified
    ));
    assert_eq!(provenance.reader, None);
}

#[test]
fn conflicting_declarations_and_unavailable_bytes_are_not_empty_success() {
    let meta = package(
        "[[sources]]\nfile = 'a.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n\
         [[sources]]\nfile = 'a.xml'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n",
    );
    let DeclaredDocumentObservation::NeedsInput { provenance, bytes } =
        observe_declared_document(&meta, "a.xml", Some(XML.as_bytes())).unwrap()
    else {
        panic!("conflict must not parse")
    };
    assert_eq!(bytes, XML.as_bytes());
    assert!(matches!(
        provenance.disposition,
        DeclaredSourceDisposition::NeedsInput { declaration_indices } if declaration_indices == vec![0, 1]
    ));
    assert!(matches!(
        observe_declared_document(&meta, "a.xml", None).unwrap(),
        DeclaredDocumentObservation::Unavailable { provenance }
            if provenance.witness.is_none()
    ));
}

#[test]
fn declared_media_controls_reader_and_invalid_bytes_keep_witness() {
    let meta = package(
        "[[sources]]\nfile = 'a.md'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    let DeclaredDocumentObservation::Invalid {
        provenance, bytes, ..
    } = observe_declared_document(&meta, "a.md", Some(b"# Looks like Markdown\n")).unwrap()
    else {
        panic!("media must select XML despite suffix")
    };
    assert_eq!(provenance.reader, Some("vibe-specdoc:xml/spec"));
    assert_eq!(provenance.witness.unwrap().bytes, 22);
    assert_eq!(bytes, b"# Looks like Markdown\n");
    let DeclaredDocumentObservation::Invalid {
        provenance,
        bytes,
        reason,
    } = observe_declared_document(&meta, "a.md", Some(&[0xff])).unwrap()
    else {
        panic!("invalid UTF-8 must not be empty success")
    };
    assert!(reason.contains("UTF-8"));
    assert_eq!(provenance.witness.unwrap().bytes, 1);
    assert_eq!(bytes, &[0xff]);
}

#[test]
fn ordinary_commonmark_reports_pending_structure_with_raw_bytes() {
    let meta = package(
        "[[sources]]\nfile = 'README.md'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
    );
    assert!(matches!(
        observe_declared_document(&meta, "README.md", Some(b"# Title\n")).unwrap(),
        DeclaredDocumentObservation::Unsupported { bytes, provenance, .. }
            if bytes == b"# Title\n" && provenance.reader.is_none()
    ));
}

#[test]
fn documentation_vocabulary_requires_doc_package_kind() {
    let rows = "[[sources]]\nfile = 'README.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-doc'\nrole = 'documentation'\nloading = 'scan'\n";
    let flow = package(rows);
    assert!(matches!(
        observe_declared_document(&flow, "README.xml", Some(XML.as_bytes())).unwrap(),
        DeclaredDocumentObservation::Unsupported { provenance, .. }
            if provenance.reader.is_none()
    ));
    let doc: PackageMeta = toml::from_str(&format!(
        "name = 'demo'\ngroup = 'org.example'\nkind = 'doc'\nversion = '1.1.0'\n{rows}"
    ))
    .unwrap();
    assert!(matches!(
        observe_declared_document(&doc, "README.xml", Some(XML.as_bytes())).unwrap(),
        DeclaredDocumentObservation::Facts { provenance, .. }
            if provenance.reader == Some("vibe-specdoc:xml/doc")
    ));
}
