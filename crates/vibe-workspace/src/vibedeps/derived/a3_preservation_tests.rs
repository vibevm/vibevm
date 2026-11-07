#[cfg(test)]
mod cases {
    use super::super::*;

    fn source(path: &str, interpretation: &str, kind: &str, bytes: &[u8]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let (media, role) = match interpretation {
            "vibe-spec" => ("text/markdown", "spec"),
            "vibe-doc" => ("application/xml", "documentation"),
            _ => ("text/markdown", "documentation"),
        };
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
        fs::write(root.path().join("vibe.toml"), format!("[package]\ngroup='org.preserve'\nname='sample'\nversion='1.0.0'\nkind='{kind}'\n{doc}[[package.sources]]\nfile='{path}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='scan'\n{subject}")).unwrap();
        vibe_core::manifest::Manifest::read(root.path().join("vibe.toml")).unwrap();
        let file = root.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, bytes).unwrap();
        root
    }
    fn install(
        root: &Path,
        source: &Path,
        format: SpecFormat,
        mode: CopyMode,
    ) -> Result<Vec<PathBuf>, WorkspaceError> {
        super::super::materialise_with_spec_format(
            root,
            &Group::parse("org.preserve").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
            source,
            mode,
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
            &Group::parse("org.preserve").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
        )
    }

    #[cfg(unix)]
    #[test]
    fn m_06_a_3_raw_unicode_mode_and_hardlink_survive_every_target() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        for format in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
            for mode in [CopyMode::Copy, CopyMode::Hardlink] {
                let source = source("docs/Привет.md", "commonmark", "flow", b"raw\xff\0\r\n");
                let root = tempfile::tempdir().unwrap();
                let input = source.path().join("docs/Привет.md");
                fs::set_permissions(&input, fs::Permissions::from_mode(0o555)).unwrap();
                let before = fs::metadata(&input).unwrap();
                install(root.path(), source.path(), format, mode).unwrap();
                let output = slot(root.path()).join("docs/Привет.md");
                let after = fs::metadata(&output).unwrap();
                assert_eq!(fs::read(output).unwrap(), b"raw\xff\0\r\n");
                assert_eq!(after.permissions().mode() & 0o777, 0o555);
                assert_eq!(
                    fs::metadata(&input).unwrap().permissions().mode() & 0o777,
                    0o555
                );
                if mode == CopyMode::Hardlink {
                    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
                } else {
                    assert_ne!((before.dev(), before.ino()), (after.dev(), after.ino()));
                }
            }
        }
    }
    #[test]
    fn m_06_a_3_native_doc_mixed_and_xml_keep_doc_vocabulary() {
        let body = "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Doc</title><note kind=\"warning\">Careful.</note></spec>";
        let original =
            vibe_specdoc::from_xml_with(body, vibe_specdoc::doc::Vocabulary::Doc).unwrap();
        for format in [SpecFormat::Mixed, SpecFormat::Xml] {
            let source = source("docs/page.blob", "vibe-doc", "doc", body.as_bytes());
            let root = tempfile::tempdir().unwrap();
            install(root.path(), source.path(), format, CopyMode::Copy).unwrap();
            let output = slot(root.path()).join(if format == SpecFormat::Mixed {
                "docs/page.blob"
            } else {
                "docs/page.xml"
            });
            let output = fs::read_to_string(output).unwrap();
            assert_eq!(
                vibe_specdoc::from_xml_with(&output, vibe_specdoc::doc::Vocabulary::Doc).unwrap(),
                original
            );
            if format == SpecFormat::Mixed {
                assert_eq!(output, body);
            }
        }
    }
    #[test]
    fn m_06_a_3_native_uri_delimiters_refuse_without_rewriting_raw_names() {
        for path in ["spec/hash#item.md", "spec/unit~r2.md"] {
            let raw = source(path, "commonmark", "flow", b"# Native-shaped raw\n");
            let raw_root = tempfile::tempdir().unwrap();
            install(raw_root.path(), raw.path(), SpecFormat::Xml, CopyMode::Copy).unwrap();
            assert_eq!(
                fs::read(slot(raw_root.path()).join(path)).unwrap(),
                b"# Native-shaped raw\n"
            );
            let native = source(path, "vibe-spec", "flow", b"# Native\n");
            let root = tempfile::tempdir().unwrap();
            if path.contains('#') {
                assert!(
                    install(root.path(), native.path(), SpecFormat::Xml, CopyMode::Copy).is_err()
                );
                assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
            } else {
                // ~r is literal in a doc path; only a fragment can pin a revision.
                install(root.path(), native.path(), SpecFormat::Xml, CopyMode::Copy).unwrap();
                assert!(slot(root.path()).join("spec/unit~r2.xml").is_file());
            }
        }
    }
}
