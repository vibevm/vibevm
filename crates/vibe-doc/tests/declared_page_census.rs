//! Selected Doc membership differs from a suffix inventory.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS",
    r = 1
);
use std::fs;

fn authored() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), "[package]\ngroup='com.example'\nname='manual'\nversion='1.0.0'\nkind='doc'\ntitle='Manual'\nabstract='Selected page fixture.'\n[[package.sources]]\nfile='guide.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n[[documents]]\npackage='com.example/subject'\nversion='^1.0'\n").unwrap();
    vibe_core::manifest::Manifest::read(root.path().join("vibe.toml")).unwrap();
    fs::write(root.path().join("guide.payload"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title><note kind='tip'>Native note.</note></spec>").unwrap();
    root
}

#[test]
fn authored_declared_page_census_reads_arbitrary_suffix() {
    let root = authored();
    let pages = vibe_doc::pages::read_package(root.path()).unwrap();
    assert_eq!(pages.pages.len(), 1);
    assert_eq!(pages.pages[0].rel, "guide.payload");
    assert!(pages.unreadable.is_empty());
}

#[test]
fn declared_documents_names_malformed_native_member() {
    let root = authored();
    fs::write(root.path().join("guide.payload"), "<spec").unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["guide.payload"]
    );
}

#[test]
fn declared_census_ignores_raw_neighbors_and_missing_ordinary_file() {
    let root = authored();
    let path = root.path().join("vibe.toml");
    let text = fs::read_to_string(&path).unwrap();
    let extra = "[[package.sources]]\nfile='absent.bin'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";
    fs::write(
        &path,
        text.replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    vibe_core::manifest::Manifest::read(&path).unwrap();
    fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    fs::write(root.path().join("vibevm/vibespecs/guide.xml"), [0xff, 0xfe]).unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["guide.payload"]
    );
}

#[cfg(unix)]
#[test]
fn native_census_skips_ordinary_directory_but_keeps_nested_native_override() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    let extra = "[[package.sources]]\ndirectory='assets'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n[[package.sources]]\nfile='assets/nested.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    fs::create_dir_all(root.path().join("assets")).unwrap();
    std::os::unix::fs::symlink("absent", root.path().join("assets/dangling")).unwrap();
    fs::copy(
        root.path().join("guide.payload"),
        root.path().join("assets/nested.payload"),
    )
    .unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["assets/nested.payload", "guide.payload"]
    );
}

#[cfg(unix)]
#[test]
fn native_directory_does_not_open_an_unreadable_ordinary_subtree() {
    use std::os::unix::fs::PermissionsExt;
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    let extra = "[[package.sources]]\ndirectory='chapters'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n[[package.sources]]\ndirectory='chapters/assets'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    let assets = root.path().join("chapters/assets");
    fs::create_dir_all(&assets).unwrap();
    fs::write(assets.join("ordinary"), "raw").unwrap();
    fs::set_permissions(&assets, fs::Permissions::from_mode(0o0)).unwrap();
    let result = vibe_doc::pages::documents(root.path());
    fs::set_permissions(&assets, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.unwrap(), ["guide.payload"]);
    let native = "[[package.sources]]\nfile='chapters/assets/native.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{native}[[documents]]")),
    )
    .unwrap();
    fs::copy(
        root.path().join("guide.payload"),
        assets.join("native.payload"),
    )
    .unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["chapters/assets/native.payload", "guide.payload"]
    );
}

#[test]
fn native_xml_with_markdown_suffix_keeps_original_rel_and_canonical_identity() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("guide.payload", "guide.md"),
    )
    .unwrap();
    fs::rename(
        root.path().join("guide.payload"),
        root.path().join("guide.md"),
    )
    .unwrap();
    let pages = vibe_doc::pages::read_package(root.path()).unwrap();
    assert_eq!(pages.pages[0].rel, "guide.md");
    assert_eq!(vibe_doc::pages::documents(root.path()).unwrap(), ["guide"]);
    assert_eq!(
        vibe_doc::build::page_path("", &pages.pages[0].rel, vibe_doc::build::Format::Md),
        "guide.md"
    );
    let options = vibe_doc::build::Options {
        format: vibe_doc::build::Format::Md,
        base: "/doc/".into(),
        manifest: vibe_doc::manifest::Options::at(chrono::Utc::now()),
        derived: Default::default(),
    };
    let built = vibe_doc::build::build(
        root.path(),
        &vibe_doc::citations::SpecSources::new(),
        &options,
    )
    .unwrap();
    let index = std::str::from_utf8(
        &built
            .files
            .iter()
            .find(|file| file.path == "llms.txt")
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert!(
        index.contains("/doc/com.example/manual/1.0.0/guide/"),
        "{index}"
    );
    assert!(!index.contains("/guide.md/"));
}

