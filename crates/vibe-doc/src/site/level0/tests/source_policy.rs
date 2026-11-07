//! Declared native level-zero input and retained observations.
use super::*;

const SPEC: &str = r#"<spec xmlns="https://vibevm.org/spec/1"><title id="root">Native promise</title><status stage="impl" state="done"/><p><PROMISE fact="true" status="impl/done">Preserved meaning.</PROMISE></p><p><OLD fact="true" status="spec/void">Retired.</OLD></p></spec>"#;

fn row(file: &str, media: &str, interpretation: &str) -> String {
    let role = match interpretation {
        "vibe-doc" => "documentation",
        "vibe-spec" => "spec",
        _ => "resource",
    };
    format!(
        "\n[[package.sources]]\nfile='{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"
    )
}

fn declared(root: &Path, rows: &str) {
    write(
        &root.join("vibe.toml"),
        &format!(
            "[package]\nname='wal'\ngroup='org.example'\nversion='1.2.0'\nkind='flow'\n{rows}"
        ),
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-LEVEL0-READ",
    r = 1
)]
fn outside_specroot_native_inputs_keep_bytes_ast_and_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    declared(
        &root,
        &(row("guide.payload", "application/xml", "vibe-spec")
            + &row("readme.md", "text/markdown", "vibe-spec")
            + &row("guide.xml", "application/xml", "xml-data")),
    );
    write(&root.join("guide.payload"), SPEC);
    write(
        &root.join("readme.md"),
        "# Native Markdown {#root}\n\nIts own words.\n",
    );
    fs::write(root.join("guide.xml"), [0xff, 0xfe]).unwrap();
    let work = tmp.path().join("work");
    let (composed, input) = compose_prepared(&root, &work, &Related::default()).unwrap();
    assert_eq!(composed.copied, 2);
    assert!(
        input.pages.unreadable.is_empty(),
        "{:?}",
        input.pages.unreadable
    );
    assert_eq!(
        fs::read(work.join(SPEC_ROOT).join("guide.payload")).unwrap(),
        SPEC.as_bytes()
    );
    let page = input
        .pages
        .pages
        .iter()
        .find(|p| p.rel == "guide.payload")
        .unwrap();
    assert_eq!(page.path, root.join("guide.payload"));
    assert_eq!(page.doc, vibe_specdoc::from_xml(SPEC).unwrap());
    assert!(!page_names(&input.pages).contains(&"guide.xml".into()));
    assert!(
        !composed.generated.contains(&README_PAGE.into()),
        "native Markdown owns readme address"
    );
    assert_eq!(
        input
            .pages
            .pages
            .iter()
            .filter(|p| crate::pages::document_of(&p.rel) == "readme")
            .count(),
        1
    );
}

#[test]
fn malformed_missing_and_ambiguous_native_declarations_preserve_prior_work() {
    for rows in [
        "\n[[package.sources]]\nfile='guide.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\n".to_string(),
        row("missing.payload", "application/xml", "vibe-spec"),
        row("guide.payload", "application/xml", "vibe-spec") + &row("guide.payload", "application/xml", "xml-data"),
    ].into_iter().enumerate() {
        let (case, rows) = rows;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("pkg");
        declared(&root, &rows);
        write(&root.join("guide.payload"), SPEC);
        let work = tmp.path().join("work");
        write(&work.join("sentinel"), "prior composition");
        let error = compose_prepared(&root, &work, &Related::default()).unwrap_err().to_string();
        let expected = ["invalid", "absent", "ambiguous"][case];
        assert!(error.contains(expected), "{error}");
        assert_eq!(fs::read_to_string(work.join("sentinel")).unwrap(), "prior composition");
    }
}

#[test]
fn actual_doc_and_spec_vocabulary_stay_distinct_in_mixed_package() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    let rows = row("manual.payload", "application/xml", "vibe-doc")
        + &row("promise.payload", "application/xml", "vibe-spec");
    declared(&root, &rows);
    let doc = r#"<spec xmlns="https://vibevm.org/spec/1"><title id="root">Manual</title><note kind="tip">Only Doc.</note></spec>"#;
    write(&root.join("manual.payload"), doc);
    write(&root.join("promise.payload"), SPEC);
    assert!(compose_prepared(&root, &tmp.path().join("bad"), &Related::default()).is_err());
    let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
    write(
        &root.join("vibe.toml"),
        &(manifest.replace(
            "kind='flow'",
            "kind='doc'\ntitle='Manual'\nabstract='Two native interpretations.'",
        ) + "\n[[documents]]\npackage='org.example/subject'\nversion='^1.0'\n"),
    );
    vibe_core::manifest::Manifest::parse_str(&fs::read_to_string(root.join("vibe.toml")).unwrap())
        .unwrap();
    let (_, input) = compose_prepared(&root, &tmp.path().join("ok"), &Related::default()).unwrap();
    assert_eq!(input.pages.unreadable.len(), 0);
    assert_eq!(input.authored.as_ref().unwrap().copies.len(), 2);
    let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
    write(
        &root.join("vibe.toml"),
        &manifest
            .replace("interpretation='vibe-doc'", "interpretation='vibe-spec'")
            .replace("role='documentation'", "role='spec'"),
    );
    let (_, input) =
        compose_prepared(&root, &tmp.path().join("spec"), &Related::default()).unwrap();
    assert!(
        input
            .pages
            .unreadable
            .iter()
            .any(|page| page.rel == "manual.payload")
    );
}

