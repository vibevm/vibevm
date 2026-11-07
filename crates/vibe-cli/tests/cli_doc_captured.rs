//! Real documentation build composition reads declared citation sources.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT",
    r = 1
);
use std::fs;
use std::path::Path;
use vibe_test_support::UserScratch;

fn source(root: &Path) {
    fs::create_dir_all(root.join("vibevm/vibespecs/common")).unwrap();
    fs::write(root.join("vibe.toml"), "[package]\ngroup='com.example'\nname='native'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/common/PROP-001-rule.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    fs::write(root.join("vibevm/vibespecs/common/PROP-001-rule.payload"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Rule</title><p><LAW fact='true' status='spec/done'>Current declared CLI rule.</LAW></p></spec>").unwrap();
    fs::write(
        root.join("vibevm/vibespecs/common/PROP-001-rule.md"),
        "raw same-stem neighbor",
    )
    .unwrap();
}

fn docs(root: &Path) {
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    fs::write(root.join("vibe.toml"), "[package]\ngroup='com.example'\nname='manual'\nkind='doc'\nversion='1.0.0'\ntitle='Manual'\nabstract='A manual.'\n[[documents]]\npackage='com.example/native'\nversion='^1.0'\n").unwrap();
    fs::write(root.join("vibevm/vibespecs/guide.xml"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title><p>Read the current rule.</p><rule ref='spec://com.example/native/common/PROP-001#LAW'/></spec>").unwrap();
}

fn declared_docs(root: &Path) {
    docs(root);
    let path = root.join("vibe.toml");
    let rows = "[[package.sources]]\nfile='guide.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("[[documents]]", &format!("{rows}[[documents]]")),
    )
    .unwrap();
    fs::rename(
        root.join("vibevm/vibespecs/guide.xml"),
        root.join("guide.payload"),
    )
    .unwrap();
}

#[test]
fn real_declared_doc_page_build_borrows_examples_and_resolves_citations() {
    let user = UserScratch::new();
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("source");
    source(&checkout);
    let original = checkout.join("vibevm/vibepacks/com.example/original/v1.0.0");
    declared_docs(&original);
    let path = original.join("vibe.toml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("name='manual'", "name='original'"),
    )
    .unwrap();
    fs::write(original.join("guide.payload"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Original</title><example id='demo' fixture='none'><run>vibe --version</run><expect>Borrowed command output.</expect></example></spec>").unwrap();
    let manual = temp.path().join("manual");
    declared_docs(&manual);
    let path = manual.join("vibe.toml");
    fs::write(
        &path,
        format!(
            "{}\n[translates]\npackage='com.example/original'\nversion='^1.0'\n",
            fs::read_to_string(&path).unwrap()
        ),
    )
    .unwrap();
    let page = manual.join("guide.payload");
    let text = fs::read_to_string(&page)
        .unwrap()
        .replace("</spec>", "<example ref='demo'/></spec>");
    fs::write(&page, &text).unwrap();
    let out = temp.path().join("output");
    build(&user, &checkout, &manual, &out);
    let rendered =
        fs::read_to_string(out.join("com.example/manual/1.0.0/guide.payload.md")).unwrap();
    assert!(rendered.contains("Borrowed command output."), "{rendered}");
    assert_eq!(fs::read_to_string(page).unwrap(), text);
}

#[test]
fn real_cache_warmup_doc_census_builds_outside_the_authoring_checkout() {
    let user = UserScratch::new();
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("checkout");
    fs::create_dir_all(&checkout).unwrap();
    fs::write(
        checkout.join("vibe.toml"),
        "[project]\ngroup='com.example'\nname='host'\nversion='1.0.0'\n",
    )
    .unwrap();
    let registry = checkout.join(vibe_core::layout::current_packages_root());
    source(&registry.join("com.example/native/v1.0.0"));
    declared_docs(&registry.join("com.example/manual/v1.0.0"));
    let refused = user
        .vibe()
        .current_dir(&checkout)
        .args([
            "install",
            "com.example/manual@=1.0.0",
            "--offline",
            "--assume-yes",
            "--registry",
        ])
        .arg(&registry)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("vibe cache add com.example/manual"));
    user.vibe()
        .current_dir(&checkout)
        .args(["cache", "add", "--offline", "com.example/manual@=1.0.0"])
        .assert()
        .success();
    let manual = vibe_registry::store::entry_dir(
        &user.settings.join("cache"),
        &vibe_core::Group::parse("com.example").unwrap(),
        "manual",
        &"1.0.0".parse().unwrap(),
    );
    let bare = temp.path().join("bare");
    fs::create_dir(&bare).unwrap();
    let out = temp.path().join("output");
    build(&user, &bare, &manual, &out);
    assert!(
        out.join("com.example/manual/1.0.0/guide.payload.md")
            .is_file()
    );
}

fn build(user: &UserScratch, checkout: &Path, manual: &Path, output: &Path) {
    user.vibe()
        .current_dir(checkout)
        .args(["doc", "build", "--path"])
        .arg(manual)
        .arg("--out")
        .arg(output)
        .args(["--format", "md", "--no-derived"])
        .assert()
        .success();
    let mut rendered = String::new();
    for entry in walkdir::WalkDir::new(output) {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            rendered.push_str(&fs::read_to_string(entry.path()).unwrap());
        }
    }
    assert!(
        rendered.contains("Current declared CLI rule."),
        "{rendered}"
    );
    assert!(!rendered.contains("raw same-stem neighbor"));
}

#[test]
fn real_doc_build_quotes_declared_authored_source() {
    let user = UserScratch::new();
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("source");
    source(&checkout);
    let manual = temp.path().join("manual");
    docs(&manual);
    build(&user, &checkout, &manual, &temp.path().join("output"));
}

#[test]
fn real_doc_build_quotes_local_registry_installed_source_in_selected_formats() {
    for format in ["mixed", "xml"] {
        let user = UserScratch::new();
        let temp = tempfile::tempdir().unwrap();
        let registry = temp.path().join("registry");
        source(&registry.join("com.example/native/v1.0.0"));
        let checkout = temp.path().join("checkout");
        fs::create_dir_all(&checkout).unwrap();
        fs::write(checkout.join("vibe.toml"), format!("[project]\ngroup='com.example'\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n")).unwrap();
        user.vibe()
            .current_dir(&checkout)
            .args([
                "install",
                "com.example/native@=1.0.0",
                "--offline",
                "--assume-yes",
                "--registry",
            ])
            .arg(&registry)
            .assert()
            .success();
        let manual = temp.path().join("manual");
        docs(&manual);
        build(&user, &checkout, &manual, &temp.path().join("output"));
        let member = checkout.join("member");
        fs::create_dir_all(&member).unwrap();
        fs::write(member.join("vibe.toml"), "[project]\ngroup='com.example'\nname='member'\nversion='1.0.0'\nspec_format='markdown'\n").unwrap();
        let manifest = checkout.join("vibe.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            format!("{text}\n[workspace]\nmembers=['member']\n"),
        )
        .unwrap();
        build(&user, &member, &manual, &temp.path().join("member-output"));
    }
}

#[test]
fn real_doc_build_refuses_invalid_selected_configuration() {
    let user = UserScratch::new();
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("source");
    source(&checkout);
    let manual = temp.path().join("manual");
    docs(&manual);
    fs::write(
        user.settings.join("config.toml"),
        "[install]\nspec_format='unknown'\n",
    )
    .unwrap();
    user.vibe()
        .current_dir(&checkout)
        .args(["doc", "build", "--path"])
        .arg(&manual)
        .arg("--out")
        .arg(temp.path().join("output"))
        .args(["--format", "md", "--no-derived"])
        .assert()
        .failure();
    fs::write(user.settings.join("config.toml"), "").unwrap();
    fs::write(checkout.join("vibe.toml"), "[broken").unwrap();
    user.vibe()
        .current_dir(&checkout)
        .args(["doc", "build", "--path"])
        .arg(&manual)
        .arg("--out")
        .arg(temp.path().join("output"))
        .args(["--format", "md", "--no-derived"])
        .assert()
        .failure();
}
