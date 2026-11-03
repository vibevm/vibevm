//! Filesystem-bound spellings cannot widen an existing source authority.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS");

use super::*;
use crate::{FsSectionSource, SectionSource};
use std::fs;

#[cfg(test)]
fn resolver(root: &Path) -> FileResolver {
    FileResolver::new(
        root,
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    )
}

#[cfg(test)]
fn address(doc: &str) -> SpecAddress {
    let mut address = SpecAddress::parse("spec://org.demo/self/DOC").unwrap();
    address.doc_path = doc.into();
    address
}

#[cfg(test)]
fn write(path: &Path, bytes: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"
)]
fn embed_refuses_parent_document_instead_of_reading_outside_sentinel() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("spec")).unwrap();
    write(
        &root.path().join("SECRET.md"),
        "# Secret\nOUTSIDE_SENTINEL\n",
    );
    let source = FsSectionSource::new(resolver(root.path()));
    let address = SpecAddress::parse("spec://org.demo/self/../SECRET").unwrap();
    let result = source.section_text(&address);
    assert!(result.is_err(), "outside source was read: {result:?}");
    assert_eq!(
        fs::read_to_string(root.path().join("SECRET.md")).unwrap(),
        "# Secret\nOUTSIDE_SENTINEL\n"
    );
}

#[test]
fn manually_constructed_absolute_document_refuses_before_root_lookup() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("SECRET.md"), "OUTSIDE_SENTINEL");
    let doc = outside.path().join("SECRET").to_str().unwrap().to_owned();
    assert!(resolver(root.path()).resolve_file(&address(&doc)).is_err());
}

#[test]
fn explicit_version_cannot_leave_its_package_slot() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("vibedeps/org.demo.dep")).unwrap();
    write(
        &root.path().join("vibedeps/outside/spec/SECRET.md"),
        "OUTSIDE_SENTINEL",
    );
    let mut address = address("SECRET");
    address.authority = Authority::Package {
        group: "org.demo".into(),
        name: "dep".into(),
        version: Some("../outside".into()),
    };
    assert!(resolver(root.path()).resolve_file(&address).is_err());
}

#[test]
fn constructed_package_slot_cannot_replace_the_dependency_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &outside.path().join("holder.dep/1.0.0/spec/SECRET.md"),
        "OUTSIDE_SENTINEL",
    );
    let mut address = address("SECRET");
    address.authority = Authority::Package {
        group: outside.path().join("holder").to_str().unwrap().into(),
        name: "dep".into(),
        version: Some("1.0.0".into()),
    };
    assert!(resolver(root.path()).resolve_file(&address).is_err());
}

#[test]
fn custom_frontend_suffix_cannot_form_a_path() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/NOTE.")).unwrap();
    write(&root.path().join("outside/SECRET"), "OUTSIDE_SENTINEL");
    let result = resolver(root.path())
        .resolve_source_file(&address("NOTE"), &["/../../outside/SECRET".into()]);
    assert!(result.is_err(), "custom suffix escaped: {result:?}");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"
)]
fn invalid_pattern_document_refuses_even_without_dependency_slots() {
    let root = tempfile::tempdir().unwrap();
    let pattern = SpecAddress::parse("spec://org.demo/plugin-*/../SECRET").unwrap();
    assert!(resolver(root.path()).expand_pattern(&pattern).is_err());
}

#[test]
fn valid_nested_unicode_percent_literals_and_safe_joined_slot_survive() {
    let root = tempfile::tempdir().unwrap();
    for doc in ["nested/Документ", "%2e%2e/NOTE"] {
        let expected = root.path().join("spec").join(format!("{doc}.md"));
        write(&expected, "# Body\n");
        assert_eq!(
            resolver(root.path()).resolve_file(&address(doc)).unwrap(),
            expected
        );
    }
    let expected = root.path().join("vibedeps/org.demo.con/1.0.0/spec/DOC.md");
    write(&expected, "# Body\n");
    let address = SpecAddress::parse("spec://org.demo/con@1.0.0/DOC").unwrap();
    assert_eq!(
        resolver(root.path()).resolve_file(&address).unwrap(),
        expected
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"
)]
fn invalid_spelling_has_typed_requirement_diagnostic_before_selected_lookup() {
    let root = tempfile::tempdir().unwrap();
    let selected = FileResolver::with_selected_world(
        root.path(),
        root.path(),
        BTreeMap::new(),
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    );
    for doc in [
        "",
        ".",
        "..",
        "../SECRET",
        "a/../SECRET",
        "a/./DOC",
        "a//DOC",
        "/DOC",
        "a\\DOC",
        "C:DOC",
        "NUL",
        "a/CON.txt",
        "a/DOC.",
        "a/DOC ",
        "a/\u{7}DOC",
    ] {
        let mut address = address(doc);
        address.authority = Authority::Package {
            group: "org.demo".into(),
            name: "unselected".into(),
            version: None,
        };
        let error = selected.resolve_file(&address).unwrap_err();
        assert!(
            matches!(
                error,
                ResolveError::InvalidLookupPath {
                    field: "document path",
                    ..
                }
            ),
            "{doc:?}: {error}"
        );
        let message = error.to_string();
        assert!(message.contains("violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"));
        assert!(message.contains("fix surface:"));
    }
}

#[test]
fn invalid_version_and_custom_suffix_refuse_before_absent_slot_lookup() {
    let root = tempfile::tempdir().unwrap();
    for version in [
        "..",
        "/outside",
        "..\\outside",
        "C:outside",
        "CON",
        "1.0.0.",
    ] {
        let mut address = address("DOC");
        address.authority = Authority::Package {
            group: "org.demo".into(),
            name: "dep".into(),
            version: Some(version.into()),
        };
        assert!(
            matches!(
                resolver(root.path()).resolve_file(&address),
                Err(ResolveError::InvalidLookupPath {
                    field: "slot version",
                    ..
                })
            ),
            "{version}"
        );
    }
    for format in ["", "../txt", "x/../../SECRET", "x\\SECRET", "x:stream", "."] {
        assert!(
            matches!(
                resolver(root.path()).resolve_source_file(&address("DOC"), &[format.into()]),
                Err(ResolveError::InvalidLookupPath {
                    field: "custom frontend filename",
                    ..
                })
            ),
            "{format}"
        );
    }
}

#[test]
fn selected_root_and_nonpattern_expansion_keep_their_existing_authority() {
    let root = tempfile::tempdir().unwrap();
    let selected = tempfile::tempdir().unwrap();
    let expected = selected.path().join("spec/DOC.md");
    write(&expected, "# Selected\n");
    let world = BTreeMap::from([(
        ("org.demo".into(), "dep".into()),
        SelectedPackage::new("1.0.0", selected.path()),
    )]);
    let resolver = FileResolver::with_selected_world(
        root.path(),
        root.path(),
        world,
        SelfCoordinate::new(None, "self".into()),
    );
    let address = SpecAddress::parse("spec://org.demo/dep@1.0.0/DOC").unwrap();
    assert_eq!(resolver.resolve_file(&address).unwrap(), expected);
    let mut unvalidated = address;
    unvalidated.doc_path = "../SECRET".into();
    assert_eq!(
        resolver.expand_pattern(&unvalidated).unwrap(),
        vec![unvalidated.clone()]
    );
    assert!(matches!(
        resolver.resolve_file(&unvalidated),
        Err(ResolveError::InvalidLookupPath { .. })
    ));
}