#[test]
fn captured_bytes_are_copied_without_reopening_and_drift_refuses_build() {
    for explicit in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("pkg");
        let relative = "vibevm/vibespecs/common/promise.xml";
        declared(
            &root,
            &if explicit {
                row(relative, "application/xml", "vibe-spec")
            } else {
                String::new()
            },
        );
        write(&root.join(relative), SPEC);
        let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
        let captured = sources::SourcePages::capture(&root, &manifest).unwrap();
        write(&root.join(relative), "replaced after capture");
        let copies = tmp.path().join("copies");
        copy_specs(&copies, &captured.copies).unwrap();
        assert_eq!(fs::read(copies.join(relative)).unwrap(), SPEC.as_bytes());
        assert!(captured.recheck().is_err());
        write(&root.join(relative), SPEC);
        let (composed, input) =
            compose_prepared(&root, &tmp.path().join("work"), &Related::default()).unwrap();
        write(&root.join(relative), "replaced before render");
        let options = crate::build::Options {
            format: crate::build::Format::Html,
            base: "/doc/".into(),
            manifest: crate::manifest::Options::at(crate::manifest::tests::rendered_at()),
            derived: Default::default(),
        };
        assert!(
            crate::build::build_generated(
                &composed.dir,
                &input,
                &crate::citations::SpecSources::new(),
                &options
            )
            .is_err()
        );
    }
}

#[test]
fn shadowed_and_empty_doc_scopes_do_not_authorize_or_require_doc_vocabulary() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    let defaults = "\n[[package.sources]]\ndirectory='scope'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    declared(
        &root,
        &(defaults.to_string() + &row("scope/promise.payload", "application/xml", "vibe-spec")),
    );
    write(&root.join("scope/promise.payload"), SPEC);
    let (_, input) =
        compose_prepared(&root, &tmp.path().join("shadowed"), &Related::default()).unwrap();
    assert_eq!(input.authored.as_ref().unwrap().copies.len(), 1);
    write(&root.join("scope/new.payload"), SPEC);
    assert!(
        input.recheck().is_err(),
        "absence decision survives until publication"
    );
    assert!(
        compose_prepared(&root, &tmp.path().join("selected-doc"), &Related::default()).is_err()
    );
    fs::remove_file(root.join("scope/new.payload")).unwrap();
    fs::remove_file(root.join("scope/promise.payload")).unwrap();
    declared(&root, defaults);
    let (_, input) =
        compose_prepared(&root, &tmp.path().join("empty"), &Related::default()).unwrap();
    assert_eq!(input.authored.as_ref().unwrap().copies.len(), 0);
}

#[test]
fn declared_intent_never_uses_incomplete_card_only_compatibility() {
    for declaration in ["sources=[]", "source_exports=[]", "sources='bad'"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("pkg");
        write(
            &root.join("vibe.toml"),
            &format!(
                "[package]\nname='wal'\ngroup='org.example'\nversion='1.2.0'\n{declaration}\n"
            ),
        );
        assert!(compose_prepared(&root, &tmp.path().join("work"), &Related::default()).is_err());
    }
}

#[test]
fn legacy_and_declared_spec_keep_status_void_and_all_projection_meaning() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    let relative = "vibevm/vibespecs/common/promise.xml";
    declared(&root, "");
    write(&root.join(relative), SPEC);
    let (_, old) =
        compose_prepared(&root, &tmp.path().join("legacy"), &Related::default()).unwrap();
    declared(&root, &row(relative, "application/xml", "vibe-spec"));
    let (_, new) =
        compose_prepared(&root, &tmp.path().join("declared"), &Related::default()).unwrap();
    let old = old
        .pages
        .pages
        .iter()
        .find(|p| p.rel == "common/promise.xml")
        .unwrap();
    let new = new.pages.pages.iter().find(|p| p.rel == old.rel).unwrap();
    assert_eq!(old.doc, new.doc);
    assert_eq!(old.path, new.path);
    for format in crate::site::render::FORMATS {
        let content = crate::content::Content::default();
        assert_eq!(
            crate::build::render_page(old, &content, *format),
            crate::build::render_page(new, &content, *format)
        );
    }
}

#[test]
fn unbound_materialized_original_is_refused_before_work_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    declared(&root, &row("guide.payload", "application/xml", "vibe-spec"));
    write(&root.join("guide.payload"), SPEC);
    write(
        &root.join(vibe_core::layout::SLOT_RECORD_FILENAME),
        "not independently bound",
    );
    let work = tmp.path().join("work");
    write(&work.join("sentinel"), "prior");
    let error = compose_prepared(&root, &work, &Related::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("independent installed binding"), "{error}");
    assert_eq!(fs::read_to_string(work.join("sentinel")).unwrap(), "prior");
}

#[test]
fn mixed_interpretations_cannot_publish_one_native_address_twice() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("pkg");
    declared(
        &root,
        &(row("guide.xml", "application/xml", "vibe-doc")
            + &row("guide.md", "text/markdown", "vibe-spec")),
    );
    let text = fs::read_to_string(root.join("vibe.toml")).unwrap();
    write(
        &root.join("vibe.toml"),
        &(text.replace(
            "kind='flow'",
            "kind='doc'\ntitle='Manual'\nabstract='Mixed source test.'",
        ) + "\n[[documents]]\npackage='org.example/subject'\nversion='^1.0'\n"),
    );
    write(&root.join("guide.xml"), SPEC);
    write(&root.join("guide.md"), "# Guide\n\nAnother source.\n");
    let work = tmp.path().join("work");
    write(&work.join("sentinel"), "prior");
    let error = compose_prepared(&root, &work, &Related::default())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("ambiguous across interpretations"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(work.join("sentinel")).unwrap(), "prior");
}