#[test]
fn declared_doc_markdown_input_remains_unsupported() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("application/xml", "text/markdown"),
    )
    .unwrap();
    fs::write(root.path().join("guide.payload"), "# Guide\n").unwrap();
    let error = vibe_doc::pages::read_package(root.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("SOURCE-ADMISSION-BOUNDARY"), "{error}");
    assert_eq!(
        fs::read_to_string(root.path().join("guide.payload")).unwrap(),
        "# Guide\n"
    );
}

#[test]
fn ordinary_only_selector_ties_do_not_select_or_poison_pages() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    let extra = "[[package.sources]]\nfile='outside'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n[[package.sources]]\nfile='outside'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n";
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    vibe_core::manifest::Manifest::read(&manifest).unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["guide.payload"]
    );
    let policy = vibe_spec::source_policy::observe_package_policy(root.path()).unwrap();
    assert!(policy.capture_host_inventory("com.example/manual").is_err());
}

#[test]
fn a_tied_native_winner_refuses_instead_of_choosing_the_ordinary_row() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    let extra = "[[package.sources]]\nfile='guide.payload'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    vibe_core::manifest::Manifest::read(&manifest).unwrap();
    assert!(
        vibe_doc::pages::documents(root.path())
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}

#[test]
fn selected_absence_ambiguity_and_empty_are_distinct() {
    let root = authored();
    fs::remove_file(root.path().join("guide.payload")).unwrap();
    assert!(vibe_doc::pages::documents(root.path()).is_err());
    let manifest = root.path().join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let (before, tail) = text.split_once("[[package.sources]]").unwrap();
    let (_, relations) = tail.split_once("[[documents]]").unwrap();
    fs::write(
        &manifest,
        format!("{before}sources=[]\n[[documents]]{relations}"),
    )
    .unwrap();
    assert!(vibe_doc::pages::documents(root.path()).unwrap().is_empty());
    assert_eq!(
        vibe_doc::pages::read_package(root.path()).unwrap().total(),
        0
    );
    let row = "[[package.sources]]\nfile='guide.xml'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    fs::write(
        &manifest,
        text.replace("guide.payload", "guide.md")
            .replace("[[documents]]", &format!("{row}[[documents]]")),
    )
    .unwrap();
    for name in ["guide.md", "guide.xml"] {
        fs::write(
            root.path().join(name),
            "<spec xmlns='https://vibevm.org/spec/1'/>",
        )
        .unwrap();
    }
    assert!(
        vibe_doc::pages::documents(root.path())
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}

#[test]
fn legacy_fullslug_page_routes_are_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let specs = root.path().join("vibevm/vibespecs/model");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("PROP-001-guide.xml"),
        "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title><p>[full](spec://com.example/manual/model/PROP-001-guide#root) [short](spec://com.example/manual/model/PROP-001#root)</p></spec>",
    )
    .unwrap();
    assert_eq!(
        vibe_doc::pages::documents(root.path()).unwrap(),
        ["model/PROP-001-guide"]
    );
    assert_eq!(
        vibe_doc::build::page_path(
            "",
            "model/PROP-001-guide.xml",
            vibe_doc::build::Format::Html
        ),
        "model/PROP-001-guide/index.html"
    );
    let report = vibe_doc::citations::check(
        root.path(),
        "com.example/manual",
        &vibe_doc::citations::SpecSources::new(),
    )
    .unwrap();
    assert_eq!(report.unresolved.len(), 1);
    assert!(report.unresolved[0].uri.ends_with("PROP-001#root"));
}

#[test]
fn declared_directory_uses_outer_precedence_and_preserves_native_ast() {
    let root = authored();
    let manifest = root.path().join("vibe.toml");
    let extra = "[[package.sources]]\ndirectory='chapters'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n[[package.sources]]\nfile='chapters/nested/vibe.toml'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n[[package.sources]]\nfile='chapters/raw.xml'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    fs::create_dir_all(root.path().join("chapters/nested")).unwrap();
    fs::write(root.path().join("chapters/nested/vibe.toml"), "[broken").unwrap();
    fs::write(root.path().join("chapters/raw.xml"), [0xff]).unwrap();
    let xml = "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Nested</title><note kind='warning'>Keep note.</note><figure src='media/image.svg' alt='Image'><caption>Keep caption.</caption></figure><example id='demo' fixture='none'><run>vibe --version</run><expect>native</expect></example></spec>";
    fs::write(root.path().join("chapters/nested/page.any"), xml).unwrap();
    let captured = vibe_doc::citations::SpecSources::new()
        .capture_pages(root.path())
        .unwrap();
    assert_eq!(
        captured.documents(),
        ["chapters/nested/page.any", "guide.payload"]
    );
    let expected = vibe_specdoc::from_xml_with(xml, vibe_specdoc::Vocabulary::Doc).unwrap();
    assert_eq!(
        captured
            .set()
            .pages
            .iter()
            .find(|page| page.rel == "chapters/nested/page.any")
            .unwrap()
            .doc,
        expected
    );
    fs::write(root.path().join("chapters/new.payload"), xml).unwrap();
    assert!(captured.recheck().is_err());
}

