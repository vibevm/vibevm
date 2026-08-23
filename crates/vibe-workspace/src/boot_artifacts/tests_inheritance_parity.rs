//! The INHERITANCE-PARITY twins of the bootgen floor (PROP-045
//! ##INHERITANCE-PARITY, owner clause 2026-08-22) — the lane machinery
//! above `render_static` (hoist / elision / de-substitution of covered
//! units) is format-blind, verified over XML-materialised snippets.
//!
//! Same genre as `render_static_projects_an_xml_snippet_deterministically`
//! and the unit-level `lane_dedup.rs` exhibits: the topology is built at the
//! unit level (a hand-assembled `EffectiveBoot` plus a zone table), the XML
//! snippet is minted through specdoc's public API at run time (`to_xml
//! (from_markdown(md))`, the live dialect form), and the verdicts/lanes of a
//! scenario carrying XML are compared against its pure-MD twin.
//!
//! Split from [`tests`] along the 600-line budget seam; the shared entry
//! builders stay in [`super::tests`] (`pub(super)` there for this reuse).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::Group;
use vibe_core::manifest::{LinkType, PackageFormat};

use crate::boot::hybrid::{UnitEdge, UnitId, UnitInput, resolve_zone};
use crate::boot::{BootBand, BootEntry};
use crate::install::desubstitute_covered_units;

use super::render_static;
use super::tests::{boot, coord, entry};

/// Which serialisation a twin snippet materialises as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    /// Authored Markdown — read verbatim.
    Md,
    /// `to_xml(from_markdown(md))` — spliced as its canonical projection.
    Xml,
}

impl Form {
    /// The file extension this form materialises as.
    fn ext(self) -> &'static str {
        match self {
            Form::Md => "md",
            Form::Xml => "xml",
        }
    }
}

/// The XML twin of a canonical Markdown snippet — minted through specdoc's
/// public API so it carries the dialect's LIVE form, never a frozen
/// snapshot. The twin precondition (the XML form projects back to the
/// byte-exact Markdown) is asserted here so a failure blames the fixture,
/// never the lane machinery under test.
fn xml_twin(md: &str) -> String {
    let doc = vibe_specdoc::from_markdown(md).expect("the twin markdown parses");
    let xml = vibe_specdoc::to_xml(&doc);
    let back =
        vibe_specdoc::to_markdown(&vibe_specdoc::from_xml(&xml).expect("the twin xml reads"));
    assert_eq!(
        back, md,
        "twin precondition: the markdown fixture must be canonical (to_markdown-stable)"
    );
    xml
}

/// Write a snippet under `vibedeps/<slot>/1.0.0/spec/boot/snippet.<ext>` in
/// the given form (an `Xml` write is the twin of `md`) and return the
/// workspace-relative path actually written.
fn write_parity_snippet(ws: &Path, slot: &str, md: &str, form: Form) -> String {
    let rel = format!("vibedeps/{slot}/1.0.0/spec/boot/snippet.{}", form.ext());
    let p = ws.join(&rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let text = match form {
        Form::Md => md.to_string(),
        Form::Xml => xml_twin(md),
    };
    fs::write(&p, text).unwrap();
    rel
}

// ---- the zone table, built at the unit level (the `lane_dedup.rs` genre) --

/// The canonical test group — every unit identity shares it.
fn g() -> Group {
    Group::parse("org.demo").unwrap()
}

/// A unit identity `<group, name>`.
fn uid(name: &str) -> UnitId {
    (g(), name.to_string())
}

/// The `<group>/<name>` pkgref — both a unit's `origin` and its entry's
/// `origin` use exactly this form, which is what
/// [`desubstitute_covered_units`] matches on.
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
/// for a contentless umbrella) and its declared edges.
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
            fragments: Vec::new(),
            origin: pkgref(name),
            when: None,
            edges,
            format: PackageFormat::Simple,
        },
    );
}

/// A `unit_substituted` static entry — its path points at a compiled
/// per-unit `STATIC.md`, the form `node_dependency_boot` produces for a
/// package reached over a `static-transitive` edge that statically links a
/// child.
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

// ---- (а) hoist/elision: a static-transitive zone over an XML snippet -----

