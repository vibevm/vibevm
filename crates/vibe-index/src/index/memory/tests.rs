//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path = "memory/tests.rs"] mod tests;`, so
//! the module-tree position — and therefore `use super::*` — is unchanged
//! from the inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s as
//! test code.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use super::*;
use crate::index::quarantine;
use crate::types::{PackageKind, VersionEntry};
use chrono::{DateTime, Utc};
use tempfile::tempdir;
use vibe_core::Group;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// The standing test index: registry `vibespecs` on example.invalid,
/// stamped at the standing fixed `now()`.
fn fresh_index() -> Index {
    Index::new(
        "vibespecs",
        "https://example.invalid",
        NamingConvention::Fqdn,
        now(),
    )
}

/// The standing write context at the same fixed instant.
fn write_ctx() -> WriteCtx {
    WriteCtx { at: now() }
}

fn entry(kind: PackageKind, group: Group, name: &str, version: &str) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group,
        name: name.into(),
        version: version.parse().unwrap(),
        content_hash: format!("sha256:{name}{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: None,
        registry: "vibespecs".into(),
        workspace_origin: None,
        license: None,
        authors: vec![],
        description: None,
        homepage: None,
        keywords: vec![],
        describes: None,
        compatibility: None,
        provides: None,
        requires: None,
        requires_any: vec![],
        obsoletes: None,
        conflicts: None,
        features: None,
        subskills: vec![],
        i18n: None,
        boot_snippet: None,
        files_count: 1,
        must_understand: vec![],
        yanked: false,
        frozen: false,
        indexed_at: now(),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

#[test]
fn upsert_replaces_existing_version() {
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    assert_eq!(idx.version_count(), 1);
}

#[test]
fn remove_version_works() {
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
    let v = "0.1.0".parse().unwrap();
    assert!(idx.remove_version(&org(), "wal", &v));
    assert_eq!(idx.version_count(), 1);
}

#[test]
fn write_then_load_round_trips() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
    idx.upsert(entry(PackageKind::Flow, org(), "atomic-commits", "0.1.0"));
    idx.upsert(entry(PackageKind::Stack, org(), "rust-cli", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let back = Index::load_from(tmp.path()).unwrap();
    assert_eq!(back.registry, idx.registry);
    assert_eq!(back.registry_url, idx.registry_url);
    assert_eq!(back.naming, idx.naming);
    assert_eq!(back.package_count(), 3);
    assert_eq!(back.version_count(), 4);
    assert!(back.get(&org(), "wal").is_some());
}

#[test]
fn candidate_set_collapses_a_shared_name_into_one_file() {
    // Two groups publish a package called `wal` — a short-name
    // collision. They land in one `by-name/wal.json` candidate set.
    let tmp = tempdir().unwrap();
    let acme = Group::parse("com.acme").unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.upsert(entry(PackageKind::Feat, acme.clone(), "wal", "1.0.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    // One file, two candidates.
    assert!(by_name::file_path(tmp.path(), "wal").exists());
    let candidates = idx.candidates_for("wal");
    assert_eq!(candidates.len(), 2);

    let back = Index::load_from(tmp.path()).unwrap();
    assert_eq!(back.package_count(), 2);
    assert!(back.get(&org(), "wal").is_some());
    assert!(back.get(&acme, "wal").is_some());
}

#[test]
fn write_creates_repomd_with_file_hashes() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    let manifest = repomd::read(tmp.path()).unwrap();
    assert!(matches!(
        manifest.files.get("primary.jsonl"),
        Some(RepomdFileEntry::File { .. })
    ));
    assert!(matches!(
        manifest.files.get("by-name"),
        Some(RepomdFileEntry::Directory { .. })
    ));
    assert!(manifest.files.contains_key("by-name/wal.json"));
}

#[test]
fn write_replaces_stale_by_name_files() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    // Drop the package; the old file MUST be gone after rewrite.
    idx.remove_package(&org(), "wal");
    idx.upsert(entry(PackageKind::Flow, org(), "atomic-commits", "0.1.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    assert!(!by_name::file_path(tmp.path(), "wal").exists());
    assert!(by_name::file_path(tmp.path(), "atomic-commits").exists());
}

/// PROP-044 §4.5 — a version naming an unknown `must_understand`
/// capability is quarantined on load. Since the loader stopped
/// DROPPING quarantined versions, this test guards the KEEP contract:
/// the version STAYS in the index (the catalog is the journal's
/// projection; a reader's capabilities never shrink what is held) AND
/// leaves exactly one record in the quarantine carrier — while the
/// named accessor (`quarantine::usable_versions`) refuses to serve
/// it, so no answering surface can hand it out by accident.
#[test]
fn load_quarantines_unknown_capability_and_keeps_the_rest() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    let mut with_cap = entry(PackageKind::Flow, org(), "wal", "0.1.0");
    with_cap.must_understand = vec!["x".into()];
    idx.upsert(with_cap);
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let back = Index::load_from(tmp.path()).unwrap();
    // BOTH versions are held — the loader no longer drops the
    // quarantined one…
    let pkg = back.get(&org(), "wal").unwrap();
    assert_eq!(pkg.versions.len(), 2);
    // …the carrier names exactly it…
    assert_eq!(back.quarantined.len(), 1);
    let q = &back.quarantined[0];
    assert_eq!(q.name, "wal");
    assert_eq!(q.version.to_string(), "0.1.0");
    assert_eq!(q.missing, vec!["x".to_string()]);
    // …and the named accessor does not hand it to any answerer.
    let usable: Vec<String> = quarantine::usable_versions(pkg)
        .map(|v| v.version.to_string())
        .collect();
    assert_eq!(usable, vec!["0.2.0".to_string()]);
}

/// The other half of the asymmetry: the WRITER never asks `is_usable`.
/// A quarantined version that survived the load goes back out with the
/// next `write_to` — the catalog stays the full projection of the
/// journal, and a reader's capabilities never shrink what is written.
#[test]
fn quarantined_version_survives_a_load_write_round_trip() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    let mut with_cap = entry(PackageKind::Flow, org(), "wal", "0.1.0");
    with_cap.must_understand = vec!["x".into()];
    idx.upsert(with_cap);
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let back = Index::load_from(tmp.path()).unwrap();
    assert_eq!(back.version_count(), 1);
    back.write_to(tmp.path(), &write_ctx()).unwrap();

    let again = Index::load_from(tmp.path()).unwrap();
    assert_eq!(
        again.version_count(),
        1,
        "the writer must re-project the quarantined version it holds"
    );
    assert_eq!(again.quarantined.len(), 1);
}

/// PROP-044 §2 — a by-name tombstone survives a full
/// load → write → load round trip.
#[test]
fn tombstone_survives_load_and_write() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.tombstones.insert(
        "wal".into(),
        crate::types::Tombstone {
            reason: "withdrawn by the owner".into(),
            superseded_by: Some("org.vibevm/wal2".into()),
        },
    );
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    assert!(by_name::file_path(tmp.path(), "wal").exists());

    let back = Index::load_from(tmp.path()).unwrap();
    assert_eq!(
        back.tombstones.get("wal").map(|t| t.reason.as_str()),
        Some("withdrawn by the owner")
    );
    back.write_to(tmp.path(), &write_ctx()).unwrap();
    let again = Index::load_from(tmp.path()).unwrap();
    assert!(again.tombstones.contains_key("wal"));
}

/// PROP-044 §2, the case the tombstone slot exists for — a name
/// whose by-name file holds ONLY a tombstone still gets its file:
/// a name that ever existed must answer, never fall silent.
#[test]
fn tombstone_only_name_still_gets_its_by_name_file() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.tombstones.insert(
        "dead-pkg".into(),
        crate::types::Tombstone {
            reason: "superseded".into(),
            superseded_by: None,
        },
    );
    idx.write_to(tmp.path(), &write_ctx()).unwrap();
    assert!(
        by_name::file_path(tmp.path(), "dead-pkg").exists(),
        "a tombstone-only name must still get its by-name file"
    );

    let back = Index::load_from(tmp.path()).unwrap();
    assert_eq!(back.package_count(), 0);
    assert_eq!(back.tombstones.len(), 1);
    // …and it survives the same round trip.
    back.write_to(tmp.path(), &write_ctx()).unwrap();
    let again = Index::load_from(tmp.path()).unwrap();
    assert!(again.tombstones.contains_key("dead-pkg"));
}

