//! The layout owner's own oracles: the live and legacy roots, the
//! derived boot names, and the role classification of the
//! system-prompt paths (PROP-076 `#SYS-LAYOUT-RESERVATION`).

use super::*;

#[test]
fn new_roots_are_the_vibevm_root_plus_their_dir_names() {
    // Expected paths are composed from the constants — never fresh
    // literals (the packet's rule; the constants are the law itself).
    assert_eq!(
        vibespecs_root(),
        PathBuf::from(VIBEVM_ROOT).join(VIBESPECS_DIR)
    );
    assert_eq!(
        vibepacks_root(),
        PathBuf::from(VIBEVM_ROOT).join(VIBEPACKS_DIR)
    );
    assert_eq!(
        vibedeps_root(),
        PathBuf::from(VIBEVM_ROOT).join(VIBEDEPS_DIR)
    );
    assert_eq!(
        vibefacts_root(),
        PathBuf::from(VIBEVM_ROOT).join(VIBEFACTS_DIR)
    );
}

#[test]
fn the_four_new_dirs_are_pairwise_distinct() {
    let dirs = [VIBESPECS_DIR, VIBEPACKS_DIR, VIBEDEPS_DIR, VIBEFACTS_DIR];
    for (i, a) in dirs.iter().enumerate() {
        assert!(!a.is_empty(), "a root dir name must not be empty");
        for b in &dirs[i + 1..] {
            assert_ne!(a, b, "the four dirs under vibevm/ must not collide");
        }
    }
}

#[test]
fn deps_and_facts_keep_their_names_only_the_parent_moves() {
    // The mandate: vibedeps and vibefacts move "с исходным именем" —
    // the dir names are identical before and after, only the parent
    // changes (root -> vibevm/).
    assert_eq!(VIBEDEPS_DIR, LEGACY_VIBEDEPS_DIR);
    assert_eq!(VIBEFACTS_DIR, LEGACY_VIBEFACTS_DIR);
    // specs and packages, by contrast, are RENAMED by the move.
    assert_ne!(VIBESPECS_DIR, LEGACY_SPECS_DIR);
    assert_ne!(VIBEPACKS_DIR, LEGACY_PACKAGES_DIR);
}

#[test]
fn derived_paths_nest_under_their_roots() {
    assert_eq!(boot_dir(), vibespecs_root().join(BOOT_DIR));
    assert_eq!(wal_xml(), vibespecs_root().join(WAL_XML));
    assert_eq!(wal_md(), vibespecs_root().join(WAL_MD));
    assert_eq!(boot_index(), boot_dir().join(INDEX_MD));
    assert_eq!(boot_static_xml(), boot_dir().join(STATIC_XML));
    assert_eq!(boot_static_md(), boot_dir().join(STATIC_MD));
}

