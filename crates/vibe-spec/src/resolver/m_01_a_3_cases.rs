//! M-01-A.3 refusal classification at the incumbent resolver boundary.

use std::fs;

use super::*;
use crate::{FsSectionSource, SectionSource};
use vibe_core::manifest::SourceInterpretation;

fn self_coord() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.demo".into()), "self".into())
}

#[test]
fn absent_document_remains_not_found() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    let resolver = FileResolver::new(root.path(), self_coord());
    let address = SpecAddress::parse("spec://org.demo/self/common/ABSENT").unwrap();
    assert!(matches!(
        resolver.resolve_file(&address),
        Err(ResolveError::DocNotFound { .. })
    ));
}

#[test]
fn non_directory_installed_slot_is_not_a_missing_package() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("vibedeps")).unwrap();
    fs::write(
        root.path().join("vibedeps/org.demo.widget"),
        "file not directory",
    )
    .unwrap();
    let resolver = FileResolver::new(root.path(), self_coord());
    let address = SpecAddress::parse("spec://org.demo/widget/common/DOC").unwrap();
    assert!(matches!(
        resolver.resolve_file(&address),
        Err(ResolveError::SourceUnreadable { .. })
    ));
}

#[test]
fn ambiguous_glob_document_refuses_instead_of_disappearing() {
    let root = tempfile::tempdir().unwrap();
    let dir = root
        .path()
        .join("vibedeps/org.demo.plugin-a/1.0.0/spec/common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PROP-042-one.md"), "# One\n").unwrap();
    fs::write(dir.join("PROP-042-two.md"), "# Two\n").unwrap();
    let resolver = FileResolver::new(root.path(), self_coord());
    let pattern = SpecAddress::parse("spec://org.demo/plugin-*/common/PROP-042").unwrap();
    assert!(matches!(
        resolver.expand_pattern(&pattern),
        Err(ResolveError::AmbiguousDoc { .. })
    ));
}

#[test]
fn pattern_member_collision_refuses_instead_of_choosing_a_slot() {
    let root = tempfile::tempdir().unwrap();
    for group in ["org.one", "org.two"] {
        let dir = root
            .path()
            .join(format!("vibedeps/{group}.plugin-a/1.0.0/spec/common"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("DOC.md"), group).unwrap();
    }
    let resolver = FileResolver::new(root.path(), self_coord());
    let pattern = SpecAddress::parse("spec://org.demo/plugin-*/common/DOC").unwrap();
    assert!(matches!(
        resolver.expand_pattern(&pattern),
        Err(ResolveError::AmbiguousPatternMember { .. })
    ));
}

#[test]
fn unreadable_pattern_slot_does_not_disappear_from_set() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("vibedeps")).unwrap();
    fs::write(root.path().join("vibedeps/org.demo.plugin-a"), "blocked").unwrap();
    let resolver = FileResolver::new(root.path(), self_coord());
    let pattern = SpecAddress::parse("spec://org.demo/plugin-*/common/DOC").unwrap();
    assert!(matches!(
        resolver.expand_pattern(&pattern),
        Err(ResolveError::SourceUnreadable { .. })
    ));
}

#[test]
fn duplicate_anchor_does_not_return_the_first_section() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("spec/common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("DOC.md"),
        "# First {#same}\n\nFIRST_ONLY\n\n# Second {#same}\n\nSECOND_ONLY\n",
    )
    .unwrap();
    let source = FsSectionSource::new(FileResolver::new(root.path(), self_coord()));
    let address = SpecAddress::parse("spec://org.demo/self/common/DOC#same").unwrap();
    let error = source.section_text(&address).unwrap_err();
    assert!(
        error.contains("ambiguous"),
        "duplicate anchor was not refused: {error}"
    );
}

#[cfg(unix)]
#[test]
fn descriptor_owned_embed_read_refuses_symlinked_output() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let specs = root.path().join("spec");
    fs::create_dir_all(specs.join("common")).unwrap();
    fs::write(
        outside.path().join("DOC.md"),
        "# Outside {#root}\nOUTSIDE\n",
    )
    .unwrap();
    symlink(outside.path().join("DOC.md"), specs.join("common/DOC.md")).unwrap();
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        root.path().to_path_buf(),
        specs.clone(),
        "sha256:selected".into(),
        "a".repeat(64),
        vec![DescriptorOutputRow {
            output: "spec/common/DOC.md".into(),
            source: "spec/common/DOC.md".into(),
            interpretation: Some(SourceInterpretation::VibeSpec),
            output_sha256: "b".repeat(64),
            document_address: Some("common/DOC".into()),
        }],
    )
    .unwrap();
    let resolver = FileResolver::new(root.path(), self_coord())
        .with_descriptor_source_map(specs, Some(map))
        .unwrap();
    let source = FsSectionSource::new(resolver);
    let address = SpecAddress::parse("spec://org.demo/self/common/DOC#root").unwrap();
    assert!(
        source
            .section_text(&address)
            .unwrap_err()
            .contains("unsafe")
    );
    assert!(
        source
            .resolved_source(&address, &[])
            .unwrap_err()
            .contains("unsafe")
    );
}

#[test]
fn compiler_facing_source_preserves_content_and_distinct_refusals() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("spec/common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("DOC.md"), "# Doc {#root}\n\nEXACT_BODY\n").unwrap();
    let source = FsSectionSource::new(FileResolver::new(root.path(), self_coord()));
    let address = SpecAddress::parse("spec://org.demo/self/common/DOC#root").unwrap();
    let (text, _, stem) = source.resolved_source(&address, &[]).unwrap().into_parts();
    assert!(text.contains("EXACT_BODY"));
    assert_eq!(stem, "DOC");

    fs::write(
        dir.join("DUP.md"),
        "# First {#same}\nFIRST_ONLY\n# Second {#same}\nSECOND_ONLY\n",
    )
    .unwrap();
    let duplicate = SpecAddress::parse("spec://org.demo/self/common/DUP#same").unwrap();
    assert!(
        source
            .resolved_source(&duplicate, &[])
            .unwrap_err()
            .contains("ambiguous")
    );

    let blocked = tempfile::tempdir().unwrap();
    fs::create_dir(blocked.path().join("spec")).unwrap();
    fs::write(blocked.path().join("spec/common"), "not a directory").unwrap();
    let blocked_source = FsSectionSource::new(FileResolver::new(blocked.path(), self_coord()));
    assert!(
        blocked_source
            .resolved_source(&address, &[])
            .unwrap_err()
            .contains("unreadable")
    );
}
