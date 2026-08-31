//! The analyzer's composition-parity law (R4.3, the §0.2 resolution):
//! `analyze_node_lane` IS the regeneration composition, proven byte for
//! byte on the tree shapes the simple fixtures cannot see — a SHARED
//! soft-static package hoisted to the root, and a MEMBER node whose lane
//! is its own.
//!
//! The analyzer entry reuses the write path's cells in place, so the
//! only way it can drift is a composition step present on one side and
//! absent on the other (round 1's missing `append_hoisted` was exactly
//! that). A reconciliation test cannot see such a drift — the analyzer's
//! numbers reconcile against ITS OWN artifact either way — so the pin is
//! byte equality against the ARTIFACT THE WRITE PATH PUBLISHED, per
//! node, on a tree that exercises hoisting, substitution and member
//! scoping at once.

use super::test_helpers::*;
use super::*;

use tempfile::TempDir;
use vibe_core::PackageName;
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};

use crate::boot_artifacts;

fn publish_resolution_lock(root: &Path, resolution: &[ResolvedDep]) {
    let mut lock = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lock.packages = resolution
        .iter()
        .map(|dep| LockedPackage {
            kind: dep.kind,
            name: PackageName::parse(&dep.name).unwrap(),
            group: dep.group.clone(),
            version: dep.version.clone(),
            registry: None,
            source_url: "file:///fixture".into(),
            source_ref: None,
            resolved_commit: None,
            content_hash: dep.source_hash.clone().unwrap(),
            boot_snippet: None,
            files_written: Vec::new(),
            dependencies: dep
                .requires
                .iter()
                .map(|(group, name)| {
                    let version = &resolution
                        .iter()
                        .find(|target| &target.group == group && &target.name == name)
                        .unwrap()
                        .version;
                    vibe_core::PackageRef::parse(&format!("{group}/{name}@={version}")).unwrap()
                })
                .collect(),
            admitted_by: dep.admitted_by.clone(),
            via_override: dep.via_override.clone(),
            overridden: false,
            source_kind: None,
            via_redirect: None,
            features: Vec::new(),
            subskills_active: Vec::new(),
            describes: None,
            language: None,
            materialization: dep
                .manifest
                .package
                .as_ref()
                .map_or(Materialization::Copy, |package| package.materialization),
        })
        .collect();
    lock.write(root.join(Lockfile::FILENAME)).unwrap();
}

/// A workspace whose root requires two static flows that BOTH statically
/// require a third — the third is soft-static-pulled twice, so it hoists
/// to the root lane — plus a member node requiring one of the flows.
fn hoisted_member_workspace() -> (TempDir, Vec<ResolvedDep>, Vec<TempDir>) {
    let ws_dir = TempDir::new().expect("a temp workspace");
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"members/alpha\"]\n\n\
         [requires.packages]\n\
         \"org.vibevm/a-flow\" = { version = \"^1.0\", link = \"static\" }\n\
         \"org.vibevm/b-flow\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# host core\n");
    write(
        ws_dir.path(),
        "members/alpha/vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\
         \"org.vibevm/a-flow\" = { version = \"^1.0\", link = \"static\" }\n",
    );

    let shared_requires = "[requires.packages]\n\
         \"org.vibevm/c-flow\" = { version = \"^1.0\", link = \"static\" }\n";
    let (a, ta) = dep_with_requires(
        "a-flow",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"boot/a.md\"\nlink = \"static\"\n\n{shared_requires}"),
        "boot/a.md",
        "# a flow\n",
        &["c-flow"],
    );
    let (b, tb) = dep_with_requires(
        "b-flow",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"boot/b.md\"\nlink = \"static\"\n\n{shared_requires}"),
        "boot/b.md",
        "# b flow\n",
        &["c-flow"],
    );
    let (c, tc) = dep_with_boot(
        "c-flow",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/c.md\"\nlink = \"static\"\n",
        "boot/c.md",
        "# c shared flow\n",
    );
    (ws_dir, vec![a, b, c], vec![ta, tb, tc])
}

/// One node's written XML lane, read back off disk.
fn written_lane(root: &Path, node_rel: &str) -> Vec<u8> {
    let node = if node_rel == "." {
        root.to_path_buf()
    } else {
        root.join(node_rel)
    };
    fs::read(
        node.join(vibe_core::layout::current_boot_dir())
            .join(boot_artifacts::static_file(SpecFormat::Xml)),
    )
    .expect("the node's written XML lane exists")
}

#[test]
fn the_analyzed_lane_is_byte_equal_to_the_written_one_with_hoisting_and_members() {
    let (ws_dir, resolution, _slots) = hoisted_member_workspace();
    let ws = Workspace::load(ws_dir.path()).expect("the fixture workspace loads");
    apply_resolution_with_spec_format(
        &ws,
        &resolution,
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        None,
        None,
    )
    .expect("the install applies");
    publish_resolution_lock(ws_dir.path(), &resolution);
    let ws = Workspace::load(ws_dir.path()).expect("the workspace reloads");
    regenerate_boot_with_spec_format(&ws, SpecFormat::Xml).expect("the regeneration writes");

    let root_written = written_lane(ws_dir.path(), ".");
    let root_text = String::from_utf8_lossy(&root_written);
    assert!(
        root_text.contains("vibe:static org.vibevm/c-flow"),
        "the fixture really hoists: the shared flow's single copy rides the \
         ROOT lane as its own static contribution — got:\n{root_text}"
    );
    // NOTE deliberately not asserted: how many times the shared body rides
    // this lane shape is the hybrid engine's own question (observed: the
    // hoisted root copy plus one zone copy), and this pin is about PARITY —
    // whatever the write path publishes, the analyzer must reproduce byte
    // for byte. The count observation is filed in BACKLOG rather than
    // frozen here as a law this test does not own.

    let analyzed_root = analyze_node_lane(&ws, ".", None)
        .expect("the root analyzes")
        .expect("the root has a static lane");
    assert_eq!(
        analyzed_root.artifact.bytes(),
        root_written.as_slice(),
        "the analyzer's root lane is the written root lane, byte for byte — \
         hoisting, substitution and dedup included"
    );

    let member_written = written_lane(ws_dir.path(), "members/alpha");
    let analyzed_member = analyze_node_lane(&ws, "members/alpha", None)
        .expect("the member analyzes")
        .expect("the member has a static lane");
    assert_eq!(
        analyzed_member.artifact.bytes(),
        member_written.as_slice(),
        "the member's analyzed lane is ITS OWN written lane — scoped by its \
         manifest, without the root's hoisted block"
    );
    assert_ne!(
        analyzed_member.artifact.bytes(),
        analyzed_root.artifact.bytes(),
        "and the two lanes really differ, so the parity above is two facts"
    );
}
