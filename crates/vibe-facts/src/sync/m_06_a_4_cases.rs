//! Exact cited host policy integration, not a whole-tree semantic scan.
#[cfg(test)]
mod tests {
    use crate::{FactEntry, FactStatus, Registry, sync};
    use std::fs;
    fn status(value: &str) -> FactStatus {
        FactStatus::parse(value).unwrap()
    }
    #[test]
    fn directory_default_only_canonical_xml_citation_ignores_unrelated_malformed_source() {
        let root = tempfile::tempdir().unwrap();
        let specs = vibe_core::layout::current_specs_root();
        fs::write(root.path().join("vibe.toml"),format!("[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n[[project.sources]]\ndirectory='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",specs.display())).unwrap();
        fs::create_dir_all(root.path().join(&specs).join("common")).unwrap();
        fs::write(root.path().join(&specs).join("common/PROP-123-readable.xml"),"<spec xmlns=\"https://vibevm.org/spec/1\"><title>Rule</title><p><LAW fact=\"true\" status=\"impl/done\">law</LAW></p></spec>").unwrap();
        fs::write(root.path().join(&specs).join("unrelated.xml"), "<invalid").unwrap();
        let address = "spec://org.example/demo/common/PROP-123#LAW";
        let mut registry = Registry::default();
        registry
            .upsert(
                root.path(),
                FactEntry::for_host(address, "org.example/demo", Some(status("spec/work")), None)
                    .unwrap(),
            )
            .unwrap();
        let changes = sync::reconcile(root.path(), &mut registry).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(
            registry.get(address).unwrap().status,
            Some(status("impl/done"))
        );
        let before = disk(root.path());
        let memory = registry.entries().cloned().collect::<Vec<_>>();
        fs::write(root.path().join(&specs).join("common/PROP-123-other.md"),"<spec xmlns=\"https://vibevm.org/spec/1\"><p><LAW fact=\"true\" status=\"impl/done\">other</LAW></p></spec>").unwrap();
        assert!(sync::reconcile(root.path(), &mut registry).is_err());
        assert_eq!(disk(root.path()), before);
        assert_eq!(registry.entries().cloned().collect::<Vec<_>>(), memory);
    }

