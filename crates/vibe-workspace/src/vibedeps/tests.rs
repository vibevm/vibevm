//! Unit tests for the vibedeps cell, out-of-line per the file-length
//! budget. Included via cfg(test) #[path] mod tests; the module-tree
//! position is unchanged, so use super::* resolves as in the inline form.
//!
//! Non-`#[test]` helpers carry `#[cfg(test)]` so the file-grain conform
//! frontend scopes their `unwrap`s as test code.

use super::*;
use specmark::verifies;
use tempfile::TempDir;

#[cfg(test)]
fn version(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

/// The one test group — slots are keyed by identity, and every case here
/// exercises layout, not group variety.
#[cfg(test)]
fn g(s: &str) -> Group {
    Group::parse(s).unwrap()
}

/// Write `body` to `dir/rel`, creating parent directories.
#[cfg(test)]
fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn slot_rel_path_is_group_name_version() {
    let rel = slot_rel_path(&g("org.vibevm"), "wal", &version("0.3.0"));
    assert_eq!(rel, "vibedeps/org.vibevm.wal/0.3.0");
}

#[test]
fn slot_abs_path_joins_under_workspace_root() {
    let root = Path::new("ws-root");
    let abs = slot_abs_path(root, &g("org.vibevm"), "rust", &version("2.1.0"));
    assert!(abs.starts_with(root));
    assert!(abs.ends_with(Path::new("vibedeps/org.vibevm.rust/2.1.0")));
}

#[test]
fn materialise_copies_the_tree_verbatim() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(
        src.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n",
    );
    write(src.path(), "boot/10-flow-wal.md", "# boot");
    write(src.path(), "spec/flows/wal/WAL.md", "# protocol");

    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();

    let slot = ws.path().join("vibedeps/org.vibevm.wal/0.3.0");
    assert_eq!(
        fs::read_to_string(slot.join("vibe.toml")).unwrap(),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("boot/10-flow-wal.md")).unwrap(),
        "# boot"
    );
    assert_eq!(
        fs::read_to_string(slot.join("spec/flows/wal/WAL.md")).unwrap(),
        "# protocol"
    );
    // The returned footprint is slot-relative, forward-slashed, sorted.
    assert_eq!(
        written,
        vec![
            PathBuf::from("boot/10-flow-wal.md"),
            PathBuf::from("spec/flows/wal/WAL.md"),
            PathBuf::from("vibe.toml"),
        ]
    );
}

#[test]
fn materialise_skips_dot_git() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    write(src.path(), ".git/config", "[core]");
    write(src.path(), ".git/objects/ab/cdef", "blob");
    // A `.git` nested deeper than the root is skipped too.
    write(src.path(), "boot/.git/HEAD", "ref: refs/heads/main");
    write(src.path(), "boot/snippet.md", "# snippet");

    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "w",
        &version("1.0.0"),
        src.path(),
    )
    .unwrap();

    let slot = ws.path().join("vibedeps/org.vibevm.w/1.0.0");
    assert!(slot.join("vibe.toml").is_file());
    assert!(slot.join("boot/snippet.md").is_file());
    assert!(!slot.join(".git").exists());
    assert!(!slot.join("boot/.git").exists());
    assert_eq!(
        written,
        vec![PathBuf::from("boot/snippet.md"), PathBuf::from("vibe.toml")]
    );
}

#[test]
fn materialise_is_idempotent_and_clears_stale_files() {
    let ws = TempDir::new().unwrap();
    let src1 = TempDir::new().unwrap();
    write(src1.path(), "vibe.toml", "v1");
    write(src1.path(), "stale.md", "remove me");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "auth",
        &version("0.1.0"),
        src1.path(),
    )
    .unwrap();

    // Re-materialise from a source that no longer carries `stale.md`.
    let src2 = TempDir::new().unwrap();
    write(src2.path(), "vibe.toml", "v2");
    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "auth",
        &version("0.1.0"),
        src2.path(),
    )
    .unwrap();

    let slot = ws.path().join("vibedeps/org.vibevm.auth/0.1.0");
    assert_eq!(fs::read_to_string(slot.join("vibe.toml")).unwrap(), "v2");
    assert!(
        !slot.join("stale.md").exists(),
        "stale file must be cleared"
    );
    assert_eq!(written, vec![PathBuf::from("vibe.toml")]);
}

#[test]
fn materialise_errors_when_source_missing() {
    let ws = TempDir::new().unwrap();
    let missing = ws.path().join("no-such-source");
    let err = materialise(
        ws.path(),
        &g("org.vibevm"),
        "ghost",
        &version("0.1.0"),
        &missing,
    )
    .unwrap_err();
    assert!(matches!(err, WorkspaceError::Io { .. }), "{err}");
}

