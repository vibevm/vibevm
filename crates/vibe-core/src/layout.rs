//! The project directory layout — the ONE home of the four root names
//! (PROP-052 L2).
//!
//! Every project (and every package, L4) carries a single distinctive
//! root `vibevm/` holding `vibevm/vibespecs`, `vibevm/vibepacks`,
//! `vibevm/vibedeps` and `vibevm/vibefacts`. This module names those
//! roots exactly once; a root string literal anywhere else in the
//! product (tests' own scaffolds excepted) is a conform-guarded defect.
//! The physical move itself is RELAYOUT-PLAN R4 — until then the host
//! lives on the legacy layout, and the `current_*` family below is the
//! transitional seam: call sites route through it today without
//! behaviour change, and R4 flips the whole product by editing the one
//! [`USE_NEW_LAYOUT`] line here.
//!
//! Spec: [PROP-052](../../../vibevm/vibespecs/common/PROP-052-directory-layout.xml).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-052#root");

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// New layout — the law (PROP-052 ##THE-LAYOUT)
// ---------------------------------------------------------------------------

/// The one distinctive root every project and package carries (PROP-052):
/// `vibevm/`. No legacy project in the world names a root directory this
/// way, so the root is unambiguous for humans, grep and dynamic loaders
/// alike. `vibe.toml` stays at the project root — it does not move.
pub const VIBEVM_ROOT: &str = "vibevm";

/// The specs home under [`VIBEVM_ROOT`] — was `spec/`.
pub const VIBESPECS_DIR: &str = "vibespecs";
/// The packages home under [`VIBEVM_ROOT`] — was `packages/`.
pub const VIBEPACKS_DIR: &str = "vibepacks";
/// The dependency-slot home under [`VIBEVM_ROOT`] — was root `vibedeps/`,
/// and the name itself is unchanged by the move.
pub const VIBEDEPS_DIR: &str = "vibedeps";
/// The facts home under [`VIBEVM_ROOT`] — was root `vibefacts/`, name
/// unchanged by the move.
pub const VIBEFACTS_DIR: &str = "vibefacts";

// ---------------------------------------------------------------------------
// Legacy names — retired whole by R4 (PROP-052 L3), spelled only here
// ---------------------------------------------------------------------------

/// The legacy specs root (pre-move). Lives in this module so nothing
/// else ever spells it; dies with the R4 flip.
pub const LEGACY_SPECS_DIR: &str = "spec";
/// The legacy packages root (pre-move).
pub const LEGACY_PACKAGES_DIR: &str = "packages";
/// The legacy dependency-slot root (pre-move; the dir name itself
/// survives inside `vibevm/`, so this equals [`VIBEDEPS_DIR`]).
pub const LEGACY_VIBEDEPS_DIR: &str = "vibedeps";
/// The legacy facts root (pre-move; equals [`VIBEFACTS_DIR`] — the name
/// survives, only the parent changes).
pub const LEGACY_VIBEFACTS_DIR: &str = "vibefacts";

// ---------------------------------------------------------------------------
// Derived names inside the specs root (the boot lane and the WAL)
// ---------------------------------------------------------------------------

/// The generated boot lane directory: `<specs>/boot` (`vibevm/vibespecs/boot` today,
/// `vibevm/vibespecs/boot` after the move).
pub const BOOT_DIR: &str = "boot";
/// The living WAL, XML form.
pub const WAL_XML: &str = "WAL.xml";
/// The living WAL, Markdown form.
pub const WAL_MD: &str = "WAL.md";
/// The generated boot manifest: `<specs>/boot/INDEX.md`.
pub const INDEX_MD: &str = "INDEX.md";
/// The generated static boot lane, XML form: `<specs>/boot/STATIC.xml`.
pub const STATIC_XML: &str = "STATIC.xml";
/// The generated static boot lane, Markdown form.
pub const STATIC_MD: &str = "STATIC.md";

/// The project-relative specs root under the NEW layout: `vibevm/vibespecs`.
pub fn vibespecs_root() -> PathBuf {
    PathBuf::from(VIBEVM_ROOT).join(VIBESPECS_DIR)
}

/// The project-relative packages root under the NEW layout:
/// `vibevm/vibepacks`.
pub fn vibepacks_root() -> PathBuf {
    PathBuf::from(VIBEVM_ROOT).join(VIBEPACKS_DIR)
}

/// The project-relative dependency-slot root under the NEW layout:
/// `vibevm/vibedeps`.
pub fn vibedeps_root() -> PathBuf {
    PathBuf::from(VIBEVM_ROOT).join(VIBEDEPS_DIR)
}

