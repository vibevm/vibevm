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

#[test]
fn a_changed_static_child_forces_the_parent_to_regenerate() {
    // PROP-038 §2.7-§2.8 — the owner's core fear ("не забыли перегенерить"):
    // a change to a static child must NOT leave a stale parent. When the child
    // bumps to a version carrying new content, the parent's fingerprint flips
    // and the dirty-subgraph regenerates it, so the parent's STATIC.md carries
    // the NEW child, not the old — the incremental result equals a full regen.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let parent_toml = "[boot_snippet]\nsource = \"boot/parent.md\"\n\n\
         [requires.packages]\n\"org.vibevm/child\" = { version = \"^1.0\", link = \"static\" }\n";
    let make_parent = || {
        dep_with_requires(
            "parent",
            "1.0.0",
            parent_toml,
            "boot/parent.md",
            "# parent boot",
            &["child"],
        )
    };
    let (parent, _p) = make_parent();
    let (child_v1, _c1) = dep_with_boot(
        "child",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        "# child ONE",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[parent, child_v1], SlotIntegrity::TrustPresence, None).unwrap();
    let parent_static_path = ws_dir
        .path()
        .join("vibedeps/flow-parent/1.0.0/spec/boot/STATIC.md");
    assert!(
        fs::read_to_string(&parent_static_path)
            .unwrap()
            .contains("# child ONE")
    );

    // The child bumps to a new version with new content; the parent's edge is
    // unchanged. A second apply must regenerate the parent (its fingerprint
    // depends on the child version) and carry the NEW child.
    let (parent2, _p2) = make_parent();
    let (child_v2, _c2) = dep_with_boot(
        "child",
        "2.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        "# child TWO",
    );
    apply_resolution(
        &ws,
        &[parent2, child_v2],
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    let parent_static = fs::read_to_string(&parent_static_path).unwrap();
    assert!(
        parent_static.contains("# child TWO"),
        "the parent must regenerate with the new child: {parent_static}"
    );
    assert!(
        !parent_static.contains("# child ONE"),
        "no stale child may survive: {parent_static}"
    );
}

#[test]
fn switching_a_child_from_dynamic_to_static_regenerates_the_parent() {
    // A link-type switch is invisible to resolution (same versions) but flips
    // the fingerprint (PROP-038 §2.7), so the dirty-subgraph still regenerates.
    // With child dynamic the parent has no STATIC.md; switching child to static
    // makes the parent compile it in.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let parent_of = |child_link: &str| {
        dep_with_requires(
            "parent",
            "1.0.0",
            &format!(
                "[boot_snippet]\nsource = \"boot/parent.md\"\n\n\
                 [requires.packages]\n\"org.vibevm/child\" = {child_link}\n"
            ),
            "boot/parent.md",
            "# parent boot",
            &["child"],
        )
    };
    let child = || {
        dep_with_boot(
            "child",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/child.md\"\n",
            "boot/child.md",
            "# child boot",
        )
    };
    let parent_static_path = ws_dir
        .path()
        .join("vibedeps/flow-parent/1.0.0/spec/boot/STATIC.md");

    // Child dynamic — the parent has no static child, so no STATIC.md.
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let (p_dyn, _pd) = parent_of("\"^1.0\"");
    let (c1, _c1) = child();
    apply_resolution(&ws, &[p_dyn, c1], SlotIntegrity::TrustPresence, None).unwrap();
    assert!(
        !parent_static_path.exists(),
        "a dynamic child leaves the parent with no STATIC.md"
    );

    // Switch child to static — the fingerprint flips, the parent regenerates
    // and now carries the child.
    let (p_stat, _ps) = parent_of("{ version = \"^1.0\", link = \"static\" }");
    let (c2, _c2) = child();
    apply_resolution(&ws, &[p_stat, c2], SlotIntegrity::TrustPresence, None).unwrap();
    let parent_static = fs::read_to_string(&parent_static_path).unwrap();
    assert!(
        parent_static.contains("# child boot"),
        "the link switch must recompile the parent with the child: {parent_static}"
    );
}

#[test]
fn verify_boot_graph_detects_a_stale_artifact() {
    // PROP-038 §3 — the integrity check (`vibe check`'s boot-graph half): a
    // freshly generated graph is consistent; a corrupted fingerprint is
    // flagged stale.
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

    // A freshly generated graph is consistent.
    let clean = super::bootgen::verify_boot_graph(&ws).unwrap();
    assert!(
        clean.is_empty(),
        "fresh graph must be consistent: {clean:?}"
    );

    // Corrupt the parent's recorded fingerprint — verify must flag it stale.
    let parent_index = ws_dir
        .path()
        .join("vibedeps/flow-parent/1.0.0/spec/boot/INDEX.md");
    let text = fs::read_to_string(&parent_index).unwrap();
    let stored = super::super::boot_artifacts::read_fingerprint(&text).unwrap();
    fs::write(&parent_index, text.replace(&stored, "deadbeef")).unwrap();

    let stale = super::bootgen::verify_boot_graph(&ws).unwrap();
    assert_eq!(
        stale.len(),
        1,
        "the corrupted unit must be flagged: {stale:?}"
    );
    assert_eq!(stale[0].1, "parent");
}
