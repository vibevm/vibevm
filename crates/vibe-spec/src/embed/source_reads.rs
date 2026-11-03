//! Actual source consumers exercise the read boundary, not a stand-in reader.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use super::*;
use crate::{Authority, SelfCoordinate};
use std::fs;

#[cfg(test)]
fn source(root: &Path) -> FsSectionSource {
    FsSectionSource::new(FileResolver::new(
        root,
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    ))
}
#[cfg(test)]
fn address() -> SpecAddress {
    SpecAddress::parse("spec://org.demo/self/DOC#root").unwrap()
}
#[cfg(test)]
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[cfg(unix)]
#[test]
fn redirected_document_and_parent_refuse_actual_embed_content() {
    for parent in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(
            &outside.path().join("DOC.md"),
            "# Outside {#root}\nOUTSIDE_SENTINEL\n",
        );
        fs::create_dir(root.path().join("spec")).unwrap();
        let mut addr = address();
        if parent {
            std::os::unix::fs::symlink(outside.path(), root.path().join("spec/nested")).unwrap();
            addr.doc_path = "nested/DOC".into();
        } else {
            std::os::unix::fs::symlink(
                outside.path().join("DOC.md"),
                root.path().join("spec/DOC.md"),
            )
            .unwrap();
        }
        let result = source(root.path()).section_text(&addr);
        assert!(result.is_err(), "redirected source was read: {result:?}");
        assert_eq!(
            fs::read_to_string(outside.path().join("DOC.md")).unwrap(),
            "# Outside {#root}\nOUTSIDE_SENTINEL\n"
        );
    }
}

#[cfg(unix)]
#[test]
fn redirected_layout_and_derived_package_slot_are_not_root_aliases() {
    for package in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let mut addr = address();
        if package {
            write(
                &outside.path().join("1.0.0/spec/DOC.md"),
                "# Outside {#root}\nOUTSIDE_SENTINEL\n",
            );
            fs::create_dir(root.path().join("vibedeps")).unwrap();
            std::os::unix::fs::symlink(outside.path(), root.path().join("vibedeps/org.demo.dep"))
                .unwrap();
            addr.authority = Authority::Package {
                group: "org.demo".into(),
                name: "dep".into(),
                version: Some("1.0.0".into()),
            };
        } else {
            write(
                &outside.path().join("DOC.md"),
                "# Outside {#root}\nOUTSIDE_SENTINEL\n",
            );
            fs::create_dir(root.path().join("vibevm")).unwrap();
            std::os::unix::fs::symlink(outside.path(), root.path().join("vibevm/vibespecs"))
                .unwrap();
        }
        let result = source(root.path()).section_text(&addr);
        assert!(
            result.is_err(),
            "derived root redirect was read: {result:?}"
        );
    }
}

#[test]
fn supplied_metadata_cannot_substitute_an_outside_document() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &root.path().join("spec/DOC.md"),
        "# Selected {#root}\nSELECTED\n",
    );
    let path = outside.path().join("DOC.md");
    write(&path, "# Outside {#root}\nOUTSIDE_SENTINEL\n");
    let metadata = ResolvedSourceMetadata::new(path, "markdown".into(), "DOC".into()).unwrap();
    let result = source(root.path()).read_resolved_source(&address(), metadata);
    assert!(
        result.is_err(),
        "foreign locator supplied content: {result:?}"
    );
}

#[cfg(unix)]
#[test]
fn explicit_root_alias_and_shared_regular_file_remain_readable() {
    let root = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    write(
        &root.path().join("spec/DOC.md"),
        "# Selected {#root}\nSELECTED\n",
    );
    fs::hard_link(
        root.path().join("spec/DOC.md"),
        root.path().join("shared.md"),
    )
    .unwrap();
    let alias = aliases.path().join("chosen-root");
    std::os::unix::fs::symlink(root.path(), &alias).unwrap();
    assert!(
        source(&alias)
            .section_text(&address())
            .unwrap()
            .contains("SELECTED")
    );
}

