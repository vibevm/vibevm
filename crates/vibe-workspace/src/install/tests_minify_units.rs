//! `xml-minify` on a PER-UNIT lane, and the freshness law it finally makes
//! observable (R4 architecture §7.1's fingerprint decision, §11 row 8).
//!
//! **What only this cell can prove.** Until a real behavior existed, every
//! owner in this repository — and in every fixture — activated nothing, so
//! every owner plan was empty, so no unit fingerprint could carry a plan
//! frame and row 8 had no witness. Here one PACKAGE activates the transform in
//! its own manifest: its unit lane gains the §7.1 header and its recorded
//! boot-graph fingerprint moves, while a sibling package that activates
//! nothing keeps the exact fingerprint it had. That is the whole of "a changed
//! owner plan cannot leave that owner's skippable unit fresh, while distinct
//! owners may legitimately transform the same authored document differently".
//!
//! The world is PROP-038's per-unit shape: `parent` statically links `child`,
//! so `parent` compiles a unit lane of its own zone. The lock is published
//! after the install exactly as in `tests_minify_activation`, because boot
//! regeneration owns no epoch and the install pass observes no world.

use super::test_helpers::*;
use super::*;

use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, PackageRef, VersionSpec};

use crate::boot_artifacts;

/// The declaration `parent` activates on its OWN unit lane.
const MINIFY_DECL: &str = r#"
[[extension]]
id = "minify"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }
"#;

/// The header a package-owned unit lane writes: the lane owner is `parent`
/// itself, seated as a grouped-coordinate host.
const UNIT_HEADER: &str = "<!-- vibe:transforms org.vibevm/parent#minify -->";

/// A document with enough nesting that minifying it beats the header's bytes.
fn body(title: &str) -> String {
    let mut text = format!("# {title} {{#root}}\n");
    for index in 0..12 {
        text.push_str(&format!(
            "\n## Section {index} {{#s{index}}}\n\nparagraph {index} of {title}\n"
        ));
    }
    text
}

/// The `parent` → `child` static zone, with `parent`'s manifest optionally
/// carrying the activating declaration.
fn zone(activated: bool) -> (Vec<ResolvedDep>, Vec<TempDir>) {
    // `parent` is DYNAMIC from the root and STATIC over `child` — PROP-038
    // §2.2's core shape, and the one that gives `parent` a unit lane of its
    // own without the root compiling that lane back in as a source document.
    let mut parent_manifest = String::from(
        "[boot_snippet]\nsource = \"boot/parent.md\"\n\n\
         [requires.packages]\n\"org.vibevm/child\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    if activated {
        parent_manifest.push_str(MINIFY_DECL);
    }
    let (parent, parent_dir) = dep_with_requires(
        "parent",
        "1.0.0",
        &parent_manifest,
        "boot/parent.md",
        &body("Parent"),
        &["child"],
    );
    let (child, child_dir) = dep_with_boot(
        "child",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        &body("Child"),
    );
    (vec![parent, child], vec![parent_dir, child_dir])
}

/// One locked package in the shape the durable world adapter reads.
fn locked(name: &str, dependencies: Vec<PackageRef>) -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Flow,
        name: PackageName::parse(name).expect("a valid name"),
        group: Group::parse("org.vibevm").expect("a valid group"),
        version: ver("1.0.0"),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").expect("a valid hash"),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies,
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }
}

/// Publish the POST-install lock: both packages, with `parent`'s own edge to
/// `child` recorded so the owner-scoped closure walk can reach it.
fn publish_lock(root: &Path) {
    let child_edge = PackageRef::new(
        None,
        Some(Group::parse("org.vibevm").expect("a valid group")),
        "child",
        VersionSpec::parse("=1.0.0").expect("a valid version spec"),
    )
    .expect("a valid pkgref");
    let mut lockfile = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lockfile.packages = vec![
        locked("parent", vec![child_edge]),
        locked("child", Vec::new()),
    ];
    lockfile
        .write(root.join(Lockfile::FILENAME))
        .expect("the fixture lock writes");
}

/// Install the zone in the XML lane and publish its lock.
fn installed(activated: bool) -> (TempDir, Vec<TempDir>) {
    let ws_dir = TempDir::new().expect("a temp workspace");
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n",
    );
    let (resolution, packages) = zone(activated);
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
    publish_lock(ws_dir.path());
    (ws_dir, packages)
}