/// The package reached over the `static-transitive` edge (R, statically
/// linking C) carries its snippet in dialect XML. The lane machinery is
/// format-blind end to end: the zone is the same (`resolve_zone` reaches C
/// over the static edge), `desubstitute_covered_units` rolls R's
/// unit-STATIC substitution back to its OWN `.xml` snippet once C is
/// present individually, and the rendered STATIC is byte-deterministic
/// across two runs with both origins' anchors qualified.
#[test]
fn a_static_transitive_zone_over_an_xml_snippet_dedups_and_renders_deterministically() {
    const R_MD: &str = "# R rules {#root}\n\n@fact:RFACT the r rule @status:spec/work\n\n";
    const C_MD: &str = "# C rules {#root}\n\n@fact:CFACT the c rule @status:impl/done\n\n";

    let ws = TempDir::new().unwrap();
    let r_xml = write_parity_snippet(ws.path(), "org.demo.r", R_MD, Form::Xml);
    let c_md = write_parity_snippet(ws.path(), "org.demo.c", C_MD, Form::Md);

    // The zone: R statically links C — the shape a `static-transitive`
    // consumer compiles into R's per-unit STATIC.
    let mut table = HashMap::new();
    insert(
        &mut table,
        "r",
        Some(&r_xml),
        vec![edge("c", LinkType::Static)],
    );
    insert(&mut table, "c", Some(&c_md), vec![]);
    let zone = resolve_zone(&uid("r"), &table);
    assert!(
        zone.static_members.contains(&uid("c")),
        "C must be a static member of R's zone: {zone:?}"
    );

    // The node's lane: R substituted up to its unit-STATIC, C present
    // individually (the closure walk's pull) — the covered-zone shape.
    let mut eff = boot(vec![
        entry_sub(
            "vibedeps/org.demo.r/1.0.0/spec/boot/STATIC.md",
            &pkgref("r"),
        ),
        entry(&c_md, LinkType::Static, &pkgref("c")),
    ]);
    desubstitute_covered_units(&mut eff, &table);

    // The verdict is form-blind: R rolls back to its OWN snippet — the
    // `.xml` that exists — never to a `.md` that does not.
    let r_entry = &eff.entries[0];
    assert!(!r_entry.elided, "R ships a snippet — rollback, not elision");
    assert!(!r_entry.unit_substituted, "R's substitution rolled back");
    assert_eq!(r_entry.path, r_xml, "rolled back to the materialised .xml");

    // Two renders are byte-deterministic, and the lane carries both origins'
    // anchors QUALIFIED (the XML snippet splices as its canonical
    // projection, qualified under R's origin exactly as an MD twin would).
    let first = render_static(&eff, ws.path(), &coord()).unwrap().unwrap();
    let second = render_static(&eff, ws.path(), &coord()).unwrap().unwrap();
    assert_eq!(first, second, "two renders must be byte-equal");
    assert!(first.contains("{#org-demo--r--root}"), "{first}");
    assert!(first.contains("@fact:org-demo--r--RFACT"), "{first}");
    assert!(first.contains("{#org-demo--c--root}"), "{first}");
    assert!(first.contains("@fact:org-demo--c--CFACT"), "{first}");
    assert!(first.contains("the c rule"), "{first}");
    assert!(!first.contains("<spec"), "no raw XML in the lane: {first}");
    assert!(
        first.starts_with("<!-- spec/boot/STATIC.md"),
        "the artifact stays STATIC.md:\n{first}"
    );
}

/// The hoist half: a shared package's single copy at the hoist point is the
/// `"<g>/<n> [shared by …]"` entry form — over an XML-materialised snippet
/// it splices once (the canonical projection), deterministically, with the
/// labels qualified under the package's own origin (the shared-by suffix is
/// not part of the slug).
#[test]
fn a_hoisted_xml_snippet_splices_once_qualified_deterministically() {
    const R_MD: &str = "# R rules {#root}\n\n@fact:RFACT the shared rule @status:spec/work\n\n";
    let ws = TempDir::new().unwrap();
    let r_xml = write_parity_snippet(ws.path(), "org.demo.r", R_MD, Form::Xml);

    let eff = boot(vec![entry(
        &r_xml,
        LinkType::Static,
        "org.demo/r [shared by org.demo/x, org.demo/y]",
    )]);
    let first = render_static(&eff, ws.path(), &coord()).unwrap().unwrap();
    let second = render_static(&eff, ws.path(), &coord()).unwrap().unwrap();
    assert_eq!(first, second, "two renders must be byte-equal");
    assert_eq!(first.matches("the shared rule").count(), 1, "{first}");
    assert!(first.contains("{#org-demo--r--root}"), "{first}");
    assert!(first.contains("@fact:org-demo--r--RFACT"), "{first}");
    assert!(!first.contains("<spec"), "no raw XML in the lane: {first}");
}

