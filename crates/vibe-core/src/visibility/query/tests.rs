specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::manifest::AccessLevel;
use crate::visibility::{
    AllowFriendsState, BlockReason, InstalledWorld, ProvenanceRule, WhyVerdict, analyze,
    load_installed_world, why,
};

fn lock_header() -> String {
    format!(
        "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-08-23T00:00:00Z\"\nschema_version = {}\n",
        crate::manifest::CURRENT_SCHEMA_VERSION
    )
}

fn write_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn lock_entry(group: &str, name: &str, version: &str) -> String {
    format!(
        "\n[[package]]\nkind = \"flow\"\ngroup = \"{group}\"\nname = \"{name}\"\nversion = \
         \"{version}\"\nsource_url = \"file:///fake\"\ncontent_hash = \"sha256:00\"\nfiles_written \
         = []\n"
    )
}

/// Write a member's slot manifest `vibedeps/<group>.<name>/<version>/vibe.toml`.
/// `requires` is the raw `[requires.packages]` body (empty string for none).
fn write_slot(project: &Path, group: &str, name: &str, version: &str, requires: &str, extra: &str) {
    let body = format!(
        "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \
         \"{version}\"\n{extra}{requires}"
    );
    write_file(
        &project
            .join(crate::layout::current_vibedeps_root())
            .join(format!("{group}.{name}"))
            .join(version)
            .join("vibe.toml"),
        &body,
    );
}

/// The chain world: the root befriends redbook, redbook re-exports wal on a
/// friends-only edge (the F10 implication), and wal keeps a private dev
/// tool. `with_wal_slot = false` removes wal's slot manifest (the unread
/// case).
fn chain_world(with_wal_slot: bool) -> InstalledWorld {
    let project = tempdir().unwrap();
    let root = project.path();
    write_file(
        &root.join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n[requires.packages]\n\"org.x/redbook\" \
         = { version = \"1.0.0\", friend = true }\n",
    );
    let mut lock = lock_header();
    lock.push_str(&lock_entry("org.x", "redbook", "1.0.0"));
    lock.push_str(&lock_entry("org.x", "wal", "1.0.0"));
    lock.push_str(&lock_entry("org.x", "hidden", "1.0.0"));
    write_file(&root.join("vibe.lock"), &lock);
    write_slot(
        root,
        "org.x",
        "redbook",
        "1.0.0",
        "[requires.packages]\n\"org.x/wal\" = { version = \"1.0.0\", access = \"friends-only\" }\n",
        "",
    );
    if with_wal_slot {
        write_slot(
            root,
            "org.x",
            "wal",
            "1.0.0",
            "[requires.packages]\n\"org.x/hidden\" = { version = \"1.0.0\", access = \"private\" }\n",
            "",
        );
    }
    write_slot(root, "org.x", "hidden", "1.0.0", "", "");
    load_installed_world(root).unwrap()
}

#[test]
fn installed_world_loads_and_analyzes() {
    let world = chain_world(true);
    assert_eq!(world.root_id, "demo");
    assert!(world.unread.is_empty());
    let analysis = analyze(&world.graph, &world.root_id);
    let wal = analysis
        .effective
        .get("org.x/wal")
        .expect("wal joins the effective set through the friends-only re-export");
    assert_eq!(wal.rule, ProvenanceRule::FriendsChain);
    assert_eq!(wal.path, ["demo", "org.x/redbook", "org.x/wal"]);
    assert!(analysis.closure.contains("org.x/redbook"));
    assert!(analysis.closure.contains("org.x/wal"));
}

#[test]
fn why_absent_names_the_private_edge() {
    let world = chain_world(true);
    match why(&world, "org.x/hidden") {
        WhyVerdict::Absent { blocked } => {
            assert_eq!(
                blocked,
                [super::super::BlockedEdge {
                    from: "org.x/wal".into(),
                    reason: BlockReason::Private,
                }]
            );
        }
        other => panic!("expected Absent, got {other:?}"),
    }
}

#[test]
fn why_unknown_coordinate() {
    let world = chain_world(true);
    assert!(matches!(
        why(&world, "org.nope/ghost"),
        WhyVerdict::UnknownCoordinate
    ));
}

#[test]
fn friends_report_three_states() {
    let project = tempdir().unwrap();
    let root = project.path();
    write_file(
        &root.join("vibe.toml"),
        "[project]\nname = \"demo\"\ngroup = \"org.r\"\nversion = \"0.0.1\"\n\n[visibility]\nfriends \
         = [\"org.x/open\", \"org.x/sealed\", \"org.x/circle\"]\n",
    );
    let mut lock = lock_header();
    for name in ["open", "sealed", "circle"] {
        lock.push_str(&lock_entry("org.x", name, "1.0.0"));
    }
    write_file(&root.join("vibe.lock"), &lock);
    write_slot(root, "org.x", "open", "1.0.0", "", "");
    write_slot(
        root,
        "org.x",
        "sealed",
        "1.0.0",
        "",
        "[visibility]\nallow-friends = []\n\n",
    );
    write_slot(
        root,
        "org.x",
        "circle",
        "1.0.0",
        "",
        "[visibility]\nallow-friends = [\"org.r/demo\"]\n\n",
    );
    let world = load_installed_world(root).unwrap();
    assert_eq!(world.root_id, "org.r/demo");

    let open = crate::visibility::friends(&world, "org.x/open").unwrap();
    assert_eq!(open.state, AllowFriendsState::Open);
    assert_eq!(open.actual_friends, ["org.r/demo"]);
    assert!(open.rejected.is_empty());
    assert!(open.in_root_closure);

    let sealed = crate::visibility::friends(&world, "org.x/sealed").unwrap();
    assert_eq!(sealed.state, AllowFriendsState::Sealed);
    assert!(sealed.actual_friends.is_empty());
    // A sealed circle with a live grant: the grant exists and is refused.
    assert_eq!(sealed.rejected, ["org.r/demo"]);
    assert!(!sealed.in_root_closure);

    let circle = crate::visibility::friends(&world, "org.x/circle").unwrap();
    assert_eq!(
        circle.state,
        AllowFriendsState::Circle(vec!["org.r/demo".into()])
    );
    assert_eq!(circle.actual_friends, ["org.r/demo"]);
    assert!(circle.in_root_closure);
}

#[test]
fn missing_slot_manifest_is_reported_not_fatal() {
    let world = chain_world(false);
    assert_eq!(world.unread, ["org.x/wal"]);
    // The world still lives: redbook resolves, and wal — whose declaration
    // is gone — still arrives through redbook's declared re-export.
    let analysis = analyze(&world.graph, &world.root_id);
    assert!(analysis.effective.contains_key("org.x/redbook"));
    assert!(analysis.effective.contains_key("org.x/wal"));
    // A friends-only edge is absent from wal's empty declaration, so the
    // private dev tool hidden behind it never surfaces.
    assert!(!analysis.effective.contains_key("org.x/hidden"));
}

/// The raw-declared edge fields reach the engine untouched: the friend
/// flag is absent on the friends-only edge (F10 supplies it), and the
/// private access arrives as `Some(Private)`.
#[test]
fn declared_edge_fields_stay_raw() {
    let world = chain_world(true);
    let redbook = world.graph.nodes.get("org.x/redbook").unwrap();
    let wal_edge = redbook
        .edges
        .iter()
        .find(|edge| edge.to == "org.x/wal")
        .unwrap();
    assert_eq!(wal_edge.access, Some(AccessLevel::FriendsOnly));
    assert_eq!(wal_edge.friend, None);
}
