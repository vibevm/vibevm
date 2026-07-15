//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use super::test_helpers::*;
use super::*;
use tempfile::TempDir;

#[test]
fn apply_resolution_materialises_and_regenerates_a_standalone_project() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/10-flow-wal.md\"\ncategory = \"flow\"\n",
        "boot/10-flow-wal.md",
        "# wal boot",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    assert_eq!(outcome.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    // The package tree is materialised verbatim into its slot.
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md")
            .is_file()
    );
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-wal/0.3.0/vibe.toml")
            .is_file()
    );
    // INDEX.md names the node's own foundation boot and the dependency.
    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(index.contains("spec/boot/00-core.md"), "{index}");
    assert!(
        index.contains("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md"),
        "{index}"
    );
    // The redirect lands at the node root.
    assert!(ws_dir.path().join("CLAUDE.md").is_file());
}

#[test]
fn apply_resolution_with_no_dependencies_still_writes_index() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence, None).unwrap();
    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    assert!(ws_dir.path().join("spec/boot/INDEX.md").is_file());
}

#[test]
fn apply_resolution_inline_dependency_produces_inline_md() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/crit\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "crit",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/crit.md\"\n",
        "boot/crit.md",
        "# critical discipline",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // The consumer declared `link = "inline"`, so the dependency's
    // boot is concatenated into INLINE.md.
    let inline = fs::read_to_string(ws_dir.path().join("spec/boot/STATIC.md")).unwrap();
    assert!(inline.contains("# critical discipline"), "{inline}");
}

#[test]
fn dynamic_dep_statically_links_its_child_into_a_per_unit_static_md() {
    // PROP-038 §2.2 — the owner's core case: a `dynamic`-linked package that
    // statically links its own dependency. `parent` is dynamic from root, but
    // declares `child` as `static`; `parent` gets its own STATIC.md compiling
    // `child` in, and root's INDEX points at that STATIC.md, not the snippet.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (parent, _p) = dep_with_requires(
        "parent",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/parent.md\"\n\n\
         [requires.packages]\n\"org.vibevm/child\" = { version = \"^1.0\", link = \"static\" }\n",
        "boot/parent.md",
        "# parent boot",
        &["child"],
    );
    let (child, _c) = dep_with_boot(
        "child",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        "# child boot",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[parent, child], SlotIntegrity::TrustPresence, None).unwrap();

    // parent's per-unit STATIC.md compiles the whole zone — child then parent.
    let parent_static = fs::read_to_string(
        ws_dir
            .path()
            .join("vibedeps/flow-parent/1.0.0/spec/boot/STATIC.md"),
    )
    .unwrap();
    assert!(parent_static.contains("# parent boot"), "{parent_static}");
    assert!(parent_static.contains("# child boot"), "{parent_static}");

    // The root does NOT compile the zone in — root→parent is dynamic.
    let root_static = ws_dir.path().join("spec/boot/STATIC.md");
    assert!(
        !root_static.exists()
            || !fs::read_to_string(&root_static)
                .unwrap()
                .contains("# child boot"),
        "root STATIC.md must not carry the child (root→parent is dynamic)"
    );

    // The child gets no STATIC.md of its own — it is a leaf.
    assert!(
        !ws_dir
            .path()
            .join("vibedeps/flow-child/1.0.0/spec/boot/STATIC.md")
            .exists()
    );

    // Root's INDEX points at parent's STATIC.md (the whole zone), not the
    // raw snippet — so loading parent pulls child with it.
    let root_index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        root_index.contains("vibedeps/flow-parent/1.0.0/spec/boot/STATIC.md"),
        "{root_index}"
    );
    assert!(
        !root_index.contains("vibedeps/flow-parent/1.0.0/boot/parent.md"),
        "the raw snippet must not be the INDEX target: {root_index}"
    );
}

#[test]
fn a_package_shared_by_two_units_is_hoisted_to_the_root() {
    // PROP-038 §2.4 — `a` and `e` both statically link `shared`; it is soft
    // and pulled twice, so it is hoisted to the global root STATIC.md once,
    // and each local zone carries a #use marker instead of a duplicate copy.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/a\" = \"^1.0\"\n\"org.vibevm/e\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let static_child = "[boot_snippet]\nsource = \"boot/{n}.md\"\n\n[requires.packages]\n\
         \"org.vibevm/shared\" = { version = \"^1.0\", link = \"static\" }\n";
    let (a, _a) = dep_with_requires(
        "a",
        "1.0.0",
        &static_child.replace("{n}", "a"),
        "boot/a.md",
        "# a boot",
        &["shared"],
    );
    let (e, _e) = dep_with_requires(
        "e",
        "1.0.0",
        &static_child.replace("{n}", "e"),
        "boot/e.md",
        "# e boot",
        &["shared"],
    );
    let (shared, _s) = dep_with_boot(
        "shared",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/shared.md\"\n",
        "boot/shared.md",
        "# shared discipline",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[a, e, shared], SlotIntegrity::TrustPresence, None).unwrap();

    // The shared text is hoisted to the global root STATIC.md — exactly once,
    // with a shared-by hint naming the consumers.
    let root_static = fs::read_to_string(ws_dir.path().join("spec/boot/STATIC.md")).unwrap();
    assert_eq!(
        root_static.matches("# shared discipline").count(),
        1,
        "hoisted exactly once: {root_static}"
    );
    assert!(
        root_static.contains("shared by"),
        "shared-by hint: {root_static}"
    );

    // a's local STATIC.md carries a #use marker, not the shared text.
    let a_static = fs::read_to_string(
        ws_dir
            .path()
            .join("vibedeps/flow-a/1.0.0/spec/boot/STATIC.md"),
    )
    .unwrap();
    assert!(a_static.contains("# a boot"), "{a_static}");
    assert!(
        a_static.contains("#use spec://org.vibevm/shared"),
        "local #use marker: {a_static}"
    );
    assert!(
        !a_static.contains("# shared discipline"),
        "shared text must not duplicate into a: {a_static}"
    );
}

