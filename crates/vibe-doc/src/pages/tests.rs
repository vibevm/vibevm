//! Unit tests for the package page walk.

use std::fs;

use super::{PageSet, read_package};

fn page(dir: &std::path::Path, rel: &str, body: &str) {
    let path = dir.join("vibevm/vibespecs").join(rel);
    fs::create_dir_all(path.parent().expect("page has a parent")).expect("mkdir");
    fs::write(
        path,
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec xmlns=\"https://vibevm.org/spec/1\">\n{body}</spec>\n"
        ),
    )
    .expect("write page");
}

#[test]
fn a_package_without_a_spec_root_reads_as_an_empty_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let set = read_package(tmp.path()).expect("an absent spec root is not a failure");
    assert_eq!(set.total(), 0);
}

#[test]
fn pages_come_back_in_address_order_regardless_of_walk_order() {
    let tmp = tempfile::tempdir().expect("tempdir");
    for rel in ["zz/last.xml", "aa/first.xml", "mm/middle.xml"] {
        page(tmp.path(), rel, "  <title id=\"root\">T</title>\n");
    }
    let set = read_package(tmp.path()).expect("reads");
    let order: Vec<&str> = set.pages.iter().map(|p| p.rel.as_str()).collect();
    assert_eq!(order, ["aa/first.xml", "mm/middle.xml", "zz/last.xml"]);
}

#[test]
fn a_non_xml_file_under_the_spec_root_is_not_a_page() {
    let tmp = tempfile::tempdir().expect("tempdir");
    page(tmp.path(), "a.xml", "  <title id=\"root\">T</title>\n");
    fs::write(
        tmp.path().join("vibevm/vibespecs/README.md"),
        "# not a page\n",
    )
    .expect("write");
    let set = read_package(tmp.path()).expect("reads");
    assert_eq!(set.total(), 1);
}

#[test]
fn the_documentation_vocabulary_is_open_for_a_documentation_package() {
    let tmp = tempfile::tempdir().expect("tempdir");
    page(
        tmp.path(),
        "p.xml",
        "  <title id=\"root\">T</title>\n  <note kind=\"tip\">Read this.</note>\n",
    );
    let set = read_package(tmp.path()).expect("reads");
    assert!(
        set.unreadable.is_empty(),
        "`note` is a member of the genre and must read: {:?}",
        set.unreadable
    );
}

#[test]
fn an_unreadable_page_is_collected_and_never_stops_the_walk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    page(tmp.path(), "good.xml", "  <title id=\"root\">T</title>\n");
    page(
        tmp.path(),
        "bad.xml",
        "  <title id=\"root\">T</title>\n  <pretzel>not in the dialect</pretzel>\n",
    );
    let set: PageSet = read_package(tmp.path()).expect("the walk itself does not fail");
    assert_eq!(set.pages.len(), 1, "the good page still reads");
    assert_eq!(set.unreadable.len(), 1);
    assert_eq!(set.unreadable[0].rel, "bad.xml");
    assert!(
        set.unreadable[0].message.contains("pretzel"),
        "the pivot's own refusal is carried through verbatim: {}",
        set.unreadable[0].message
    );
}
