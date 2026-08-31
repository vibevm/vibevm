//! Hybrid-linking (PROP-038) install tests for [`super`], out-of-line per the
//! file-length budget — the per-unit compilation, soft hoisting, and
//! dirty-subgraph behaviour driven through `apply_resolution`. The R4.1
//! transaction reds live one level down in [`transaction`].

// This file is itself loaded through a `#[path]` module declaration, so its
// children are spelled explicitly rather than inherited from a directory.
#[path = "tests_hybrid/transaction.rs"]
mod transaction;

use super::test_helpers::*;
use super::*;
use std::path::Path;
use tempfile::TempDir;
use vibe_core::PackageName;
use vibe_core::manifest::{LockedPackage, Lockfile};

struct SourceHash;

impl SlotVerifier for SourceHash {
    fn source_hash<'a>(&'a self, _dep: &ResolvedDep) -> Option<&'a str> {
        Some("sha256:test-source")
    }

    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Unverifiable
    }
}

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
                    let version = resolution
                        .iter()
                        .find(|target| &target.group == group && &target.name == name)
                        .unwrap()
                        .version
                        .clone();
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
            materialization: dep.manifest.package.as_ref().unwrap().materialization,
        })
        .collect();
    lock.write(root.join(Lockfile::FILENAME)).unwrap();
}

#[test]
fn copy_materialisation_requires_a_fetched_source_hash() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, _package) = dep_with_boot("wal", "0.3.0", "", "boot/wal.md", "# wal");
    dep.source_hash = None;
    let ws = Workspace::load({
        write(
            workspace.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        );
        workspace.path()
    })
    .unwrap();
    let error = apply_resolution(&ws, &[dep], SlotIntegrity::Verify, None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires the fetched source_hash"),
        "{error}"
    );
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

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
            .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/STATIC.md")),
    )
    .unwrap();
    assert!(parent_static.contains("# parent boot"), "{parent_static}");
    assert!(parent_static.contains("# child boot"), "{parent_static}");

    // The root does NOT compile the zone in — root→parent is dynamic.
    let root_static = ws_dir.path().join(boot_rel("STATIC.md"));
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
            .join(deps_slot_specs("org.vibevm.child/1.0.0", "boot/STATIC.md"))
            .exists()
    );

    // Root's INDEX points at parent's STATIC.md (the whole zone), not the
    // raw snippet — so loading parent pulls child with it.
    let root_index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(
        root_index.contains(&deps_slot_specs(
            "org.vibevm.parent/1.0.0",
            "boot/STATIC.md"
        )),
        "{root_index}"
    );
    assert!(
        !root_index.contains(&deps_rel("org.vibevm.parent/1.0.0/boot/parent.md")),
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

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
    let root_static = fs::read_to_string(ws_dir.path().join(boot_rel("STATIC.md"))).unwrap();
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
            .join(deps_slot_specs("org.vibevm.a/1.0.0", "boot/STATIC.md")),
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
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
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/INDEX.md"));
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

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
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/STATIC.md"));
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

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
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/STATIC.md"));

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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
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
    let resolution = vec![parent, child];
    apply_resolution(&ws, &resolution, SlotIntegrity::TrustPresence, None).unwrap();
    publish_resolution_lock(ws_dir.path(), &resolution);

    // A freshly generated graph is consistent.
    let clean = super::bootgen::verify_boot_graph(&ws).unwrap();
    assert!(
        clean.is_empty(),
        "fresh graph must be consistent: {clean:?}"
    );

    // Corrupt the parent's recorded fingerprint — verify must flag it stale.
    let parent_index = ws_dir
        .path()
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/INDEX.md"));
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

/// The dynamic-STATIC case as a target-format twin: the owner's core PROP-038
/// topology (root →dynamic parent →static child) runs once for each generated
/// static spelling. The root INDEX must follow the per-unit target exactly.
#[test]
fn the_dynamic_static_case_follows_the_target_format() {
    const PARENT_MD: &str = "# Parent boot {#parent-boot}\n\nParent body.\n\n";
    const CHILD_MD: &str = "# Child boot {#child-boot}\n\nChild body.\n\n";

    let run = |spec_format: SpecFormat| -> (String, String) {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n",
        );
        write(ws_dir.path(), boot_rel("00-core.md"), "# core");
        let (parent, _p) = dep_with_requires(
            "parent",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/parent.md\"\n\n\
             [requires.packages]\n\"org.vibevm/child\" = { version = \"^1.0\", link = \"static\" }\n",
            "boot/parent.md",
            PARENT_MD,
            &["child"],
        );
        let (child, _c) = dep_with_boot(
            "child",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/child.md\"\n",
            "boot/child.md",
            CHILD_MD,
        );
        let ws = Workspace::load(ws_dir.path()).unwrap();
        apply_resolution_with_spec_format(
            &ws,
            &[parent, child],
            SlotIntegrity::TrustPresence,
            spec_format,
            Some(&SourceHash),
            None,
        )
        .unwrap();
        let parent_static = fs::read_to_string(
            ws_dir
                .path()
                .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot"))
                .join(crate::boot_artifacts::static_file(spec_format)),
        )
        .unwrap();
        let root_index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
        (parent_static, root_index)
    };

    let (md_static, md_index) = run(SpecFormat::Markdown);
    let (xml_static, xml_index) = run(SpecFormat::Xml);
    assert!(
        xml_index.contains(&deps_slot_specs(
            "org.vibevm.parent/1.0.0",
            "boot/STATIC.xml"
        )),
        "the XML target must reach the per-unit XML lane: {xml_index}"
    );
    assert!(md_static.contains("Parent body."), "{md_static}");
    assert!(md_static.contains("Child body."), "{md_static}");
    assert_eq!(xml_static.matches("<spec ").count(), 2, "{xml_static}");
    assert!(
        md_index.contains(&deps_slot_specs(
            "org.vibevm.parent/1.0.0",
            "boot/STATIC.md"
        )),
        "{md_index}"
    );
}