    fn row(selector: &str, media: &str, interpretation: &str) -> String {
        let role = if interpretation == "vibe-spec" {
            "spec"
        } else if interpretation == "vibe-doc" {
            "documentation"
        } else {
            "resource"
        };
        format!(
            "[[project.sources]]\n{selector}\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"
        )
    }
    fn host(rows: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("vibe.toml"),
            format!("[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n{rows}"),
        )
        .unwrap();
        root
    }
    fn write(root: &std::path::Path, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn seeded_registry(root: &std::path::Path, docs: &[&str]) -> Registry {
        let mut registry = Registry::default();
        for doc in docs {
            registry
                .upsert(
                    root,
                    FactEntry::for_host(
                        format!("spec://org.example/demo/{doc}"),
                        "org.example/demo",
                        Some(status("spec/work")),
                        None,
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        registry
    }
    fn disk(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let home = root.join(vibe_core::layout::current_vibefacts_root());
        let mut rows = std::collections::BTreeMap::new();
        if home.is_dir() {
            for entry in fs::read_dir(home).unwrap() {
                let entry = entry.unwrap();
                rows.insert(
                    entry.file_name().to_str().unwrap().into(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
        rows
    }
    #[test]
    fn native_arbitrary_suffix_xml_and_markdown_keep_status_void_and_missing_anchor_rules() {
        for (media, body) in [
            (
                "application/xml",
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Native</title><p><LAW fact=\"true\" status=\"impl/done\">law</LAW></p><p><VOID fact=\"true\" status=\"spec/void\">void</VOID></p><p><UNMARKED fact=\"true\">none</UNMARKED></p></spec>",
            ),
            (
                "text/markdown",
                "# Native\n\n@fact:LAW law. @status:impl/done\n\n@fact:VOID void. @status:spec/void\n\n@fact:UNMARKED none.\n",
            ),
        ] {
            let root = host(&row("file='native.payload'", media, "vibe-spec"));
            write(root.path(), "native.payload", body);
            let mut registry = seeded_registry(
                root.path(),
                &[
                    "native.payload#LAW",
                    "native.payload#VOID",
                    "native.payload#UNMARKED",
                ],
            );
            assert_eq!(
                sync::reconcile(root.path(), &mut registry).unwrap().len(),
                3
            );
            assert_eq!(
                registry
                    .get("spec://org.example/demo/native.payload#LAW")
                    .unwrap()
                    .status,
                Some(status("impl/done"))
            );
            assert_eq!(
                registry
                    .get("spec://org.example/demo/native.payload#VOID")
                    .unwrap()
                    .status,
                Some(status("spec/void"))
            );
            assert!(
                registry
                    .get("spec://org.example/demo/native.payload#UNMARKED")
                    .is_none()
            );
            assert_eq!(
                fs::read(root.path().join("native.payload")).unwrap(),
                body.as_bytes()
            );
        }
    }
    #[test]
    fn later_bad_citation_refuses_without_disk_or_inmemory_registry_change() {
        for bad in [
            "malformed",
            "raw",
            "missing",
            "ambiguous",
            "directory",
            "unsupported-doc",
        ] {
            let second = if bad == "raw" {
                row("file='B.md'", "text/markdown", "commonmark")
            } else if bad == "unsupported-doc" {
                row("file='B.md'", "application/xml", "vibe-doc")
            } else {
                row("file='B.md'", "text/markdown", "vibe-spec")
            };
            let mut rows = row("file='A.md'", "text/markdown", "vibe-spec") + &second;
            if bad == "ambiguous" {
                rows += &row("file='B.xml'", "application/xml", "vibe-spec");
            }
            let root = host(&rows);
            write(
                root.path(),
                "A.md",
                "# A\n\n@fact:LAW changed. @status:impl/done\n",
            );
            match bad {
                "missing" => {}
                "directory" => fs::create_dir(root.path().join("B.md")).unwrap(),
                "malformed" => write(root.path(), "B.md", [0xff, 0]),
                _ => write(
                    root.path(),
                    "B.md",
                    "# B\n\n@fact:LAW bad. @status:impl/done\n",
                ),
            }
            if bad == "ambiguous" {
                write(root.path(), "B.xml", "<invalid/>");
            }
            let mut registry = seeded_registry(root.path(), &["A#LAW", "B#LAW"]);
            let memory = registry.entries().cloned().collect::<Vec<_>>();
            let before = disk(root.path());
            assert!(
                sync::reconcile(root.path(), &mut registry).is_err(),
                "{bad}"
            );
            assert_eq!(
                registry.entries().cloned().collect::<Vec<_>>(),
                memory,
                "{bad}"
            );
            assert_eq!(disk(root.path()), before, "{bad}");
        }
    }
    #[test]
    fn missing_file_refuses_but_missing_anchor_in_readable_native_file_removes() {
        let root = host(&row("file='A.md'", "text/markdown", "vibe-spec"));
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        let before = disk(root.path());
        assert!(sync::reconcile(root.path(), &mut registry).is_err());
        assert_eq!(disk(root.path()), before);
        write(root.path(), "A.md", "# A\n\nNo LAW anchor.\n");
        assert_eq!(
            sync::reconcile(root.path(), &mut registry).unwrap().len(),
            1
        );
        assert!(registry.entries().next().is_none());
    }
    #[test]
    fn exact_native_over_raw_default_coexists_with_raw_same_stem_and_unrelated_default() {
        let specs = vibe_core::layout::current_specs_root();
        let native = format!("{}/RULE.xml", specs.display());
        let rows = row(
            &format!("directory='{}'", specs.display()),
            "text/markdown",
            "commonmark",
        ) + &row(&format!("file='{native}'"), "application/xml", "vibe-spec")
            + &row("directory='unrelated'", "application/xml", "vibe-spec");
        let root = host(&rows);
        write(
            root.path(),
            &native,
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><LAW fact=\"true\" status=\"impl/done\">law</LAW></p></spec>",
        );
        write(
            root.path(),
            &format!("{}/RULE.md", specs.display()),
            [0xff, 0],
        );
        write(root.path(), "unrelated/broken.xml", "<bad");
        let mut registry = seeded_registry(root.path(), &["RULE#LAW"]);
        assert_eq!(
            sync::reconcile(root.path(), &mut registry).unwrap().len(),
            1
        );
    }
    #[test]
    fn no_citations_is_read_free_and_no_group_is_required_only_at_address_operation() {
        let missing = std::path::Path::new("/nonexistent/nst-host-sync-no-citations");
        let mut registry = Registry::default();
        assert!(sync::reconcile(missing, &mut registry).unwrap().is_empty());
        let root = host(&row("file='A.md'", "text/markdown", "vibe-spec"));
        let manifest = root.path().join("vibe.toml");
        let text = fs::read_to_string(&manifest)
            .unwrap()
            .replace("group='org.example'\n", "");
        fs::write(&manifest, &text).unwrap();
        assert!(vibe_core::manifest::Manifest::parse_str(&text).is_ok());
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        let before = disk(root.path());
        assert!(sync::reconcile(root.path(), &mut registry).is_err());
        assert_eq!(disk(root.path()), before);
    }
    #[test]
    fn prepared_reconcile_refuses_manifest_and_file_drift_before_any_write() {
        for drift in ["manifest", "bytes", "native-member"] {
            let root = host(&row("directory='spec'", "text/markdown", "vibe-spec"));
            write(
                root.path(),
                "spec/A.md",
                "# A\n\n@fact:LAW law. @status:impl/done\n",
            );
            let mut registry = seeded_registry(root.path(), &["A#LAW"]);
            let memory = registry.entries().cloned().collect::<Vec<_>>();
            let before = disk(root.path());
            let snapshot = sync::load_spec_snapshot(
                root.path(),
                &sync::needed_doc_paths(&registry),
                &registry,
            )
            .unwrap();
            match drift {
                "manifest" => {
                    let path = root.path().join("vibe.toml");
                    fs::write(&path, fs::read_to_string(&path).unwrap() + "\n# changed\n").unwrap();
                }
                "bytes" => write(root.path(), "spec/A.md", "# different\n"),
                _ => write(root.path(), "spec/A.xml", "<invalid/>"),
            }
            assert!(
                sync::reconcile_snapshot(root.path(), &mut registry, snapshot).is_err(),
                "{drift}"
            );
            assert_eq!(registry.entries().cloned().collect::<Vec<_>>(), memory);
            assert_eq!(disk(root.path()), before);
        }
    }
    #[cfg(unix)]
    #[test]
    fn exact_only_ignores_unrelated_legacy_symlink_and_unrepresentable_name_is_not_rewritten() {
        use std::os::unix::fs::symlink;
        let specs = vibe_core::layout::current_specs_root();
        let native = format!("{}/A.md", specs.display());
        let root = host(&row(
            &format!("file='{native}'"),
            "text/markdown",
            "vibe-spec",
        ));
        write(
            root.path(),
            &native,
            "# A\n\n@fact:LAW law. @status:impl/done\n",
        );
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join("spec")).unwrap();
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        assert_eq!(
            sync::reconcile(root.path(), &mut registry).unwrap().len(),
            1
        );
        let root = host(&row("directory='spec'", "text/markdown", "vibe-spec"));
        write(
            root.path(),
            "spec/A.md",
            "# A\n\n@fact:LAW law. @status:impl/done\n",
        );
        write(root.path(), "spec/A\\evil.md", [0xff, 0]);
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        assert_eq!(
            sync::reconcile(root.path(), &mut registry).unwrap().len(),
            1
        );
    }
    #[cfg(unix)]
    #[test]
    fn shared_regular_sources_work_but_symlink_fifo_and_parent_replacement_refuse() {
        use std::os::unix::fs::symlink;
        let root = host(&row(
            "file='spec/common/A.md'",
            "text/markdown",
            "vibe-spec",
        ));
        write(
            root.path(),
            "original",
            "# A\n\n@fact:LAW law. @status:impl/done\n",
        );
        fs::create_dir_all(root.path().join("spec/common")).unwrap();
        fs::hard_link(
            root.path().join("original"),
            root.path().join("spec/common/A.md"),
        )
        .unwrap();
        let mut registry = seeded_registry(root.path(), &["common/A#LAW"]);
        assert_eq!(
            sync::reconcile(root.path(), &mut registry).unwrap().len(),
            1
        );
        let snapshot =
            sync::load_spec_snapshot(root.path(), &sync::needed_doc_paths(&registry), &registry)
                .unwrap();
        let before = disk(root.path());
        fs::rename(
            root.path().join("spec/common"),
            root.path().join("spec/old"),
        )
        .unwrap();
        fs::create_dir(root.path().join("spec/common")).unwrap();
        fs::hard_link(
            root.path().join("original"),
            root.path().join("spec/common/A.md"),
        )
        .unwrap();
        assert!(sync::reconcile_snapshot(root.path(), &mut registry, snapshot).is_err());
        assert_eq!(disk(root.path()), before);
        fs::remove_file(root.path().join("spec/common/A.md")).unwrap();
        symlink(
            root.path().join("original"),
            root.path().join("spec/common/A.md"),
        )
        .unwrap();
        assert!(sync::reconcile(root.path(), &mut registry).is_err());
        fs::remove_file(root.path().join("spec/common/A.md")).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(root.path().join("spec/common/A.md"))
                .status()
                .unwrap()
                .success()
        );
        assert!(sync::reconcile(root.path(), &mut registry).is_err());
        assert_eq!(disk(root.path()), before);
    }

    #[test]
    fn prepared_root_replacement_refuses_and_unrelated_raw_addition_does_not() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("project");
        fs::create_dir(&root).unwrap();
        write(
            &root,
            "vibe.toml",
            format!(
                "[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n{}",
                row("file='A.md'", "text/markdown", "vibe-spec")
            ),
        );
        write(&root, "A.md", "# A\n\n@fact:LAW law. @status:impl/done\n");
        let mut registry = seeded_registry(&root, &["A#LAW"]);
        let snapshot =
            sync::load_spec_snapshot(&root, &sync::needed_doc_paths(&registry), &registry).unwrap();
        let memory = registry.entries().cloned().collect::<Vec<_>>();
        fs::rename(&root, outer.path().join("old")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(sync::reconcile_snapshot(&root, &mut registry, snapshot).is_err());
        assert_eq!(registry.entries().cloned().collect::<Vec<_>>(), memory);
        assert!(disk(&root).is_empty());
        let rows = row("directory='spec'", "text/markdown", "commonmark")
            + &row("file='spec/A.md'", "text/markdown", "vibe-spec");
        let root = host(&rows);
        write(
            root.path(),
            "spec/A.md",
            "# A\n\n@fact:LAW law. @status:impl/done\n",
        );
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        let snapshot =
            sync::load_spec_snapshot(root.path(), &sync::needed_doc_paths(&registry), &registry)
                .unwrap();
        write(root.path(), "spec/A.xml", [0xff, 0]);
        write(root.path(), "spec/unrelated.md", [0xff, 0]);
        assert_eq!(
            sync::reconcile_snapshot(root.path(), &mut registry, snapshot)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn legacy_to_declared_policy_drift_cannot_publish_a_legacy_snapshot() {
        let root = host("");
        let relative = format!("{}/A.md", vibe_core::layout::current_specs_root().display());
        write(
            root.path(),
            &relative,
            "# A\n\n@fact:LAW law. @status:impl/done\n",
        );
        let mut registry = seeded_registry(root.path(), &["A#LAW"]);
        let before = disk(root.path());
        let memory = registry.entries().cloned().collect::<Vec<_>>();
        let snapshot =
            sync::load_spec_snapshot(root.path(), &sync::needed_doc_paths(&registry), &registry)
                .unwrap();
        let manifest = root.path().join("vibe.toml");
        fs::write(
            &manifest,
            fs::read_to_string(&manifest).unwrap() + "sources=[]\n",
        )
        .unwrap();
        assert!(sync::reconcile_snapshot(root.path(), &mut registry, snapshot).is_err());
        assert_eq!(disk(root.path()), before);
        assert_eq!(registry.entries().cloned().collect::<Vec<_>>(), memory);
    }

    #[test]
    fn invalid_host_name_cannot_readdress_or_remove_a_valid_registry_citation() {
        let relative = format!(
            "{}/name/A.md",
            vibe_core::layout::current_specs_root().display()
        );
        let root = host(&row(
            &format!("file='{relative}'"),
            "text/markdown",
            "vibe-spec",
        ));
        let path = root.path().join("vibe.toml");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("name='demo'", "name='bad/name'");
        assert!(vibe_core::manifest::Manifest::parse_str(&text).is_err());
        fs::write(path, text).unwrap();
        write(
            root.path(),
            &relative,
            "# A\n\n@fact:LAW valid. @status:impl/done\n",
        );
        let address = "spec://org.example/bad/name/A#LAW";
        let mut registry = Registry::default();
        registry
            .upsert(
                root.path(),
                FactEntry::for_host(address, "org.example/bad", Some(status("spec/work")), None)
                    .unwrap(),
            )
            .unwrap();
        let before = disk(root.path());
        let memory = registry.entries().cloned().collect::<Vec<_>>();
        let result = sync::reconcile(root.path(), &mut registry);
        assert!(
            matches!(&result,Err(crate::RegistryError::SourcePolicy(error)) if error.kind==vibe_spec::source_policy::SourcePolicyErrorKind::InvalidManifest),
            "{result:?}; retained citation={}",
            registry.get(address).is_some()
        );
        assert_eq!(disk(root.path()), before);
        assert_eq!(registry.entries().cloned().collect::<Vec<_>>(), memory);
    }
}
