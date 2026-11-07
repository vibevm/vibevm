//! Actual transformed installation consumed by the requirements query.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-QUERY-READ",
    r = 1
);
use super::*;
use crate::tests_query::{ctx, find_source, project, write_lock};
use std::fs;
use vibe_core::manifest::SpecFormat;
use vibe_wire::generated::requirements_report::SourceResultState;

pub(crate) fn installed_fixture(format: SpecFormat) -> tempfile::TempDir {
    let root = project("# Host\n\n##HOST host.\n");
    let source = tempfile::tempdir().unwrap();
    let relative = vibe_core::layout::current_specs_root().join("rules/RULE.payload");
    let native = source.path().join(&relative);
    fs::create_dir_all(native.parent().unwrap()).unwrap();
    fs::write(&native, "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Rule</title><p><REAL fact=\"true\" status=\"spec/done\">actual fact</REAL></p></spec>").unwrap();
    fs::write(source.path().join("ordinary.md"), "##FAKE not a fact\n").unwrap();
    fs::write(source.path().join("ordinary.xml"), [0xff, 0, 60, 33]).unwrap();
    fs::write(source.path().join("vibe.toml"), format!("[package]\nkind='feat'\ngroup='org.example'\nname='pkg'\nversion='1.0.0'\n[[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[package.sources]]\nfile='ordinary.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n",relative.to_str().unwrap())).unwrap();
    let manifest = source.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[[package.sources]]\nfile='ordinary.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n");
    fs::write(manifest, text).unwrap();
    let hash = vibe_core::ContentHash::from_validated(format!("sha256:{}", "a".repeat(64)));
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &vibe_core::Group::parse("org.example").unwrap(),
        "pkg",
        &"1.0.0".parse().unwrap(),
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        format,
        &hash,
    )
    .unwrap();
    write_lock(root.path(), false);
    root
}
fn report(
    root: &std::path::Path,
    format: Option<SpecFormat>,
) -> vibe_wire::generated::requirements_report::RequirementsReport {
    let mut context = ctx(root);
    context.materialized_format = format;
    query(
        &RequirementsQuery::try_new(None, 256, false).unwrap(),
        &context,
        None,
    )
    .unwrap()
}
fn slot(root: &std::path::Path) -> std::path::PathBuf {
    vibe_workspace::vibedeps::slot_abs_path(
        root,
        &vibe_core::Group::parse("org.example").unwrap(),
        "pkg",
        &"1.0.0".parse().unwrap(),
    )
}
#[test]
fn installed_declared_package_query_preserves_original_addresses_across_formats() {
    for format in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
        let root = installed_fixture(format);
        let mut context = ctx(root.path());
        context.materialized_format = Some(format);
        let report = query(
            &RequirementsQuery::try_new(None, 256, false).unwrap(),
            &context,
            None,
        )
        .unwrap();
        assert_eq!(
            find_source(&report, "org.example/pkg").state,
            SourceResultState::Available,
            "{format:?}: {report:?}"
        );
        assert!(
            report
                .rows
                .iter()
                .any(|row| row.address == "spec://org.example/pkg/rules/RULE.payload#REAL"),
            "{format:?}: {report:?}"
        );
        assert!(!report.rows.iter().any(|row| row.address.contains("FAKE")));
    }
}

