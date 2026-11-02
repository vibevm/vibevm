//! `vibe init --kind mcp` and `--kind remap` scaffold a package their own
//! validators accept, and no kind leaves a folder behind when init refuses
//! (PROP-024 `#INIT-ROOT-ROLE`). The desktop shell's creation wizard runs
//! these commands and opens what they write without patching it.

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use vibe_core::manifest::Manifest;

mod common;

use common::UserScratch;

fn assert_checks(user: &UserScratch, project: &Path) {
    user.vibe()
        .args(["check", "--path"])
        .arg(project)
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest_validity").not());
}

fn manifest(package: &Path) -> Manifest {
    Manifest::parse_str(&fs::read_to_string(package.join("vibe.toml")).unwrap()).unwrap()
}

fn assert_declares_a_server(package: &Path, name: &str) {
    let manifest = manifest(package);
    let [server] = manifest.mcp_servers.as_slice() else {
        panic!("one server, got {:?}", manifest.mcp_servers);
    };
    assert_eq!(server.name, name);
    assert_eq!(server.binary.as_deref(), Some(name));
    assert_eq!(
        manifest.binaries[0].crate_dir,
        Path::new("crates").join(name)
    );
    let crate_dir = package.join("crates").join(name);
    assert!(crate_dir.join("Cargo.toml").is_file());
    assert!(crate_dir.join("src/main.rs").is_file());
}

fn assert_declares_a_row(package: &Path) {
    let manifest = manifest(package);
    let [row] = manifest.remap.rows.as_slice() else {
        panic!("one row, got {:?}", manifest.remap.rows);
    };
    assert_eq!(row.prefix.as_deref(), Some("https://example.org/"));
    assert_eq!(row.url.as_deref(), Some("https://mirror.example.org/"));
    assert!(row.reason.is_some());
    // A remap package may carry nothing a boot lane could run.
    assert!(manifest.boot_snippet.is_none());
}

fn init_root(user: &UserScratch, root: &Path, name: &str, kind: &str) {
    user.vibe()
        .args(["init", "--type", "package", "--group", "org.example"])
        .args(["--name", name, "--kind", kind, "--no-registry", "--path"])
        .arg(root)
        .assert()
        .success();
}

fn init_nested(user: &UserScratch, project: &Path, pkgref: &str, kind: &str) {
    user.vibe()
        .args(["init", "package", pkgref, "--kind", kind, "--path"])
        .arg(project)
        .assert()
        .success();
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn root_mcp_package_checks_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("server");
    init_root(&user, &root, "server", "mcp");
    assert_declares_a_server(&root, "server");
    assert_checks(&user, &root);

    // A second init keeps the author's server and adds nothing.
    let main = root.join("crates/server/src/main.rs");
    fs::write(&main, "fn main() { serve() }\n").unwrap();
    user.vibe()
        .args(["init", "--kind", "mcp", "--path"])
        .arg(&root)
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(&main).unwrap(),
        "fn main() { serve() }\n"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn nested_mcp_package_checks_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    init_nested(&user, dir.path(), "org.example/server", "mcp");
    let package = dir
        .path()
        .join(common::pack_rel("org.example/server/v0.1.0"));
    assert_declares_a_server(&package, "server");
    assert_checks(&user, dir.path());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn root_remap_package_checks_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("mirrors");
    init_root(&user, &root, "mirrors", "remap");
    assert_declares_a_row(&root);
    assert_checks(&user, &root);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn nested_remap_package_checks_as_scaffolded() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    init_nested(&user, dir.path(), "org.example/mirrors", "remap");
    let package = dir
        .path()
        .join(common::pack_rel("org.example/mirrors/v0.1.0"));
    assert_declares_a_row(&package);
    assert!(
        !package.join("vibevm").exists(),
        "no boot lane for a remap package"
    );
    assert_checks(&user, dir.path());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE")]
fn a_refused_init_leaves_no_folder_behind() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tool", "app", "mcp", "remap", "doc"] {
        // A version that is not a version refuses in both forms; the
        // refusal must come before the project container exists.
        let project = dir.path().join(format!("nested-{kind}"));
        user.vibe()
            .args(["init", "package", "org.example/broken", "--kind", kind])
            .args(["--version", "one", "--path"])
            .arg(&project)
            .assert()
            .failure();
        assert!(!project.exists(), "{kind}: nested refusal left {project:?}");

        let root = dir.path().join(format!("root-{kind}"));
        user.vibe()
            .args(["init", "--type", "package", "--group", "org.example"])
            .args(["--name", "broken", "--kind", kind, "--version", "one"])
            .arg("--path")
            .arg(&root)
            .assert()
            .failure();
        assert!(!root.exists(), "{kind}: root refusal left {root:?}");
    }
}

#[test]
fn a_quote_in_the_description_lands_escaped_in_the_manifest() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("nested-quoted");
    user.vibe()
        .args(["init", "package", "org.example/quoted", "--kind", "tool"])
        .args(["--description", "a \"quoted\" word", "--path"])
        .arg(&project)
        .assert()
        .success();
    let package = project.join(common::pack_rel("org.example/quoted/v0.1.0"));
    let manifest = std::fs::read_to_string(package.join("vibe.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&manifest).unwrap();
    assert_eq!(
        parsed["package"]["description"].as_str(),
        Some("a \"quoted\" word"),
        "{manifest}"
    );
}

/// Every free-text field a nested package takes, each carrying the three
/// characters that broke the interpolated TOML (B-214): a quote, a
/// backslash and a newline.
const DESCRIPTION: &str = "a \"quoted\" \\ back\\slash\nand a second line";
const LICENSE: &str = "Custom \"Licence\" \\ 1.0";
const AUTHOR: &str = "Jane \"JD\" Doe \\ <jane@example.org>";

fn assert_free_text_round_trips(user: &UserScratch, kind: &str, name: &str) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join(format!("nested-{kind}"));
    user.vibe()
        .args(["init", "package", &format!("org.example/{name}")])
        .args(["--kind", kind, "--description", DESCRIPTION])
        .args(["--license", LICENSE, "--author", AUTHOR, "--path"])
        .arg(&project)
        .assert()
        .success();
    let package = project.join(common::pack_rel(&format!("org.example/{name}/v0.1.0")));
    let text = fs::read_to_string(package.join("vibe.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let table = &parsed["package"];
    assert_eq!(table["description"].as_str(), Some(DESCRIPTION), "{text}");
    assert_eq!(table["license"].as_str(), Some(LICENSE), "{text}");
    let authors = table["authors"].as_array().expect("authors array");
    assert_eq!(authors.len(), 1, "{text}");
    assert_eq!(authors[0].as_str(), Some(AUTHOR), "{text}");
    assert_checks(user, &project);
}

#[test]
fn a_nested_tool_package_writes_free_text_through_the_toml_writer() {
    let user = UserScratch::new();
    assert_free_text_round_trips(&user, "tool", "escaped");
}

#[test]
fn a_nested_doc_package_writes_free_text_through_the_toml_writer() {
    let user = UserScratch::new();
    assert_free_text_round_trips(&user, "doc", "escaped-docs");
}
