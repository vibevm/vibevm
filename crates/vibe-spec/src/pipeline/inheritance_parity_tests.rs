//! The INHERITANCE-PARITY twin family (PROP-045 ##INHERITANCE-PARITY, owner
//! clause 2026-08-22): the C++-inheritance machinery is format-blind —
//! **verified, not assumed**.
//!
//! Every B-011 mechanism — `#use … as X` alias binding, `@!X` references,
//! the qualified splice (rename-on-splice with every reference kept valid),
//! and fact-grain inheritance — runs over a dependency authored in Markdown
//! and over the SAME dependency serialised as dialect XML, and the two
//! compiled closures are compared byte-for-byte: the lane AND the per-node
//! rename map (the tombstone's source). Mixed trees (one dep MD, one XML)
//! ride the same pins. The law the family rests on is the loader's
//! projection: a `.xml` source never reaches the compiler raw —
//! `load_spec_text` delivers the canonical Markdown projection — so a
//! canonical MD twin and its XML serialisation compile identical text, or
//! the machinery is NOT format-blind and the twin fails.
//!
//! The XML twin is minted at run time through specdoc's public API
//! (`to_xml(from_markdown(md))`), so the family carries whatever dialect
//! form is live (named sections; the current fact spelling) instead of
//! freezing a snapshot in a fixture constant — a future form change re-runs
//! these twins for free.
//!
//! Split from [`tests`] along the same seam as [`fold_tests`] (the 600-line
//! budget); the shared `MD_DEP_TWIN` fixture is `pub(super)` there for
//! exactly this reuse.

use super::tests::MD_DEP_TWIN;
use crate::{DocTree, RenameEntry, SpecAddress};

/// Which serialisation a twin dependency ships in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    /// Authored Markdown — read verbatim by the loader.
    Md,
    /// `to_xml(from_markdown(md))` — read through the canonical projection.
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

/// The XML twin of a canonical Markdown text — minted through specdoc's
/// public API so it carries the dialect's LIVE form (named sections, the
/// current fact spelling), never a frozen snapshot. The twin precondition —
/// the XML form projects back to the byte-exact Markdown — is asserted here
/// so a failure blames the fixture (non-canonical Markdown), never the
/// compiler under test.
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

/// A fixture rel under the legacy specs root, `/`-separated (PROP-052: the
/// layout names live once in `crates/vibe-core/src/layout.rs`; this test
/// scaffold routes through the crate's sanctioned pair).
fn under_specs(rel: &str) -> String {
    format!("{}/{}", crate::resolver::LEGACY_SPECS_ROOT, rel)
}

/// A fixture rel inside a materialised dependency slot:
/// `vibedeps/<identity>/<version>/spec/<rel>` (same sanctioned pair).
fn in_slot(identity: &str, version: &str, rel: &str) -> String {
    format!(
        "{}/{identity}/{version}/{}/{rel}",
        crate::resolver::LEGACY_VIBEDEPS_ROOT,
        crate::resolver::LEGACY_SPECS_ROOT
    )
}

/// A workspace with the authored entry at the boot lane's `00-entry.md`
/// plus each dependency at its extension-less workspace-relative path in
/// the given form (the helper appends `.md` / `.xml`). An `Xml`
/// dependency is written as the twin of its MD text, so two lanes built
/// over the same `(rel, md)` list differ ONLY in which serialisation
/// each dependency ships in.
fn parity_ws(entry: &str, deps: &[(&str, &str, Form)]) -> tempfile::TempDir {
    let ws = tempfile::TempDir::new().unwrap();
    let boot = crate::resolver::specs_root_under(ws.path()).join("boot");
    std::fs::create_dir_all(&boot).unwrap();
    std::fs::write(boot.join("00-entry.md"), entry).unwrap();
    for (rel, md, form) in deps {
        let path = ws.path().join(format!("{rel}.{}", form.ext()));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = match form {
            Form::Md => (*md).to_string(),
            Form::Xml => xml_twin(md),
        };
        std::fs::write(&path, text).unwrap();
    }
    ws
}

