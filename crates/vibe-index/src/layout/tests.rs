//! The layout-registry projection: tree-only registries, journal facts
//! (provenance, yank, tombstone), the tree winning over a stale fact, and
//! the reader falling back to the in-memory projection.

use std::fs;

use super::*;
use crate::types::Group;

fn identity() -> LayoutIdentity {
    LayoutIdentity {
        registry: "corp".into(),
        registry_url: "ssh://git@git.corp.example/vibe.git".into(),
        branch: Some("main".into()),
    }
}

fn package(root: &Path, name: &str, version: &str, body: &str) {
    let dir = root.join(format!("org.corp/{name}/v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.corp'\nname='{name}'\nkind='flow'\nversion='{version}'\n\
             description='{body} package'\nkeywords=['{body}']\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), body).unwrap();
}

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn primary_bytes(root: &Path) -> Vec<u8> {
    fs::read(layout::index_dir(root).join("primary.jsonl")).unwrap()
}

#[test]
fn a_tree_without_journal_projects_from_the_tree_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    package(root, "demo", "1.0.0", "alpha");
    package(root, "demo", "1.1.0", "beta");
    // A version directory without a manifest is skipped, not fatal.
    fs::create_dir_all(root.join("org.corp/broken/v0.1.0")).unwrap();

    let projection = project(root, &identity()).unwrap();
    assert_eq!(projection.index.version_count(), 2);
    assert_eq!(projection.skipped.len(), 1);
    let group = Group::parse("org.corp").unwrap();
    let pkg = projection.index.get(&group, "demo").unwrap();
    let v1 = &pkg.versions[0];
    assert_eq!(
        v1.source_url,
        "ssh://git@git.corp.example/vibe.git#org.corp/demo/v1.0.0"
    );
    assert_eq!(v1.source_ref, "main");
    assert_eq!(v1.registry, "corp");
    assert_eq!(
        v1.content_hash,
        crate::content_hash::compute_content_hash(&root.join("org.corp/demo/v1.0.0")).unwrap()
    );
}

#[test]
fn refresh_persists_provenance_and_yanks_and_the_reader_uses_the_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    package(root, "demo", "1.0.0", "alpha");
    let dir = root.join("org.corp/demo/v1.0.0");
    let mut mirrored = entry_from_tree(
        &dir,
        EntrySource {
            source_url: "https://github.com/vibespecs/org.corp.demo.git".into(),
            source_ref: "v1.0.0".into(),
            resolved_commit: Some("0123abcd".into()),
            registry: "corp".into(),
            indexed_at: at("2026-11-08T10:00:00Z"),
            indexed_by: "vibe test".into(),
        },
    )
    .unwrap();
    mirrored.frozen = false;
    let group = Group::parse("org.corp").unwrap();
    refresh(
        root,
        &identity(),
        vec![
            Event::Published {
                entry: Box::new(mirrored),
            },
            Event::Yanked {
                group: group.clone(),
                name: "demo".into(),
                version: "1.0.0".parse().unwrap(),
                reason: "carried from the source".into(),
            },
        ],
        at("2026-11-08T10:00:00Z"),
    )
    .unwrap();

    assert_eq!(layout::index_state(root).unwrap(), IndexState::Current);
    assert!(
        layout::journal_dir(root).join("2026-11.ndjson").is_file(),
        "the journal lives inside the registry"
    );
    let ignore = fs::read_to_string(layout::index_dir(root).join(".gitignore")).unwrap();
    assert!(ignore.contains("!/state/journal/"), "{ignore}");

    let (index, state) = load_or_project(root, &identity()).unwrap();
    assert_eq!(state, IndexState::Current);
    let entry = &index.get(&group, "demo").unwrap().versions[0];
    assert_eq!(
        entry.source_url,
        "https://github.com/vibespecs/org.corp.demo.git"
    );
    assert_eq!(entry.resolved_commit.as_deref(), Some("0123abcd"));
    assert!(entry.yanked);

    // The in-memory projection is the same projection, byte for byte.
    let before = primary_bytes(root);
    let memory = project(root, &identity()).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    memory
        .index
        .write_to(
            scratch.path(),
            &WriteCtx {
                at: at("2026-11-08T10:00:00Z"),
            },
        )
        .unwrap();
    assert_eq!(
        fs::read(scratch.path().join("primary.jsonl")).unwrap(),
        before
    );
}

#[test]
fn a_hand_filled_version_makes_the_index_stale_and_the_reader_projects_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    package(root, "demo", "1.0.0", "alpha");
    refresh(root, &identity(), Vec::new(), at("2026-11-08T10:00:00Z")).unwrap();
    package(root, "other", "2.0.0", "gamma");

    assert!(matches!(
        layout::index_state(root).unwrap(),
        IndexState::Stale { .. }
    ));
    let (index, state) = load_or_project(root, &identity()).unwrap();
    assert!(matches!(state, IndexState::Stale { .. }));
    let group = Group::parse("org.corp").unwrap();
    assert!(index.get(&group, "other").is_some());
    assert_eq!(index.version_count(), 2);

    // A refresh with no new fact catches the index up; a second one
    // changes no byte.
    refresh(root, &identity(), Vec::new(), at("2026-11-08T11:00:00Z")).unwrap();
    assert_eq!(layout::index_state(root).unwrap(), IndexState::Current);
    let first = primary_bytes(root);
    refresh(root, &identity(), Vec::new(), at("2026-11-08T12:00:00Z")).unwrap();
    assert_eq!(primary_bytes(root), first);
}

#[test]
fn the_tree_wins_over_a_published_fact_whose_bytes_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    package(root, "demo", "1.0.0", "alpha");
    let dir = root.join("org.corp/demo/v1.0.0");
    let recorded = entry_from_tree(
        &dir,
        EntrySource {
            source_url: "https://upstream.example/demo.git".into(),
            source_ref: "v1.0.0".into(),
            resolved_commit: None,
            registry: "corp".into(),
            indexed_at: at("2026-11-08T10:00:00Z"),
            indexed_by: "vibe test".into(),
        },
    )
    .unwrap();
    refresh(
        root,
        &identity(),
        vec![Event::Published {
            entry: Box::new(recorded),
        }],
        at("2026-11-08T10:00:00Z"),
    )
    .unwrap();
    fs::write(dir.join("README.md"), "edited by hand").unwrap();

    let projection = project(root, &identity()).unwrap();
    let group = Group::parse("org.corp").unwrap();
    let entry = &projection.index.get(&group, "demo").unwrap().versions[0];
    assert_eq!(
        entry.content_hash,
        crate::content_hash::compute_content_hash(&dir).unwrap()
    );
    assert!(
        entry.source_url.ends_with("#org.corp/demo/v1.0.0"),
        "provenance belongs to the recorded bytes only: {}",
        entry.source_url
    );
}

#[test]
fn a_buried_name_answers_with_its_tombstone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    package(root, "demo", "1.0.0", "alpha");
    refresh(
        root,
        &identity(),
        vec![Event::Buried {
            name: "old-demo".into(),
            reason: "renamed to `demo`".into(),
            superseded_by: Some("demo".into()),
        }],
        at("2026-11-08T10:00:00Z"),
    )
    .unwrap();
    let (index, _) = load_or_project(root, &identity()).unwrap();
    assert_eq!(
        index
            .tombstones
            .get("old-demo")
            .and_then(|t| t.superseded_by.as_deref()),
        Some("demo")
    );
    assert!(
        layout::index_dir(root)
            .join("by-name/old-demo.json")
            .is_file()
    );
}
