//! E1-W4-DYNLANE — the dynamic-lane exhibit (B-011 slice W4 / M-LOAD's second
//! half, made executable).
//!
//! An integration test over a **real temp filesystem** proving the design's §6
//! composition claims end to end — `spec/design/deterministic-loading-aliasing.md`
//! §6 (the dynamic case), §6.1 (the stale-short defence), and `TOOLING-MAP.md`
//! `##M-LOAD`. The merged W1–W3 machinery it exercises: `render_static`
//! (qualify-on-splice, the resolution preamble, the renamed-anchors tombstone),
//! `qualify_contribution`, `compile_static`'s `@!X` → `@spec://…` rewrite, the
//! `FsSectionSource` resolver (source-side resolution + candidates-on-miss), and
//! `DocTree` (qualified anchors).
//!
//! Three exhibits:
//!
//! - **A — append-only composition (§6).** Two `simple` packages define the SAME
//!   short labels `{#root}` + `##THE-RULE`. Rendering A alone, then the world
//!   A+B, leaves A's block byte-identical (B's arrival renamed nothing of A's);
//!   the A+B lane has zero duplicate anchors; the tombstone lists `root` with
//!   both heirs.
//! - **B — the alias survives a cleaned carrier (§4 / §6.1; M-LOAD's second
//!   measurement).** A `normal` package aliases another package's anchor and
//!   cites it via `@!ARULE`. The compiled lane rewrites `@!ARULE` to the full
//!   `@spec://…` address; resolving that address goes to the **source** under
//!   `vibedeps/` — never the lane — and keeps resolving once the lane (the
//!   carrier) is cleaned.
//! - **C — a missed short anchor answers with its heirs (§6.1 layer 3, at the
//!   tree level).** Over the A+B lane's DocTree, `find_by_anchor("root")` misses
//!   but `qualified_candidates("root")` returns both heirs, sorted.
//!
//! The lane's boot types are built directly through the crate's public surface
//! (`vibe_workspace::boot::{BootEntry, BootBand, EffectiveBoot}` are all `pub`
//! with `pub` fields, and `render_static` is `pub` via `pub mod boot_artifacts`)
//! — no source widening was needed (R2). The fixtures mirror the in-crate
//! unit-test helpers' on-disk shapes (R3): `write_simple_boot` (a `simple`
//! `vibedeps/<slot>/1.0.0/boot.md`) and `write_aliaser_fixture` (a `normal`
//! package's files under `vibedeps/<slot>/1.0.0/spec/<doc>.md`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::{LinkType, PackageFormat};
use vibe_spec::{DocTree, FileResolver, FsSectionSource, SectionSource, SpecAddress};
use vibe_workspace::boot::{BootBand, BootEntry, EffectiveBoot};
use vibe_workspace::boot_artifacts::render_static;

// ----- helpers (public-surface mirrors of the in-crate unit-test helpers) --

/// A `simple`-format `static` boot entry — the verbatim-contribution path
/// (PROP-035 §3). Mirrors `boot_artifacts::tests::entry`.
fn entry_simple(path: &str, origin: &str) -> BootEntry {
    BootEntry {
        path: path.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: origin.to_string(),
        use_ref: false,
        format: PackageFormat::Simple,
    }
}

/// A `normal`-format `static` boot entry — the compile-the-closure path
/// (PROP-035 §8). Mirrors `boot_artifacts::tests::entry_normal`.
fn entry_normal(path: &str, origin: &str) -> BootEntry {
    BootEntry {
        path: path.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: origin.to_string(),
        use_ref: false,
        format: PackageFormat::Normal,
    }
}

/// Build an [`EffectiveBoot`] from its entries (the composed sequence).
fn boot(entries: Vec<BootEntry>) -> EffectiveBoot {
    EffectiveBoot { entries }
}

