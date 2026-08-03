//! E4-W1-LANE-DEDUP — the once-each lane (B-006, slice W-A), executable.
//!
//! An integration test of the **de-substitution of a covered zone**: when a
//! package's compiled per-unit `STATIC.md` (its `unit_substituted` entry) is
//! fully covered by the same lane carrying every boot-bearing member of its
//! zone individually, the lane-dedup pass either rolls the entry back to the
//! package's own snippet (it keeps its text, once) or elides a contentless
//! umbrella to a provenance stub. The lane then emits each package's text
//! exactly **once** — never at the cost of losing coverage (a member missing
//! from the lane keeps the substitution in place).
//!
//! The topologies are built **at the unit level** — a hand-assembled
//! `EffectiveBoot` plus a `HashMap<UnitId, UnitInput>` zone table — and the
//! property under test is the verdict of `desubstitute_covered_units` followed
//! by the `render_static` lane. No full `vibe install` is needed (У2): the
//! function is a pure pass over the composition and the table, so the verdicts
//! are observable directly. The on-disk shapes (a `simple`
//! `vibedeps/<slot>/1.0.0/boot.md`) mirror the in-crate unit-test helpers and
//! `dynamic_lane.rs`'s fixtures (R3), and the lane types are reached through
//! the crate's public surface — `boot::*`, `boot::hybrid::*`, `render_static`,
//! and the `desubstitute_covered_units` re-export (R2: the only widening is
//! that one re-export; see Decisions in the worker report).
//!
//! Seven exhibits (T1–T7), one per topology the contract must hold for:
//!
//! - **T1** — today's shape: a contentless aggregator statically linking two
//!   snippet-bearing members. The aggregator elides; each member renders once.
//! - **T2** — the coverage guard: a zone member resolves dynamic on the node
//!   (absent from the static lane). The aggregator keeps its substitution.
//! - **T3** — mixed consumers: two parents statically link one shared
//!   umbrella; identity-dedup keeps one entry, which elides; a member renders
//!   once. The verdict is declaration-order invariant (both permutations).
//! - **T4** — an aggregator that ships its own snippet: it de-substitutes to
//!   the snippet (its own text once), neither elided nor substituted.
//! - **T5** — append safety: an unrelated package X changes no T1 verdict.
//! - **T6** — nested umbrellas: two stacked contentless umbrellas both elide
//!   in a single pass (a contentless umbrella is never a boot-bearing member
//!   of its parent's zone — У1).
//! - **T7** — the presence matcher: a member present as a hoisted shared-by
//!   entry counts as present, and a different package whose name is a prefix
//!   extension does not (the `[` guard keeps the pkgref match exact).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::Group;
use vibe_core::manifest::{LinkType, PackageFormat};
use vibe_workspace::boot::hybrid::{UnitEdge, UnitId, UnitInput};
use vibe_workspace::boot::{BootBand, BootEntry, EffectiveBoot};
use vibe_workspace::boot_artifacts::render_static;
use vibe_workspace::install::desubstitute_covered_units;

// ----- helpers (the topology, built at the unit level) ---------------------

/// The canonical test group — every unit identity shares it.
fn g() -> Group {
    Group::parse("org.demo").unwrap()
}

/// A unit identity `<group, name>`.
fn uid(name: &str) -> UnitId {
    (g(), name.to_string())
}

/// The `<group>/<name>` pkgref — both a unit's `origin` and its entry's
/// `origin` use exactly this form, which is what `desubstitute_covered_units`
/// matches on.
fn pkgref(name: &str) -> String {
    format!("org.demo/{name}")
}

/// One static edge to `target`.
fn edge(target: &str, link: LinkType) -> UnitEdge {
    UnitEdge {
        target: uid(target),
        link,
    }
}

/// Insert a unit into the zone table: its own boot snippet path (or `None`
/// for a contentless umbrella) and its declared edges. `origin` is the pkgref,
/// matching how `build_unit_table` seeds it.
fn insert(
    table: &mut HashMap<UnitId, UnitInput>,
    name: &str,
    boot: Option<&str>,
    edges: Vec<UnitEdge>,
) {
    table.insert(
        uid(name),
        UnitInput {
            own_boot_path: boot.map(str::to_string),
            origin: pkgref(name),
            when: None,
            edges,
            format: PackageFormat::Simple,
        },
    );
}