#[cfg(unix)]
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
)]
fn metadata_locator_does_not_freeze_explicit_root_alias_across_reads() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    write(
        &first.path().join("spec/DOC.md"),
        "# First {#root}\nFIRST_ROOT\n",
    );
    write(
        &second.path().join("spec/DOC.md"),
        "# Second {#root}\nSECOND_ROOT\n",
    );
    let alias = aliases.path().join("selected");
    std::os::unix::fs::symlink(first.path(), &alias).unwrap();
    let reader = source(&alias);
    let metadata = reader.source_metadata(&address(), &[]).unwrap();
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(second.path(), &alias).unwrap();
    let observed = reader.read_resolved_source(&address(), metadata).unwrap();
    assert!(observed.text.contains("SECOND_ROOT") && !observed.text.contains("FIRST_ROOT"));
    assert!(
        fs::read_to_string(first.path().join("spec/DOC.md"))
            .unwrap()
            .contains("FIRST_ROOT")
    );
}

#[test]
fn relative_root_retains_exact_overlay_key_and_never_reads_overlay_fallback() {
    let root = tempfile::tempdir_in(".").unwrap();
    let logical = Path::new(".").join(root.path().file_name().unwrap());
    assert!(logical.is_relative());
    let path = logical.join("spec/DOC.md");
    write(&path, "# Disk {#root}\nDISK\n");
    let reader = FsSectionSource::with_overlay(
        source(&logical).resolver,
        BTreeMap::from([(
            path.clone(),
            Arc::<[u8]>::from(&b"# Overlay {#root}\nOVERLAY\n"[..]),
        )]),
    );
    vibe_safefs::arm_before_bounded_read(Some(Box::new(|_, _| {
        panic!("overlay caused a content read")
    })));
    let result = reader.section_text(&address());
    vibe_safefs::arm_before_bounded_read(None);
    assert!(result.unwrap().contains("OVERLAY"));
    let broken = FsSectionSource::with_overlay(
        source(&logical).resolver,
        BTreeMap::from([(path, Arc::<[u8]>::from([0xff_u8].as_slice()))]),
    );
    vibe_safefs::arm_before_bounded_read(Some(Box::new(|_, _| {
        panic!("bad overlay fell back to disk")
    })));
    let result = broken.section_text(&address());
    vibe_safefs::arm_before_bounded_read(None);
    assert!(result.unwrap_err().contains("overlay spec source"));
    assert!(
        source(&logical)
            .section_text(&address())
            .unwrap()
            .contains("DISK")
    );
}

#[test]
fn custom_metadata_is_content_free_and_the_admitted_read_keeps_raw_bytes() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let root = tempfile::tempdir().unwrap();
    let raw = "<not-vibe> raw {#marker} </not-vibe>\n";
    write(&root.path().join("spec/DOC.txt"), raw);
    let reader = source(root.path());
    let count = Arc::new(AtomicUsize::new(0));
    let probe = count.clone();
    vibe_safefs::arm_before_bounded_read(Some(Box::new(move |_, _| {
        probe.fetch_add(1, Ordering::SeqCst);
    })));
    let metadata = reader.source_metadata(&address(), &["txt".into()]).unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let resolved = reader.read_resolved_source(&address(), metadata).unwrap();
    vibe_safefs::arm_before_bounded_read(None);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(resolved.format(), "txt");
    assert_eq!(resolved.text, raw);
}

#[test]
fn metadata_drift_refuses_without_reading_a_substituted_id_match() {
    let root = tempfile::tempdir().unwrap();
    let old = root.path().join("spec/PROP-001-old.md");
    write(&old, "# Old {#root}\nOLD\n");
    let addr = SpecAddress::parse("spec://org.demo/self/PROP-001#root").unwrap();
    let reader = source(root.path());
    let metadata = reader.source_metadata(&addr, &[]).unwrap();
    fs::rename(&old, root.path().join("spec/PROP-001-new.md")).unwrap();
    vibe_safefs::arm_before_bounded_read(Some(Box::new(|_, _| {
        panic!("drifted locator read content")
    })));
    let result = reader.read_resolved_source(&addr, metadata);
    vibe_safefs::arm_before_bounded_read(None);
    assert!(result.unwrap_err().contains("locator"));
}

