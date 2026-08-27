//! Reds for [`Workspace::discover_with_selected_manifest`].
//!
//! Every test here CORRUPTS or DELETES the selected node's `vibe.toml` between
//! the caller's read and the discovery. That is the only way to prove the
//! override is real: an implementation that quietly read disk again would pass
//! a happy-path test byte for byte, and fail every test below.

use super::*;
use std::fs;
use tempfile::TempDir;

#[cfg(test)]
fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[cfg(test)]
fn project(name: &str) -> String {
    format!("[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n")
}

#[cfg(test)]
fn package(name: &str, kind: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n"
    )
}

/// The caller's read, made once, exactly as a command makes it.
#[cfg(test)]
fn snapshot(dir: &Path) -> Manifest {
    Manifest::read(dir.join(Manifest::FILENAME)).unwrap()
}

/// What a command's own `--git` rewrite — or an editor, or a concurrent
/// process — can do to the file after the snapshot was taken.
#[cfg(test)]
fn corrupt(dir: &Path) {
    fs::write(dir.join(Manifest::FILENAME), "[project\nname = broken\n").unwrap();
}

#[cfg(test)]
fn package_name(manifest: &Manifest) -> String {
    manifest.package.as_ref().unwrap().name.to_string()
}

#[test]
fn a_standalone_node_loads_from_the_snapshot_after_the_file_is_corrupted() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), Manifest::FILENAME, &project("solo"));
    let snapshot = snapshot(tmp.path());

    corrupt(tmp.path());
    assert!(
        Workspace::discover(tmp.path()).is_err(),
        "the ordinary path really is broken now — which is what makes the next line a proof",
    );

    let ws = Workspace::discover_with_selected_manifest(tmp.path(), &snapshot).unwrap();
    assert_eq!(ws.root_manifest.project.as_ref().unwrap().name, "solo");
    assert!(ws.members.is_empty());
}

#[test]
fn a_deleted_selected_manifest_still_selects_that_node() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), Manifest::FILENAME, &project("solo"));
    let snapshot = snapshot(tmp.path());
    fs::remove_file(tmp.path().join(Manifest::FILENAME)).unwrap();

    let ws = Workspace::discover_with_selected_manifest(tmp.path(), &snapshot).unwrap();
    assert_eq!(ws.root_manifest.project.as_ref().unwrap().name, "solo");
    assert_eq!(ws.root, canonical(tmp.path()).unwrap());
}

#[test]
fn a_selected_workspace_root_still_expands_its_members_after_corruption() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!("{}\n[workspace]\nmembers = [\"child\"]\n", project("root")),
    );
    write(tmp.path(), "child/vibe.toml", &package("child", "flow"));
    let snapshot = snapshot(tmp.path());

    corrupt(tmp.path());
    let ws = Workspace::discover_with_selected_manifest(tmp.path(), &snapshot).unwrap();
    assert_eq!(ws.members.len(), 1);
    assert_eq!(ws.members[0].rel_path.as_str(), "child");
    assert_eq!(package_name(&ws.members[0].manifest), "child");
}

#[test]
fn a_selected_member_comes_from_the_snapshot_and_still_finds_its_root() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!("{}\n[workspace]\nmembers = [\"child\"]\n", project("root")),
    );
    let child = tmp.path().join("child");
    write(tmp.path(), "child/vibe.toml", &package("child", "flow"));
    let snapshot = snapshot(&child);

    corrupt(&child);
    let ws = Workspace::discover_with_selected_manifest(&child, &snapshot).unwrap();
    assert_eq!(
        ws.root,
        canonical(tmp.path()).unwrap(),
        "the ancestor root is still discovered",
    );
    assert_eq!(
        package_name(&ws.member_by_rel_path("child").unwrap().manifest),
        "child",
        "and the selected member is the snapshot, not the corrupt file",
    );
}