fn regenerate(root: &Path) {
    let ws = Workspace::load(root).expect("the workspace reloads");
    regenerate_boot_with_spec_format(&ws, SpecFormat::Xml).expect("the regeneration succeeds");
}

/// One package's compiled unit lane.
fn unit_lane(root: &Path, package: &str) -> String {
    fs::read_to_string(root.join(deps_slot_specs(
        format!("org.vibevm.{package}/1.0.0"),
        format!("boot/{}", boot_artifacts::static_file(SpecFormat::Xml)),
    )))
    .expect("the unit's XML lane exists")
}

/// One package's recorded boot-graph fingerprint, or `None` when its unit
/// INDEX carries no fingerprint header.
fn unit_fingerprint(root: &Path, package: &str) -> Option<String> {
    let index = fs::read_to_string(root.join(deps_slot_specs(
        format!("org.vibevm.{package}/1.0.0"),
        format!("boot/{}", boot_artifacts::INDEX_FILE),
    )))
    .ok()?;
    boot_artifacts::read_fingerprint(&index)
}

/// The unit lane a PACKAGE activates carries the header, shrinks, and its
/// recorded fingerprint moves — while a sibling that activates nothing keeps
/// its exact fingerprint.
///
/// The fingerprint half is §11 row 8, live for the first time: a plan digest
/// is a freshness INPUT, so an owner whose plan changed must be stale and an
/// owner whose plan is still empty must not be. Both halves are read off the
/// same run, so a mutation that framed every unit — or none — is red either
/// way.
#[test]
fn a_package_owned_unit_lane_activates_and_only_that_owners_fingerprint_moves() {
    let (plain, _plain_packages) = installed(false);
    regenerate(plain.path());
    let baseline = unit_lane(plain.path(), "parent");
    let baseline_parent_fp = unit_fingerprint(plain.path(), "parent");
    let baseline_child_fp = unit_fingerprint(plain.path(), "child");
    assert!(
        baseline_parent_fp.is_some(),
        "a package that statically links a child records a fingerprint"
    );

    let (activated, _packages) = installed(true);
    regenerate(activated.path());
    let minified = unit_lane(activated.path(), "parent");

    assert!(
        minified.len() < baseline.len(),
        "the activated unit lane is strictly smaller: {} → {}",
        baseline.len(),
        minified.len()
    );
    assert_eq!(
        minified.lines().nth(3),
        Some(UNIT_HEADER),
        "the unit lane's header names ITS OWN owner's active list"
    );
    assert_eq!(
        vibe_specdoc::from_xml(document(&minified)).expect("the minified document parses"),
        vibe_specdoc::from_xml(document(&baseline)).expect("the baseline document parses"),
        "minifying a unit lane preserves its documents' parsed node set"
    );

    // Row 8: the activating owner's fingerprint moved, and only that owner's.
    assert_ne!(
        unit_fingerprint(activated.path(), "parent"),
        baseline_parent_fp,
        "a changed owner plan invalidates that owner's unit"
    );
    assert_eq!(
        unit_fingerprint(activated.path(), "child"),
        baseline_child_fp,
        "an owner that activates nothing keeps the exact fingerprint it had"
    );
}

/// The check half observes the same world and frames the same digests, so a
/// tree the generator just left fresh verifies clean.
///
/// This is the mutation T10C's occurrence-COUNTED fence anticipated in
/// spelling and could not yet catch behaviourally: with a real behavior
/// registered, a `verify_boot_graph` that framed no plan digest would call the
/// activating unit stale immediately.
#[test]
fn verify_boot_graph_calls_a_freshly_generated_activated_tree_clean() {
    let (workspace, _packages) = installed(true);
    regenerate(workspace.path());
    let ws = Workspace::load(workspace.path()).expect("the workspace reloads");
    assert!(
        verify_boot_graph(&ws)
            .expect("verification observes the same world")
            .is_empty(),
        "the check half must frame the same owner-plan digests the generate half did"
    );
}

/// The first `<?xml …?> … </spec>` document of one emitted tape.
fn document(tape: &str) -> &str {
    let start = tape
        .find("<?xml version=")
        .expect("the unit lane carries a document");
    let end = start
        + tape[start..]
            .find("</spec>")
            .expect("an opened document closes")
        + "</spec>".len();
    &tape[start..end]
}