/// Write a `simple` boot file under `vibedeps/<slot>/1.0.0/boot.md` and return
/// its workspace-relative path. Mirrors `boot_artifacts::tests::write_simple_boot`
/// (the `simple`-format on-disk shape, R3).
fn write_simple_boot(ws: &Path, slot: &str, body: &str) -> String {
    let p = ws.join(format!("vibedeps/{slot}/1.0.0/boot.md"));
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
    format!("vibedeps/{slot}/1.0.0/boot.md")
}

/// Write a `normal` package's source file under `vibedeps/<slot>/1.0.0/spec/<rel>`
/// (the layout `normal_seed` derives a `spec://` seed from — the `normal`-format
/// on-disk shape, R3) and return its workspace-relative path. Mirrors the
/// `write_aliaser_fixture` / `write_greeter_fixture` layout.
fn write_spec_doc(ws: &Path, slot: &str, rel: &str, body: &str) -> String {
    let p = ws.join(format!("vibedeps/{slot}/1.0.0/spec/{rel}"));
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
    format!("vibedeps/{slot}/1.0.0/spec/{rel}")
}

/// Extract one package's compiled block (its provenance marker + origin-
/// qualified body) from a rendered lane — the slice from its provenance marker
/// to the next package's marker, or end-of-lane. Because `qualify_contribution`
/// is a pure function of `(body, origin)` with no view of any sibling, the same
/// origin yields a byte-identical block whether the package stands alone or a
/// sibling follows it (design §6 — append-independence at the lane level).
fn block_for<'a>(lane: &'a str, origin: &str) -> &'a str {
    // The provenance marker is `<!-- vibe:static <origin> — <path> -->` — the
    // em-dash is U+2014, exactly as `render_static` emits it.
    let needle = format!("<!-- vibe:static {origin} — ");
    let start = lane
        .find(&needle)
        .unwrap_or_else(|| panic!("no provenance marker for `{origin}`:\n{lane}"));
    // The block runs to the next provenance marker (another package) or to the
    // end of the lane (this package is last).
    let scan_from = start + needle.len();
    let end = lane[scan_from..]
        .find("<!-- vibe:static ")
        .map(|i| scan_from + i)
        .unwrap_or(lane.len());
    &lane[start..end]
}

// ----- Exhibit A ----------------------------------------------------------

/// Exhibit A — append-only composition (design §6: «composition without
/// recomposition»).
///
/// Two `simple` packages each define the SAME short labels `{#root}` and
/// `##THE-RULE`. Rendering A alone, then the world A+B, must leave A's block
/// byte-identical (B's arrival renamed nothing of A's), the A+B lane must carry
/// zero duplicate anchors (M-LOAD's first half, executable), and the tombstone
/// must list `root` with both heirs.
#[test]
fn exhibit_a_append_only_composition_is_byte_stable() {
    let ws = TempDir::new().unwrap();
    let alpha = write_simple_boot(
        ws.path(),
        "flow-alpha",
        "# Alpha {#root}\n\n##THE-RULE the alpha rule.\n",
    );
    let beta = write_simple_boot(
        ws.path(),
        "flow-beta",
        "# Beta {#root}\n\n##THE-RULE the beta rule.\n",
    );
    let origin_a = "org.demo/alpha";
    let origin_b = "org.demo/beta";

    // Lane 1: only package A is spliced. Lane 2: the world A+B.
    let lane_a = render_static(&boot(vec![entry_simple(&alpha, origin_a)]), ws.path())
        .unwrap()
        .unwrap();
    let lane_ab = render_static(
        &boot(vec![
            entry_simple(&alpha, origin_a),
            entry_simple(&beta, origin_b),
        ]),
        ws.path(),
    )
    .unwrap()
    .unwrap();

    // (i) A's block is byte-identical whether A stands alone or B follows it.
    // This is the append-independence claim at the lane level: a label's meaning
    // is decided when its package is authored, never when the world is assembled
    // (design §6). The qualified names depend only on A's own origin.
    let block_a_alone = block_for(&lane_a, origin_a);
    let block_a_in_world = block_for(&lane_ab, origin_a);
    assert_eq!(
        block_a_alone, block_a_in_world,
        "A's block must be byte-stable across append:\n\
         --- alone ---\n{block_a_alone}\n\
         --- in world ---\n{block_a_in_world}"
    );
    // Sanity: the block actually carries A's qualified labels (slug
    // `org-demo--alpha` — dots → `-`, the group/name `/` → `--`).
    assert!(block_a_in_world.contains("{#org-demo--alpha--root}"));
    assert!(block_a_in_world.contains("##org-demo--alpha--THE-RULE"));

    // (ii) The composed lane is collision-free by construction — M-LOAD's first
    // half (zero `duplicate-anchor` warnings over the compiled boot lane). The
    // two colliding `{#root}` and `##THE-RULE` are qualified apart.
    let dups = DocTree::parse(&lane_ab).duplicate_anchors().to_vec();
    assert!(
        dups.is_empty(),
        "the composed lane must carry zero duplicate anchors:\n{lane_ab}"
    );

    // (iii) The tombstone lists `root` with BOTH heirs (qualified form + origin),
    // directly under the header (START-placement, §6.1 layer 2). The same for
    // the second collided label, for completeness.
    assert!(
        lane_ab.contains(
            "root → org-demo--alpha--root (org.demo/alpha), \
             org-demo--beta--root (org.demo/beta)"
        ),
        "the tombstone must list `root` with both heirs:\n{lane_ab}"
    );
    assert!(
        lane_ab.contains(
            "THE-RULE → org-demo--alpha--THE-RULE (org.demo/alpha), \
             org-demo--beta--THE-RULE (org.demo/beta)"
        ),
        "the tombstone must list `THE-RULE` with both heirs:\n{lane_ab}"
    );
}