/// Compile the entry seed's qualified closure — the lane AND the per-node
/// rename map (what the STATIC tombstone renders from) — so a twin compares
/// BOTH halves of the compiled artifact, not just the body.
fn compile_entry_qualified(ws: &tempfile::TempDir) -> (String, Vec<(String, RenameEntry)>) {
    use crate::{FileResolver, FsSectionSource, SelfCoordinate};
    let resolver = FileResolver::new(
        ws.path(),
        SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into()),
    );
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/boot/00-entry#root").unwrap();
    super::compile_static_qualified(&seed, &FsSectionSource::new(resolver)).unwrap()
}

/// No directive line survives either lane (asserted line-wise: the compiled
/// text may legitimately DOCUMENT a directive inside a comment).
fn assert_no_directive_lines(lane: &str) {
    for kw in ["#use ", "#embed ", "#source "] {
        assert!(
            !lane.lines().any(|l| l.trim_start().starts_with(kw)),
            "a `{kw}` line survived the compile:\n{lane}"
        );
    }
}

// ---- Step 1 — `#use … as X` aliasing + `@!X` references (B-011 §7.4) ------

/// The entry binds `#use … as dep` and references `@!dep` in prose; the
/// dependency ships as MD vs as its XML twin. The compiled closures — lane
/// and rename map — are byte-identical, and `@!dep` is rewritten to the full
/// address identically in both lanes.
#[test]
fn alias_binding_and_at_bang_reference_compile_identically_over_an_xml_dependency() {
    const ENTRY: &str = concat!(
        "# Entry {#root}\n\n",
        "#use spec://org.vibevm.core/vibevm/common/DEP#laws as dep\n\n",
        "Sees @!dep here.\n"
    );
    let md_ws = parity_ws(
        ENTRY,
        &[(under_specs("common/DEP").as_str(), MD_DEP_TWIN, Form::Md)],
    );
    let xml_ws = parity_ws(
        ENTRY,
        &[(under_specs("common/DEP").as_str(), MD_DEP_TWIN, Form::Xml)],
    );
    let (md_lane, md_renames) = compile_entry_qualified(&md_ws);
    let (xml_lane, xml_renames) = compile_entry_qualified(&xml_ws);
    assert_eq!(
        md_lane, xml_lane,
        "md lane:\n{md_lane}\nxml lane:\n{xml_lane}"
    );
    assert_eq!(
        md_renames, xml_renames,
        "the rename maps must be identical:\nmd: {md_renames:?}\nxml: {xml_renames:?}"
    );
    // `@!dep` became the alias target's full address — identically in both.
    assert!(
        md_lane.contains("@spec://org.vibevm.core/vibevm/common/DEP"),
        "{md_lane}"
    );
    assert!(!md_lane.contains("@!dep"), "{md_lane}");
    // The dependency really is in the closure, qualified under its origin
    // exactly as over the Markdown twin (byte-equality already pins this;
    // the witnesses name what that equality means).
    assert!(
        md_lane.contains("## The laws {#org-vibevm-core--vibevm--laws}"),
        "{md_lane}"
    );
    assert!(
        md_lane.contains("@fact:org-vibevm-core--vibevm--FACT-ONE"),
        "{md_lane}"
    );
    assert_no_directive_lines(&md_lane);
}