#[test]
fn missing_format_and_mismatched_lock_or_record_are_unavailable_without_digest() {
    let root = installed_fixture(SpecFormat::Xml);
    for format in [None, Some(SpecFormat::Mixed)] {
        let report = report(root.path(), format);
        let source = find_source(&report, "org.example/pkg");
        assert_eq!(source.state, SourceResultState::Unavailable);
        assert!(source.digest.is_none());
        assert_eq!(
            source.reason_code.as_deref(),
            Some("installed-source-binding-unavailable")
        );
    }
    let lock = root.path().join("vibe.lock");
    let original = fs::read_to_string(&lock).unwrap();
    fs::write(&lock, original.replace(&"a".repeat(64), &"b".repeat(64))).unwrap();
    assert_eq!(
        find_source(
            &report(root.path(), Some(SpecFormat::Xml)),
            "org.example/pkg"
        )
        .state,
        SourceResultState::Unavailable
    );
    let without_hash = original
        .lines()
        .filter(|line| !line.trim_start().starts_with("content_hash"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&lock, without_hash).unwrap();
    let mut context = ctx(root.path());
    context.materialized_format = Some(SpecFormat::Xml);
    assert!(matches!(
        query(
            &RequirementsQuery::try_new(None, 256, false).unwrap(),
            &context,
            None
        ),
        Err(QueryError::Lock { .. })
    ));
    fs::write(&lock, original).unwrap();
    fs::remove_file(slot(root.path()).join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME))
        .unwrap();
    assert_eq!(
        find_source(
            &report(root.path(), Some(SpecFormat::Xml)),
            "org.example/pkg"
        )
        .state,
        SourceResultState::Unavailable
    );
}
#[test]
fn syntax_invalid_retains_complete_materialized_witness_digest_but_tampering_does_not() {
    let root = installed_fixture(SpecFormat::Mixed);
    let slot = slot(root.path());
    let relative = vibe_core::machine_json_path(
        &vibe_core::layout::current_specs_root().join("rules/RULE.payload"),
    );
    let malformed = b"<spec><unfinished";
    fs::write(slot.join(&relative), malformed).unwrap();
    let unavailable = report(root.path(), Some(SpecFormat::Mixed));
    assert_eq!(
        find_source(&unavailable, "org.example/pkg").state,
        SourceResultState::Unavailable
    );
    assert!(
        find_source(&unavailable, "org.example/pkg")
            .digest
            .is_none()
    );
    let mut record = vibe_workspace::vibedeps::read_slot_record(&slot).unwrap();
    record
        .files
        .iter_mut()
        .find(|row| row.path == relative)
        .unwrap()
        .sha256 = vibe_workspace::vibedeps::sha256_bytes(malformed);
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    let invalid = report(root.path(), Some(SpecFormat::Mixed));
    let source = find_source(&invalid, "org.example/pkg");
    assert_eq!(source.state, SourceResultState::Invalid);
    let mut witnesses = vec![
        vibe_facts::SourceFileWitness::of(&relative, malformed),
        vibe_facts::SourceFileWitness::of("ordinary.md", b"##FAKE not a fact\n"),
    ];
    witnesses.push(vibe_facts::SourceFileWitness::of(
        "ordinary.xml",
        &[0xff, 0, 60, 33],
    ));
    witnesses.sort_by(|a, b| a.path.cmp(&b.path));
    let expected = crate::digest::source_result_digest(
        &vibe_wire::generated::requirements_report::RequirementSourceKind::Package,
        "org.example/pkg",
        &witnesses,
    );
    assert_eq!(source.digest.as_deref(), Some(expected.as_str()));
    assert!(
        !invalid
            .rows
            .iter()
            .any(|row| row.source.package == "org.example/pkg")
    );
}