// ----- Exhibit B ----------------------------------------------------------

/// Exhibit B — the alias survives a cleaned carrier (design §4 / §6.1; M-LOAD's
/// second measurement: «a dynamic module resolves an alias whose carrier was
/// cleaned»).
///
/// Package B (a *different* package from A — the genuine dynamic case) declares
/// `#use spec://org.demo/alpha/rule#root as ARULE` and cites it via `@!ARULE`.
/// Compiling the lane rewrites `@!ARULE` to the full `@spec://…` address. The
/// alias binds to the **address**, not to compiled text (design §4
/// `##alias-binding`), so resolving that address goes to A's source under
/// `vibedeps/` through `FsSectionSource` — never to the lane. R1: the lane (the
/// carrier) is never written to disk (option a — the structural proof: the
/// resolver has no path to it), and dropping the in-memory lane string entirely
/// cannot change the resolution outcome.
#[test]
fn exhibit_b_alias_survives_a_cleaned_carrier() {
    let ws = TempDir::new().unwrap();

    // A — a source-of-truth doc under vibedeps, addressable as
    // `spec://org.demo/alpha/rule#root`. It is NOT a boot entry; it is reached
    // only through B's `#use`, exactly as the aliaser fixture's prelude is.
    write_spec_doc(
        ws.path(),
        "flow-alpha",
        "rule.md",
        "# The canonical rule {#root}\n\nRULE_BODY — the alpha source-of-truth.\n",
    );

    // B — a `normal` package whose contract aliases A's `#root` and cites it via
    // `@!ARULE`. B's own heading uses a distinct label (`beta-root`) so A's
    // pulled-in `#root` and B's heading qualify apart under B's origin when the
    // whole compiled closure is qualified.
    let beta_contract = write_spec_doc(
        ws.path(),
        "flow-beta",
        "boot/contract.md",
        "# Beta contract {#beta-root}\n\
         #use spec://org.demo/alpha/rule#root as ARULE\n\
         \n\
         Beta cites the rule through @!ARULE — its carrier may later be cleaned.\n",
    );

    // Compile the lane. The lane is the *carrier*; it is held in memory only and
    // is never written to disk in this test (R1, option a).
    let lane = render_static(
        &boot(vec![entry_normal(&beta_contract, "org.demo/beta")]),
        ws.path(),
    )
    .unwrap()
    .unwrap();

    // (1) `@!ARULE` was rewritten to the alias target's full address (fork C1,
    // design §4 `##alias-compiled-form`) — strip-proof and self-describing.
    assert!(
        lane.contains("@spec://org.demo/alpha/rule#root"),
        "the compiled lane must carry the alias's full address:\n{lane}"
    );
    assert!(
        !lane.contains("@!ARULE"),
        "the alias sigil must be consumed by the rewrite:\n{lane}"
    );

    // (2) The alias's address resolves through the FILE resolver against the
    // `vibedeps/` SOURCE tree — `section_text` reads
    // `vibedeps/flow-alpha/1.0.0/spec/rule.md`. The resolver never consults the
    // lane string: it is constructed from the workspace root and reads only the
    // source files. The alias binds to the address, never to compiled text.
    let src = FsSectionSource::new(FileResolver::new(ws.path(), "vibevm"));
    let addr = SpecAddress::parse("spec://org.demo/alpha/rule#root").unwrap();
    let resolved = src
        .section_text(&addr)
        .expect("the alias target resolves against the vibedeps source tree");
    assert!(
        resolved.contains("RULE_BODY"),
        "resolution must return A's source-of-truth text:\n{resolved}"
    );

    // (3) R1 — «the carrier was cleaned»: drop the lane (the carrier that held
    // the alias's target text, including A's pulled-in block) entirely, then
    // resolve the address again. Resolution is a pure function of the workspace
    // root and the source tree; the lane string is never an input, so cleaning
    // it cannot change the outcome. This is M-LOAD's second measurement, made
    // observable.
    drop(lane);
    let cleaned = src
        .section_text(&addr)
        .expect("resolution is independent of the cleaned carrier");
    assert!(
        cleaned.contains("RULE_BODY"),
        "the alias must still resolve after its carrier is cleaned:\n{cleaned}"
    );
}

