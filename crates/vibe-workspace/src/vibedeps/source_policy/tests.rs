#[cfg(test)]
mod cases {
    use super::super::*;
    use crate::vibedeps::{self, CopyMode};
    use std::fs;
    use vibe_core::{ContentHash, Group};

    const HEAD: &str =
        "[package]\ngroup='org.policy'\nname='sample'\nversion='1.0.0'\nkind='tool'\n";
    const ROW: &str = "[[package.sources]]\nfile='data.payload'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";

    fn source(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), text).unwrap();
        fs::write(dir.path().join("data.payload"), b"original\0payload\xff").unwrap();
        dir
    }
    fn hash() -> ContentHash {
        ContentHash::parse(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap()
    }
    fn slot(root: &Path) -> PathBuf {
        vibedeps::slot_abs_path(
            root,
            &Group::parse("org.policy").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
        )
    }
    fn run(
        root: &Path,
        src: &Path,
        format: SpecFormat,
        mode: CopyMode,
    ) -> Result<Vec<PathBuf>, WorkspaceError> {
        vibedeps::materialise_with_spec_format(
            root,
            &Group::parse("org.policy").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
            src,
            mode,
            format,
            &hash(),
        )
    }
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(base: &Path, here: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            if !here.exists() {
                return;
            }
            for row in fs::read_dir(here).unwrap() {
                let path = row.unwrap().path();
                if path.is_dir() {
                    walk(base, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(base).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(root, root, &mut files);
        files
    }

    #[test]
    fn declared_mixed_and_explicit_empty_keep_original_payload_and_record() {
        for manifest in [format!("{HEAD}{ROW}"), format!("{HEAD}sources=[]\n")] {
            let root = tempfile::tempdir().unwrap();
            let src = source(&manifest);
            let original = snapshot(src.path());
            run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).unwrap();
            let mut payload = snapshot(&slot(root.path()));
            payload.remove(Path::new(vibedeps::SLOT_RECORD_FILENAME));
            assert_eq!(payload, original);
            let record = vibedeps::read_slot_record(&slot(root.path())).unwrap();
            assert_eq!(record.spec_format, SpecFormat::Mixed);
            assert_eq!(record.files.len(), original.len());
            vibedeps::verify_recorded_files(&slot(root.path()), &record).unwrap();
            assert_eq!(snapshot(src.path()), original);
        }
    }

    #[test]
    fn direct_missing_manifest_and_valid_legacy_preserve_content_tree_compatibility() {
        for present in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let src = source(HEAD);
            if !present {
                fs::remove_file(src.path().join("vibe.toml")).unwrap();
            }
            let before = snapshot(src.path());
            run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).unwrap();
            let mut after = snapshot(&slot(root.path()));
            after.remove(Path::new(vibedeps::SLOT_RECORD_FILENAME));
            assert_eq!(after, before);
        }
    }

    #[test]
    fn malformed_missing_conflicting_and_unsupported_sources_refuse_before_slot_changes() {
        let cases = [
            ("[broken".to_owned(), SpecFormat::Mixed, false),
            ("x".to_owned(), SpecFormat::Mixed, false),
            ("v1".to_owned(), SpecFormat::Mixed, false),
            ("v2".to_owned(), SpecFormat::Mixed, false),
            ("[package]\n".to_owned(), SpecFormat::Mixed, false),
            (
                "[package]\nname='sample'\n".to_owned(),
                SpecFormat::Mixed,
                false,
            ),
            (format!("{HEAD}{ROW}"), SpecFormat::Mixed, true),
            (
                format!(
                    "{HEAD}{ROW}{}",
                    ROW.replace("application/octet-stream", "text/plain")
                ),
                SpecFormat::Mixed,
                false,
            ),
            (
                format!("{HEAD}sources=[]\n[boot_snippet]\nsource='data.payload'\n"),
                SpecFormat::Mixed,
                false,
            ),
            (
                format!("{HEAD}sources=[]\nmaterialization='in-place'\n"),
                SpecFormat::Mixed,
                false,
            ),
        ];
        for (manifest, format, missing) in cases {
            let root = tempfile::tempdir().unwrap();
            let src = source(&manifest);
            let target = slot(root.path());
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("sentinel"), b"old slot").unwrap();
            if missing {
                fs::remove_file(src.path().join("data.payload")).unwrap();
            }
            let before = snapshot(root.path());
            assert!(
                run(root.path(), src.path(), format, CopyMode::Copy).is_err(),
                "{manifest}"
            );
            assert_eq!(snapshot(root.path()), before, "{manifest}");
        }
    }

    #[test]
    fn known_declared_manifest_cannot_disappear_or_change_boot_and_requires() {
        let src = source(&format!("{HEAD}{ROW}"));
        let known = Manifest::read(src.path().join("vibe.toml")).unwrap();
        let root = tempfile::tempdir().unwrap();
        let refuse = || {
            let before = snapshot(root.path());
            let result = vibedeps::materialise_with_spec_format_report(
                root.path(),
                &Group::parse("org.policy").unwrap(),
                "sample",
                &"1.0.0".parse().unwrap(),
                src.path(),
                CopyMode::Copy,
                SpecFormat::Mixed,
                &hash(),
                Some(&known),
            );
            assert!(result.is_err());
            assert_eq!(snapshot(root.path()), before);
        };
        fs::remove_file(src.path().join("vibe.toml")).unwrap();
        refuse();
        for tail in [
            "[requires.packages]\n'org.legacy/dep'='=1.0.0'\n",
            "[boot]\ndefault_link='static'\n",
        ] {
            fs::write(src.path().join("vibe.toml"), format!("{HEAD}{ROW}{tail}")).unwrap();
            refuse();
        }
    }

    #[test]
    fn a3_transformed_raw_and_zero_match_directory_defaults_preserve_original_payload() {
        for (manifest, format) in [
            (format!("{HEAD}{ROW}"), SpecFormat::Xml),
            (format!("{HEAD}{ROW}"), SpecFormat::Markdown),
            (
                format!(
                    "{HEAD}{}",
                    ROW.replace("file='data.payload'", "directory='data'")
                ),
                SpecFormat::Mixed,
            ),
        ] {
            let source = source(&manifest);
            let root = tempfile::tempdir().unwrap();
            let before = snapshot(source.path());
            run(root.path(), source.path(), format, CopyMode::Copy).unwrap();
            let mut after = snapshot(&slot(root.path()));
            after.remove(Path::new(vibedeps::SLOT_RECORD_FILENAME));
            assert_eq!(after, before);
            assert_eq!(snapshot(source.path()), before);
            let record = vibedeps::read_slot_record(&slot(root.path())).unwrap();
            assert_eq!(record.spec_format, format);
            vibedeps::verify_recorded_files(&slot(root.path()), &record).unwrap();
        }
    }

    #[test]
    fn declared_legacy_dependencies_are_not_a_blanket_refusal() {
        let src = source(&format!(
            "{HEAD}{ROW}[requires.packages]\n'org.legacy/dep'='=1.0.0'\n"
        ));
        let root = tempfile::tempdir().unwrap();
        run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).unwrap();
    }

    #[test]
    fn direct_in_place_refuses_declared_default_copy_and_invalid_manifest_before_move() {
        for manifest in [
            format!("{HEAD}sources=[]\n"),
            format!("{HEAD}{ROW}"),
            "[broken".to_owned(),
        ] {
            let src = source(&manifest);
            let root = tempfile::tempdir().unwrap();
            fs::create_dir(src.path().join(".git")).unwrap();
            fs::write(src.path().join(".git/HEAD"), b"fixture").unwrap();
            let target = vibedeps::in_place_slot_abs_path(
                root.path(),
                &Group::parse("org.policy").unwrap(),
                "sample",
            );
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("sentinel"), b"old").unwrap();
            let before_root = snapshot(root.path());
            let before_source = snapshot(src.path());
            assert!(
                vibedeps::materialise_in_place(
                    root.path(),
                    &Group::parse("org.policy").unwrap(),
                    "sample",
                    src.path()
                )
                .is_err()
            );
            assert_eq!(snapshot(root.path()), before_root);
            assert_eq!(snapshot(src.path()), before_source);
        }
    }

    #[test]
    fn changed_manifest_or_captured_source_refuses_before_reconcile() {
        for path in ["vibe.toml", "data.payload"] {
            let src = source(&format!("{HEAD}{ROW}"));
            let policy = SourcePolicy::observe(src.path(), SpecFormat::Mixed, None).unwrap();
            let captured = policy.captured(Path::new(path)).unwrap();
            fs::write(src.path().join(path), b"changed").unwrap();
            assert!(policy.recheck().is_err());
            assert_ne!(captured.bytes(), b"changed");
        }
    }

    #[test]
    fn prepared_copy_uses_captured_bytes_even_when_original_name_changes_later() {
        let src = source(&format!("{HEAD}{ROW}"));
        let root = tempfile::tempdir().unwrap();
        let original = snapshot(src.path());
        let policy = SourcePolicy::observe(src.path(), SpecFormat::Mixed, None).unwrap();
        let files =
            vibedeps::slot_diff::prepare_source_tree(src.path(), CopyMode::Copy, &policy).unwrap();
        policy.validate_prepared(&files).unwrap();
        fs::write(src.path().join("data.payload"), b"later bytes").unwrap();
        assert!(
            policy.recheck().is_err(),
            "public entry must refuse this drift before reconcile"
        );
        let record = vibedeps::SlotRecord {
            schema: vibedeps::SLOT_RECORD_SCHEMA,
            source_hash: hash(),
            spec_format: SpecFormat::Mixed,
            converter_recipe: None,
            derived_hash: None,
            overlay_hash: None,
            files: files
                .iter()
                .map(|file| vibedeps::SlotFile {
                    path: file.path_wire(),
                    sha256: file.sha256().to_owned(),
                    source: None,
                    disposition: None,
                })
                .collect(),
        };
        // Isolate the lower prepared-copy invariant from the public preflight:
        // these already captured bytes must never be replaced by a reopen.
        vibedeps::slot_diff::reconcile_slot(&slot(root.path()), &files, &record).unwrap();
        let mut result = snapshot(&slot(root.path()));
        result.remove(Path::new(vibedeps::SLOT_RECORD_FILENAME));
        assert_eq!(result, original);
        assert_eq!(
            fs::read(src.path().join("data.payload")).unwrap(),
            b"later bytes"
        );
    }

    #[test]
    fn exact_reserved_record_alias_refuses_with_destination_named() {
        let src = source(&format!("{HEAD}{ROW}"));
        let root = tempfile::tempdir().unwrap();
        fs::write(src.path().join(".VIBE-SLOT.TOML"), b"authored collision").unwrap();
        let error = run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).unwrap_err();
        assert!(error.to_string().contains(".vibe-slot.toml"), "{error}");
        assert!(!slot(root.path()).exists());
    }

    #[cfg(unix)]
    #[test]
    fn legacy_uppercase_record_name_remains_an_unclassified_payload_file() {
        for manifest in [false, true] {
            let src = source(HEAD);
            let root = tempfile::tempdir().unwrap();
            if !manifest {
                fs::remove_file(src.path().join("vibe.toml")).unwrap();
            }
            fs::write(src.path().join(".VIBE-SLOT.TOML"), b"legacy data").unwrap();
            run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).unwrap();
            assert_eq!(
                fs::read(slot(root.path()).join(".VIBE-SLOT.TOML")).unwrap(),
                b"legacy data"
            );
            vibedeps::read_slot_record(&slot(root.path())).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_declared_file_and_manifest_refuse_without_destination_writes() {
        for path in ["vibe.toml", "data.payload"] {
            let src = source(&format!("{HEAD}{ROW}"));
            let outside = source(&format!("{HEAD}{ROW}"));
            fs::remove_file(src.path().join(path)).unwrap();
            std::os::unix::fs::symlink(outside.path().join(path), src.path().join(path)).unwrap();
            let root = tempfile::tempdir().unwrap();
            assert!(run(root.path(), src.path(), SpecFormat::Mixed, CopyMode::Copy).is_err());
            assert!(!slot(root.path()).exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn declared_executable_mode_and_requested_hardlink_identity_survive() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        for mode in [CopyMode::Copy, CopyMode::Hardlink] {
            let src = source(&format!("{HEAD}{ROW}"));
            let root = tempfile::tempdir().unwrap();
            let input = src.path().join("data.payload");
            fs::set_permissions(&input, fs::Permissions::from_mode(0o555)).unwrap();
            let before = fs::metadata(&input).unwrap();
            run(root.path(), src.path(), SpecFormat::Mixed, mode).unwrap();
            let output = fs::metadata(slot(root.path()).join("data.payload")).unwrap();
            assert_eq!(output.permissions().mode() & 0o777, 0o555);
            assert_eq!(
                fs::metadata(&input).unwrap().permissions().mode() & 0o777,
                0o555
            );
            if mode == CopyMode::Hardlink {
                assert_eq!((before.dev(), before.ino()), (output.dev(), output.ino()));
            } else {
                assert_ne!((before.dev(), before.ino()), (output.dev(), output.ino()));
            }
        }
    }
}
