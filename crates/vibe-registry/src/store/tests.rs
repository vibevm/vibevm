//! Unit tests for the machine store: the reclaim API, the
//! `--older-than` walk, and the integrity sidecar lifecycle (written
//! by `insert_at` write-once, dead with its entry). Split from
//! `store.rs` along the file-length budget seam — same spec unit as
//! the module it tests.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#layout");

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;

/// A tiny shippable source tree for `insert_at`: a `vibe.toml`
/// carrying the identity the entry is keyed by.
fn src_pkg(root: &Path, group: &str, name: &str, version: &str) -> PathBuf {
    let group_dir = group.replace('.', "-");
    let dir = root.join(format!("src-{group_dir}-{name}-{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n"
        ),
    )
    .unwrap();
    dir
}

fn v(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

fn g(s: &str) -> Group {
    Group::parse(s).unwrap()
}

/// Removing the last version of a name prunes the empty
/// `<group>/<name>/` and `<group>/` directories — no husk survives
/// to name the deleted package.
#[test]
fn remove_entry_prunes_emptied_parents() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();
    assert!(root.join("org.example/wal/v0.1.0").is_dir());

    assert!(remove_entry_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap());
    assert!(!root.join("org.example/wal/v0.1.0").exists());
    assert!(
        !root.join("org.example/wal").exists(),
        "the name dir must not linger"
    );
    assert!(
        !root.join("org.example").exists(),
        "the emptied group dir must not linger"
    );
}

/// Removing one version of a multi-version name leaves its siblings
/// — and their parent directories — intact.
#[test]
fn remove_entry_keeps_sibling_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    for ver in ["0.1.0", "0.2.0"] {
        let src = src_pkg(tmp.path(), "org.example", "wal", ver);
        insert_at(&root, &src, &g("org.example"), "wal", &v(ver)).unwrap();
    }
    assert!(remove_entry_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap());
    assert!(!root.join("org.example/wal/v0.1.0").exists());
    assert!(
        root.join("org.example/wal/v0.2.0").is_dir(),
        "the sibling survives"
    );
    // Absent identity: nothing removed, not an error.
    assert!(!remove_entry_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap());
}

/// `remove_name_at` takes every version of the name in one call and
/// reports how many entries died.
#[test]
fn remove_name_takes_all_versions_and_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    for ver in ["0.1.0", "0.2.0"] {
        let src = src_pkg(tmp.path(), "org.example", "wal", ver);
        insert_at(&root, &src, &g("org.example"), "wal", &v(ver)).unwrap();
    }
    // A second name keeps the group dir alive after wal goes.
    let src = src_pkg(tmp.path(), "org.example", "other", "1.0.0");
    insert_at(&root, &src, &g("org.example"), "other", &v("1.0.0")).unwrap();

    assert_eq!(remove_name_at(&root, &g("org.example"), "wal").unwrap(), 2);
    assert!(!root.join("org.example/wal").exists());
    assert!(root.join("org.example/other/v1.0.0").is_dir());
    assert!(
        root.join("org.example").is_dir(),
        "the group dir survives its living name"
    );
    assert_eq!(
        remove_name_at(&root, &g("org.example"), "ghost").unwrap(),
        0
    );
}

/// The `--older-than` walk partitions by the entry directory's
/// mtime against the cutoff: everything is older than a far-future
/// cutoff, nothing is older than the epoch.
#[test]
fn older_than_partitions_by_cutoff() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();

    let far_future = SystemTime::now() + std::time::Duration::from_secs(86_400 * 365);
    assert_eq!(list_older_than_at(&root, far_future).len(), 1);
    assert_eq!(
        list_older_than_at(&root, SystemTime::UNIX_EPOCH).len(),
        0,
        "nothing predates the epoch"
    );
}

/// `remove_all_at` empties the store but keeps the root itself, and
/// counts the entries that died.
#[test]
fn remove_all_empties_but_keeps_the_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    for (name, ver) in [("wal", "0.1.0"), ("other", "1.0.0")] {
        let src = src_pkg(tmp.path(), "org.example", name, ver);
        insert_at(&root, &src, &g("org.example"), name, &v(ver)).unwrap();
    }
    // A foreign file in the root is not ours — it survives --all.
    fs::write(root.join("foreign.txt"), "operator's own\n").unwrap();

    assert_eq!(remove_all_at(&root).unwrap(), 2);
    assert!(root.is_dir(), "the store root itself survives");
    assert!(
        root.join("foreign.txt").is_file(),
        "a foreign file in the root is not ours to touch"
    );
    assert!(list_all_at(&root).is_empty());
}