#[test]
fn is_materialised_reflects_slot_presence() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    assert!(!is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0")
    ));
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0"),
        src.path(),
    )
    .unwrap();
    assert!(is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0")
    ));
}

#[test]
fn remove_slot_deletes_and_reports() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();

    assert!(remove_slot(ws.path(), &g("org.vibevm"), "wal", &version("0.3.0")).unwrap());
    assert!(!is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0")
    ));
    // A second removal finds nothing to do.
    assert!(!remove_slot(ws.path(), &g("org.vibevm"), "wal", &version("0.3.0")).unwrap());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink",
    r = 1
)]
fn materialise_hardlink_mode_places_the_full_tree() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    write(src.path(), "boot/s.md", "# s");
    let written = materialise_with(
        ws.path(),
        &g("org.vibevm"),
        "w",
        &version("1.0.0"),
        src.path(),
        CopyMode::Hardlink,
    )
    .unwrap();
    let slot = ws.path().join("vibedeps/org.vibevm.w/1.0.0");
    // Hardlinked (or copy-fallback) — either way the content is present
    // and the footprint matches a copy materialisation.
    assert_eq!(fs::read_to_string(slot.join("vibe.toml")).unwrap(), "x");
    assert!(slot.join("boot/s.md").is_file());
    assert_eq!(
        written,
        vec![PathBuf::from("boot/s.md"), PathBuf::from("vibe.toml")]
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn in_place_slot_path_is_unversioned() {
    let rel = in_place_slot_rel_path(&g("org.vibevm"), "chromium");
    assert_eq!(rel, "vibedeps/org.vibevm.chromium");
    let abs = in_place_slot_abs_path(Path::new("ws"), &g("org.vibevm"), "chromium");
    assert!(abs.ends_with(Path::new("vibedeps/org.vibevm.chromium")));
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn materialise_in_place_moves_the_clone_keeping_git() {
    let ws = TempDir::new().unwrap();
    // A fetched clone: content plus a `.git` (the live working tree).
    let clone = TempDir::new().unwrap();
    write(clone.path(), "vibe.toml", "[package]\n");
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "src/main.rs", "fn main() {}");

    materialise_in_place(ws.path(), &g("org.vibevm"), "giant", clone.path()).unwrap();

    let slot = ws.path().join("vibedeps/org.vibevm.giant");
    assert!(slot.join("vibe.toml").is_file());
    assert!(slot.join("src/main.rs").is_file());
    // The `.git` is preserved — the slot stays a git working tree.
    assert!(slot.join(".git/HEAD").is_file());
    assert!(is_in_place_slot(ws.path(), &g("org.vibevm"), "giant"));
    // The source was moved, not copied.
    assert!(!clone.path().join("vibe.toml").exists());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn is_in_place_slot_false_for_a_versioned_copy() {
    let ws = TempDir::new().unwrap();
    // A versioned `copy` slot has no `.git` at the <group>.<name> level,
    // so it is not mistaken for an in-place slot.
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();
    assert!(!is_in_place_slot(ws.path(), &g("org.vibevm"), "wal"));
}

#[test]
fn remove_in_place_slot_deletes_and_reports() {
    let ws = TempDir::new().unwrap();
    let clone = TempDir::new().unwrap();
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "f", "x");
    materialise_in_place(ws.path(), &g("org.vibevm"), "big", clone.path()).unwrap();
    assert!(remove_in_place_slot(ws.path(), &g("org.vibevm"), "big").unwrap());
    assert!(!is_in_place_slot(ws.path(), &g("org.vibevm"), "big"));
    // A second removal finds nothing to do.
    assert!(!remove_in_place_slot(ws.path(), &g("org.vibevm"), "big").unwrap());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#vendoring",
    r = 1
)]
fn ensure_gitignored_appends_once() {
    let ws = TempDir::new().unwrap();
    ensure_gitignored(ws.path(), "vibedeps/org.vibevm.giant").unwrap();
    let gi = fs::read_to_string(ws.path().join(".gitignore")).unwrap();
    assert!(gi.contains("vibedeps/org.vibevm.giant/"), "{gi}");
    // Idempotent — a second call does not duplicate the entry.
    ensure_gitignored(ws.path(), "vibedeps/org.vibevm.giant").unwrap();
    let gi2 = fs::read_to_string(ws.path().join(".gitignore")).unwrap();
    assert_eq!(gi2.matches("vibedeps/org.vibevm.giant").count(), 1, "{gi2}");
}
