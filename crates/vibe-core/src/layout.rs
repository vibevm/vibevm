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
/// Identity and footprint record filename at a materialized dependency root.
pub const SLOT_RECORD_FILENAME: &str = ".vibe-slot.toml";
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
/// The generated cache-stable system-prompt prefix:
/// `<specs>/boot/SYSTEM.md` (PROP-076 `#SYS-FILES`). The `.md` extension
/// is a name, not a parse — the file's lane segment holds the compiled
/// lane's bytes verbatim, whatever their dialect.
pub const SYSTEM_MD: &str = "SYSTEM.md";
/// The generated sidecar of the prefix: `<specs>/boot/SYSTEM.meta.json`,
/// protocol `vibe-system-prompt/1` (PROP-076 `#SYS-SIDECAR`).
pub const SYSTEM_META_JSON: &str = "SYSTEM.meta.json";
/// The generated agent-mode lane: `<specs>/boot/AGENT-LANE.md`
/// (PROP-076 `#SYS-AGENT-LANE`) — the compiled static lane followed by every
/// unconditional INDEX file, each wrapped in a marker pair. It is **not**
/// `SYSTEM.md` minus a segment: the prefix is for an API caller with no base
/// prompt, and this is for a harness that has already loaded the entrypoint
/// itself, so neither is derivable from the other by truncation.
pub const AGENT_LANE_MD: &str = "AGENT-LANE.md";
/// The ignore file `vibe` writes **into the boot directory it owns**, which
/// is how the three generated prompt artifacts stay untracked
/// (PROP-076 `#SYS-ARTIFACTS-NOT-TRACKED`). The repository root's
/// `.gitignore` is never touched: that file is the developer's, and a
/// generator that edits it is a generator that will one day lose a line of
/// theirs.
pub const BOOT_GITIGNORE: &str = ".gitignore";
/// The **authored** system-prompt template: `<specs>/boot/SYSTEM-TEMPLATE.md`
/// (PROP-076 `#SYS-FILES`). Its presence is the whole opt-in. It is the one
/// input the user writes, so it is deliberately NOT a member of
/// [`generated_boot_paths`]: see [`is_reserved_boot_authored_input`].
pub const SYSTEM_TEMPLATE_MD: &str = "SYSTEM-TEMPLATE.md";
/// The transaction engine's persistent lock in the boot directory
/// (vibe-workspace's crash-released exclusive lock). Unlike the journal
/// and stage files it is not crash state: it survives a successful
/// transaction as normal layout, so a checker treats exactly this name
/// as engine-owned clean state. The basename is law here once —
/// vibe-workspace sources its lock name from this constant.
pub const BOOT_ARTIFACTS_LOCK: &str = ".vibe-boot-artifacts.lock";
/// Direct generated boot publication names share this reserved prefix.
pub const BOOT_PUBLICATION_PREFIX: &str = ".vibe-boot-";

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

/// The WAL stem, Markdown form, of the live layout
/// (`vibevm/vibespecs/WAL.md` today).
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

/// The generated system-prompt prefix of the live layout
/// (`vibevm/vibespecs/boot/SYSTEM.md` today).
pub fn current_boot_system() -> PathBuf {
    current_boot_dir().join(SYSTEM_MD)
}

/// The generated prefix sidecar of the live layout
/// (`vibevm/vibespecs/boot/SYSTEM.meta.json` today).
pub fn current_boot_system_meta() -> PathBuf {
    current_boot_dir().join(SYSTEM_META_JSON)
}

/// The generated agent-mode lane of the live layout
/// (`vibevm/vibespecs/boot/AGENT-LANE.md` today).
pub fn current_boot_agent_lane() -> PathBuf {
    current_boot_dir().join(AGENT_LANE_MD)
}

/// The ignore file `vibe` writes into the boot directory of the live layout
/// (`vibevm/vibespecs/boot/.gitignore` today).
pub fn current_boot_gitignore() -> PathBuf {
    current_boot_dir().join(BOOT_GITIGNORE)
}

/// The three generated prompt artifacts of the live layout, in publication
/// order — the prefix, its sidecar and the agent lane.
///
/// One list, because every rule that applies to one applies to all three:
/// they are produced by the same generation, covered by the same vibe-written
/// ignore file, removed together when a node opts out, and reported together
/// when git tracks one (PROP-076 `#SYS-ARTIFACTS-NOT-TRACKED`). A caller that
/// enumerated two of them would be a caller that one day forgets the third.
pub fn current_boot_prompt_artifacts() -> [PathBuf; 3] {
    [
        current_boot_system(),
        current_boot_system_meta(),
        current_boot_agent_lane(),
    ]
}

