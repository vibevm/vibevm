//! Real requested host facts query over explicit original sources.
#[cfg(test)]
mod cases {
    use crate::tests_query::{ctx, find_source};
    use crate::{RequirementsQuery, query};
    use std::fs;
    fn write(root: &std::path::Path, relative: &str, body: impl AsRef<[u8]>) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "vibe.toml",
            "[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n[[project.sources]]\nfile='guide.md'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='guide.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n[[project.sources]]\ndirectory='native'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\ndirectory='ordinary'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n",
        );
        write(
            root.path(),
            "guide.md",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Guide</title><p><REAL fact=\"true\" status=\"impl/done\">real</REAL></p></spec>",
        );
        write(root.path(), "guide.xml", [0xff, 0]);
        write(
            root.path(),
            "native/ONE.md",
            "# One\n\n@fact:VOID removed. @status:spec/void\n",
        );
        write(
            root.path(),
            "ordinary/raw.md",
            "@fact:FAKE not native. @status:impl/done\n#embed spec://org.example/demo/guide#REAL\n",
        );
        write(
            root.path(),
            "ordinary/nested/vibe.toml",
            "[broken nested manifest is ordinary bytes",
        );
        root
    }
    fn report(
        root: &std::path::Path,
    ) -> vibe_wire::generated::requirements_report::RequirementsReport {
        query(&RequirementsQuery::default(), &ctx(root), None).unwrap()
    }
    fn digest(root: &std::path::Path, paths: &[&str]) -> String {
        let mut paths = paths.to_vec();
        paths.sort();
        let witnesses = paths
            .iter()
            .map(|path| {
                vibe_facts::SourceFileWitness::of(*path, &fs::read(root.join(path)).unwrap())
            })
            .collect::<Vec<_>>();
        crate::digest::source_result_digest(
            &vibe_wire::generated::requirements_report::RequirementSourceKind::Host,
            "org.example/demo",
            &witnesses,
        )
    }
    const SELECTED: &[&str] = &[
        "guide.md",
        "guide.xml",
        "native/ONE.md",
        "ordinary/nested/vibe.toml",
        "ordinary/raw.md",
    ];
    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-INVENTORY",
        r = 1
    )]
    fn actual_host_query_reads_explicit_native_sources_and_inert_raw_witnesses() {
        let root = fixture();
        let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
        assert_eq!(
            find_source(&report, "org.example/demo").state,
            vibe_wire::generated::requirements_report::SourceResultState::Available
        );
        let addresses = report
            .rows
            .iter()
            .map(|row| row.address.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            addresses,
            vec![
                "spec://org.example/demo/guide#REAL",
                "spec://org.example/demo/native/ONE#VOID"
            ]
        );
    }

    #[test]
    fn unsupported_host_reader_dominates_syntax_failure_independently_of_path_order() {
        let mut outcomes = Vec::new();
        for unsupported_first in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let (bad, unsupported) = if unsupported_first {
                ("z.xml", "a.xml")
            } else {
                ("a.xml", "z.xml")
            };
            write(
                root.path(),
                "vibe.toml",
                format!(
                    "[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n[[project.sources]]\nfile='{bad}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='{unsupported}'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n"
                ),
            );
            write(root.path(), bad, "<broken");
            write(
                root.path(),
                unsupported,
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Doc</title></spec>",
            );
            let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
            let source = find_source(&report, "org.example/demo");
            outcomes.push((source.state.clone(), source.digest.clone()));
        }
        assert!(
            outcomes.iter().all(|(state, digest)| *state
                == vibe_wire::generated::requirements_report::SourceResultState::Unavailable
                && digest.is_none()),
            "{outcomes:?}"
        );
    }

    #[test]
    fn complete_native_invalid_keeps_all_witnesses_but_missing_and_role_failures_are_unavailable() {
        let root = fixture();
        write(root.path(), "native/ONE.md", [0xff, 0]);
        let invalid = report(root.path());
        let source = find_source(&invalid, "org.example/demo");
        assert_eq!(
            source.state,
            vibe_wire::generated::requirements_report::SourceResultState::Invalid
        );
        assert_eq!(
            source.digest.as_deref(),
            Some(digest(root.path(), SELECTED).as_str())
        );
        assert!(invalid.rows.is_empty());
        fs::remove_file(root.path().join("guide.xml")).unwrap();
        let missing = report(root.path());
        let source = find_source(&missing, "org.example/demo");
        assert_eq!(
            source.state,
            vibe_wire::generated::requirements_report::SourceResultState::Unavailable
        );
        assert!(source.digest.is_none());
        let root = fixture();
        write(
            root.path(),
            vibe_core::layout::SLOT_RECORD_FILENAME,
            b"not parsed as original-host authority",
        );
        let source = report(root.path());
        assert_eq!(
            find_source(&source, "org.example/demo").state,
            vibe_wire::generated::requirements_report::SourceResultState::Unavailable
        );
    }
    #[test]
    fn raw_same_stem_is_valid_but_two_native_document_claims_are_invalid() {
        let root = fixture();
        let valid = report(root.path());
        assert_eq!(
            find_source(&valid, "org.example/demo").state,
            vibe_wire::generated::requirements_report::SourceResultState::Available
        );
        let path = root.path().join("vibe.toml");
        let text = fs::read_to_string(&path).unwrap().replace(
            "interpretation='xml-data'\nrole='resource'",
            "interpretation='vibe-spec'\nrole='spec'",
        );
        fs::write(path, text).unwrap();
        write(
            root.path(),
            "guide.xml",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><OTHER fact=\"true\" status=\"impl/done\">different anchor, same native document</OTHER></p></spec>",
        );
        let invalid = report(root.path());
        let source = find_source(&invalid, "org.example/demo");
        assert_eq!(
            source.state,
            vibe_wire::generated::requirements_report::SourceResultState::Invalid
        );
        assert_eq!(
            source.digest.as_deref(),
            Some(digest(root.path(), SELECTED).as_str())
        );
        assert!(invalid.rows.is_empty());
    }
    #[test]
    fn inventory_excludes_only_named_control_scope_and_keeps_dot_and_nested_manifest_bytes() {
        let root = fixture();
        write(root.path(), "ordinary/.git/config", "SECRET-GIT-CONTROL");
        write(root.path(), "ordinary/.gitignore", "not excluded");
        write(
            root.path(),
            "ordinary/.github/workflow",
            "not excluded either",
        );
        let current = report(root.path());
        let mut expected = SELECTED.to_vec();
        expected.extend(["ordinary/.gitignore", "ordinary/.github/workflow"]);
        expected.sort();
        let captured = vibe_spec::source_policy::observe_package_policy(root.path())
            .unwrap()
            .capture_host_inventory("org.example/demo")
            .unwrap();
        assert_eq!(
            captured
                .resources()
                .iter()
                .map(|resource| resource.relative())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            find_source(&current, "org.example/demo").digest.as_deref(),
            Some(digest(root.path(), &expected).as_str())
        );
        for selector in ["file='ordinary/.git/config'", "directory='ordinary/.git'"] {
            let path = root.path().join("vibe.toml");
            let original = fs::read_to_string(&path).unwrap();
            fs::write(&path,format!("{original}\n[[project.sources]]\n{selector}\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n")).unwrap();
            let result = report(root.path());
            let source = find_source(&result, "org.example/demo");
            assert_eq!(
                source.state,
                vibe_wire::generated::requirements_report::SourceResultState::Unavailable
            );
            assert!(source.digest.is_none());
            fs::write(path, original).unwrap();
        }
    }
    #[test]
    fn actual_doc_root_grants_vocabulary_but_nested_doc_manifest_does_not() {
        let root = tempfile::tempdir().unwrap();
        let content = "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Guide</title><note kind=\"tip\"><NOTE fact=\"true\" status=\"doc/done\">note</NOTE></note></spec>";
        let rows = "[[package.sources]]\ndirectory='docs'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
        let package = format!(
            "[package]\ngroup='org.example'\nname='demo'\nkind='doc'\nversion='1.0.0'\ntitle='Guide'\nabstract='Original docs.'\n{rows}\n[[documents]]\npackage='org.subject/tool'\nversion='^1.0'\n"
        );
        write(root.path(), "vibe.toml", &package);
        write(root.path(), "docs/guide.xml", content);
        let doc = report(root.path());
        assert_eq!(doc.rows.len(), 1);
        assert!(doc.rows[0].address.ends_with("docs/guide#NOTE"));
        write(
            root.path(),
            "vibe.toml",
            format!(
                "[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n{}",
                rows.replace("package.sources", "project.sources")
            ),
        );
        write(root.path(), "docs/vibe.toml", package);
        let refused = report(root.path());
        assert_eq!(
            find_source(&refused, "org.example/demo").state,
            vibe_wire::generated::requirements_report::SourceResultState::Unavailable
        );
    }
    #[test]
    fn complete_query_can_be_invalid_while_exact_cited_sync_ignores_uncited_bad_source() {
        let root = fixture();
        write(root.path(), "native/bad.md", [0xff, 0]);
        let mut registry = vibe_facts::Registry::default();
        let address = "spec://org.example/demo/guide#REAL";
        registry
            .upsert(
                root.path(),
                vibe_facts::FactEntry::for_host(
                    address,
                    "org.example/demo",
                    Some(vibe_facts::FactStatus::parse("spec/work").unwrap()),
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            vibe_facts::sync::reconcile(root.path(), &mut registry)
                .unwrap()
                .len(),
            1
        );
        let queried = report(root.path());
        assert_eq!(
            find_source(&queried, "org.example/demo").state,
            vibe_wire::generated::requirements_report::SourceResultState::Invalid
        );
    }
}