/// The alias machinery sits INSIDE the flipped node: the dependency itself
/// declares `#use … as inner` (a paragraph through the XML serialisation)
/// and references `@!inner` in its prose. This is the stronger half of the
/// alias parity — the scanner, the alias table, the `#use` strip and the
/// `@!X` rewrite all run over a PROJECTED node — with a mixed tree (the
/// sub-dependency stays Markdown) riding the same pin.
#[test]
fn an_alias_declared_inside_the_dependency_compiles_identically_in_xml() {
    const DEP: &str = concat!(
        "# Dep {#d}\n\n",
        "## The laws {#laws}\n\n",
        "#use spec://org.vibevm.core/vibevm/common/SUB#sub as inner\n\n",
        "Sees @!inner and self (#laws).\n\n",
        "@fact:DEP-FACT the dep fact @status:impl/done\n\n"
    );
    const SUB: &str = "# Sub {#sub}\n\nSUB_BODY one.\n\n";
    const ENTRY: &str = concat!(
        "# Entry {#root}\n\n",
        "#use spec://org.vibevm.core/vibevm/common/DEP#laws\n\n",
        "Entry prose.\n"
    );
    let md_ws = parity_ws(
        ENTRY,
        &[
            (under_specs("common/DEP").as_str(), DEP, Form::Md),
            (under_specs("common/SUB").as_str(), SUB, Form::Md),
        ],
    );
    let xml_ws = parity_ws(
        ENTRY,
        &[
            (under_specs("common/DEP").as_str(), DEP, Form::Xml),
            (under_specs("common/SUB").as_str(), SUB, Form::Md),
        ],
    );
    let (md_lane, md_renames) = compile_entry_qualified(&md_ws);
    let (xml_lane, xml_renames) = compile_entry_qualified(&xml_ws);
    assert_eq!(
        md_lane, xml_lane,
        "md lane:\n{md_lane}\nxml lane:\n{xml_lane}"
    );
    assert_eq!(
        md_renames, xml_renames,
        "md: {md_renames:?}\nxml: {xml_renames:?}"
    );
    // The projected node's own alias bound and resolved identically.
    assert!(
        md_lane.contains("@spec://org.vibevm.core/vibevm/common/SUB"),
        "{md_lane}"
    );
    assert!(!md_lane.contains("@!inner"), "{md_lane}");
    assert!(md_lane.contains("SUB_BODY one."), "{md_lane}");
    // The node's self-reference survived the rename — qualified within the
    // node, exactly as over the Markdown twin.
    assert!(
        md_lane.contains("(#org-vibevm-core--vibevm--laws)"),
        "{md_lane}"
    );
    assert!(
        md_lane.contains("@fact:org-vibevm-core--vibevm--DEP-FACT"),
        "{md_lane}"
    );
    assert_no_directive_lines(&md_lane);
    assert!(
        DocTree::parse(&md_lane).duplicate_anchors().is_empty(),
        "{md_lane}"
    );
}

// ---- Step 2 — the qualified splice over a same-short-anchor pair ---------

/// Two dependencies in DIFFERENT packages define the SAME short anchor
/// `#laws`. The splice qualifies each per-node under its own origin and
/// every reference stays valid — and the whole closure compiles
/// byte-identically whether both deps ship MD, both ship XML, or the tree is
/// MIXED (one of each). Three runs, pairwise byte-for-byte, rename maps
/// included.
#[test]
fn two_same_short_anchors_splice_qualified_apart_identically_over_md_xml_and_mixed_trees() {
    const LAWS_A: &str = concat!(
        "# A doc {#a-doc}\n\n",
        "## The laws {#laws}\n\n",
        "@fact:A-LAW the a-side law @status:impl/done\n\n",
        "A self link (#laws).\n\n"
    );
    const LAWS_B: &str = concat!(
        "# B doc {#b-doc}\n\n",
        "## The laws {#laws}\n\n",
        "@fact:B-LAW the b-side law @status:spec/work\n\n",
        "B self link (#laws).\n\n"
    );
    const ENTRY: &str = concat!(
        "# Entry {#root}\n\n",
        "#use spec://org.a/pkg/doc#laws\n\n",
        "#use spec://org.b/qty/doc#laws\n\n",
        "Entry prose.\n"
    );
    let lane = |a: Form, b: Form| {
        let ws = parity_ws(
            ENTRY,
            &[
                (in_slot("org.a.pkg", "1.0.0", "doc").as_str(), LAWS_A, a),
                (in_slot("org.b.qty", "1.0.0", "doc").as_str(), LAWS_B, b),
            ],
        );
        compile_entry_qualified(&ws)
    };
    let (md_md, md_renames) = lane(Form::Md, Form::Md);
    let (xml_xml, xml_renames) = lane(Form::Xml, Form::Xml);
    let (mixed, mixed_renames) = lane(Form::Md, Form::Xml);
    assert_eq!(md_md, xml_xml, "md/md vs xml/xml:\n{md_md}\n---\n{xml_xml}");
    assert_eq!(md_md, mixed, "md/md vs mixed:\n{md_md}\n---\n{mixed}");
    assert_eq!(md_renames, xml_renames);
    assert_eq!(md_renames, mixed_renames);
    // The same short anchor qualified apart per-node, under each package's
    // own origin — and the self-references rewritten to the qualified heirs.
    assert!(md_md.contains("{#org-a--pkg--laws}"), "{md_md}");
    assert!(md_md.contains("{#org-b--qty--laws}"), "{md_md}");
    assert!(md_md.contains("(#org-a--pkg--laws)"), "{md_md}");
    assert!(md_md.contains("(#org-b--qty--laws)"), "{md_md}");
    assert!(
        !md_md.contains("(#laws)"),
        "a bare short link survived: {md_md}"
    );
    assert!(md_md.contains("@fact:org-a--pkg--A-LAW"), "{md_md}");
    assert!(md_md.contains("@fact:org-b--qty--B-LAW"), "{md_md}");
    assert!(md_md.contains("Entry prose."), "{md_md}");
    assert_no_directive_lines(&md_md);
    assert!(
        DocTree::parse(&md_md).duplicate_anchors().is_empty(),
        "{md_md}"
    );
}

