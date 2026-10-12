//! M-01-A.2 portable physical lookup corpus over the existing component law.

use std::fs;

use super::*;
use vibe_core::manifest::SourceInterpretation;

fn self_resolver(root: &std::path::Path) -> FileResolver {
    FileResolver::new(
        root,
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    )
}

fn resolve(root: &std::path::Path, doc: &str) -> Result<PathBuf, ResolveError> {
    let mut address = SpecAddress::parse("spec://org.demo/self/placeholder").unwrap();
    address.doc_path = doc.to_owned();
    self_resolver(root).resolve_file(&address)
}

#[test]
fn nested_portable_document_keeps_its_exact_logical_name() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("spec/common/nested/PROP-042-title.xml");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "<spec/>").unwrap();
    assert_eq!(
        resolve(root.path(), "common/nested/PROP-042").unwrap(),
        file
    );
}

#[test]
fn portable_refusals_precede_lookup_of_hostile_components() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("spec")).unwrap();
    for doc in [
        "../outside",
        "C:/secret",
        "//server/share",
        "\\\\server\\share",
        "common/guide:ads",
        "common/CON",
        "common/LPT9.txt",
        "common/trailing.",
        "common/bell\u{7}",
    ] {
        assert!(
            matches!(
                resolve(root.path(), doc),
                Err(ResolveError::UnsafeSourcePath { .. })
            ),
            "unsafe spelling {doc:?} escaped shared refusal"
        );
    }
}

#[test]
fn directory_file_case_and_nfc_aliases_refuse_instead_of_opening() {
    let root = tempfile::tempdir().unwrap();
    let specs = root.path().join("spec");
    fs::create_dir_all(specs.join("Common")).unwrap();
    fs::write(specs.join("Common/guide.md"), "# guide\n").unwrap();
    assert!(matches!(
        resolve(root.path(), "common/guide"),
        Err(ResolveError::PortableSourceAlias { .. })
    ));
    fs::create_dir_all(specs.join("docs")).unwrap();
    fs::write(specs.join("docs/Guide.md"), "# guide\n").unwrap();
    assert!(matches!(
        resolve(root.path(), "docs/guide"),
        Err(ResolveError::PortableSourceAlias { .. })
    ));
    fs::write(specs.join("docs/\u{e9}dition.md"), "# edition\n").unwrap();
    assert!(matches!(
        resolve(root.path(), "docs/e\u{301}dition"),
        Err(ResolveError::PortableSourceAlias { .. })
    ));
    fs::write(specs.join("docs/PROP-042-title.MD"), "# title\n").unwrap();
    assert!(matches!(
        resolve(root.path(), "docs/PROP-042"),
        Err(ResolveError::PortableSourceAlias { .. })
    ));
    fs::write(specs.join("docs/PROP-043-title.md."), "# title\n").unwrap();
    assert!(matches!(
        resolve(root.path(), "docs/PROP-043"),
        Err(ResolveError::PortableSourceAlias { .. })
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_child_refuses_before_observing_outside_target() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("spec")).unwrap();
    fs::write(outside.path().join("secret.md"), "OUTSIDE\n").unwrap();
    symlink(outside.path(), root.path().join("spec/docs")).unwrap();
    assert!(matches!(
        resolve(root.path(), "docs/secret"),
        Err(ResolveError::UnsafeSourcePath { .. })
    ));
    symlink(
        outside.path().join("secret.md"),
        root.path().join("spec/secret.md"),
    )
    .unwrap();
    assert!(matches!(
        resolve(root.path(), "secret"),
        Err(ResolveError::UnsafeSourcePath { .. })
    ));
}

#[test]
fn descriptor_rows_use_the_same_portable_component_and_identity_law() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    let specs = slot.join("spec");
    let row = |path: &str| DescriptorOutputRow {
        output: path.into(),
        source: path.into(),
        interpretation: Some(SourceInterpretation::Commonmark),
        output_sha256: "a".repeat(64),
        document_address: None,
    };
    let build = |rows| {
        ValidatedDescriptorMap::from_witnessed_rows(
            slot.clone(),
            specs.clone(),
            "sha256:selected".into(),
            "b".repeat(64),
            rows,
        )
    };
    assert!(matches!(
        build(vec![row("spec/docs/guide.md:ads")]),
        Err(ResolveError::DescriptorMapInvalid { .. })
    ));
    assert!(matches!(
        build(vec![row("spec/docs/COM1.md")]),
        Err(ResolveError::DescriptorMapInvalid { .. })
    ));
    assert!(matches!(
        build(vec![
            row("spec/Docs/\u{e9}dition.md"),
            row("spec/docs/e\u{301}dition.md"),
        ]),
        Err(ResolveError::DescriptorMapInvalid { .. })
    ));
}

#[cfg(unix)]
#[test]
fn classified_strict_resource_refuses_a_symlinked_descriptor_output() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let specs = root.path().join("spec");
    fs::create_dir_all(specs.join("common")).unwrap();
    fs::write(outside.path().join("guide.md"), "OUTSIDE\n").unwrap();
    symlink(
        outside.path().join("guide.md"),
        specs.join("common/guide.md"),
    )
    .unwrap();
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        root.path().to_path_buf(),
        specs.clone(),
        "sha256:selected".into(),
        "a".repeat(64),
        vec![DescriptorOutputRow {
            output: "spec/common/guide.md".into(),
            source: "spec/common/guide.md".into(),
            interpretation: Some(SourceInterpretation::VibeSpec),
            output_sha256: "b".repeat(64),
            document_address: Some("common/guide".into()),
        }],
    )
    .unwrap();
    let resolver = self_resolver(root.path())
        .with_descriptor_source_map(specs, Some(map))
        .unwrap();
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/self/common/guide"),
        Err(crate::ResourceAddressError::Lookup(
            ResolveError::UnsafeSourcePath { .. }
        ))
    ));
}

#[cfg(unix)]
#[test]
fn layout_root_probe_does_not_follow_an_outside_symlink() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("vibevm")).unwrap();
    fs::create_dir_all(outside.path().join("common")).unwrap();
    fs::write(outside.path().join("common/guide.md"), "OUTSIDE\n").unwrap();
    symlink(outside.path(), root.path().join("vibevm/vibespecs")).unwrap();
    assert!(matches!(
        resolve(root.path(), "common/guide"),
        Err(ResolveError::DocNotFound { .. })
    ));
    symlink(outside.path(), root.path().join("spec")).unwrap();
    assert!(matches!(
        resolve(root.path(), "common/guide"),
        Err(ResolveError::UnsafeSourcePath { .. })
    ));
}
