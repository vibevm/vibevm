//! The R4.1 atom-B transaction reds, split from `tests_hybrid.rs` so no
//! authored file approaches the length budget: the per-unit publication
//! joins the crash-recoverable transaction manager, so a backend refusal
//! leaves a unit's pre-existing bytes byte-exact, a fresh reinstall skips
//! with no mtime churn, and the unit write path binds the manager-owned
//! wrapper instead of publishing directly. Fixtures and helpers come from
//! the parent (`use super::*`), never copied.

use super::*;
use specmark::verifies;

/// R4.1 atom B — the per-unit lane renders/compiles every fallible semantic
/// byte BEFORE touching the old artifact set, so a compile/backend refusal
/// leaves the unit's pre-existing INDEX and STATIC bytes byte-exact. The
/// refusal vehicle is the XML target: a child whose boot content declares one
/// label twice is refused by the real `static-xml` backend at the parent's
/// zone-compile boundary (the markdown lane does not refuse it). The
/// pre-transaction implementation published the INDEX before compiling, so a
/// refusal replaced the INDEX of a half-published unit; the node path already
/// carried this law (`boot_artifacts::tests_ir_characterization`), and the
/// unit path now joins it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#UNIT-PUBLICATION-TRANSACTION")]
fn a_per_unit_backend_refusal_leaves_the_units_artifacts_byte_exact() {
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
    let (child, _c) = dep_with_boot(
        "child",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        "# child boot",
    );

    // Seed one good generation so the unit's boot lane exists on disk.
    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution_with_spec_format(
        &ws,
        &[parent, child],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();

    // The byte-exactness targets: sentinels in the unit's own boot lane. The
    // sentinel INDEX also destroys the recorded fingerprint, forcing the
    // dirty path whatever the resolution says.
    let parent_boot = ws_dir
        .path()
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot"));
    let parent_index = parent_boot.join("INDEX.md");
    let parent_static = parent_boot.join("STATIC.xml");
    let parent_stale = parent_boot.join("STATIC.md");
    fs::write(&parent_index, b"UNIT-INDEX-SENTINEL").unwrap();
    fs::write(&parent_static, b"UNIT-STATIC-SENTINEL").unwrap();
    fs::write(&parent_stale, b"UNIT-STALE-SENTINEL").unwrap();

    // The child bumps to a version whose boot content the real backend
    // refuses (duplicate anchors) — the parent's zone compile must fail.
    let (parent2, _p2) = make_parent();
    let (child_bad, _b) = dep_with_boot(
        "child",
        "2.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"\n",
        "boot/child.md",
        "# Child {#root}\n\n##DUP one.\n\n##DUP two.\n",
    );
    let error = apply_resolution_with_spec_format(
        &ws,
        &[parent2, child_bad],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("org-vibevm--child--DUP")
            && error.to_string().contains("defined twice"),
        "the duplicate-anchor child must fail the zone compile: {error}"
    );

    // Every pre-existing byte is byte-exact — nothing was published.
    assert_eq!(fs::read(&parent_index).unwrap(), b"UNIT-INDEX-SENTINEL");
    assert_eq!(fs::read(&parent_static).unwrap(), b"UNIT-STATIC-SENTINEL");
    assert_eq!(fs::read(&parent_stale).unwrap(), b"UNIT-STALE-SENTINEL");
}

/// R4.1 atom B — successful per-unit bytes and the unchanged-reinstall skip
/// stay identical under the transactional publication: a first apply produces
/// the unit's fingerprinted INDEX and compiled STATIC, and a second apply with
/// the same resolution leaves BOTH files' bytes and mtimes untouched (the
/// fingerprint-fresh early skip never enters the transaction).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#UNIT-PUBLICATION-TRANSACTION")]
fn an_unchanged_reinstall_preserves_unit_bytes_and_mtimes() {
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
    let parent_static = ws_dir
        .path()
        .join(deps_slot_specs("org.vibevm.parent/1.0.0", "boot/STATIC.md"));
    let index_before = fs::read(&parent_index).unwrap();
    let static_before = fs::read(&parent_static).unwrap();
    assert!(
        crate::boot_artifacts::read_fingerprint(&String::from_utf8(index_before.clone()).unwrap())
            .is_some(),
        "a successful unit INDEX carries the fingerprint header"
    );
    assert!(
        String::from_utf8(static_before.clone())
            .unwrap()
            .contains("# child boot"),
        "a successful unit STATIC compiles the zone: {static_before:?}"
    );
    let index_mtime = fs::metadata(&parent_index).unwrap().modified().unwrap();
    let static_mtime = fs::metadata(&parent_static).unwrap().modified().unwrap();

    apply_resolution(&ws, &[parent, child], SlotIntegrity::TrustPresence, None).unwrap();

    assert_eq!(fs::read(&parent_index).unwrap(), index_before);
    assert_eq!(fs::read(&parent_static).unwrap(), static_before);
    assert_eq!(
        fs::metadata(&parent_index).unwrap().modified().unwrap(),
        index_mtime,
        "a fresh unit is never rewritten — no INDEX mtime churn"
    );
    assert_eq!(
        fs::metadata(&parent_static).unwrap().modified().unwrap(),
        static_mtime,
        "a fresh unit is never rewritten — no STATIC mtime churn"
    );
}

/// R4.1 atom B — the per-unit write path contains NO direct publication: no
/// bare write/remove/create of boot bytes in `hybrid_emit`, which instead
/// binds the manager-owned wrapper, and the wrapper itself routes through the
/// same crash-recoverable transaction manager as the node path. The
/// transaction's own fault/rollback suite stays the crash oracle; this pins
/// only the binding.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#UNIT-PUBLICATION-TRANSACTION")]
fn the_unit_write_path_publishes_only_through_the_transaction_manager() {
    let unit_lane = include_str!("../bootgen/hybrid_emit.rs");
    for forbidden in [
        "fs::write",
        "fs::remove_file",
        "fs::create_dir_all",
        "remove_if_exists",
        "write_redirect",
    ] {
        assert!(
            !unit_lane.contains(forbidden),
            "the unit lane must not publish boot bytes directly (`{forbidden}` found)"
        );
    }
    assert!(
        unit_lane.contains("boot_artifacts::publish_unit_artifacts"),
        "the unit lane must bind the manager-owned publication wrapper"
    );

    let manager = include_str!("../../boot_artifacts/publication.rs");
    let wrapper_start = manager
        .find("fn publish_unit_artifacts")
        .expect("the manager owns the unit publication wrapper");
    let wrapper = &manager[wrapper_start..];
    let wrapper = &wrapper[..wrapper.find("\n}").expect("the wrapper closes")];
    assert!(
        wrapper.contains("transaction::write_production_with_selectors"),
        "the unit wrapper must route through the crash-recoverable transaction"
    );
}