#[test]
fn the_three_system_prompt_paths_each_hold_exactly_one_role() {
    // PROP-076 #SYS-LAYOUT-RESERVATION: the generated outputs — the
    // prefix, its sidecar, the agent lane and the ignore file vibe writes
    // to cover them — and the one authored input. The roles are disjoint,
    // and the authored one is reserved without ever being owned: the
    // distinction that keeps a format change from deleting the user's
    // own file.
    let generated = [
        current_boot_system(),
        current_boot_system_meta(),
        current_boot_agent_lane(),
        current_boot_gitignore(),
    ];
    let authored = current_boot_system_template();
    for path in &generated {
        assert!(
            is_generated_boot_output(path),
            "{} is generated",
            path.display()
        );
        assert!(!is_reserved_boot_authored_input(path));
        assert!(is_reserved_boot_source(path));
    }
    assert!(
        !is_generated_boot_output(&authored),
        "the template must never be classified as a generated output"
    );
    assert!(is_reserved_boot_authored_input(&authored));
    assert!(is_reserved_boot_source(&authored));
    // The three prompt artifacts are one list, so a caller cannot
    // enumerate two of them and forget the third.
    assert_eq!(
        current_boot_prompt_artifacts().to_vec(),
        vec![
            current_boot_system(),
            current_boot_system_meta(),
            current_boot_agent_lane()
        ]
    );
    // Pairwise distinct, and every one lives in the one boot directory —
    // the invariant the publication transaction's common parent needs.
    let mut all = generated.to_vec();
    all.push(authored);
    for (i, a) in all.iter().enumerate() {
        assert_eq!(a.parent(), Some(current_boot_dir().as_path()));
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn an_ascii_case_alias_of_every_reserved_boot_path_is_refused_as_a_source() {
    for path in reserved_boot_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let alias = current_boot_dir().join(name.to_ascii_uppercase());
        assert!(
            is_reserved_boot_source(&alias),
            "{} must be refused as a declared source row",
            alias.display()
        );
    }
}

#[test]
fn the_system_prompt_names_are_composed_from_the_constants() {
    assert_eq!(current_boot_system(), current_boot_dir().join(SYSTEM_MD));
    assert_eq!(
        current_boot_system_meta(),
        current_boot_dir().join(SYSTEM_META_JSON)
    );
    assert_eq!(
        current_boot_system_template(),
        current_boot_dir().join(SYSTEM_TEMPLATE_MD)
    );
}

#[test]
fn the_boot_artifacts_lock_name_is_pinned() {
    // A wire-format golden, not a derivation: the exact basename is
    // the contract with the on-disk world (`.gitignore`, checkers,
    // vibe-workspace's lock), so here the literal is the point — the
    // composed-paths rule above does not apply to it.
    assert_eq!(BOOT_ARTIFACTS_LOCK, ".vibe-boot-artifacts.lock");
}

#[test]
fn current_roots_follow_the_flip_const() {
    // The assertion set is chosen by the same const the resolvers
    // read, so this test states the contract under BOTH flip states
    // and never reddens when R4 edits the one line.
    if USE_NEW_LAYOUT {
        assert_eq!(current_specs_root(), vibespecs_root());
        assert_eq!(current_packages_root(), vibepacks_root());
        assert_eq!(current_vibedeps_root(), vibedeps_root());
        assert_eq!(current_vibefacts_root(), vibefacts_root());
    } else {
        assert_eq!(current_specs_root(), PathBuf::from(LEGACY_SPECS_DIR));
        assert_eq!(current_packages_root(), PathBuf::from(LEGACY_PACKAGES_DIR));
        assert_eq!(current_vibedeps_root(), PathBuf::from(LEGACY_VIBEDEPS_DIR));
        assert_eq!(
            current_vibefacts_root(),
            PathBuf::from(LEGACY_VIBEFACTS_DIR)
        );
    }
}

#[test]
fn current_derived_paths_nest_under_the_live_specs_root() {
    // Derived current_* paths derive from current_specs_root(), so
    // the R4 flip re-maps them with no second edit (e.g. today they
    // name vibevm/vibespecs/boot/..., after it vibevm/vibespecs/boot/...).
    assert_eq!(current_boot_dir(), current_specs_root().join(BOOT_DIR));
    assert_eq!(current_wal_xml(), current_specs_root().join(WAL_XML));
    assert_eq!(current_wal_md(), current_specs_root().join(WAL_MD));
    assert_eq!(current_boot_index(), current_boot_dir().join(INDEX_MD));
    assert_eq!(
        current_boot_static_xml(),
        current_boot_dir().join(STATIC_XML)
    );
    assert_eq!(current_boot_static_md(), current_boot_dir().join(STATIC_MD));
}

#[test]
fn a_document_keeps_its_suffix_under_either_layout() {
    // L1 — physics moves, addresses do not: a document's path under
    // the specs root is identical in both layouts; only the root
    // prefix maps differently. (The doc-path half of the law — that
    // canonical_doc_path canonicalises both physical prefixes to one
    // doc-path — is proven in vibe-spec's resolver tests.)
    let doc = "common/PROP-000.xml";
    let new_full = vibespecs_root().join(doc);
    let old_full = current_specs_root().join(doc);
    assert_eq!(
        new_full.strip_prefix(vibespecs_root()),
        old_full.strip_prefix(current_specs_root()),
    );
    // And the legacy full path really ends with that same suffix —
    // the two full paths differ by the root prefix and nothing else.
    let suffix = new_full
        .strip_prefix(vibespecs_root())
        .unwrap_or(new_full.as_path());
    assert!(old_full.ends_with(suffix));
}
