use super::*;
use crate::types::PackageKind;
use chrono::{DateTime, Utc};
use specmark::verifies;
use vibe_core::Group;

// This module is pulled in via `#[cfg(test)] mod tests;`, so the
// frontend sees the file standalone and cannot read the `cfg(test)` off
// the parent's `mod` line. Non-`#[test]` helpers must therefore carry
// their own `#[cfg(test)]`, or their `.unwrap()`s read as domain code.
#[cfg(test)]
fn sample_entry() -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").unwrap(),
        name: "wal".into(),
        version: "0.1.0".parse().unwrap(),
        content_hash: "sha256:0000".into(),
        source_url: "https://example.invalid/flow-wal.git".into(),
        source_ref: "v0.1.0".into(),
        resolved_commit: Some("abc123".into()),
        registry: "vibespecs".into(),
        workspace_origin: None,
        license: Some("EULA".into()),
        authors: vec!["Oleg".into()],
        description: Some("WAL discipline".into()),
        homepage: None,
        keywords: vec!["wal".into()],
        describes: None,
        // Empty projections are ABSENCE, not present-but-empty: the
        // writer normalises emptiness to a missing key, so a fixture
        // with nothing to say says nothing (§ the `is_empty` guards).
        compatibility: None,
        provides: None,
        requires: None,
        requires_any: vec![],
        obsoletes: None,
        conflicts: None,
        features: None,
        subskills: vec![],
        i18n: None,
        boot_snippet: Some(BootSnippetEntry {
            source: "boot/10-flow-wal.md".into(),
            category: Some("flow".into()),
        }),
        files_count: 5,
        must_understand: vec![],
        yanked: false,
        frozen: false,
        indexed_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry",
    r = 1
)]
fn version_entry_round_trips_through_json() {
    let v = sample_entry();
    let json = serde_json::to_string(&v).unwrap();
    let back: VersionEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(v, back);
}

#[test]
fn empty_subsections_are_omitted() {
    let v = sample_entry();
    let json = serde_json::to_string(&v).unwrap();
    assert!(!json.contains("provides"));
    assert!(!json.contains("requires_any"));
    assert!(!json.contains("subskills"));
}

#[test]
fn package_entry_finalise_picks_latest_stable() {
    let mut p = PackageEntry::new(
        Group::parse("org.vibevm").unwrap(),
        "wal",
        DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let mut v1 = sample_entry();
    v1.version = "0.1.0".parse().unwrap();
    let mut v2 = sample_entry();
    v2.version = "0.2.0".parse().unwrap();
    let mut v_pre = sample_entry();
    v_pre.version = "0.3.0-rc.1".parse().unwrap();
    p.versions.push(v2);
    p.versions.push(v1);
    p.versions.push(v_pre);
    p.finalise();
    assert_eq!(p.latest_stable.as_ref().unwrap().to_string(), "0.2.0");
    // versions sorted ascending
    assert_eq!(p.versions[0].version.to_string(), "0.1.0");
    assert_eq!(p.versions[1].version.to_string(), "0.2.0");
    assert_eq!(p.versions[2].version.to_string(), "0.3.0-rc.1");
}

#[test]
fn delivery_mode_serde_kebab() {
    let v = serde_json::to_string(&DeliveryMode::LazyPush).unwrap();
    assert_eq!(v, "\"lazy-push\"");
    let parsed: DeliveryMode = serde_json::from_str("\"lazy-pull\"").unwrap();
    assert_eq!(parsed, DeliveryMode::LazyPull);
}

#[test]
fn workspace_origin_round_trips_through_json() {
    let mut v = sample_entry();
    v.workspace_origin = Some(WorkspaceOriginEntry {
        upstream: "https://github.com/you/monorepo".into(),
        path: "packages/flow-wal".into(),
        commit: Some("abc123".into()),
        generated_by: "vibe 0.1.0".into(),
        generated_at: "2026-05-20T00:00:00Z".into(),
    });
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("workspace_origin"));
    let back: VersionEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(v, back);
}

#[test]
fn name_entry_finalise_sorts_candidates_by_group() {
    let now = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut ne = NameEntry::new("wal", now);
    ne.packages.push(PackageEntry::new(
        Group::parse("org.vibevm").unwrap(),
        "wal",
        now,
    ));
    ne.packages.push(PackageEntry::new(
        Group::parse("com.acme").unwrap(),
        "wal",
        now,
    ));
    ne.finalise();
    assert_eq!(ne.packages[0].group.as_str(), "com.acme");
    assert_eq!(ne.packages[1].group.as_str(), "org.vibevm");
    let json = serde_json::to_string(&ne).unwrap();
    let back: NameEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(ne, back);
}

