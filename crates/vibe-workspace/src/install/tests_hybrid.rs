//! Hybrid-linking (PROP-038) install tests for [`super`], out-of-line per the
//! file-length budget — the per-unit compilation, soft hoisting, and
//! dirty-subgraph behaviour driven through `apply_resolution`.

use super::test_helpers::*;
use super::*;
use tempfile::TempDir;

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
fn an_unchanged_reinstall_skips_a_package_via_its_fingerprint() {
    // PROP-038 §2.8 — a package with static children gets a fingerprinted
    // INDEX; a second apply with the same resolution finds the fingerprint
    // unchanged and skips the rewrite.
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
    apply_resolution(
        &ws,
        &[parent.clone(), child.clone()],
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    let parent_index = ws_dir
        .path()
        .join("vibedeps/flow-parent/1.0.0/spec/boot/INDEX.md");
    let index_text = fs::read_to_string(&parent_index).unwrap();
    assert!(
        index_text.contains("# vibe:fp "),
        "the per-unit INDEX carries a fingerprint header: {index_text}"
    );

    // Append a marker but keep the fingerprint line intact. A second apply with
    // the same resolution sees the unchanged fingerprint and skips the rewrite,
    // so the marker survives — proving the skip (not a coincidental identical
    // rewrite, which would have erased it).
    fs::write(
        &parent_index,
        format!("{index_text}\n# SKIP-PROOF-MARKER\n"),
    )
    .unwrap();
    apply_resolution(&ws, &[parent, child], SlotIntegrity::TrustPresence, None).unwrap();
    let after = fs::read_to_string(&parent_index).unwrap();
    assert!(
        after.contains("# SKIP-PROOF-MARKER"),
        "the dirty-subgraph skip left the file untouched: {after}"
    );
}