/// A `unit_substituted` static entry — its path points at a compiled per-unit
/// `STATIC.md` (the form `node_dependency_boot` produces for a unit that
/// statically links a child).
fn entry_sub(static_md: &str, origin: &str) -> BootEntry {
    BootEntry {
        path: static_md.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: origin.to_string(),
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: true,
        elided: false,
    }
}

/// An individual `static` entry — the package's own snippet text, present in
/// the lane under its own pkgref.
fn entry_static(snippet: &str, origin: &str) -> BootEntry {
    BootEntry {
        path: snippet.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: origin.to_string(),
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: false,
        elided: false,
    }
}

/// A `dynamic` entry — read by reference (INDEX.md), absent from the static
/// lane. Models a member that resolves dynamic on the node (a `when` gate or a
/// consumer `dynamic` link) and so does NOT count as "present" for the guard.
fn entry_dyn(snippet: &str, origin: &str) -> BootEntry {
    BootEntry {
        path: snippet.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Dynamic,
        when: None,
        origin: origin.to_string(),
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: false,
        elided: false,
    }
}

/// A hoisted single-copy entry (PROP-038 §2.4) — origin carries the
/// `"<g>/<n> [shared by …]"` shared-by hint. It is the member's text at the
/// hoist point, so it counts as "present" (its `use_ref` is `false`).
fn hoisted_entry(snippet: &str, name: &str, shared: &[&str]) -> BootEntry {
    BootEntry {
        path: snippet.to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: format!("{} [shared by {}]", pkgref(name), shared.join(", ")),
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: false,
        elided: false,
    }
}

/// Wrap entries into an [`EffectiveBoot`].
fn boot(entries: Vec<BootEntry>) -> EffectiveBoot {
    EffectiveBoot { entries }
}

/// Write a `simple` boot file under `vibedeps/<slot>/1.0.0/boot.md` and return
/// its workspace-relative path (the `simple`-format on-disk shape, R3).
fn write_snippet(ws: &Path, slot: &str, body: &str) -> String {
    let p = ws.join(format!("vibedeps/{slot}/1.0.0/boot.md"));
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
    format!("vibedeps/{slot}/1.0.0/boot.md")
}

/// The entry whose origin pkgref is `name`.
fn find<'a>(eff: &'a EffectiveBoot, name: &str) -> &'a BootEntry {
    eff.entries
        .iter()
        .find(|e| e.origin == pkgref(name))
        .unwrap_or_else(|| panic!("no entry for `{name}`"))
}

/// The provenance marker comment HTML block for `origin` — `<-- vibe:static
/// <origin> — … -->`. Used to inspect an elided stub's text directly.
fn marker_line<'a>(lane: &'a str, origin: &str) -> &'a str {
    let needle = format!("<!-- vibe:static {origin} — ");
    let start = lane
        .find(&needle)
        .unwrap_or_else(|| panic!("no provenance marker for `{origin}`:\n{lane}"));
    let end = start + lane[start..].find("-->").unwrap() + 3;
    lane[start..end].trim()
}

// ----- T1 — today's shape: contentless aggregator --------------------------

