//! B-072 — the index writes itself idempotently by content.
//!
//! An identical mutation changes ZERO bytes on disk; only a real change
//! earns a fresh `generated_at`. Asserted on the bytes and the stamp
//! rather than on the writer's own report, because the writer's report is
//! exactly what these tests exist to doubt.
//!
//! File-backed submodule of [`super`] so every cell stays inside the
//! AI-Native file budget. Fixtures and the fixed clock are [`super`]'s;
//! nothing moved but the file.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use super::*;

/// A later wall-clock reading — the second command's clock, one hour
/// past the standing fixed `now()`.
fn later() -> DateTime<Utc> {
    now() + chrono::Duration::hours(1)
}

#[cfg(test)]
fn snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    walk(root)
        .into_iter()
        .map(|rel| {
            let bytes = std::fs::read(root.join(&rel)).unwrap();
            (rel, bytes)
        })
        .collect()
}

/// B-072, the invariant itself: two identical upserts ⇒ after the
/// second write NOT ONE file of the catalog changed bytes — and the
/// manifest keeps the FIRST write's stamp. Before the fix the second
/// write stamped a fresh `generated_at` into `repomd.json` (and fresh
/// by-name labels), so an identical mutation always dirtied the tree.
#[test]
fn identical_rewrite_changes_zero_bytes_and_keeps_the_stamp() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Stack, org(), "rust", "0.1.0"));
    idx.tombstones.insert(
        "dead-pkg".into(),
        crate::types::Tombstone {
            reason: "superseded".into(),
            superseded_by: None,
        },
    );
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    let before = snapshot(tmp.path());

    // The identical mutation again — a full hour later by the clock.
    assert!(
        !idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0")),
        "the repeat changes no state"
    );
    idx.write_to(tmp.path(), &WriteCtx { at: later() }).unwrap();

    assert_eq!(
        snapshot(tmp.path()),
        before,
        "an identical mutation must change zero bytes on disk"
    );
    assert_eq!(
        repomd::read(tmp.path()).unwrap().generated_at,
        now(),
        "the stamp is not refreshed when nothing changed"
    );
}

/// The other half of the invariant: a REAL change gets the fresh
/// stamp and new bytes — idempotence must never swallow an update.
#[test]
fn real_change_gets_the_fresh_stamp_and_new_bytes() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    let before = snapshot(tmp.path());

    let mut changed = entry(PackageKind::Flow, org(), "wal", "0.1.0");
    changed.description = Some("a real content change".into());
    assert!(idx.upsert(changed), "a differing entry is a real update");
    idx.write_to(tmp.path(), &WriteCtx { at: later() }).unwrap();

    let manifest = repomd::read(tmp.path()).unwrap();
    assert_eq!(
        manifest.generated_at,
        later(),
        "a real change earns the fresh stamp"
    );
    assert_ne!(snapshot(tmp.path()), before, "the bytes moved");
    let by_name_bytes = std::fs::read(by_name::file_path(tmp.path(), "wal")).unwrap();
    assert!(
        String::from_utf8_lossy(&by_name_bytes).contains("a real content change"),
        "the changed content is in the published catalog"
    );
}

/// The first write into an empty directory is the unchanged path: no
/// on-disk manifest to compare against ⇒ project with the stamp the
/// caller brought, stamp it, land every file.
#[test]
fn first_write_into_an_empty_dir_stamps_and_lands() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let manifest = repomd::read(tmp.path()).unwrap();
    assert_eq!(manifest.generated_at, now(), "the first write stamps");
    assert!(manifest.files.contains_key("by-name/wal.json"));
    assert!(tmp.path().join("hello.json").is_file());
    assert!(by_name::file_path(tmp.path(), "wal").is_file());
}

/// F2-1, the test the phase exists for: one index state + one
/// `WriteCtx` ⇒ byte-identical output across two independent
/// writes into two different directories. A writer that called
/// the clock internally would stamp two different `now()`s and
/// this test would go red the moment it measured them.
#[test]
fn same_state_and_ctx_write_byte_identical_trees() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
    idx.upsert(entry(PackageKind::Stack, org(), "rust", "0.1.0"));
    idx.upsert(entry(
        PackageKind::Feat,
        Group::parse("com.acme").unwrap(),
        "wal",
        "1.0.0",
    ));
    idx.tombstones.insert(
        "dead-pkg".into(),
        crate::types::Tombstone {
            reason: "superseded".into(),
            superseded_by: None,
        },
    );
    let ctx = write_ctx();
    idx.write_to(left.path(), &ctx).unwrap();
    idx.write_to(right.path(), &ctx).unwrap();
    assert_trees_byte_identical(left.path(), right.path());
}

/// Walk both trees and compare every file path-for-path,
/// byte-for-byte. File *sets* must match too — a stray or missing
/// file is as much a difference as a changed byte.
#[cfg(test)]
fn assert_trees_byte_identical(a: &std::path::Path, b: &std::path::Path) {
    let mut la = walk(a);
    let mut lb = walk(b);
    la.sort();
    lb.sort();
    assert_eq!(la, lb, "the two trees hold different file sets");
    for rel in &la {
        let ca = std::fs::read(a.join(rel)).unwrap();
        let cb = std::fs::read(b.join(rel)).unwrap();
        assert_eq!(
            ca,
            cb,
            "{} differs byte-for-byte between the two writes",
            rel.display()
        );
    }
}

#[cfg(test)]
fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
            } else {
                out.push(e.path().strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    out
}