#[test]
fn installed_spec_and_doc_fact_metadata_matches_original_native_observation() {
    for kind in ["feat", "doc"] {
        for format in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
            if kind == "doc" && format == SpecFormat::Markdown {
                continue;
            }
            let root = project("# Host\n");
            let source = tempfile::tempdir().unwrap();
            let relative = vibe_core::machine_json_path(
                &vibe_core::layout::current_specs_root().join("rules/PARITY.payload"),
            );
            let interpretation = if kind == "doc" {
                "vibe-doc"
            } else {
                "vibe-spec"
            };
            let role = if kind == "doc" {
                "documentation"
            } else {
                "spec"
            };
            let extras = if kind == "doc" {
                "title='Guide'\nabstract='Facts parity.'\n"
            } else {
                ""
            };
            let subject = if kind == "doc" {
                "\n[[documents]]\npackage='org.subject/tool'\nversion='^1.0'\n"
            } else {
                ""
            };
            let manifest = format!(
                "[package]\nkind='{kind}'\ngroup='org.example'\nname='pkg'\nversion='1.0.0'\n{extras}[[package.sources]]\nfile='{relative}'\nmedia='application/xml'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n{subject}"
            );
            fs::write(source.path().join("vibe.toml"), &manifest).unwrap();
            let doc_extra = if kind == "doc" {
                "<note kind=\"warning\"><NOTE fact=\"true\" status=\"doc/work\">note body</NOTE></note><figure src=\"image.png\" alt=\"image\"><caption><CAPTION fact=\"true\">caption</CAPTION></caption></figure>"
            } else {
                ""
            };
            let xml = format!(
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Metadata</title><p><MARKED fact=\"true\" status=\"impl/done\" requires=\"implementation,verification\">marked</MARKED></p><p><UNMARKED fact=\"true\">unmarked</UNMARKED></p><p><VOID fact=\"true\" status=\"spec/void\">void</VOID></p>{doc_extra}</spec>"
            );
            fs::create_dir_all(source.path().join(&relative).parent().unwrap()).unwrap();
            fs::write(source.path().join(&relative), &xml).unwrap();
            let parsed = vibe_core::manifest::Manifest::parse_str(&manifest).unwrap();
            let original = vibe_facts::observe_declared_document(
                parsed.package.as_ref().unwrap(),
                &relative,
                Some(xml.as_bytes()),
            )
            .unwrap();
            let vibe_facts::DeclaredDocumentObservation::Facts {
                facts: expected, ..
            } = original
            else {
                panic!("original native input must parse: {original:?}");
            };
            assert_eq!(expected.len(), if kind == "doc" { 5 } else { 3 });
            let hash = vibe_core::ContentHash::from_validated(format!("sha256:{}", "a".repeat(64)));
            vibe_workspace::vibedeps::materialise_with_spec_format(
                root.path(),
                &vibe_core::Group::parse("org.example").unwrap(),
                "pkg",
                &"1.0.0".parse().unwrap(),
                source.path(),
                vibe_workspace::vibedeps::CopyMode::Copy,
                format,
                &hash,
            )
            .unwrap();
            write_lock(root.path(), false);
            let lock = root.path().join("vibe.lock");
            fs::write(
                &lock,
                fs::read_to_string(&lock)
                    .unwrap()
                    .replace("kind = \"feat\"", &format!("kind = \"{kind}\"")),
            )
            .unwrap();
            let selected = vibe_workspace::Workspace::discover_selected(root.path()).unwrap();
            let registry = vibe_facts::Registry::load(root.path()).unwrap();
            let coords = crate::sources::enumerate(root.path(), &selected, &registry).unwrap();
            let coord = coords
                .iter()
                .find(|coord| coord.package == "org.example/pkg")
                .unwrap();
            let observed =
                crate::sources::observe(coord, root.path(), &coords, Some(format)).unwrap();
            assert_eq!(
                observed.result.state,
                SourceResultState::Available,
                "{kind}/{format:?}"
            );
            let mut expected = expected;
            expected.sort_by(|a, b| a.address.cmp(&b.address));
            assert_eq!(
                observed.facts, expected,
                "{kind}/{format:?}: status, requirements and original ids must match"
            );
            assert!(
                observed
                    .facts
                    .iter()
                    .any(|fact| fact.address.ends_with("#VOID") && fact.status.is_some())
            );
        }
    }
}

#[test]
fn materialized_legacy_query_keeps_clean_behavior_and_refuses_record_incoherence() {
    let root = project("# Host\n");
    let source = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("vibe.toml"),
        "[package]\nkind='feat'\ngroup='org.example'\nname='pkg'\nversion='1.0.0'\n",
    )
    .unwrap();
    let relative = vibe_core::layout::current_specs_root().join("LEGACY.md");
    fs::create_dir_all(source.path().join(&relative).parent().unwrap()).unwrap();
    fs::write(
        source.path().join(&relative),
        "# Legacy\n\n##OLD old fact.\n",
    )
    .unwrap();
    let hash = vibe_core::ContentHash::from_validated(format!("sha256:{}", "a".repeat(64)));
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &vibe_core::Group::parse("org.example").unwrap(),
        "pkg",
        &"1.0.0".parse().unwrap(),
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    write_lock(root.path(), false);
    let clean = report(root.path(), None);
    assert_eq!(
        find_source(&clean, "org.example/pkg").state,
        SourceResultState::Available
    );
    assert!(
        clean
            .rows
            .iter()
            .any(|row| row.address == "spec://org.example/pkg/LEGACY#OLD")
    );
    let lock = root.path().join("vibe.lock");
    fs::write(
        &lock,
        fs::read_to_string(&lock)
            .unwrap()
            .replace(&"a".repeat(64), &"b".repeat(64)),
    )
    .unwrap();
    let bad = report(root.path(), None);
    assert_eq!(
        find_source(&bad, "org.example/pkg").state,
        SourceResultState::Unavailable
    );
    assert!(find_source(&bad, "org.example/pkg").digest.is_none());
}