// ---------------------------------------------------------------------------
// The integrity sidecar
// ---------------------------------------------------------------------------

/// A fresh insert records the content hash of the bytes it landed, as
/// a sidecar SIBLING of the version directory — never inside it (a
/// file inside the entry would change the hashed tree itself).
#[test]
fn insert_records_sidecar_beside_the_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();

    let sidecar = root.join("org.example/wal/v0.1.0.sha256");
    assert!(sidecar.is_file(), "the sidecar lands beside the entry");
    let recorded = recorded_hash_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap();
    let recorded = recorded.expect("the sidecar carries a line");
    let computed =
        crate::shippable::compute_content_hash(&root.join("org.example/wal/v0.1.0")).unwrap();
    assert_eq!(
        recorded, computed,
        "the sidecar records the hash of the entry as inserted"
    );
    assert!(recorded.starts_with("sha256:"));
    assert!(
        !root.join("org.example/wal/v0.1.0/v0.1.0.sha256").exists(),
        "never inside the entry — that would change the hashed tree"
    );
}

/// The sidecar is write-once: a second insert of the same identity is
/// `AlreadyPresent` and must not rewrite the record — not even a
/// hand-tampered one (the record, not the re-insert, is the authority
/// the sweep compares against).
#[test]
fn second_insert_never_rewrites_the_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();

    // Hand-tamper the record, then re-insert: write-once means the
    // tampered line stands (and `vibe cache check` will name it).
    let sidecar = root.join("org.example/wal/v0.1.0.sha256");
    fs::write(&sidecar, "sha256:deadbeef\n").unwrap();
    let src2 = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    let outcome = insert_at(&root, &src2, &g("org.example"), "wal", &v("0.1.0")).unwrap();
    assert!(matches!(outcome, InsertOutcome::AlreadyPresent(_)));
    assert_eq!(
        fs::read_to_string(&sidecar).unwrap(),
        "sha256:deadbeef\n",
        "a re-insert must not rewrite the recorded hash"
    );
    // And the explicit recorder honours the same rule.
    assert!(!record_hash_at(&root, &g("org.example"), "wal", &v("0.1.0"), "sha256:other").unwrap());
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "sha256:deadbeef\n");
}

/// The sidecar dies with its entry: `remove_entry_at` takes the record
/// too, so a reclaimed version leaves no residue — and the parent
/// pruning still fires (the sidecar file no longer holds the name dir
/// open).
#[test]
fn remove_entry_kills_the_sidecar_and_still_prunes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();
    assert!(root.join("org.example/wal/v0.1.0.sha256").is_file());

    assert!(remove_entry_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap());
    assert!(!root.join("org.example/wal/v0.1.0.sha256").exists());
    assert!(
        !root.join("org.example/wal").exists(),
        "the name dir still prunes with the sidecar gone too"
    );
}

/// `record_hash_at` writes only when absent, and `recorded_hash_at`
/// distinguishes a missing record (`None` — the unrecorded class)
/// from a present one.
#[test]
fn record_hash_writes_only_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let src = src_pkg(tmp.path(), "org.example", "wal", "0.1.0");
    insert_at(&root, &src, &g("org.example"), "wal", &v("0.1.0")).unwrap();
    // Erase the recorded sidecar — the "inserted before hash
    // recording / interrupted write" state.
    fs::remove_file(root.join("org.example/wal/v0.1.0.sha256")).unwrap();

    assert_eq!(
        recorded_hash_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap(),
        None,
        "an erased sidecar reads back as the unrecorded class"
    );
    assert!(record_hash_at(&root, &g("org.example"), "wal", &v("0.1.0"), "sha256:abc").unwrap());
    assert_eq!(
        recorded_hash_at(&root, &g("org.example"), "wal", &v("0.1.0")).unwrap(),
        Some("sha256:abc".to_string())
    );
    assert!(!record_hash_at(&root, &g("org.example"), "wal", &v("0.1.0"), "sha256:xyz").unwrap());
}
