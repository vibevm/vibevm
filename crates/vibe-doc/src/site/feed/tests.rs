//! The host half of the feed, against a checkout built in a temporary
//! directory.
//!
//! The registry half needs an index over HTTP and is exercised by the
//! live run of the whole builder; what a test can hold still is the
//! checkout — and the checkout is where the render key is COMPUTED rather
//! than read, which makes it the half worth pinning.

use std::fs;
use std::path::Path;

use super::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    fs::write(path, text).expect("writing");
}

/// A checkout shaped like the host's: a `[project]` root, a spec tree,
/// and one in-tree documentation package beside one package of another
/// kind.
fn checkout(root: &Path) {
    write(
        &root.join("vibe.toml"),
        "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\nversion = \"1.0.0\"\n",
    );
    write(&root.join("README.md"), "# vibevm\n");
    write(
        &root.join("vibevm/vibespecs/common/PROP-001.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">One</title></spec>\n",
    );
    write(
        &root.join("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibe.toml"),
        "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nversion = \"0.1.0\"\nkind = \"doc\"\n",
    );
    write(
        &root.join("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/page.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Page</title></spec>\n",
    );
    // Another kind, with something expensive inside it: the walk must
    // never enter it.
    write(
        &root.join("vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/vibe.toml"),
        "[package]\nname = \"web\"\ngroup = \"org.vibevm.doc\"\nversion = \"0.1.0\"\nkind = \"app\"\n",
    );
    write(
        &root.join("vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/node_modules/x/index.js"),
        "module.exports = 1;\n",
    );
}

fn host(root: &Path) -> Host {
    Host {
        git: "https://example.invalid/host".into(),
        git_ref: "main".into(),
        checkout: root.to_path_buf(),
        debounce_minutes: 60,
    }
}

#[test]
fn the_host_contributes_its_project_and_its_documentation_packages() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(tmp.path());
    let (pairs, line) = host_feed(&host(tmp.path())).expect("a feed");

    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].spelled(), "org.vibevm.core/vibevm@1.0.0");
    assert!(matches!(pairs[0].origin, Origin::HostProject { .. }));
    assert_eq!(pairs[1].spelled(), "org.vibevm.core/vibevm-docs@0.1.0");
    assert!(matches!(pairs[1].origin, Origin::HostPackage { .. }));
    assert!(line.contains("1 documentation package(s)"), "{line}");
}

/// `##SITE-HOST-CHECKOUT` names two things the host contributes, and the
/// site package is neither: it reaches a site by being published.
#[test]
fn a_package_of_another_kind_is_not_a_host_pair() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(tmp.path());
    let (pairs, _) = host_feed(&host(tmp.path())).expect("a feed");
    assert!(
        !pairs.iter().any(|p| p.name == "web"),
        "the app package is in the feed"
    );
}

/// The commit cannot answer this question, and that is why the key is a
/// recomputation: an in-tree package keeps its version number while its
/// content moves.
#[test]
fn an_edit_in_place_moves_the_render_key_although_the_version_stands_still() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(tmp.path());
    let before = host_feed(&host(tmp.path())).expect("a feed").0;

    write(
        &tmp.path()
            .join("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/page.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Page two</title></spec>\n",
    );
    let after = host_feed(&host(tmp.path())).expect("a feed").0;

    assert_eq!(before[1].version, after[1].version);
    assert_ne!(before[1].content_hash, after[1].content_hash);
    // The project's own hash covers different files and must not move
    // when a package beside it does.
    assert_eq!(before[0].content_hash, after[0].content_hash);
}

/// A hash over contents alone would miss a rename.
#[test]
fn a_renamed_page_moves_the_render_key() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(tmp.path());
    let before = host_feed(&host(tmp.path())).expect("a feed").0;

    let pages = tmp
        .path()
        .join("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs");
    fs::rename(pages.join("page.xml"), pages.join("renamed.xml")).expect("the rename");
    let after = host_feed(&host(tmp.path())).expect("a feed").0;
    assert_ne!(before[1].content_hash, after[1].content_hash);
}

/// Two runs over an unchanged checkout must key the same content the same
/// way, or every poll would rebuild everything.
#[test]
fn a_checkout_that_did_not_move_keys_the_same() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(tmp.path());
    let first = host_feed(&host(tmp.path())).expect("a feed").0;
    let second = host_feed(&host(tmp.path())).expect("a feed").0;
    assert_eq!(first, second);
}

#[test]
fn a_checkout_that_is_not_there_names_what_the_deploy_was_meant_to_do() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let message = host_feed(&host(&tmp.path().join("absent")))
        .expect_err("a refusal")
        .to_string();
    assert!(message.contains("holds no `vibe.toml`"), "{message}");
    assert!(message.contains("git pull"), "{message}");
}

#[test]
fn a_checkout_whose_manifest_names_no_coordinate_is_refused() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    write(&tmp.path().join("vibe.toml"), "[project]\nname = \"x\"\n");
    let message = host_feed(&host(tmp.path()))
        .expect_err("a refusal")
        .to_string();
    assert!(message.contains("[project]` coordinate"), "{message}");
}
