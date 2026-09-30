use super::*;

fn package(rows: &str) -> PackageMeta {
    toml::from_str(&format!(
        "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n{rows}"
    ))
    .unwrap()
}

#[test]
fn full_manifest_parses_package_sources_tables() {
    let manifest = crate::manifest::Manifest::parse_str(
        "[package]\nname = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
         [[package.sources]]\nfile = 'vibevm/vibespecs/common/PROP-001.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    )
    .unwrap();
    let package = manifest.require_package().unwrap();
    assert_eq!(package.sources.len(), 1);
    assert!(matches!(
        package
            .select_source("vibevm/vibespecs/common/PROP-001.xml")
            .unwrap(),
        SelectedSource::Descriptor { .. }
    ));
}

#[test]
fn exact_file_overrides_directory_default_without_changing_simple_format() {
    let meta = package(
        "[[sources]]\ndirectory = 'docs'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n\
         [[sources]]\nfile = 'docs/contract.md'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    assert_eq!(meta.format, super::super::PackageFormat::Simple);
    let SelectedSource::Descriptor {
        identity,
        declaration_index,
        descriptor,
    } = meta.select_source("docs/contract.md").unwrap()
    else {
        panic!("exact declaration must win")
    };
    assert_eq!(identity, "org.example/demo:docs/contract.md");
    assert_eq!(declaration_index, 1);
    assert_eq!(descriptor.interpretation(), SourceInterpretation::VibeSpec);
    assert!(matches!(
        meta.select_source("docs/guide.md").unwrap(),
        SelectedSource::Descriptor { descriptor, .. }
            if descriptor.interpretation() == SourceInterpretation::Commonmark
    ));
}

#[test]
fn longest_directory_matches_only_descendant_components() {
    let meta = package(
        "[[sources]]\ndirectory = 'docs'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n\
         [[sources]]\ndirectory = 'docs/deep'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
    );
    assert!(matches!(
        meta.select_source("docs/deep/one.md").unwrap(),
        SelectedSource::Descriptor {
            declaration_index: 1,
            ..
        }
    ));
    assert!(matches!(
        meta.select_source("docs/deeper/one.md").unwrap(),
        SelectedSource::Descriptor {
            declaration_index: 0,
            ..
        }
    ));
    assert!(matches!(
        meta.select_source("docs").unwrap(),
        SelectedSource::Unclassified { .. }
    ));
}

#[test]
fn incompatible_equal_specificity_requires_input_but_identical_rows_coalesce() {
    let meta = package(
        "[[sources]]\nfile = 'docs/x.xml'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n\
         [[sources]]\nfile = 'docs/x.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
    );
    assert_eq!(
        meta.select_source("docs/x.xml").unwrap(),
        SelectedSource::NeedsInput {
            identity: "org.example/demo:docs/x.xml".into(),
            declaration_indices: vec![0, 1],
        }
    );
    let same = package(
        "[[sources]]\nfile = 'docs/x.xml'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n\
         [[sources]]\nfile = 'docs/x.xml'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n",
    );
    assert!(matches!(
        same.select_source("docs/x.xml").unwrap(),
        SelectedSource::Descriptor {
            declaration_index: 0,
            ..
        }
    ));
}

#[test]
fn foreign_xml_stays_opaque_and_undeclared_xml_stays_unclassified() {
    let meta = package(
        "bridge = true\n[[sources]]\nfile = 'foreign/data.xml'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n",
    );
    assert!(matches!(
        meta.select_source("foreign/data.xml").unwrap(),
        SelectedSource::Descriptor { descriptor, .. }
            if descriptor.interpretation() == SourceInterpretation::Opaque
    ));
    assert!(matches!(
        meta.select_source("foreign/valid-vibe.xml").unwrap(),
        SelectedSource::Unclassified { .. }
    ));
}

#[test]
fn opaque_documentation_remains_raw_and_generated_outputs_never_inherit_defaults() {
    let meta = package(
        "[[sources]]\ndirectory = 'vibevm/vibespecs'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'documentation'\nloading = 'scan'\n",
    );
    assert!(matches!(
        meta.select_source("vibevm/vibespecs/manual.pdf").unwrap(),
        SelectedSource::Descriptor { descriptor, .. }
            if descriptor.interpretation() == SourceInterpretation::Opaque
    ));
    for generated in [
        "vibevm/vibespecs/boot/INDEX.md",
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibespecs/boot/static.XML",
    ] {
        assert!(matches!(
            meta.select_source(generated),
            Err(SourceSelectionError::InvalidPath(_))
        ));
    }
}

#[test]
fn authored_trust_and_invalid_paths_are_refused() {
    for extra in [
        "trust = 'verified'",
        "file = 'another.md'",
        "directory = 'docs'",
    ] {
        let text = format!(
            "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
             [[sources]]\nfile = 'docs/x.md'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n{extra}\n"
        );
        assert!(toml::from_str::<PackageMeta>(&text).is_err(), "{extra}");
    }
    for path in [
        "../escape.md",
        "docs/*.md",
        "docs\\x.md",
        "docs//x.md",
        "docs/",
    ] {
        let text = format!(
            "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
             [[sources]]\nfile = '{path}'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n"
        );
        assert!(toml::from_str::<PackageMeta>(&text).is_err(), "{path}");
    }
    let trailing_directory = "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
         [[sources]]\ndirectory = 'docs/'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n";
    assert!(toml::from_str::<PackageMeta>(trailing_directory).is_err());
    let meta = package("");
    assert!(matches!(
        meta.select_source("../escape.md"),
        Err(SourceSelectionError::InvalidPath(_))
    ));
}

#[test]
fn generated_outputs_and_incompatible_media_role_pairs_are_refused() {
    for (path, media, interpretation, role) in [
        (
            "vibevm/vibespecs/boot/STATIC.xml",
            "application/xml",
            "vibe-spec",
            "spec",
        ),
        ("docs/x.xml", "text/markdown", "xml-data", "resource"),
        ("docs/x.md", "text/markdown", "commonmark", "spec"),
    ] {
        let text = format!(
            "name = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\n\
             [[sources]]\nfile = '{path}'\nmedia = '{media}'\ninterpretation = '{interpretation}'\nrole = '{role}'\nloading = 'scan'\n"
        );
        assert!(toml::from_str::<PackageMeta>(&text).is_err(), "{path}");
    }
}
