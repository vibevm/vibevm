//! Cross-boundary controls on the same public source-selection path.
use super::*;

#[test]
fn patterns_do_not_manufacture_another_self_installation() {
    let ws = tempfile::tempdir().unwrap();
    document(ws.path(), "DOC", "AUTHORED_SELF");
    document(
        &slot(ws.path(), "org.host", "self", "1.0.0"),
        "DOC",
        "INSTALLED_SELF",
    );
    let ordinary = FileResolver::new(ws.path(), own());
    assert!(
        ordinary
            .expand_pattern(&address("spec://org.host/*/common/DOC"))
            .unwrap()
            .is_empty()
    );
    let selected = FileResolver::with_selected_world(
        ws.path(),
        ws.path(),
        BTreeMap::from([(
            ("org.host".into(), "self".into()),
            SelectedPackage::new("1.0.0", "/never-open-self-map"),
        )]),
        own(),
    );
    assert!(
        selected
            .expand_pattern(&address("spec://org.host/*/common/DOC"))
            .unwrap()
            .is_empty()
    );
    assert!(
        FsSectionSource::new(selected)
            .section_text(&address("spec://org.host/self/common/DOC#root"))
            .unwrap()
            .contains("AUTHORED_SELF")
    );
}

#[cfg(unix)]
#[test]
fn selected_explicit_root_alias_and_hardlinked_percent_source_remain_readable() {
    use std::os::unix::fs::{MetadataExt, symlink};
    let ws = tempfile::tempdir().unwrap();
    let source_root = tempfile::tempdir().unwrap();
    let source = document(source_root.path(), "P%20DOC", "SHARED_SOURCE");
    let hardlink = source_root.path().join("original");
    fs::hard_link(&source, &hardlink).unwrap();
    let original = hardlink.metadata().unwrap();
    symlink(source_root.path(), ws.path().join("chosen-alias")).unwrap();
    let resolver = FileResolver::with_selected_world(
        ws.path(),
        ws.path(),
        BTreeMap::from([(
            ("org.demo".into(), "plugin-a".into()),
            SelectedPackage::new("1.0.0", ws.path().join("chosen-alias")),
        )]),
        own(),
    );
    let addresses = resolver
        .expand_pattern(&address("spec://org.demo/plugin-*/common/P%20DOC#root"))
        .unwrap();
    assert_eq!(addresses.len(), 1);
    assert!(
        FsSectionSource::new(resolver)
            .section_text(&addresses[0])
            .unwrap()
            .contains("SHARED_SOURCE")
    );
    let after = source.metadata().unwrap();
    assert_eq!((original.dev(), original.ino()), (after.dev(), after.ino()));
    assert!(after.nlink() >= 2);
}

#[cfg(unix)]
#[test]
fn matching_version_and_document_parent_symlinks_are_errors_not_absence() {
    use std::os::unix::fs::symlink;
    for link_version in [true, false] {
        let ws = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        document(outside.path(), "DOC", "OUTSIDE");
        let package = ws.path().join("vibedeps/org.demo.plugin-a");
        fs::create_dir_all(&package).unwrap();
        if link_version {
            symlink(outside.path(), package.join("1.0.0")).unwrap();
        } else {
            fs::create_dir_all(package.join("1.0.0/spec")).unwrap();
            symlink(
                outside.path().join("spec/common"),
                package.join("1.0.0/spec/common"),
            )
            .unwrap();
        }
        let resolver = FileResolver::new(ws.path(), own());
        assert!(matches!(
            resolver.expand_pattern(&address("spec://org.demo/plugin-*/common/DOC")),
            Err(vibe_spec::ResolveError::SourceObservation { .. })
        ));
    }
}