/// Nesting: the TOPMOST enclosing workspace wins, exactly as it does without
/// an override.
#[test]
fn a_nested_selected_member_returns_the_topmost_enclosing_root() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!("{}\n[workspace]\nmembers = [\"mid\"]\n", project("top")),
    );
    write(
        tmp.path(),
        "mid/vibe.toml",
        &format!("{}\n[workspace]\nmembers = [\"leaf\"]\n", project("mid")),
    );
    let leaf = tmp.path().join("mid").join("leaf");
    write(tmp.path(), "mid/leaf/vibe.toml", &package("leaf", "flow"));
    let snapshot = snapshot(&leaf);

    corrupt(&leaf);
    let ws = Workspace::discover_with_selected_manifest(&leaf, &snapshot).unwrap();
    assert_eq!(ws.root, canonical(tmp.path()).unwrap());
    let selected = ws
        .member_by_rel_path("mid/leaf")
        .expect("the leaf is a member of the topmost root");
    assert_eq!(package_name(&selected.manifest), "leaf");
}

/// The clone inside the workspace is FINALISED with the rest of the tree; the
/// caller's snapshot stays raw. Both halves matter: the loader must not hand
/// back an unresolved member, and it must not mutate the value the command is
/// about to write back to disk.
#[test]
fn the_selected_clone_is_version_finalised_while_the_snapshot_stays_raw() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!(
            "{}\n[workspace]\nmembers = [\"child\"]\n\n[workspace.versions]\nwal = \"^0.3\"\n",
            project("root")
        ),
    );
    let child = tmp.path().join("child");
    write(
        tmp.path(),
        "child/vibe.toml",
        &format!(
            "{}\n[requires]\npackages = {{ \"org.vibevm/wal\" = {{ version.var = \"wal\" }} }}\n",
            package("child", "flow")
        ),
    );
    let snapshot = snapshot(&child);
    assert_eq!(
        snapshot.requires.var_packages.len(),
        1,
        "the caller's copy is raw, with the placeholder unresolved",
    );

    corrupt(&child);
    let ws = Workspace::discover_with_selected_manifest(&child, &snapshot).unwrap();
    let selected = ws.member_by_rel_path("child").unwrap();
    assert!(
        selected.manifest.requires.var_packages.is_empty(),
        "the loader's clone resolved the placeholder against the ancestor table",
    );
    assert_eq!(
        selected.manifest.requires.packages.len(),
        1,
        "and folded it into a concrete requirement",
    );
    assert_eq!(
        snapshot.requires.var_packages.len(),
        1,
        "while the caller's own copy was not touched",
    );
}

/// The override is for ONE node. A different malformed member is still an
/// error — the seam narrows a single read, it does not make loading tolerant.
#[test]
fn a_malformed_sibling_still_fails() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!(
            "{}\n[workspace]\nmembers = [\"a\", \"b\"]\n",
            project("root")
        ),
    );
    let a = tmp.path().join("a");
    write(tmp.path(), "a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "b/vibe.toml", &package("b", "flow"));
    let snapshot = snapshot(&a);

    corrupt(&a);
    corrupt(&tmp.path().join("b"));
    assert!(
        Workspace::discover_with_selected_manifest(&a, &snapshot).is_err(),
        "a sibling nobody vouched for is read from disk, and it is broken",
    );
}

/// A selected node that is itself a nested workspace still expands its own
/// members from the snapshot's `[workspace]` table.
#[test]
fn a_selected_nested_workspace_expands_its_own_members() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!("{}\n[workspace]\nmembers = [\"mid\"]\n", project("top")),
    );
    let mid = tmp.path().join("mid");
    write(
        tmp.path(),
        "mid/vibe.toml",
        &format!("{}\n[workspace]\nmembers = [\"leaf\"]\n", project("mid")),
    );
    write(tmp.path(), "mid/leaf/vibe.toml", &package("leaf", "flow"));
    let snapshot = snapshot(&mid);

    corrupt(&mid);
    let ws = Workspace::discover_with_selected_manifest(&mid, &snapshot).unwrap();
    assert_eq!(ws.root, canonical(tmp.path()).unwrap());
    assert!(
        ws.member_by_rel_path("mid/leaf").is_some(),
        "the selected node's own members expanded from the snapshot: {:?}",
        ws.members
            .iter()
            .map(|m| m.rel_path.as_str())
            .collect::<Vec<_>>(),
    );
}

