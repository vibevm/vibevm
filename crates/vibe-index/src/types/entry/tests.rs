use super::*;
use specmark::verifies;

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
        compatibility: CompatibilityEntry::default(),
        provides: ProvidesEntry::default(),
        requires: RequiresEntry::default(),
        requires_any: vec![],
        obsoletes: ObsoletesEntry::default(),
        conflicts: ConflictsEntry::default(),
        features: FeaturesEntry::default(),
        subskills: vec![],
        i18n: I18nEntry::default(),
        boot_snippet: Some(BootSnippetEntry {
            source: "boot/10-flow-wal.md".into(),
            category: Some("flow".into()),
        }),
        files_count: 5,
        indexed_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-index/PROP-005#entry", r = 1)]
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