/// The project-relative facts root under the NEW layout:
/// `vibevm/vibefacts`.
pub fn vibefacts_root() -> PathBuf {
    PathBuf::from(VIBEVM_ROOT).join(VIBEFACTS_DIR)
}

/// The generated boot lane directory under the NEW layout:
/// `vibevm/vibespecs/boot`.
pub fn boot_dir() -> PathBuf {
    vibespecs_root().join(BOOT_DIR)
}

/// The WAL stem, XML form, under the NEW layout: `vibevm/vibespecs/WAL.xml`.
pub fn wal_xml() -> PathBuf {
    vibespecs_root().join(WAL_XML)
}

/// The WAL stem, Markdown form, under the NEW layout.
pub fn wal_md() -> PathBuf {
    vibespecs_root().join(WAL_MD)
}

/// The generated boot manifest under the NEW layout:
/// `vibevm/vibespecs/boot/INDEX.md`.
pub fn boot_index() -> PathBuf {
    boot_dir().join(INDEX_MD)
}

/// The generated static boot lane, XML form, under the NEW layout.
pub fn boot_static_xml() -> PathBuf {
    boot_dir().join(STATIC_XML)
}

/// The generated static boot lane, Markdown form, under the NEW layout.
pub fn boot_static_md() -> PathBuf {
    boot_dir().join(STATIC_MD)
}

// ---------------------------------------------------------------------------
// Transitional seam — `current_*` names whichever layout is live
// ---------------------------------------------------------------------------

/// The single flip point of the whole relayout (PROP-052, RELAYOUT-PLAN
/// R4). `false` while the host physically lives on the legacy layout:
/// every `current_*` resolver then returns the OLD root, so migrating
/// crates to this module changes no behaviour. When R4 moves the four
/// roots into `vibevm/`, this line — and only this line — flips to
/// `true`, and every `current_*` call site names the new root at once.
const USE_NEW_LAYOUT: bool = true;

/// The specs root of whichever layout is live: `spec` today,
/// `vibevm/vibespecs` after the R4 flip of [`USE_NEW_LAYOUT`].
///
/// This is what crate code should call instead of spelling a root:
/// PROP-052 L2 allows the root names to exist only in this module.
pub fn current_specs_root() -> PathBuf {
    if USE_NEW_LAYOUT {
        vibespecs_root()
    } else {
        PathBuf::from(LEGACY_SPECS_DIR)
    }
}

/// The packages root of whichever layout is live: `packages` today,
/// `vibevm/vibepacks` after the flip.
pub fn current_packages_root() -> PathBuf {
    if USE_NEW_LAYOUT {
        vibepacks_root()
    } else {
        PathBuf::from(LEGACY_PACKAGES_DIR)
    }
}

/// The dependency-slot root of whichever layout is live: `vibedeps`
/// today, `vibevm/vibedeps` after the flip.
pub fn current_vibedeps_root() -> PathBuf {
    if USE_NEW_LAYOUT {
        vibedeps_root()
    } else {
        PathBuf::from(LEGACY_VIBEDEPS_DIR)
    }
}

/// The facts root of whichever layout is live: `vibefacts` today,
/// `vibevm/vibefacts` after the flip.
pub fn current_vibefacts_root() -> PathBuf {
    if USE_NEW_LAYOUT {
        vibefacts_root()
    } else {
        PathBuf::from(LEGACY_VIBEFACTS_DIR)
    }
}

/// The boot lane directory of the live layout: `vibevm/vibespecs/boot` today,
/// `vibevm/vibespecs/boot` after the flip. Derived from
/// [`current_specs_root`], so the flip needs no second edit here.
pub fn current_boot_dir() -> PathBuf {
    current_specs_root().join(BOOT_DIR)
}

/// The WAL stem, XML form, of the live layout (`vibevm/vibespecs/WAL.xml` today).
pub fn current_wal_xml() -> PathBuf {
    current_specs_root().join(WAL_XML)
}

/// The WAL stem, Markdown form, of the live layout (`spec/WAL.md` today).
pub fn current_wal_md() -> PathBuf {
    current_specs_root().join(WAL_MD)
}

/// The boot manifest of the live layout (`vibevm/vibespecs/boot/INDEX.md` today).
pub fn current_boot_index() -> PathBuf {
    current_boot_dir().join(INDEX_MD)
}

/// The static boot lane, XML form, of the live layout
/// (`vibevm/vibespecs/boot/STATIC.xml` today).
pub fn current_boot_static_xml() -> PathBuf {
    current_boot_dir().join(STATIC_XML)
}

/// The static boot lane, Markdown form, of the live layout
/// (`spec/boot/STATIC.md` today).
pub fn current_boot_static_md() -> PathBuf {
    current_boot_dir().join(STATIC_MD)
}

#[cfg(test)]
mod tests {
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
}