/// The match is on the CANONICAL path, so an equivalent spelling of the same
/// directory selects the same node — and a different node never does.
#[test]
fn only_the_exact_canonical_node_is_overridden() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!(
            "{}\n[workspace]\nmembers = [\"a\", \"b\"]\n",
            project("root")
        ),
    );
    let a = tmp.path().join("a");
    write(tmp.path(), "a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "b/vibe.toml", &package("b", "flow"));
    let snapshot = snapshot(&a);

    corrupt(&a);
    // An equivalent spelling of `a` — down into `b` and back out.
    let equivalent = tmp.path().join("b").join("..").join("a");
    let ws = Workspace::discover_with_selected_manifest(&equivalent, &snapshot).unwrap();
    assert_eq!(
        package_name(&ws.member_by_rel_path("a").unwrap().manifest),
        "a",
        "the same directory by another spelling is still the selected node",
    );

    // And the override never stands in for a DIFFERENT node: `b` is read from
    // disk, so corrupting it fails even though `a`'s snapshot is sound.
    corrupt(&tmp.path().join("b"));
    assert!(Workspace::discover_with_selected_manifest(&a, &snapshot).is_err());
}

/// An UNRELATED ancestor workspace higher up must not capture the selected
/// node: discovery stops at the topmost workspace whose tree really contains
/// it, and the override does not change that rule.
#[test]
fn an_unrelated_ancestor_workspace_does_not_capture_the_selected_node() {
    let tmp = TempDir::new().unwrap();
    // An enclosing workspace whose members list names something else entirely.
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!(
            "{}
[workspace]
members = [\"other\"]
",
            project("outer")
        ),
    );
    write(tmp.path(), "other/vibe.toml", &package("other", "flow"));
    let solo = tmp.path().join("solo");
    write(tmp.path(), "solo/vibe.toml", &package("solo", "flow"));
    let snapshot = snapshot(&solo);

    corrupt(&solo);
    let ws = Workspace::discover_with_selected_manifest(&solo, &snapshot).unwrap();
    assert_eq!(
        ws.root,
        canonical(&solo).unwrap(),
        "a node the ancestor never declared stands alone",
    );
    assert_eq!(package_name(&ws.root_manifest), "solo");
}

/// A member reached through a GLOB is still the exact selected node, and a
/// glob is exactly where a deleted manifest would otherwise be skipped in
/// silence rather than reported.
#[test]
fn a_glob_selected_member_survives_its_manifest_being_deleted() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        Manifest::FILENAME,
        &format!(
            "{}
[workspace]
members = [\"pkgs/*\"]
",
            project("root")
        ),
    );
    let leaf = tmp.path().join("pkgs").join("leaf");
    write(tmp.path(), "pkgs/leaf/vibe.toml", &package("leaf", "flow"));
    write(
        tmp.path(),
        "pkgs/other/vibe.toml",
        &package("other", "flow"),
    );
    let snapshot = snapshot(&leaf);

    std::fs::remove_file(leaf.join(Manifest::FILENAME)).unwrap();
    let ws = Workspace::discover_with_selected_manifest(&leaf, &snapshot).unwrap();
    assert_eq!(ws.root, canonical(tmp.path()).unwrap());
    assert_eq!(
        package_name(&ws.member_by_rel_path("pkgs/leaf").unwrap().manifest),
        "leaf",
        "the glob kept the selected node instead of skipping the empty directory",
    );
    assert!(
        ws.member_by_rel_path("pkgs/other").is_some(),
        "and its siblings still expanded",
    );
}