/// T1 — today's vibevm shape (B-006): root --static-transitive--> AGG (no
/// snippet) --static--> M1, M2 (snippets). The aggregator's compiled
/// unit-STATIC is a covered duplicate — every boot-bearing member of its zone
/// (M1, M2) is present individually — so AGG elides to a stub and each member
/// renders exactly once.
#[test]
fn t1_contentless_aggregator_elides_members_render_once() {
    let ws = TempDir::new().unwrap();
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let mut table = HashMap::new();
    insert(
        &mut table,
        "agg",
        None,
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    insert(&mut table, "m2", Some(&m2), vec![]);

    let mut eff = boot(vec![
        entry_sub("vibedeps/agg/1.0.0/spec/boot/STATIC.md", &pkgref("agg")),
        entry_static(&m1, &pkgref("m1")),
        entry_static(&m2, &pkgref("m2")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    // Verdict: AGG elided (and still flagged substituted for the stub's
    // provenance); M1/M2 untouched as individual static entries.
    let agg = find(&eff, "agg");
    assert!(
        agg.elided,
        "AGG elides — its zone is covered member-by-member"
    );
    assert!(
        agg.unit_substituted,
        "an elided entry keeps its substituted provenance"
    );
    assert!(!find(&eff, "m1").elided);
    assert!(!find(&eff, "m2").elided);

    // Render: each member's text exactly once; AGG is a stub with no `#use`.
    let lane = render_static(&eff, ws.path()).unwrap().unwrap();
    assert_eq!(lane.matches("M1BODY").count(), 1, "M1 text once:\n{lane}");
    assert_eq!(lane.matches("M2BODY").count(), 1, "M2 text once:\n{lane}");
    let stub = marker_line(&lane, &pkgref("agg"));
    assert!(
        stub.contains("zone elided") && stub.contains("B-006"),
        "elided stub:\n{stub}"
    );
    assert!(
        !stub.contains("#use"),
        "an elided stub carries no #use:\n{stub}"
    );
}

// ----- T2 — the coverage guard --------------------------------------------

/// T2 — coverage is never lost. AGG --static--> M1, M2; M1 is present
/// individually, but M2 resolves dynamic on the node (modelled as a `dynamic`
/// entry — it is absent from the static lane). AGG's zone still needs M2, so
/// the unit-STATIC substitution MUST stay (its compiled zone carries M2).
#[test]
fn t2_coverage_guard_retains_substitution_when_member_absent() {
    let ws = TempDir::new().unwrap();
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let mut table = HashMap::new();
    insert(
        &mut table,
        "agg",
        None,
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    // M2 is a static-zone member of AGG (no `when`), yet resolves dynamic on
    // the node — the consumer/link divergence that makes the guard necessary.
    insert(&mut table, "m2", Some(&m2), vec![]);

    let mut eff = boot(vec![
        entry_sub("vibedeps/agg/1.0.0/spec/boot/STATIC.md", &pkgref("agg")),
        entry_static(&m1, &pkgref("m1")),
        entry_dyn(&m2, &pkgref("m2")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    let agg = find(&eff, "agg");
    assert!(
        !agg.elided,
        "AGG must NOT elide — M2 is absent as static (coverage)"
    );
    assert!(
        agg.unit_substituted,
        "AGG retains the unit-STATIC substitution — M2's text is carried by it"
    );
    // No render here: a retained-substituted entry points at a unit-STATIC the
    // fixture does not materialise. The verdict (not elided, still substituted)
    // IS the coverage guard.
}

// ----- T3 — mixed consumers, identity-dedup, order-invariant ---------------

/// T3 — mixed consumers. root --static-transitive--> A --static--> GP;
/// root --static--> B --static--> GP; GP (no snippet) --static--> M1, M2.
/// The closure walk reaches GP through two parents but keeps ONE entry
/// (identity-dedup); GP elides; M1 renders once. The verdict does not depend
/// on declaration order — both permutations are run.
#[test]
fn t3_mixed_consumers_identity_dedup_order_invariant() {
    let ws = TempDir::new().unwrap();
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let mut table = HashMap::new();
    insert(&mut table, "a", None, vec![edge("gp", LinkType::Static)]);
    insert(&mut table, "b", None, vec![edge("gp", LinkType::Static)]);
    insert(
        &mut table,
        "gp",
        None,
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    insert(&mut table, "m2", Some(&m2), vec![]);

    // Build the composition in a given declaration order: A/B/GP are
    // substituted umbrellas, M1/M2 are individual static entries.
    let build = |names: &[&str]| {
        boot(
            names
                .iter()
                .map(|&n| {
                    if n == "m1" {
                        entry_static(&m1, &pkgref("m1"))
                    } else if n == "m2" {
                        entry_static(&m2, &pkgref("m2"))
                    } else {
                        entry_sub(
                            &format!("vibedeps/{n}/1.0.0/spec/boot/STATIC.md"),
                            &pkgref(n),
                        )
                    }
                })
                .collect(),
        )
    };

    let p1: &[&str] = &["a", "b", "gp", "m1", "m2"];
    let p2: &[&str] = &["b", "a", "gp", "m2", "m1"];
    for names in [p1, p2] {
        let mut eff = build(names);
        // GP appears once — the closure walk dedups it despite two parents.
        assert_eq!(
            eff.entries
                .iter()
                .filter(|e| e.origin == pkgref("gp"))
                .count(),
            1,
            "GP identity-deduped to one entry (order {names:?})"
        );
        desubstitute_covered_units(&mut eff, &table);
        assert!(find(&eff, "a").elided, "A elides (order {names:?})");
        assert!(find(&eff, "b").elided, "B elides (order {names:?})");
        assert!(find(&eff, "gp").elided, "GP elides (order {names:?})");
        assert!(!find(&eff, "m1").elided && !find(&eff, "m2").elided);
        let lane = render_static(&eff, ws.path()).unwrap().unwrap();
        assert_eq!(
            lane.matches("M1BODY").count(),
            1,
            "M1 once (order {names:?}):\n{lane}"
        );
    }
}

// ----- T4 — aggregator WITH its own snippet --------------------------------

/// T4 — an aggregator that ships its own snippet. R (snippet!) --static-->
/// M1, M2 (both present individually). R's entry de-substitutes: its path
/// rolls back to R's own snippet (its text enters the lane once),
/// `unit_substituted == false`, `elided == false`, and R's unit-STATIC path
/// never reaches the lane.
#[test]
fn t4_aggregator_with_snippet_de_substitutes_to_snippet() {
    let ws = TempDir::new().unwrap();
    let r = write_snippet(ws.path(), "r", "RBODY marker.\n");
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let mut table = HashMap::new();
    insert(
        &mut table,
        "r",
        Some(&r),
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    insert(&mut table, "m2", Some(&m2), vec![]);

    let mut eff = boot(vec![
        entry_sub("vibedeps/r/1.0.0/spec/boot/STATIC.md", &pkgref("r")),
        entry_static(&m1, &pkgref("m1")),
        entry_static(&m2, &pkgref("m2")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    let r_entry = find(&eff, "r");
    assert!(
        !r_entry.elided,
        "R is NOT elided — it ships its own snippet"
    );
    assert!(!r_entry.unit_substituted, "R de-substituted");
    assert_eq!(r_entry.path, r, "R path rolled back to its own snippet");

    let lane = render_static(&eff, ws.path()).unwrap().unwrap();
    assert_eq!(
        lane.matches("RBODY").count(),
        1,
        "R's own text once:\n{lane}"
    );
    assert_eq!(lane.matches("M1BODY").count(), 1);
    assert!(
        !lane.contains("vibedeps/r/1.0.0/spec/boot/STATIC.md"),
        "R's unit-STATIC path not in the lane:\n{lane}"
    );
}

// ----- T5 — append safety --------------------------------------------------

/// T5 — append safety. T1's topology plus an UNRELATED package X (its own
/// snippet, statically standalone). The pass is local to each unit's zone, so
/// X's arrival changes nothing about AGG/M1/M2.
#[test]
fn t5_unrelated_append_leaves_t1_verdicts_unchanged() {
    let ws = TempDir::new().unwrap();
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let x = write_snippet(ws.path(), "x", "XBODY marker.\n");
    let mut table = HashMap::new();
    insert(
        &mut table,
        "agg",
        None,
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    insert(&mut table, "m2", Some(&m2), vec![]);
    insert(&mut table, "x", Some(&x), vec![]);

    let mut eff = boot(vec![
        entry_sub("vibedeps/agg/1.0.0/spec/boot/STATIC.md", &pkgref("agg")),
        entry_static(&m1, &pkgref("m1")),
        entry_static(&m2, &pkgref("m2")),
        entry_static(&x, &pkgref("x")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    assert!(find(&eff, "agg").elided);
    assert!(!find(&eff, "m1").elided);
    assert!(!find(&eff, "m2").elided);
    assert!(!find(&eff, "x").elided);
    let lane = render_static(&eff, ws.path()).unwrap().unwrap();
    assert_eq!(lane.matches("M1BODY").count(), 1);
    assert_eq!(lane.matches("XBODY").count(), 1, "X renders once:\n{lane}");
}

// ----- T6 — nested umbrellas, one pass (У1) --------------------------------

/// T6 — nested umbrellas. P (no snippet) --static--> GP (no snippet)
/// --static--> M1, M2. BOTH umbrellas elide in a SINGLE pass: GP is
/// contentless, so it is NOT a boot-bearing member of P's zone, and therefore
/// never gates P's elision. P is decided against the original snapshot (where
/// GP is still substituted) and still elides — proving the single pass (У1).
#[test]
fn t6_nested_umbrellas_collapse_in_one_pass() {
    let ws = TempDir::new().unwrap();
    let m1 = write_snippet(ws.path(), "m1", "M1BODY marker.\n");
    let m2 = write_snippet(ws.path(), "m2", "M2BODY marker.\n");
    let mut table = HashMap::new();
    insert(&mut table, "p", None, vec![edge("gp", LinkType::Static)]);
    insert(
        &mut table,
        "gp",
        None,
        vec![edge("m1", LinkType::Static), edge("m2", LinkType::Static)],
    );
    insert(&mut table, "m1", Some(&m1), vec![]);
    insert(&mut table, "m2", Some(&m2), vec![]);

    // P is listed BEFORE GP: when P is decided, GP is still substituted in the
    // snapshot — yet P elides, because GP (contentless) is not a boot-bearing
    // member of P's zone. That is the single-pass property.
    let mut eff = boot(vec![
        entry_sub("vibedeps/p/1.0.0/spec/boot/STATIC.md", &pkgref("p")),
        entry_sub("vibedeps/gp/1.0.0/spec/boot/STATIC.md", &pkgref("gp")),
        entry_static(&m1, &pkgref("m1")),
        entry_static(&m2, &pkgref("m2")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    assert!(find(&eff, "p").elided, "P elides in one pass");
    assert!(find(&eff, "gp").elided, "GP elides in one pass");
    let lane = render_static(&eff, ws.path()).unwrap().unwrap();
    assert_eq!(lane.matches("M1BODY").count(), 1);
    assert_eq!(lane.matches("M2BODY").count(), 1);
}

// ----- T7 — the presence matcher (hoisted form + prefix guard) -------------

/// T7 — the `present` predicate. (i) A member present ONLY as a hoisted
/// shared-by entry (`"n [shared by …]"`) counts as present. (ii) A different
/// package whose name is a prefix extension does NOT match — the `[` guard
/// keeps the pkgref match exact. Two probes:
///   • UMB-A statically links A; A is present only hoisted → present(A) true
///     → UMB-A elides (positive hoisted match).
///   • UMB-M statically links M (snippet); only a hoisted entry for "member"
///     (an M prefix-extension) is present, M itself absent → present(M) false
///     → UMB-M retains (the `[` guard is not fooled by "member").
#[test]
fn t7_presence_matcher_hoisted_form_and_prefix_guard() {
    let ws = TempDir::new().unwrap();
    let a = write_snippet(ws.path(), "a", "ABODY marker.\n");
    let m = write_snippet(ws.path(), "m", "MBODY marker.\n");
    let mem = write_snippet(ws.path(), "member", "MEMBERBODY marker.\n");
    let mut table = HashMap::new();
    insert(&mut table, "umb-a", None, vec![edge("a", LinkType::Static)]);
    insert(&mut table, "umb-m", None, vec![edge("m", LinkType::Static)]);
    insert(&mut table, "a", Some(&a), vec![]);
    insert(&mut table, "m", Some(&m), vec![]);
    insert(&mut table, "member", Some(&mem), vec![]);

    // Probe 1 — a hoisted shared-by entry IS present.
    let mut eff_a = boot(vec![
        entry_sub("vibedeps/umb-a/1.0.0/spec/boot/STATIC.md", &pkgref("umb-a")),
        hoisted_entry(&a, "a", &["org.demo/x"]),
    ]);
    desubstitute_covered_units(&mut eff_a, &table);
    assert!(
        find(&eff_a, "umb-a").elided,
        "A's hoisted form counts as present → UMB-A elides"
    );

    // Probe 2 — a prefix-extension name does NOT match.
    let mut eff_m = boot(vec![
        entry_sub("vibedeps/umb-m/1.0.0/spec/boot/STATIC.md", &pkgref("umb-m")),
        hoisted_entry(&mem, "member", &["org.demo/x"]),
    ]);
    desubstitute_covered_units(&mut eff_m, &table);
    let umb_m = find(&eff_m, "umb-m");
    assert!(
        !umb_m.elided,
        "M is absent; 'member' must not match M (the `[` guard) → UMB-M retains"
    );
    assert!(umb_m.unit_substituted);
}