#[test]
fn sort_key_orders_by_group_then_name_then_version() {
    let mut a = sample_entry();
    a.group = Group::parse("com.acme").unwrap();
    let b = sample_entry(); // org.vibevm
    // com.acme sorts before org.vibevm regardless of name.
    assert!(a.sort_key() < b.sort_key());
}

// --- PROP-044 §2a/§4.5 — the four catalog-record slots -----------------

#[test]
fn empty_slots_are_omitted_from_the_wire() {
    // All four slots empty: none of `must_understand`, `yanked`,
    // `frozen` reaches the wire — old readers see the old shape.
    let v = sample_entry();
    assert!(v.must_understand.is_empty());
    assert!(!v.yanked);
    assert!(!v.frozen);
    let json = serde_json::to_string(&v).unwrap();
    assert!(!json.contains("must_understand"), "{json}");
    assert!(!json.contains("yanked"), "{json}");
    assert!(!json.contains("frozen"), "{json}");
}

#[test]
fn set_slots_round_trip_through_json() {
    let mut v = sample_entry();
    v.must_understand = vec!["x".into()];
    v.yanked = true;
    v.frozen = true;
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("must_understand"), "{json}");
    assert!(json.contains("yanked"), "{json}");
    assert!(json.contains("frozen"), "{json}");
    let back: VersionEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.must_understand, vec!["x".to_string()]);
    assert!(back.yanked);
    assert!(back.frozen);
    assert_eq!(v, back);
}

#[test]
fn tombstone_round_trips_and_absence_is_omitted() {
    // A tombstone survives the wire intact…
    let now = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut ne = NameEntry::new("wal", now);
    ne.tombstone = Some(Tombstone {
        reason: "withdrawn by the owner".into(),
        superseded_by: Some("org.vibevm/wal2".into()),
    });
    let json = serde_json::to_string(&ne).unwrap();
    assert!(json.contains("tombstone"), "{json}");
    let back: NameEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tombstone, ne.tombstone);

    // …and its absence stays off the wire.
    let plain = NameEntry::new("wal", now);
    let json = serde_json::to_string(&plain).unwrap();
    assert!(!json.contains("tombstone"), "{json}");
    let back: NameEntry = serde_json::from_str(&json).unwrap();
    assert!(back.tombstone.is_none());
}

// --- F33 (PROP-044 §4.4): the tolerant catalog reader -------------------

#[test]
fn unknown_root_field_is_read() {
    // A field this build does not know is the future arriving early, not
    // an error — see the tolerance note on `VersionEntry` for why the
    // strictness left and why dropping it is safe.
    let v = sample_entry();
    let mut value = serde_json::to_value(&v).unwrap();
    value["future_field"] = serde_json::json!("written by a newer vibe");
    let back: VersionEntry = serde_json::from_value(value).unwrap();
    assert_eq!(back.name, "wal");
}

#[test]
fn unknown_nested_section_field_is_read() {
    // Tolerance is not only at the root: a nested section this build
    // reads partially still parses, and every key it does know arrives
    // intact.
    let v = sample_entry();
    let mut value = serde_json::to_value(&v).unwrap();
    value["compatibility"] = serde_json::json!({
        "min_vibe_version": "0.1.0",
        "future_key": true,
    });
    let back: VersionEntry = serde_json::from_value(value).unwrap();
    assert_eq!(
        back.compatibility
            .as_ref()
            .and_then(|c| c.min_vibe_version.as_deref()),
        Some("0.1.0")
    );
}

#[test]
fn repomd_unknown_field_is_read() {
    // `repomd.json` is a catalog file like the entries: a newer
    // generator may write a key this reader does not know yet.
    let json = r#"{
        "schema_version": 1,
        "registry": "vibespecs",
        "registry_url": "https://github.com/vibespecs",
        "naming": "fqdn",
        "generated_at": "2026-05-06T12:00:00Z",
        "generator": "vibe-index 0.1.0-dev",
        "package_count": 3,
        "version_count": 5,
        "files": {},
        "future_extension": "not in the type yet"
    }"#;
    let parsed: crate::types::Repomd = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.registry, "vibespecs");
}

#[test]
fn unknown_field_is_read_but_not_written_back() {
    // Tolerance ends at the pen: a field this build does not know is
    // read — the record still answers — and dropped when the record is
    // serialised again, because it has no home in this build's type.
    // That is exactly why the catalog is written from the journal
    // projection and never read-modify-written: a writer fed by this
    // reader would silently strip the very fields the reader was made
    // tolerant enough to accept. What is read is never written back
    // (PROP-044 §4.4).
    let v = sample_entry();
    let mut value = serde_json::to_value(&v).unwrap();
    value["future_field"] = serde_json::json!("written by a newer vibe");
    let read: VersionEntry = serde_json::from_value(value).unwrap();
    let rewritten = serde_json::to_value(&read).unwrap();
    assert!(rewritten.get("future_field").is_none(), "{rewritten}");
}
