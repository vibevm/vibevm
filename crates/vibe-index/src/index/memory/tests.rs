//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path = "memory/tests.rs"] mod tests;`, so
//! the module-tree position — and therefore `use super::*` — is unchanged
//! from the inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s as
//! test code.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use super::*;
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
        compatibility: Default::default(),
        provides: Default::default(),
        requires: Default::default(),
        requires_any: vec![],
        obsoletes: Default::default(),
        conflicts: Default::default(),
        features: Default::default(),
        subskills: vec![],
        i18n: Default::default(),
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
/// capability is quarantined on load; its sibling versions keep
/// loading (the filter must not over-cut).
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
    // The understood version is there…
    let pkg = back.get(&org(), "wal").unwrap();
    assert_eq!(pkg.versions.len(), 1);
    assert_eq!(pkg.versions[0].version.to_string(), "0.2.0");
    // …the quarantined one is not, and left exactly one record.
    assert!(
        !pkg.versions
            .iter()
            .any(|v| v.version.to_string() == "0.1.0")
    );
    assert_eq!(back.quarantined.len(), 1);
    let q = &back.quarantined[0];
    assert_eq!(q.name, "wal");
    assert_eq!(q.version.to_string(), "0.1.0");
    assert_eq!(q.missing, vec!["x".to_string()]);
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