#[test]
fn apply_resolution_renders_when_from_a_boot_snippet() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/win\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "win",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/win.md\"\nwhen = \"os:windows\"\n",
        "boot/win.md",
        "# windows-only guidance",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // The `[boot_snippet].when` rides into INDEX.md, and the entry is
    // dynamic — a condition forces the dynamic INCLUDE form even with
    // no `link` declared anywhere.
    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        index.contains("vibedeps/flow-win/1.0.0/boot/win.md"),
        "{index}"
    );
    assert!(index.contains("kind = \"dynamic\""), "{index}");
    assert!(index.contains("when = \"os:windows\""), "{index}");
}

#[test]
fn apply_resolution_skips_a_dependency_outside_the_node_requires() {
    // The resolution carries `flow:extra`, but the project does not
    // require it — it is materialised, but contributes no boot entry.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (wal, _w) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let (extra, _e) = dep_with_boot(
        "extra",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/extra.md\"\n",
        "boot/extra.md",
        "# extra",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[wal, extra], SlotIntegrity::TrustPresence, None).unwrap();

    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        index.contains("vibedeps/flow-wal/0.3.0/boot/wal.md"),
        "{index}"
    );
    // `flow:extra` is materialised but not in the boot index.
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-extra/0.1.0/boot/extra.md")
            .is_file()
    );
    assert!(!index.contains("flow-extra"), "{index}");
}

#[test]
fn apply_resolution_prunes_a_stale_slot_on_version_bump() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();

    let (wal_v1, _v1) = dep_with_boot(
        "wal",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v1",
    );
    apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v1),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(ws_dir.path().join("vibedeps/flow-wal/0.1.0").is_dir());

    // Re-apply with wal bumped to 0.2.0 — the 0.1.0 slot is now stale.
    let (wal_v2, _v2) = dep_with_boot(
        "wal",
        "0.2.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v2",
    );
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v2),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(ws_dir.path().join("vibedeps/flow-wal/0.2.0").is_dir());
    assert!(
        !ws_dir.path().join("vibedeps/flow-wal/0.1.0").exists(),
        "the stale 0.1.0 slot must be pruned"
    );
    assert_eq!(outcome.pruned, vec!["vibedeps/flow-wal/0.1.0"]);
}

// --- PROP-011 §2.3 — materialise only the diff -----------------------

#[test]
fn apply_resolution_skips_a_present_slot_under_trust_presence() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    // First apply — the slot is absent, so it is materialised.
    let first = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert_eq!(first.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(first.skipped.is_empty());

    // A sentinel inside the slot — a file the source never had. If
    // the second apply re-copies the slot, `materialise` clears it
    // first and the sentinel vanishes; if it skips, the sentinel
    // survives. It is the proof the skip actually skipped.
    let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
    fs::write(&sentinel, "untouched").unwrap();

    let second = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(
        second.materialised.is_empty(),
        "a present slot must not be re-copied"
    );
    assert_eq!(second.skipped, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(
        sentinel.is_file(),
        "TrustPresence must leave the slot untouched"
    );
}

#[test]
fn apply_resolution_rematerialises_a_present_slot_under_verify() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
    fs::write(&sentinel, "doomed").unwrap();

    // Second apply under Verify — the present slot is re-materialised,
    // so the sentinel is cleared along with it.
    let second =
        apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    assert_eq!(second.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(second.skipped.is_empty(), "Verify must re-copy, never skip");
    assert!(!sentinel.exists(), "Verify must re-materialise the slot");
}

#[test]
fn apply_resolution_rematerialises_a_mutable_file_source_under_trust_presence() {
    // A `source_mutable` (local `file://`) dependency is never presence-trusted
    // (PROP-011 §2.6): even under the default TrustPresence its present slot is
    // re-copied, so an in-place source edit lands in `vibedeps/`.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let (mut dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    dep.source_mutable = true;
    let ws = Workspace::load(ws_dir.path()).unwrap();

    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
    fs::write(&sentinel, "doomed").unwrap();

    // Second apply — TrustPresence would normally skip a present slot, but the
    // mutable source overrides that, so the slot is re-materialised.
    let second = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert_eq!(second.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(
        second.skipped.is_empty(),
        "a mutable file:// source must not be presence-trusted (§2.6)"
    );
    assert!(
        !sentinel.exists(),
        "the mutable source's slot must be re-materialised"
    );
}

// --- PROP-020 2.1 — pre-install hooks ride the materialise pass ---------
