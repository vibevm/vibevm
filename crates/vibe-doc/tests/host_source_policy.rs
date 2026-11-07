//! Host activation must not grant ordinary files documentation/source meaning.
use std::fs;
#[test]
fn declared_host_pages_and_citations_refuse_before_suffix_parsing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("vibe.toml");
    let base = "[project]\ngroup='org.example'\nname='host'\nversion='1.0.0'\n";
    fs::write(&path,format!("{base}[[project.sources]]\nfile='data.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n")).unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("DATA.xml"), "<invalid").unwrap();
    let error = vibe_doc::pages::read_package(root.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("explicitly classified"), "{error}");
    let sources =
        vibe_doc::citations::SpecSources::for_checkout(root.path(), Some("org.example"), "host");
    let error =
        vibe_doc::citations::resolve("spec://org.example/host/DATA#LAW", &sources).unwrap_err();
    assert!(
        error.contains("classified") || error.contains("SOURCE-ADMISSION-BOUNDARY"),
        "{error}"
    );
    fs::write(&path, format!("{base}sources=[]\n")).unwrap();
    let pages = vibe_doc::pages::read_package(root.path()).unwrap();
    assert!(pages.pages.is_empty());
    assert!(pages.unreadable.is_empty());
}