/// F2-2, the preservation test: a catalog carrying a FOREIGN schema
/// version survives a load → write round trip. The writer stamps its
/// own constant only into artifacts it creates from scratch
/// (`Index::new`); a version it READ is state, and re-stamping it
/// with the reader's own constant would make a future-version
/// catalog silently claim to be ours. Fails on any writer that
/// reaches for the constant instead of the field.
#[test]
fn foreign_schema_version_survives_load_and_write() {
    let src = tempdir().unwrap();
    let mut idx = fresh_index();
    idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
    idx.write_to(src.path(), &write_ctx()).unwrap();

    // Pass the catalog off as a FUTURE writer's product: bump the
    // manifest's schema_version above ours, touch nothing else.
    let foreign = Repomd::SCHEMA_VERSION + 1;
    let manifest_path = src.path().join(repomd::FILENAME);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["schema_version"] = serde_json::json!(foreign);
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let back = Index::load_from(src.path()).unwrap();
    assert_eq!(back.schema_version, foreign);

    let dst = tempdir().unwrap();
    back.write_to(dst.path(), &write_ctx()).unwrap();
    let rewritten = repomd::read(dst.path()).unwrap();
    assert_eq!(
        rewritten.schema_version, foreign,
        "the writer must re-stamp the version it read, not its own constant"
    );
}

/// F2-3 — `upsert` reports whether it changed the state: `false` on
/// an identical repeat, `true` on the first insert and on a
/// DIFFERING entry under the same version number. The third case is
/// the load-bearing one: an implementation comparing only the
/// version number would answer `false` there and silently drop real
/// updates.
#[test]
fn upsert_reports_whether_state_changed() {
    let mut idx = fresh_index();
    assert!(
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0")),
        "the first insert changes the state"
    );
    assert!(
        !idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0")),
        "an identical repeat changes nothing"
    );
    let mut differing = entry(PackageKind::Flow, org(), "wal", "0.1.0");
    differing.description = Some("a real content change".into());
    assert!(
        idx.upsert(differing),
        "a differing entry under the same version number is an update, not a no-op"
    );
    // …and the update actually landed, replacing the old value.
    let pkg = idx.get(&org(), "wal").unwrap();
    assert_eq!(pkg.versions.len(), 1);
    assert_eq!(
        pkg.versions[0].description.as_deref(),
        Some("a real content change")
    );
}

// ---------------------------------------------------------------------------
// B-072 — the index writes itself idempotently by content: an
// identical mutation changes ZERO bytes on disk; only a real change
// earns a fresh `generated_at`.
// ---------------------------------------------------------------------------

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