// ---- (б) de-substitution over a MIXED set vs the pure-MD twin ------------

/// The T1 topology (contentless aggregator statically linking two
/// snippet-bearing members) run twice: once pure-MD, once MIXED (M2's
/// snippet XML, M1's MD). `desubstitute_covered_units` decides identically —
/// the aggregator elides, both members keep their individual entries — and
/// the rendered lanes are byte-identical modulo the one honest difference:
/// M2's provenance names the `.xml` file that exists on disk.
#[test]
fn desubstitution_over_a_mixed_md_xml_lane_matches_the_pure_md_twin() {
    const M1_MD: &str = "# M1 rules {#root}\n\n@fact:M1FACT the m1 rule @status:impl/done\n\n";
    const M2_MD: &str = "# M2 rules {#root}\n\n@fact:M2FACT the m2 rule @status:spec/work\n\n";

    let build = |m2_form: Form| {
        let ws = TempDir::new().unwrap();
        let m1 = write_parity_snippet(ws.path(), "org.demo.m1", M1_MD, Form::Md);
        let m2 = write_parity_snippet(ws.path(), "org.demo.m2", M2_MD, m2_form);
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
            entry_sub(
                "vibedeps/org.demo.agg/1.0.0/spec/boot/STATIC.md",
                &pkgref("agg"),
            ),
            entry(&m1, LinkType::Static, &pkgref("m1")),
            entry(&m2, LinkType::Static, &pkgref("m2")),
        ]);
        desubstitute_covered_units(&mut eff, &table);
        (ws, eff)
    };

    let (pure_ws, pure) = build(Form::Md);
    let (mixed_ws, mixed) = build(Form::Xml);

    // The verdicts are identical entry by entry — elision, substitution, and
    // the path modulo the one form difference (M2's snippet extension).
    assert_eq!(pure.entries.len(), mixed.entries.len());
    for (p, m) in pure.entries.iter().zip(&mixed.entries) {
        assert_eq!(p.origin, m.origin);
        assert_eq!(p.elided, m.elided, "{}: {p:?} vs {m:?}", p.origin);
        assert_eq!(
            p.unit_substituted, m.unit_substituted,
            "{}: {p:?} vs {m:?}",
            p.origin
        );
        assert_eq!(
            p.path.replace(".xml", ".md"),
            m.path.replace(".xml", ".md"),
            "{}: {p:?} vs {m:?}",
            p.origin
        );
    }
    // The shape itself: the aggregator elides (its zone is covered
    // member-by-member), both members keep their own entries.
    assert!(pure.entries[0].elided, "agg elides in the pure-md twin");
    assert!(mixed.entries[0].elided, "agg elides in the mixed lane too");
    assert!(!pure.entries[1].elided && !pure.entries[2].elided);

    // And the rendered lanes agree byte-for-byte once M2's honest `.xml`
    // path is mapped back to its MD twin's spelling.
    let pure_lane = render_static(&pure, pure_ws.path(), &coord())
        .unwrap()
        .unwrap();
    let mixed_lane = render_static(&mixed, mixed_ws.path(), &coord())
        .unwrap()
        .unwrap();
    let m2_xml = mixed.entries[2].path.clone();
    let m2_md = pure.entries[2].path.clone();
    assert_eq!(
        mixed_lane.replace(&m2_xml, &m2_md),
        pure_lane,
        "the mixed lane must equal the pure-md lane modulo M2's materialised path"
    );
    // Each member's text exactly once in both lanes.
    assert_eq!(pure_lane.matches("the m1 rule").count(), 1);
    assert_eq!(pure_lane.matches("the m2 rule").count(), 1);
    assert!(
        pure_lane.contains("@fact:org-demo--m1--M1FACT"),
        "{pure_lane}"
    );
    assert!(
        pure_lane.contains("@fact:org-demo--m2--M2FACT"),
        "{pure_lane}"
    );
}
