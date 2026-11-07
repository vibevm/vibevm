//! Real declared host point reads through the existing SectionSource.
#[cfg(test)]
mod cases {
    use crate::{FileResolver, FsSectionSource, SectionSource, SelfCoordinate, SpecAddress};
    use std::fs;
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n[[project.sources]]\nfile='native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        fs::write(root.path().join("native.payload"),"<spec xmlns=\"https://vibevm.org/spec/1\"><title>Host</title><LAW title=\"Law\"><p>`req r2`</p><p>HOST-NATIVE-BODY</p></LAW></spec>").unwrap();
        root
    }
    fn new_source(root: &std::path::Path) -> FsSectionSource {
        FsSectionSource::new(FileResolver::new(
            root,
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        ))
    }
    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-READ",
        r = 1
    )]
    fn declared_host_arbitrary_suffix_reaches_actual_section_source_anchor_and_pin() {
        let root = fixture();
        let source = new_source(root.path());
        let address = SpecAddress::parse("spec://org.host/host/native.payload#LAW~r2").unwrap();
        let body = source.section_text(&address).unwrap();
        assert!(body.contains("HOST-NATIVE-BODY"));
        assert!(!body.contains("<spec"));
    }

    #[test]
    fn legacy_and_declared_short_id_and_full_slug_are_the_same_point_source() {
        let root = tempfile::tempdir().unwrap();
        let relative = vibe_core::layout::current_specs_root().join("common/PROP-123-title.xml");
        fs::create_dir_all(root.path().join(&relative).parent().unwrap()).unwrap();
        fs::write(root.path().join(&relative),"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Alias</title><p>ALIAS-BODY</p></spec>").unwrap();
        let base = "[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n";
        fs::write(root.path().join("vibe.toml"), base).unwrap();
        for doc in ["common/PROP-123", "common/PROP-123-title"] {
            let addr = SpecAddress::parse(&format!("spec://org.host/host/{doc}#root")).unwrap();
            assert!(
                new_source(root.path())
                    .section_text(&addr)
                    .unwrap()
                    .contains("ALIAS-BODY")
            );
        }
        fs::write(root.path().join("vibe.toml"),format!("{base}[[project.sources]]\ndirectory='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",vibe_core::layout::current_specs_root().display())).unwrap();
        let source = new_source(root.path());
        for doc in ["common/PROP-123", "common/PROP-123-title"] {
            let addr = SpecAddress::parse(&format!("spec://org.host/host/{doc}#root")).unwrap();
            assert!(source.section_text(&addr).unwrap().contains("ALIAS-BODY"));
        }
    }
    #[test]
    fn file_reader_preserves_send_sync_auto_traits() {
        fn assert_traits<T: Send + Sync>() {}
        assert_traits::<FileResolver>();
        assert_traits::<FsSectionSource>();
    }

    fn row(file: &str, media: &str, interpretation: &str) -> String {
        let role = if interpretation == "vibe-spec" {
            "spec"
        } else if interpretation == "vibe-doc" {
            "documentation"
        } else {
            "resource"
        };
        format!(
            "[[project.sources]]\nfile='{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"
        )
    }
    fn write(root: &std::path::Path, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn project(root: &std::path::Path, rows: &str) {
        write(
            root,
            "vibe.toml",
            format!("[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n{rows}"),
        );
    }
    fn addr(doc: &str) -> SpecAddress {
        SpecAddress::parse(&format!("spec://org.host/host/{doc}")).unwrap()
    }
    #[test]
    fn raw_neighbors_and_custom_frontend_suffix_do_not_gain_native_addresses() {
        let root = tempfile::tempdir().unwrap();
        project(
            root.path(),
            &(row("spec/PAIR.md", "application/xml", "vibe-spec")
                + &row("spec/PAIR.xml", "application/xml", "xml-data")
                + &row("spec/RAW.txt", "text/plain", "opaque")
                + &row("spec/NOTE.md", "text/markdown", "commonmark")),
        );
        write(
            root.path(),
            "spec/PAIR.md",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Native</title><p>NATIVE-ONLY</p></spec>",
        );
        for path in ["spec/PAIR.xml", "spec/RAW.txt", "spec/NOTE.md"] {
            write(
                root.path(),
                path,
                "# Native-looking {#root}\n#embed spec://org.host/host/PAIR#root\nRAW-MUST-NOT-EXECUTE\n",
            );
        }
        let source = new_source(root.path());
        assert!(
            source
                .section_text(&addr("PAIR#root"))
                .unwrap()
                .contains("NATIVE-ONLY")
        );
        let metadata = source
            .source_metadata(&addr("PAIR#root"), &["md".into(), "txt".into()])
            .unwrap();
        assert_eq!(metadata.format(), "markdown");
        for doc in ["RAW#root", "RAW.txt#root", "NOTE#root"] {
            assert!(source.source_metadata(&addr(doc), &["txt".into()]).is_err());
            assert!(
                crate::expand_embeds(&format!("#embed spec://org.host/host/{doc}\n"), &source)
                    .is_err()
            );
        }
    }
    #[test]
    fn metadata_to_read_refuses_byte_manifest_parent_membership_and_record_drift() {
        for drift in ["bytes", "manifest", "parent", "membership", "record"] {
            let root = tempfile::tempdir().unwrap();
            project(
                root.path(),
                "[[project.sources]]\ndirectory='spec'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
            );
            write(
                root.path(),
                "spec/common/PROP-123-title.md",
                "# Native {#root}\nBODY\n",
            );
            let source = new_source(root.path());
            let address = addr("common/PROP-123#root");
            let metadata = source.source_metadata(&address, &[]).unwrap();
            match drift {
                "bytes" => write(
                    root.path(),
                    "spec/common/PROP-123-title.md",
                    "# Native {#root}\nCHANGED\n",
                ),
                "manifest" => {
                    let path = root.path().join("vibe.toml");
                    fs::write(&path, fs::read_to_string(&path).unwrap() + "\n# drift\n").unwrap();
                }
                "membership" => write(
                    root.path(),
                    "spec/common/PROP-123-other.md",
                    "# Other {#root}\n",
                ),
                "record" => write(
                    root.path(),
                    vibe_core::layout::SLOT_RECORD_FILENAME,
                    "invalid record must still prevent host reclassification",
                ),
                _ => {
                    fs::rename(
                        root.path().join("spec/common"),
                        root.path().join("spec/old"),
                    )
                    .unwrap();
                    fs::create_dir(root.path().join("spec/common")).unwrap();
                    fs::hard_link(
                        root.path().join("spec/old/PROP-123-title.md"),
                        root.path().join("spec/common/PROP-123-title.md"),
                    )
                    .unwrap();
                }
            }
            assert!(
                source.read_resolved_source(&address, metadata).is_err(),
                "{drift}"
            );
            assert!(source.admit_reference(&address).is_err(), "{drift}");
        }
    }
    #[test]
    fn exact_overlay_matches_capture_and_bad_overlay_never_uses_disk() {
        use std::{collections::BTreeMap, sync::Arc};
        let root = fixture();
        let path = root.path().join("native.payload");
        let original = fs::read(&path).unwrap();
        for good in [true, false] {
            let resolver = FileResolver::new(
                root.path(),
                SelfCoordinate::new(Some("org.host".into()), "host".into()),
            );
            let overlay = if good {
                original.clone()
            } else {
                b"<spec>WRONG</spec>".to_vec()
            };
            let source = FsSectionSource::with_overlay(
                resolver,
                BTreeMap::from([(path.clone(), Arc::from(overlay))]),
            );
            let result = source.section_text(&addr("native.payload#LAW~r2"));
            if good {
                assert!(result.unwrap().contains("HOST-NATIVE-BODY"));
            } else {
                assert!(result.unwrap_err().contains("overlay"));
            }
        }
    }
    #[test]
    fn self_identity_version_and_locator_boundaries_preserve_legacy_revision_transport() {
        let root = fixture();
        let source = new_source(root.path());
        // Existing lookup transports revision spelling; it does not assert
        // that a req marker equals that revision.
        let pinned = addr("native.payload#LAW~r3");
        assert_eq!(pinned.pinned_r, Some(3));
        assert_eq!(
            source.section_text(&pinned).unwrap(),
            source.section_text(&addr("native.payload#LAW~r2")).unwrap()
        );
        assert!(
            source
                .section_text(
                    &SpecAddress::parse("spec://org.host/host@1.0.0/native.payload#LAW").unwrap()
                )
                .unwrap_err()
                .contains("unversioned")
        );
        let wrong = FsSectionSource::new(FileResolver::new(
            root.path(),
            SelfCoordinate::new(Some("org.wrong".into()), "host".into()),
        ));
        assert!(
            wrong
                .section_text(
                    &SpecAddress::parse("spec://org.wrong/host/native.payload#LAW").unwrap()
                )
                .is_err()
        );
        let resolver = FileResolver::new(
            root.path(),
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        );
        assert!(resolver.resolve_file(&addr("native.payload#LAW")).is_err());
        assert!(
            resolver
                .resolve_source_file(&addr("native.payload#LAW"), &["payload".into()])
                .is_err()
        );
        let pattern = SpecAddress::parse("spec://org.host/hos*/native.payload#LAW").unwrap();
        assert!(source.expand_pattern(&pattern).is_err());
        assert!(resolver.expand_pattern(&pattern).is_err());
    }
    #[test]
    fn actual_embed_and_compiler_consume_captured_native_host() {
        let root = fixture();
        let path = root.path().join("vibe.toml");
        fs::write(
            &path,
            fs::read_to_string(&path).unwrap()
                + &row("spec/ENTRY.md", "text/markdown", "vibe-spec"),
        )
        .unwrap();
        write(
            root.path(),
            "spec/ENTRY.md",
            "# Entry {#root}\n#embed spec://org.host/host/native.payload#LAW~r2\n",
        );
        let source = new_source(root.path());
        let embedded = crate::expand_embeds(
            "before\n#embed spec://org.host/host/native.payload#LAW~r2\nafter\n",
            &source,
        )
        .unwrap();
        assert!(
            embedded.contains("HOST-NATIVE-BODY")
                && embedded.contains("before")
                && embedded.contains("after")
        );
        let compiled = crate::compile_static(&addr("ENTRY#root"), &source).unwrap();
        assert!(compiled.contains("HOST-NATIVE-BODY"));
        assert!(!compiled.contains("#embed"));
    }
    #[test]
    fn declared_package_authored_root_uses_actual_doc_vocabulary_but_project_doc_refuses() {
        let root = tempfile::tempdir().unwrap();
        let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Guide</title><note kind=\"tip\">DOC-NOTE</note></spec>";
        write(root.path(), "guide.payload", xml);
        let rows = row("guide.payload", "application/xml", "vibe-doc");
        project(root.path(), &rows);
        assert!(
            new_source(root.path())
                .section_text(&addr("guide.payload#root"))
                .is_err()
        );
        let package = format!(
            "[package]\ngroup='org.host'\nname='host'\nversion='1.0.0'\nkind='doc'\ntitle='Guide'\nabstract='Native documentation.'\n{}\n[[documents]]\npackage='org.subject/tool'\nversion='^1.0'\n",
            rows.replace("project.sources", "package.sources")
        );
        write(root.path(), "vibe.toml", package);
        assert!(
            new_source(root.path())
                .section_text(&addr("guide.payload#root"))
                .unwrap()
                .contains("DOC-NOTE")
        );
    }
    #[test]
    fn real_host_under_namesake_directory_and_different_selected_self_root_remain_hosts() {
        use std::collections::BTreeMap;
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("vibedeps/notes/caller");
        fs::create_dir_all(&root).unwrap();
        project(&root, &row("spec/A.md", "text/markdown", "vibe-spec"));
        write(&root, "spec/A.md", "# A {#root}\nREAL-HOST\n");
        assert!(
            new_source(&root)
                .section_text(&addr("A#root"))
                .unwrap()
                .contains("REAL-HOST")
        );
        let resolver = FileResolver::with_selected_world(
            &root,
            outer.path(),
            BTreeMap::from([(
                ("org.host".into(), "host".into()),
                crate::SelectedPackage::new("1.0.0", outer.path().join("different-slot")),
            )]),
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        );
        assert!(
            FsSectionSource::new(resolver)
                .section_text(&addr("A#root"))
                .unwrap()
                .contains("REAL-HOST")
        );
    }
    #[test]
    fn selected_and_structural_slots_without_independent_binding_never_enter_host_route() {
        use std::collections::BTreeMap;
        let outer = tempfile::tempdir().unwrap();
        let root = outer
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.host.host/1.0.0");
        fs::create_dir_all(&root).unwrap();
        project(&root, &row("spec/A.md", "text/markdown", "vibe-spec"));
        write(&root, "spec/A.md", "# A {#root}\nNOT-HOST\n");
        assert!(new_source(&root).section_text(&addr("A#root")).is_err());
        let independent = fixture();
        let resolver = FileResolver::with_selected_world(
            independent.path(),
            outer.path(),
            BTreeMap::from([(
                ("org.host".into(), "host".into()),
                crate::SelectedPackage::new("1.0.0", independent.path()),
            )]),
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        );
        assert!(
            FsSectionSource::new(resolver)
                .section_text(&addr("native.payload#LAW"))
                .is_err()
        );
    }
    #[test]
    fn legacy_empty_and_legacy_to_declared_transition_have_distinct_results() {
        let root = tempfile::tempdir().unwrap();
        project(root.path(), "");
        write(root.path(), "spec/A.md", "# A {#root}\nLEGACY\n");
        let source = new_source(root.path());
        let address = addr("A#root");
        assert!(source.section_text(&address).unwrap().contains("LEGACY"));
        project(
            root.path(),
            &row("spec/A.md", "text/markdown", "commonmark"),
        );
        assert!(
            source
                .admit_reference(&address)
                .unwrap_err()
                .contains("legacy to declared")
        );
        project(root.path(), "sources=[]\n");
        assert!(new_source(root.path()).section_text(&address).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn declared_alias_is_valid_but_retarget_cannot_change_read_role() {
        use std::os::unix::fs::symlink;
        let outer = tempfile::tempdir().unwrap();
        let first = outer.path().join("first");
        fs::create_dir(&first).unwrap();
        project(&first, &row("spec/A.md", "text/markdown", "vibe-spec"));
        write(&first, "spec/A.md", "# A {#root}\nFIRST\n");
        let alias = outer.path().join("alias");
        symlink(&first, &alias).unwrap();
        let source = new_source(&alias);
        let address = addr("A#root");
        let metadata = source.source_metadata(&address, &[]).unwrap();
        let slot = outer
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.host.host/1.0.0");
        fs::create_dir_all(&slot).unwrap();
        project(&slot, "");
        write(&slot, "spec/A.md", "# A {#root}\nSECOND\n");
        fs::remove_file(&alias).unwrap();
        symlink(&slot, &alias).unwrap();
        assert!(source.read_resolved_source(&address, metadata).is_err());
        project(&slot, &row("spec/A.md", "text/markdown", "vibe-spec"));
        assert!(new_source(&alias).section_text(&address).is_err());
    }
}