#[test]
fn custom_locator_keeps_compound_suffix_and_full_active_collision_context() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("spec/DOC.foo.bar"), "RAW_COMPOUND_SUFFIX");
    let reader = source(root.path());
    let metadata = reader
        .source_metadata(&address(), &["foo.bar".into()])
        .unwrap();
    assert_eq!(
        reader
            .read_resolved_source(&address(), metadata)
            .unwrap()
            .text,
        "RAW_COMPOUND_SUFFIX"
    );
    fs::remove_file(root.path().join("spec/DOC.foo.bar")).unwrap();
    write(&root.path().join("spec/DOC.txt"), "FIRST");
    let metadata = reader
        .source_metadata(&address(), &["txt".into(), "other".into()])
        .unwrap();
    write(&root.path().join("spec/DOC.other"), "SECOND");
    vibe_safefs::arm_before_bounded_read(Some(Box::new(|_, _| {
        panic!("ambiguous locator read content")
    })));
    let result = reader.read_resolved_source(&address(), metadata);
    vibe_safefs::arm_before_bounded_read(None);
    assert!(
        result
            .unwrap_err()
            .contains("multiple active physical forms")
    );
}

#[test]
fn optional_version_selection_ignores_regular_files_that_are_not_slots() {
    let root = tempfile::tempdir().unwrap();
    write(
        &root.path().join("vibedeps/org.demo.dep/1.0.0/spec/DOC.md"),
        "# Selected {#root}\nSELECTED\n",
    );
    write(
        &root.path().join("vibedeps/org.demo.dep/9.0.0"),
        "not a version directory",
    );
    let address = SpecAddress::parse("spec://org.demo/dep/DOC#root").unwrap();
    assert!(
        source(root.path())
            .section_text(&address)
            .unwrap()
            .contains("SELECTED")
    );
}

#[test]
fn regular_files_in_current_layout_preserve_legacy_spec_fallback() {
    for blocker in ["vibevm", "vibevm/vibespecs"] {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join(blocker), "ORDINARY_PACKAGE_FILE");
        write(
            &root.path().join("spec/DOC.md"),
            "# Legacy {#root}\nLEGACY_SOURCE\n",
        );
        let observed = source(root.path()).section_text(&address()).unwrap();
        assert!(observed.contains("LEGACY_SOURCE"), "{blocker}");
        assert_eq!(
            fs::read_to_string(root.path().join(blocker)).unwrap(),
            "ORDINARY_PACKAGE_FILE"
        );
    }
}

#[cfg(unix)]
#[test]
fn same_byte_replacement_is_refused_by_builtin_and_custom_consumers() {
    for extension in ["md", "xml", "txt"] {
        let root = tempfile::tempdir().unwrap();
        let body = if extension == "xml" {
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Stable</title></spec>"
        } else {
            "# Stable {#root}\nSAME\n"
        };
        let path = root.path().join(format!("spec/DOC.{extension}"));
        write(&path, body);
        let reader = source(root.path());
        let formats = if extension == "txt" {
            vec!["txt".into()]
        } else {
            vec![]
        };
        let metadata = reader.source_metadata(&address(), &formats).unwrap();
        vibe_safefs::arm_before_bounded_read(Some(Box::new(|parent, name| {
            let bytes = fs::read(parent.join(name)).unwrap();
            fs::rename(parent.join(name), parent.join("old-source")).unwrap();
            fs::write(parent.join(name), bytes).unwrap();
        })));
        let result = reader.read_resolved_source(&address(), metadata);
        vibe_safefs::arm_before_bounded_read(None);
        assert!(result.unwrap_err().contains("replaced"), "{extension}");
        assert_eq!(fs::read_to_string(path).unwrap(), body);
    }
}

#[cfg(unix)]
#[test]
fn retained_parent_cannot_redirect_the_held_read_to_outside_content() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &root.path().join("spec/nested/DOC.md"),
        "# Held {#root}\nHELD\n",
    );
    write(
        &outside.path().join("DOC.md"),
        "# Outside {#root}\nOUTSIDE\n",
    );
    let redirected = root.path().join("spec/nested");
    let retained = root.path().join("spec/retained");
    let target = outside.path().to_path_buf();
    vibe_safefs::arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(&redirected, &retained).unwrap();
        std::os::unix::fs::symlink(&target, &redirected).unwrap();
    })));
    let mut addr = address();
    addr.doc_path = "nested/DOC".into();
    let result = source(root.path()).section_text(&addr);
    vibe_safefs::arm_before_bounded_read(None);
    let text = result.unwrap();
    assert!(text.contains("HELD") && !text.contains("OUTSIDE"));
    assert!(
        fs::symlink_metadata(root.path().join("spec/nested"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