// ---- Step 3 — fact-grain inheritance through an alias ---------------------

/// A FACT-grain `#use …#FACT-ID as dep` — the alias bound to a fact-leaf
/// address, `@!dep` in the entry's prose — over the MD twin vs its XML
/// serialisation. Extends `a_fact_grain_use_into_an_xml_dependency_resolves`
/// from "resolves" to twin-EQUALITY: the closures (lane + rename map) are
/// byte-for-byte, the fact grain carried and qualified identically.
#[test]
fn a_fact_grain_use_through_an_alias_compiles_identically_over_an_xml_dependency() {
    const ENTRY: &str = concat!(
        "# Entry {#root}\n\n",
        "#use spec://org.vibevm.core/vibevm/common/DEP#FACT-ONE as dep\n\n",
        "See @!dep.\n"
    );
    let md_ws = parity_ws(
        ENTRY,
        &[(under_specs("common/DEP").as_str(), MD_DEP_TWIN, Form::Md)],
    );
    let xml_ws = parity_ws(
        ENTRY,
        &[(under_specs("common/DEP").as_str(), MD_DEP_TWIN, Form::Xml)],
    );
    let (md_lane, md_renames) = compile_entry_qualified(&md_ws);
    let (xml_lane, xml_renames) = compile_entry_qualified(&xml_ws);
    assert_eq!(
        md_lane, xml_lane,
        "md lane:\n{md_lane}\nxml lane:\n{xml_lane}"
    );
    assert_eq!(md_renames, xml_renames);
    // The fact grain really is the closure — body, fact line, and the alias
    // reference rewritten to the full address.
    assert!(md_lane.contains("the fact body"), "{md_lane}");
    assert!(
        md_lane.contains("@fact:org-vibevm-core--vibevm--FACT-ONE"),
        "{md_lane}"
    );
    assert!(
        md_lane.contains("@spec://org.vibevm.core/vibevm/common/DEP"),
        "{md_lane}"
    );
    assert!(!md_lane.contains("@!dep"), "{md_lane}");
    assert_no_directive_lines(&md_lane);
}

// ---- XML dependencies (PROP-045 ##PROJECTION-READ, the S4b residue) --------
//
// A `normal` boot entry compiles its `#use`/`#source` closure through
// `FsSectionSource`. When a dependency document is authored (or
// materialised) as dialect XML, the source reads it through
// `load_spec_text`'s canonical Markdown projection — so the compiled lane
// over the XML form is BYTE-EQUAL to the lane over the dependency's
// canonical Markdown twin. That closes the `normal`-format residue PROP-045
// §5b recorded: the entry itself stays authored Markdown; the closure no
// longer cares which form each dependency ships in.

/// The dependency in dialect XML.
const XML_DEP: &str = concat!(
    "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
    "  <title id=\"d\">Dep</title>\n",
    "  <section id=\"laws\" title=\"The laws\">\n",
    "    <p>`req r1`</p>\n",
    "    <p><fact id=\"FACT-ONE\" status=\"impl/done\">the fact body</fact></p>\n",
    "  </section>\n",
    "</spec>\n"
);

