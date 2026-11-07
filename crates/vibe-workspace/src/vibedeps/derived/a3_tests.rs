#[cfg(test)]
mod cases {
    use super::super::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn row(selector: &str, media: &str, interpretation: &str) -> String {
        let role = if interpretation == "vibe-spec" {
            "spec"
        } else if interpretation == "commonmark" || interpretation == "vibe-doc" {
            "documentation"
        } else {
            "resource"
        };
        format!(
            "[[package.sources]]\n{selector}\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='scan'\n"
        )
    }
    fn source(rows: &str, kind: &str) -> TempDir {
        let source = TempDir::new().unwrap();
        let doc = if kind == "doc" {
            "title='Documentation'\nabstract='A bounded documentation fixture.'\n"
        } else {
            ""
        };
        let subject = if kind == "doc" {
            "\n[[documents]]\npackage='org.subject/tool'\nversion='^1.0'\n"
        } else {
            ""
        };
        fs::write(
            source.path().join("vibe.toml"),
            format!(
                "[package]\ngroup='org.a3'\nname='sample'\nversion='1.0.0'\nkind='{kind}'\n{doc}{rows}{subject}"
            ),
        )
        .unwrap();
        vibe_core::manifest::Manifest::read(source.path().join("vibe.toml")).unwrap();
        source
    }
    fn write(root: &Path, path: &str, bytes: &[u8]) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn install(
        root: &Path,
        source: &Path,
        format: SpecFormat,
    ) -> Result<Vec<PathBuf>, WorkspaceError> {
        super::super::materialise_with_spec_format(
            root,
            &Group::parse("org.a3").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
            source,
            CopyMode::Copy,
            format,
            &ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
        )
    }
    fn slot(root: &Path) -> PathBuf {
        slot_abs_path(
            root,
            &Group::parse("org.a3").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
        )
    }
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, directory: &Path, found: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for item in fs::read_dir(directory).unwrap() {
                let path = item.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, found);
                } else {
                    found.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut found = BTreeMap::new();
        walk(root, root, &mut found);
        found
    }

    #[test]
    fn m_06_a_3_raw_same_stem_invalid_and_non_utf8_keep_original_bytes_all_targets() {
        for target in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
            let md = path_to_slash(&layout::current_specs_root().join("docs/pair.md"));
            let xml = path_to_slash(&layout::current_specs_root().join("docs/pair.xml"));
            let invalid = path_to_slash(&layout::current_specs_root().join("docs/invalid.xml"));
            let unclassified = path_to_slash(&layout::current_specs_root().join("unclassified.md"));
            let rows = row(&format!("file='{md}'"), "text/markdown", "commonmark")
                + &row(&format!("file='{xml}'"), "application/xml", "xml-data")
                + &row(&format!("file='{invalid}'"), "application/xml", "xml-data")
                + &row("file='opaque.xml'", "application/octet-stream", "opaque");
            let source = source(&rows, "flow");
            let root = TempDir::new().unwrap();
            write(source.path(), &md, b"# Ordinary\r\n<!-- intact -->\r\n");
            // A syntactically native-looking XML payload is still XML-data.
            write(source.path(), &xml, b"<!-- intact -->\r\n<spec xmlns=\"https://vibevm.org/spec/1\"><title>Raw data</title><p> keep </p></spec>\r\n");
            write(source.path(), &invalid, b"\xff\0<?xml broken <data>");
            write(source.path(), "opaque.xml", b"\0\xfeopaque");
            write(
                source.path(),
                &unclassified,
                b"# Unclassified native-looking heading\r\nuntouched\r\n",
            );
            let before = snapshot(source.path());
            install(root.path(), source.path(), target).unwrap();
            let mut after = snapshot(&slot(root.path()));
            after.remove(Path::new(SLOT_RECORD_FILENAME));
            assert_eq!(before, after, "{target:?}");
            assert_eq!(snapshot(source.path()), before);
            let record = read_slot_record(&slot(root.path())).unwrap();
            assert!(record.files.iter().all(|file| target == SpecFormat::Mixed
                || file.disposition == Some(SlotFileDisposition::Copied)));
            if target != SpecFormat::Mixed {
                assert_eq!(record.converter_recipe.as_deref(), Some("specdoc/4"));
            }
        }
    }

    #[test]
    fn m_06_a_3_native_declared_media_beats_suffix_and_preserves_original_row_identity() {
        let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Native</title><p>body</p></spec>";
        let md = "# Native {#root}\n\nbody\n";
        for (source_path, media, body, target, expected) in [
            (
                "spec/native.md",
                "application/xml",
                xml,
                SpecFormat::Markdown,
                vibe_specdoc::to_markdown(&vibe_specdoc::from_xml(xml).unwrap()),
            ),
            (
                "spec/native.xml",
                "text/markdown",
                md,
                SpecFormat::Xml,
                vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap()),
            ),
            (
                "spec/native.blob",
                "text/markdown",
                md,
                SpecFormat::Xml,
                vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap()),
            ),
        ] {
            let source = source(
                &row(&format!("file='{source_path}'"), media, "vibe-spec"),
                "flow",
            );
            let root = TempDir::new().unwrap();
            write(source.path(), source_path, body.as_bytes());
            install(root.path(), source.path(), target).unwrap();
            let output = Path::new(source_path).with_extension(if target == SpecFormat::Xml {
                "xml"
            } else {
                "md"
            });
            assert_eq!(
                fs::read(slot(root.path()).join(&output)).unwrap(),
                expected.as_bytes()
            );
            let record = read_slot_record(&slot(root.path())).unwrap();
            let retained = record
                .files
                .iter()
                .find(|file| file.path == path_to_slash(&output))
                .unwrap();
            assert_eq!(retained.source.as_deref(), Some(source_path));
            assert_eq!(retained.disposition, Some(SlotFileDisposition::Converted));
            assert_eq!(record.converter_recipe.as_deref(), Some("specdoc/4"));
        }
    }

    #[test]
    fn m_06_a_3_actual_output_and_original_native_address_collisions_refuse_before_writes() {
        for (second_kind, target) in [
            ("xml-data", SpecFormat::Xml),
            ("vibe-spec", SpecFormat::Mixed),
        ] {
            let rows = row("file='spec/pair.md'", "text/markdown", "vibe-spec")
                + &row("file='spec/pair.xml'", "application/xml", second_kind);
            let source = source(&rows, "flow");
            let root = TempDir::new().unwrap();
            write(source.path(), "spec/pair.md", b"# Native\n");
            write(
                source.path(),
                "spec/pair.xml",
                b"<spec xmlns=\"https://vibevm.org/spec/1\"><title>Pair</title></spec>",
            );
            fs::write(root.path().join("sentinel"), b"untouched").unwrap();
            let before = snapshot(root.path());
            let error = install(root.path(), source.path(), target).unwrap_err();
            assert!(error.to_string().contains("collide"), "{error}");
            assert_eq!(snapshot(root.path()), before);
        }
    }

    #[test]
    fn m_06_a_3_directory_defaults_overrides_exclusions_and_empty_inventory_are_usable() {
        let rows = row("directory='docs'", "text/markdown", "commonmark")
            + &row("file='docs/native.weird'", "text/markdown", "vibe-spec")
            + &row("directory='empty'", "application/octet-stream", "opaque");
        let source = source(&rows, "flow");
        let root = TempDir::new().unwrap();
        write(source.path(), "docs/data.md", b"ordinary\xff");
        write(source.path(), "docs/data.xml", b"<broken data>");
        write(source.path(), "docs/native.weird", b"# Native\n");
        write(source.path(), "docs/.git/config", b"excluded metadata");
        write(
            source.path(),
            &path_to_slash(&layout::current_boot_static_xml()),
            b"generated poison",
        );
        fs::create_dir(source.path().join("empty")).unwrap();
        install(root.path(), source.path(), SpecFormat::Xml).unwrap();
        assert_eq!(
            fs::read(slot(root.path()).join("docs/data.md")).unwrap(),
            b"ordinary\xff"
        );
        assert_eq!(
            fs::read(slot(root.path()).join("docs/data.xml")).unwrap(),
            b"<broken data>"
        );
        assert!(slot(root.path()).join("docs/native.xml").exists());
        assert!(!slot(root.path()).join("docs/.git").exists());
        assert!(
            !slot(root.path())
                .join(layout::current_boot_static_xml())
                .exists()
        );
    }

    #[test]
    fn m_06_a_3_invalid_native_and_one_way_doc_target_are_honest_refusals() {
        for (kind, interpretation, body, target) in [
            (
                "flow",
                "vibe-spec",
                b"\xffnot XML".as_slice(),
                SpecFormat::Xml,
            ),
            (
                "flow",
                "vibe-spec",
                b"<invalid/>".as_slice(),
                SpecFormat::Markdown,
            ),
            (
                "doc",
                "vibe-doc",
                b"<spec xmlns=\"https://vibevm.org/spec/1\"><title>Doc</title></spec>".as_slice(),
                SpecFormat::Markdown,
            ),
        ] {
            let source = source(
                &row("file='spec/native.any'", "application/xml", interpretation),
                kind,
            );
            let root = TempDir::new().unwrap();
            write(source.path(), "spec/native.any", body);
            let before = snapshot(root.path());
            assert!(install(root.path(), source.path(), target).is_err());
            assert_eq!(snapshot(root.path()), before);
        }
    }

    #[test]
    fn m_06_a_3_declared_native_reuses_exact_legacy_emitter_recipe() {
        for target in [SpecFormat::Markdown, SpecFormat::Xml] {
            let (path, media, body) = if target == SpecFormat::Xml {
                (
                    "spec/entry.md",
                    "text/markdown",
                    "# Heading {#root}\n\ntext\n",
                )
            } else {
                (
                    "spec/entry.xml",
                    "application/xml",
                    "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Heading</title><p>text</p></spec>",
                )
            };
            let path = path_to_slash(
                &layout::current_specs_root().join(Path::new(path).file_name().unwrap()),
            );
            let declared = source(&row(&format!("file='{path}'"), media, "vibe-spec"), "flow");
            let legacy = source("", "flow");
            write(declared.path(), &path, body.as_bytes());
            write(legacy.path(), &path, body.as_bytes());
            let target_a = TempDir::new().unwrap();
            let target_b = TempDir::new().unwrap();
            install(target_a.path(), declared.path(), target).unwrap();
            install(target_b.path(), legacy.path(), target).unwrap();
            let output = Path::new(&path).with_extension(if target == SpecFormat::Xml {
                "xml"
            } else {
                "md"
            });
            assert_eq!(
                fs::read(slot(target_a.path()).join(&output)).unwrap(),
                fs::read(slot(target_b.path()).join(&output)).unwrap()
            );
            assert_eq!(CONVERTER_RECIPE, "specdoc/4");
        }
    }
}