/// The authored system-prompt template of the live layout
/// (`vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md` today).
pub fn current_boot_system_template() -> PathBuf {
    current_boot_dir().join(SYSTEM_TEMPLATE_MD)
}

/// Whether a package-relative path belongs to generated boot publication.
/// Only the owned boot directory and its direct publication artifacts match.
/// ```
/// use std::path::Path;
/// use vibe_core::layout;
/// assert!(layout::is_generated_boot_output(&layout::current_boot_static_xml()));
/// assert!(layout::is_generated_boot_output(&layout::current_boot_system()));
/// assert!(layout::is_generated_boot_output(&layout::current_boot_system_meta()));
/// assert!(layout::is_generated_boot_output(&layout::current_boot_dir().join(".vibe-boot-stage")));
/// assert!(!layout::is_generated_boot_output(Path::new("data/STATIC.xml")));
/// // The authored template is reserved, but it is NOT a generated output:
/// // nothing owns it, nothing removes it as a stale artifact.
/// assert!(!layout::is_generated_boot_output(&layout::current_boot_system_template()));
/// ```
pub fn is_generated_boot_output(relative: &std::path::Path) -> bool {
    generated_boot_paths().iter().any(|path| relative == path)
        || (relative.parent() == Some(current_boot_dir().as_path())
            && relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(BOOT_PUBLICATION_PREFIX)))
}

/// Whether a package-relative path is the **authored** boot input the
/// generator reserves but never owns (PROP-076 `#SYS-LAYOUT-RESERVATION`).
///
/// It is a separate predicate rather than a member of
/// [`generated_boot_paths`] on purpose. The template is excluded from
/// automatic boot enrolment and from declaration admission as a source row,
/// but it is not generated, is not owned by publication, is never deleted as
/// a stale output, and is preserved like any other authored file. Collapsing
/// the two roles into one list would be the convenient mistake that loses
/// the user's own file on a format change.
/// ```
/// use vibe_core::layout;
/// assert!(layout::is_reserved_boot_authored_input(&layout::current_boot_system_template()));
/// assert!(!layout::is_reserved_boot_authored_input(&layout::current_boot_system()));
/// assert!(!layout::is_reserved_boot_authored_input(&layout::current_boot_index()));
/// ```
pub fn is_reserved_boot_authored_input(relative: &std::path::Path) -> bool {
    relative == current_boot_system_template()
}

fn generated_boot_paths() -> [PathBuf; 7] {
    [
        current_boot_static_md(),
        current_boot_static_xml(),
        current_boot_index(),
        current_boot_system(),
        current_boot_system_meta(),
        current_boot_agent_lane(),
        current_boot_gitignore(),
    ]
}

/// Every boot path the generator reserves, in either role: the generated
/// outputs plus the one authored input. Declaration admission refuses all of
/// them, because a source row naming any one of them would make an output (or
/// the template delivered twice) an ordinary boot input.
fn reserved_boot_paths() -> [PathBuf; 8] {
    let [
        static_md,
        static_xml,
        index,
        system,
        system_meta,
        agent_lane,
        ignore,
    ] = generated_boot_paths();
    [
        static_md,
        static_xml,
        index,
        system,
        system_meta,
        agent_lane,
        ignore,
        current_boot_system_template(),
    ]
}

/// Declaration admission also excludes ASCII aliases of reserved outputs.
/// This is stricter than exact payload ownership, not a claim that every alias
/// is a generated file on a case-sensitive host. Unicode aliases are separate.
/// ```
/// use vibe_core::layout;
/// let alias = layout::current_boot_dir().join("static.XML");
/// assert!(!layout::is_generated_boot_output(&alias));
/// assert!(layout::is_reserved_boot_source(&alias));
/// assert!(!layout::is_reserved_boot_source(&layout::current_boot_dir().join("nested/.vibe-boot-note")));
/// // The authored template is refused as a declared source in any ASCII
/// // case spelling, although it is not a generated output.
/// assert!(layout::is_reserved_boot_source(&layout::current_boot_dir().join("system-template.MD")));
/// assert!(layout::is_reserved_boot_source(&layout::current_boot_dir().join("system.META.json")));
/// ```
pub fn is_reserved_boot_source(relative: &std::path::Path) -> bool {
    let spelling = relative.to_string_lossy().replace('\\', "/");
    reserved_boot_paths()
        .iter()
        .any(|path| spelling.eq_ignore_ascii_case(&path.to_string_lossy().replace('\\', "/")))
        || relative.parent().is_some_and(|parent| {
            parent
                .to_string_lossy()
                .replace('\\', "/")
                .eq_ignore_ascii_case(&current_boot_dir().to_string_lossy().replace('\\', "/"))
                && relative
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.to_ascii_lowercase()
                            .starts_with(BOOT_PUBLICATION_PREFIX)
                    })
        })
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;