/// The authored entry — a `normal` boot file whose closure reaches the
/// dependency's section.
const ENTRY_MD: &str =
    "# Entry {#root}\n\n#use spec://org.vibevm.core/vibevm/common/DEP#laws\n\nEntry prose.\n";

/// A workspace carrying the entry plus the dependency in `form`.
fn entry_ws(form: &str, dep_text: &str) -> tempfile::TempDir {
    let ws = tempfile::TempDir::new().unwrap();
    let specs = crate::resolver::specs_root_under(ws.path());
    let boot = specs.join("boot");
    std::fs::create_dir_all(&boot).unwrap();
    std::fs::create_dir_all(specs.join("common")).unwrap();
    std::fs::write(boot.join("00-entry.md"), ENTRY_MD).unwrap();
    std::fs::write(specs.join("common").join(format!("DEP.{form}")), dep_text).unwrap();
    ws
}

fn compile_entry(ws: &tempfile::TempDir) -> String {
    use crate::embed::FsSectionSource;
    use crate::resolver::FileResolver;
    use crate::resolver::SelfCoordinate;
    let resolver = FileResolver::new(
        ws.path(),
        SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into()),
    );
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/boot/00-entry#root").unwrap();
    let (out, _) = super::compile_static_qualified(&seed, &FsSectionSource::new(resolver)).unwrap();
    out
}

#[test]
fn a_normal_entry_compiles_the_same_closure_over_an_xml_dependency() {
    let md_ws = entry_ws("md", MD_DEP_TWIN);
    let xml_ws = entry_ws("xml", XML_DEP);
    let md_lane = compile_entry(&md_ws);
    let xml_lane = compile_entry(&xml_ws);
    assert_eq!(
        md_lane, xml_lane,
        "md lane:\n{md_lane}\nxml lane:\n{xml_lane}"
    );
    // Both serialisations of the one dependency (PROP-045
    // ##NAMED-FACT-ELEMENTS clause (b)): `XML_DEP` pins the generic
    // `<section id>`/`<fact id>` form frozen above; the runtime twin
    // carries the live NAMED shape. One closure, three lanes, all equal.
    let named_ws = entry_ws(
        "xml",
        &vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(MD_DEP_TWIN).expect("twin parses")),
    );
    let named_lane = compile_entry(&named_ws);
    assert_eq!(
        md_lane, named_lane,
        "md lane:\n{md_lane}\nnamed-xml lane:\n{named_lane}"
    );
    // The dependency's section really is inside the closure — fact and kind
    // line included, each qualified under its own origin exactly as over the
    // Markdown twin (the per-node qualify renames both lanes identically,
    // which the byte-equality above already pins).
    assert!(
        md_lane.contains("## The laws {#org-vibevm-core--vibevm--laws}"),
        "{md_lane}"
    );
    assert!(
        md_lane.contains("@fact:org-vibevm-core--vibevm--FACT-ONE"),
        "{md_lane}"
    );
    assert!(md_lane.contains("Entry prose."), "{md_lane}");
    assert!(!md_lane.contains("#use"), "{md_lane}");
}

/// The S4b leftover closes: a FACT-grain address into an XML dependency
/// resolves — the projection writes the qualified `@fact:` spelling and the
/// fact grammar now reads both spellings (PROP-043 §8).
#[test]
fn a_fact_grain_use_into_an_xml_dependency_resolves() {
    const FACT_ENTRY: &str = "# Entry {#root}

#use spec://org.vibevm.core/vibevm/common/DEP#FACT-ONE
";
    let ws = tempfile::TempDir::new().unwrap();
    let specs = crate::resolver::specs_root_under(ws.path());
    let boot = specs.join("boot");
    std::fs::create_dir_all(&boot).unwrap();
    std::fs::create_dir_all(specs.join("common")).unwrap();
    std::fs::write(boot.join("00-entry.md"), FACT_ENTRY).unwrap();
    std::fs::write(specs.join("common").join("DEP.xml"), XML_DEP).unwrap();
    let out = compile_entry(&ws);
    assert!(
        out.contains("the fact body"),
        "the fact-grain closure over an xml dependency must carry the fact:
{out}"
    );
}