// ----- Exhibit C ----------------------------------------------------------

/// Exhibit C — a missed short anchor answers with its qualified heirs (design
/// §6.1 layer 3, at the tree level; §5's «fail with candidates, never a silent
/// pick»).
///
/// Over the composed A+B lane's DocTree, the short anchor `root` (which the
/// qualify phase renamed away) is NOT directly findable, but its qualified
/// heirs are returned, sorted — the resolver's «never emptiness» posture.
#[test]
fn exhibit_c_a_missed_short_anchor_answers_with_qualified_heirs() {
    let ws = TempDir::new().unwrap();
    let alpha = write_simple_boot(
        ws.path(),
        "flow-alpha",
        "# Alpha {#root}\n\n##THE-RULE the alpha rule.\n",
    );
    let beta = write_simple_boot(
        ws.path(),
        "flow-beta",
        "# Beta {#root}\n\n##THE-RULE the beta rule.\n",
    );
    let lane_ab = render_static(
        &boot(vec![
            entry_simple(&alpha, "org.demo/alpha"),
            entry_simple(&beta, "org.demo/beta"),
        ]),
        ws.path(),
    )
    .unwrap()
    .unwrap();

    let tree = DocTree::parse(&lane_ab);

    // The short `root` is gone from the lane (it was qualified apart) — a direct
    // lookup misses. The rename converts a silent ambiguity into a loud miss;
    // the question is whether the miss is handled (§6.1).
    assert!(
        tree.find_by_anchor("root").is_none(),
        "the short `root` must not be a direct anchor in the composed lane"
    );

    // ...but its qualified heirs are returned, sorted (alpha before beta). The
    // resolver answers a missed `STATIC#<short>` with the qualified candidates
    // (§6.1 layer 3) — never a silent pick, never emptiness.
    let candidates = tree.qualified_candidates("root");
    assert_eq!(
        candidates,
        vec!["org-demo--alpha--root", "org-demo--beta--root"],
        "qualified_candidates must return both heirs, sorted"
    );
}