#[test]
fn selected_self_anchors_use_census_and_keep_positional_exemption() {
    let root = authored();
    fs::write(root.path().join("guide.payload"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title><p>[root](spec://com.example/manual/guide.payload#root) [position](spec://com.example/manual/guide.payload#p99) [missing](spec://com.example/manual/guide.payload#absent)</p></spec>").unwrap();
    let report = vibe_doc::citations::check(
        root.path(),
        "com.example/manual",
        &vibe_doc::citations::SpecSources::new(),
    )
    .unwrap();
    assert_eq!(report.unresolved.len(), 1);
    assert!(report.unresolved[0].uri.ends_with("#absent"));
}

#[test]
fn explicit_root_is_not_replaced_by_a_nearer_coordinate_instance() {
    let selected = authored();
    let registry = tempfile::tempdir().unwrap();
    let newer = registry.path().join("com.example/manual/v2.0.0");
    fs::create_dir_all(&newer).unwrap();
    fs::write(
        newer.join("vibe.toml"),
        fs::read_to_string(selected.path().join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='2.0.0'"),
    )
    .unwrap();
    fs::write(
        newer.join("guide.payload"),
        "<spec xmlns='https://vibevm.org/spec/1'><title>Different instance</title></spec>",
    )
    .unwrap();
    let sources = vibe_doc::citations::SpecSources::new().with_in_tree(registry.path());
    assert_eq!(
        sources
            .instance_of("com.example", "manual")
            .unwrap()
            .version,
        "2.0.0"
    );
    let captured = sources.capture_pages(selected.path()).unwrap();
    assert_eq!(
        captured.set().pages[0].doc.title.as_ref().unwrap().text,
        "Guide"
    );
}

#[test]
fn borrowed_point_keeps_original_page_and_guard_without_reading_other_pages() {
    let source = authored();
    let adaptation = authored();
    let manifest = adaptation.path().join("vibe.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap()
        .replace("name='manual'", "name='manual-ru'");
    fs::write(
        &manifest,
        format!("{text}\n[translates]\npackage='com.example/manual'\nversion='^1.0'\n"),
    )
    .unwrap();
    let original = source.path().join("guide.payload");
    fs::write(&original, "<spec xmlns='https://vibevm.org/spec/1'><title>Guide</title><example id='demo' fixture='none'><run>vibe --version</run><expect>native output</expect></example></spec>").unwrap();
    let source_manifest = source.path().join("vibe.toml");
    let extra = "[[package.sources]]\nfile='missing.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    fs::write(
        &source_manifest,
        fs::read_to_string(&source_manifest)
            .unwrap()
            .replace("[[documents]]", &format!("{extra}[[documents]]")),
    )
    .unwrap();
    let world = vibe_doc::citations::SpecSources::for_checkout(
        source.path(),
        Some("com.example"),
        "manual",
    );
    let prepared =
        vibe_doc::translations::prepare_borrowed_by(adaptation.path(), "guide.payload", &world)
            .unwrap();
    assert_eq!(prepared.examples()["demo"].expect, "native output");
    assert!(vibe_doc::translations::borrowed(adaptation.path(), &world).is_err());
    prepared.recheck().unwrap();
    fs::write(original, "changed").unwrap();
    assert!(prepared.recheck().is_err());
}

#[test]
fn prepared_write_refuses_late_page_and_membership_changes_before_output() {
    let root = authored();
    let options = vibe_doc::build::Options {
        format: vibe_doc::build::Format::Md,
        base: "/doc/".into(),
        manifest: vibe_doc::manifest::Options::at(chrono::Utc::now()),
        derived: Default::default(),
    };
    let prepared = vibe_doc::build::prepare_observed(
        root.path(),
        &vibe_doc::citations::SpecSources::new(),
        &options,
        &vibe_core::progress::Progress::default(),
    )
    .unwrap();
    fs::write(root.path().join("guide.payload"), "changed").unwrap();
    let out = root.path().join("out");
    assert!(prepared.write(&out).is_err());
    assert!(!out.exists());
}
