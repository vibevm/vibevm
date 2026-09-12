//! Reading a fixture recipe and the deferred list.

use std::fs;
use std::path::Path;

use super::{Captured, ancestry, read_deferred, read_fixture};

fn fixture(root: &Path, name: &str, body: &str) {
    let dir = root.join("examples").join(name);
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("example.toml"), body).expect("write");
}

#[test]
fn the_defaults_are_the_safe_classes_and_nothing_declared_is_needed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fixture(tmp.path(), "none", "schema = 1\n");
    let decl = read_fixture(tmp.path(), "none").expect("reads");
    assert!(decl.normalize.paths && decl.normalize.slashes && decl.normalize.crlf);
    assert!(decl.normalize.sort_blocks.is_empty());
    assert!(decl.fixture.from.is_none());
    assert!(decl.fixture.step.is_empty());
}

#[test]
fn an_unknown_schema_is_refused_rather_than_guessed_at() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fixture(tmp.path(), "none", "schema = 2\n");
    let refused = read_fixture(tmp.path(), "none").expect_err("refused");
    assert!(refused.to_string().contains("refuses to guess"));
}

#[test]
fn an_unknown_key_is_refused_so_a_typo_never_reads_as_a_default() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fixture(
        tmp.path(),
        "none",
        "schema = 1\n[normalize]\nslashez = true\n",
    );
    let refused = read_fixture(tmp.path(), "none").expect_err("refused");
    assert!(refused.to_string().contains("does not parse"));
}

#[test]
fn a_missing_fixture_names_the_file_the_package_owes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let refused = read_fixture(tmp.path(), "ghost").expect_err("refused");
    let text = refused.to_string();
    assert!(text.contains("examples/ghost/example.toml"), "{text}");
}

#[test]
fn the_ancestry_comes_back_parent_first() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fixture(tmp.path(), "none", "schema = 1\n");
    fixture(
        tmp.path(),
        "empty",
        "schema = 1\n[fixture]\nfrom = \"none\"\n",
    );
    fixture(
        tmp.path(),
        "hello",
        "schema = 1\n[fixture]\nfrom = \"empty\"\n[[fixture.step]]\nrun = \"vibe init hello\"\n",
    );
    let chain = ancestry(tmp.path(), "hello").expect("resolves");
    let names: Vec<&str> = chain.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["none", "empty", "hello"]);
}

#[test]
fn a_recipe_cycle_is_refused_by_name_rather_than_hung_on() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fixture(tmp.path(), "a", "schema = 1\n[fixture]\nfrom = \"b\"\n");
    fixture(tmp.path(), "b", "schema = 1\n[fixture]\nfrom = \"a\"\n");
    let refused = ancestry(tmp.path(), "a").expect_err("refused");
    assert!(refused.to_string().contains("its own ancestor"));
}

#[test]
fn an_absent_deferred_list_means_nothing_is_deferred() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(read_deferred(tmp.path()).expect("reads").is_empty());
}

#[test]
fn a_deferred_entry_carries_the_event_that_will_capture_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("examples")).expect("mkdir");
    fs::write(
        tmp.path().join("examples/deferred.toml"),
        "schema = 1\n\n[[deferred]]\npage = \"start/install-vibe.xml\"\nid = \"windows-install\"\n\
         captured = \"release\"\nreason = \"the installer prints from a release distribution\"\n",
    )
    .expect("write");
    let list = read_deferred(tmp.path()).expect("reads");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].captured, Captured::Release);
    assert_eq!(list[0].captured.as_str(), "release");
}
