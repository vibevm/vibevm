//! Actual installed-map composition, rather than caller-built media rows.

use std::fs;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};
use vibe_spec::{FileResolver, FsSectionSource, SelfCoordinate, SpecAddress};

fn installed_reader(
    format: SpecFormat,
    file: &str,
    media: &str,
    interpretation: &str,
    body: &str,
) -> (tempfile::TempDir, FsSectionSource) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let host = root.path().join("host");
    fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
    fs::create_dir_all(&host).unwrap();
    let mut authored = format!(
        "[package]\ngroup='org.demo'\nname='selected'\nkind='{}'\nversion='1.0.0'\ntitle='Selected documentation'\nabstract='Fixture for retained source interpretation.'\n\
         [[package.sources]]\nfile='vibevm/vibespecs/{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{}'\nloading='explicit'\n\
         [[package.sources]]\nfile='vibevm/vibespecs/ENTRY.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
        if interpretation == "vibe-doc" {
            "doc"
        } else {
            "flow"
        },
        if interpretation == "vibe-doc" {
            "documentation"
        } else {
            "spec"
        }
    );
    if interpretation == "vibe-doc" {
        authored.push_str("\n[[documents]]\npackage='org.demo/subject'\nversion='^1.0'\n");
    }
    fs::write(source.join("vibe.toml"), authored).unwrap();
    fs::write(source.join("vibevm/vibespecs").join(file), body).unwrap();
    fs::write(
        source.join("vibevm/vibespecs/ENTRY.md"),
        "# Entry {#entry}\n\n#embed spec://org.demo/selected/DOC#solution\n",
    )
    .unwrap();
    let group: Group = "org.demo".parse().unwrap();
    let version = "1.0.0".parse().unwrap();
    // An independently supplied selection row, not an original-source proof.
    let pin = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    crate::vibedeps::materialise_with_spec_format(
        &host,
        &group,
        "selected",
        &version,
        &source,
        crate::vibedeps::CopyMode::Copy,
        format,
        &pin,
    )
    .unwrap();
    let slot = crate::vibedeps::slot_abs_path(&host, &group, "selected", &version);
    let map = crate::vibedeps::read_validated_descriptor_map(
        &slot, &group, "selected", &version, &pin, format,
    )
    .unwrap()
    .unwrap();
    let reader = FsSectionSource::new(
        FileResolver::new(
            &slot,
            SelfCoordinate::new(Some("org.demo".into()), "selected".into()),
        )
        .with_descriptor_source_map(slot.join("vibevm/vibespecs"), Some(map))
        .unwrap(),
    );
    (root, reader)
}

#[test]
fn factory_mixed_misleading_suffix_and_converted_tables_reach_compiler() {
    let md = "# D {#root}\n\n## Solution {#solution}\n\n| Header | Value |\n| --- | --- |\n| @fact:CC-A-4-1 One. @status:impl/done | @fact:CC-A-4-2 Two. @status:impl/done |\n";
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap());
    for (format, file, media, body) in [
        (SpecFormat::Mixed, "DOC.xml", "text/markdown", md),
        (SpecFormat::Mixed, "DOC.md", "application/xml", xml.as_str()),
        (SpecFormat::Xml, "DOC.md", "text/markdown", md),
    ] {
        let (_root, reader) = installed_reader(format, file, media, "vibe-spec", body);
        let compiled = vibe_spec::compile_static(
            &SpecAddress::parse("spec://org.demo/selected/ENTRY#entry").unwrap(),
            &reader,
        )
        .unwrap();
        assert_eq!(compiled.matches("@fact:CC-A-4-1").count(), 1);
        assert_eq!(compiled.matches("@fact:CC-A-4-2").count(), 1);
        assert!(!compiled.contains("#embed"));
    }
}

#[test]
fn factory_mixed_doc_vocabulary_reaches_compiler_without_suffix_guess() {
    let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"solution\" title=\"Solution\"><p><p01 fact=\"true\">One.</p01></p><note kind=\"tip\">Tip.</note></section></spec>";
    let (_root, reader) = installed_reader(
        SpecFormat::Mixed,
        "DOC.md",
        "application/xml",
        "vibe-doc",
        xml,
    );
    let compiled = vibe_spec::compile_static(
        &SpecAddress::parse("spec://org.demo/selected/ENTRY#entry").unwrap(),
        &reader,
    )
    .unwrap();
    assert!(compiled.contains("One."));
    assert!(compiled.contains("Tip."));
}

#[test]
fn factory_copied_unconverted_keeps_markdown_and_structured_refusal() {
    let bad = "# D {#root}\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";
    let (root, reader) =
        installed_reader(SpecFormat::Xml, "DOC.md", "text/markdown", "vibe-spec", bad);
    let slot = crate::vibedeps::slot_abs_path(
        &root.path().join("host"),
        &"org.demo".parse().unwrap(),
        "selected",
        &"1.0.0".parse().unwrap(),
    );
    let output = "vibevm/vibespecs/DOC.md";
    assert_eq!(fs::read(slot.join(output)).unwrap(), bad.as_bytes());
    let record = crate::vibedeps::read_slot_record(&slot).unwrap();
    assert_eq!(record.spec_format, SpecFormat::Xml);
    let row = record.files.iter().find(|row| row.path == output).unwrap();
    assert_eq!(row.source.as_deref(), Some(output));
    let Some(crate::vibedeps::SlotFileDisposition::CopiedUnconverted(failure)) = &row.disposition
    else {
        panic!("expected retained failed conversion: {:?}", row.disposition);
    };
    assert_eq!(failure.issues.len(), 1);
    assert_eq!(failure.issues[0].line, 5);
    assert!(failure.issues[0].message.starts_with("standalone marker"));
    let error = vibe_spec::compile_static(
        &SpecAddress::parse("spec://org.demo/selected/ENTRY#entry").unwrap(),
        &reader,
    )
    .unwrap_err();
    assert!(error.to_string().contains("standalone marker"), "{error}");
    assert!(error.to_string().contains("line 5"), "{error}");
}
