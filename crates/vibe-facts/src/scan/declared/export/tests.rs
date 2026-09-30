use super::*;
use crate::scan::declared::observe_declared_document;

fn package(headings: &str) -> PackageMeta {
    toml::from_str(&format!(
        "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
         [[sources]]\nfile = 'docs/guide.md'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n\
         [[source_exports]]\nname = 'intro'\nfile = 'docs/guide.md'\nheadings = {headings}\n"
    )).unwrap()
}

#[test]
fn witnessed_nested_export_retains_durable_name_and_snapshot_local_node() {
    let package = package("['Guide', 'Introduction']");
    let raw = b"# Guide\n\n## Introduction\n#use and @requires:plan are literal. `@fact:X`\n";
    let observation = observe_declared_document(&package, "docs/guide.md", Some(raw)).unwrap();
    let resolved =
        resolve_declared_export(&package, &package.source_exports[0], &observation).unwrap();
    assert_eq!(resolved.package_coordinate, "org.example/demo");
    assert_eq!(resolved.export_name, "intro");
    assert_eq!(resolved.resource_identity, "org.example/demo:docs/guide.md");
    assert_eq!(
        resolved.witness,
        SourceFileWitness::of("docs/guide.md", raw)
    );
    assert_eq!(
        &raw[resolved.span.clone()],
        b"## Introduction\n#use and @requires:plan are literal. `@fact:X`\n"
    );
    assert!(matches!(
        observation,
        DeclaredDocumentObservation::Structure { .. }
    ));
}

#[test]
fn changed_witness_and_incomplete_observation_refuse_without_first_match() {
    let package = package("['Guide', 'Introduction']");
    let old = b"# Guide\n## Introduction\nOld.\n";
    let new = b"# Guide\n## Introduction\nNew.\n";
    let mut observation = observe_declared_document(&package, "docs/guide.md", Some(new)).unwrap();
    if let DeclaredDocumentObservation::Structure { provenance, .. } = &mut observation {
        provenance.witness = Some(SourceFileWitness::of("docs/guide.md", old));
    } else {
        panic!("expected structure")
    }
    assert_eq!(
        resolve_declared_export(&package, &package.source_exports[0], &observation),
        Err(DeclaredExportError::ChangedWitness)
    );

    let mut wrong_resource =
        observe_declared_document(&package, "docs/guide.md", Some(new)).unwrap();
    if let DeclaredDocumentObservation::Structure { provenance, .. } = &mut wrong_resource {
        provenance.identity = "org.example/demo:docs/other.md".into();
    }
    assert_eq!(
        resolve_declared_export(&package, &package.source_exports[0], &wrong_resource),
        Err(DeclaredExportError::WrongResource)
    );

    let absent = observe_declared_document(&package, "docs/guide.md", None).unwrap();
    assert_eq!(
        resolve_declared_export(&package, &package.source_exports[0], &absent),
        Err(DeclaredExportError::IncompleteObservation)
    );
}

#[test]
fn duplicate_sibling_heading_and_undeclared_export_refuse() {
    let missing = package("['Guide', 'Missing']");
    let package = package("['Guide', 'Repeated']");
    let observation = observe_declared_document(
        &package,
        "docs/guide.md",
        Some(b"# Guide\n## Repeated\nA.\n## Repeated\nB.\n"),
    )
    .unwrap();
    assert!(matches!(
        resolve_declared_export(&package, &package.source_exports[0], &observation),
        Err(DeclaredExportError::Selector(
            ExportResolutionError::Ambiguous { index: 1, .. }
        ))
    ));
    let other =
        SourceExport::new("other".into(), "docs/guide.md".into(), vec!["Guide".into()]).unwrap();
    assert_eq!(
        resolve_declared_export(&package, &other, &observation),
        Err(DeclaredExportError::UndeclaredExport)
    );

    let observation =
        observe_declared_document(&missing, "docs/guide.md", Some(b"# Guide\n## Present\n"))
            .unwrap();
    assert!(matches!(
        resolve_declared_export(&missing, &missing.source_exports[0], &observation),
        Err(DeclaredExportError::Selector(
            ExportResolutionError::Missing { index: 1, .. }
        ))
    ));
}
