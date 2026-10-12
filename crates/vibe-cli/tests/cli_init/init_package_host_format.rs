//! Existing-host `init package` boot dialect and derived-slot regressions.

use std::path::{Path, PathBuf};

use super::*;

fn seed_static_host(spec_format: Option<&str>) -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    user.vibe()
        .args(["init", "--no-registry", "--path"])
        .arg(root)
        .assert()
        .success();
    user.vibe()
        .args([
            "init",
            "package",
            "org.demo/first",
            "--kind",
            "flow",
            "--link",
            "static",
            "--path",
        ])
        .arg(root)
        .assert()
        .success();

    let manifest = root.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    if let Some(format) = spec_format {
        let selected = format!("[project]\nspec_format = \"{format}\"\n");
        let replaced = text.replacen("[project]\n", &selected, 1);
        assert_ne!(replaced, text, "fixture project section must exist");
        text = replaced;
    }
    text.push_str(
        "\n[requires.packages]\n\"org.demo/first\" = { version = \"^0.1\", link = \"static\" }\n",
    );
    fs::write(manifest, text).unwrap();
    user.vibe()
        .args([
            "install",
            "--offline",
            "--no-default-registry",
            "--prefer-local",
            "--assume-yes",
            "--path",
        ])
        .arg(root)
        .assert()
        .success();
    (user, dir)
}

fn file_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, at: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    visit(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn add_second_package(user: &UserScratch, root: &Path) {
    user.vibe()
        .args([
            "init",
            "package",
            "org.demo/second",
            "--kind",
            "flow",
            "--path",
        ])
        .arg(root)
        .assert()
        .success();
    assert!(
        root.join(common::pack_rel("org.demo/second/v0.1.0/vibe.toml"))
            .is_file()
    );
}

#[test]
fn init_package_keeps_existing_xml_host_and_derived_slots() {
    let (user, dir) = seed_static_host(Some("xml"));
    let root = dir.path();
    let xml = root.join(common::static_xml_rel());
    assert!(
        xml.is_file(),
        "fixture must have a generated XML static lane"
    );
    let deps = root.join(common::deps_root());
    let before_deps = file_bytes(&deps);
    assert!(
        !before_deps.is_empty(),
        "fixture must have a materialized slot"
    );
    let before_xml = fs::read(&xml).unwrap();

    add_second_package(&user, root);

    assert!(xml.is_file(), "init package removed the XML static lane");
    assert!(!root.join(common::static_md_rel()).exists());
    assert_eq!(fs::read(&xml).unwrap(), before_xml);
    assert_eq!(
        file_bytes(&deps),
        before_deps,
        "init rewrote a derived slot"
    );
    let index = fs::read_to_string(root.join(common::index_rel())).unwrap();
    assert!(
        index.contains(&format!("static = \"{}\"", common::static_xml_rel())),
        "{index}"
    );
    for launcher in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let body = fs::read_to_string(root.join(launcher)).unwrap();
        assert!(
            body.contains(&common::static_xml_rel()),
            "{launcher}: {body}"
        );
        assert!(
            !body.contains(&common::static_md_rel()),
            "{launcher}: {body}"
        );
    }
}

#[test]
fn init_package_keeps_existing_markdown_host_and_derived_slots() {
    let (user, dir) = seed_static_host(None);
    let root = dir.path();
    let md = root.join(common::static_md_rel());
    assert!(
        md.is_file(),
        "fixture must have a generated Markdown static lane"
    );
    let deps = root.join(common::deps_root());
    let before_deps = file_bytes(&deps);
    assert!(!before_deps.is_empty());

    add_second_package(&user, root);

    assert!(md.is_file());
    assert!(!root.join(common::static_xml_rel()).exists());
    assert_eq!(file_bytes(&deps), before_deps);
    let index = fs::read_to_string(root.join(common::index_rel())).unwrap();
    assert!(
        index.contains(&format!("static = \"{}\"", common::static_md_rel())),
        "{index}"
    );
    for launcher in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let body = fs::read_to_string(root.join(launcher)).unwrap();
        assert!(
            body.contains(&common::static_md_rel()),
            "{launcher}: {body}"
        );
    }
}
