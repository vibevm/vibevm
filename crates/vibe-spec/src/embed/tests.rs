use super::*;
use std::fs;

struct MockSource(HashMap<String, String>);

impl MockSource {
    fn new(pairs: &[(&str, &str)]) -> Self {
        MockSource(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )
    }
}

impl SectionSource for MockSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&addr.without_pin())
            .cloned()
            .ok_or_else(|| "not in mock".to_string())
    }
}

#[test]
fn expands_a_simple_embed() {
    let src = MockSource::new(&[("spec://org.vibevm.core/vibevm/a#x", "EMBEDDED BODY")]);
    let out = expand_embeds(
        "before\n#embed spec://org.vibevm.core/vibevm/a#x\nafter\n",
        &src,
    )
    .unwrap();
    assert!(out.contains("EMBEDDED BODY"));
    assert!(out.contains("before"));
    assert!(out.contains("after"));
    // No directive survives.
    assert!(!out.contains("#embed"));
}

#[test]
fn expands_recursively_to_a_fixed_point() {
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#x",
            "level1\n#embed spec://org.vibevm.core/vibevm/b#y",
        ),
        ("spec://org.vibevm.core/vibevm/b#y", "level2"),
    ]);
    let out = expand_embeds("#embed spec://org.vibevm.core/vibevm/a#x\n", &src).unwrap();
    assert!(out.contains("level1"));
    assert!(out.contains("level2"));
    assert!(!out.contains("#embed"));
}

#[test]
fn detects_a_cycle_with_its_path() {
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#x",
            "#embed spec://org.vibevm.core/vibevm/b#y",
        ),
        (
            "spec://org.vibevm.core/vibevm/b#y",
            "#embed spec://org.vibevm.core/vibevm/a#x",
        ),
    ]);
    let err = expand_embeds("#embed spec://org.vibevm.core/vibevm/a#x\n", &src).unwrap_err();
    match err {
        EmbedError::Cycle(path) => {
            assert_eq!(path.first().unwrap(), "spec://org.vibevm.core/vibevm/a#x");
            assert_eq!(path.last().unwrap(), "spec://org.vibevm.core/vibevm/a#x");
            assert!(path.contains(&"spec://org.vibevm.core/vibevm/b#y".to_string()));
        }
        other => panic!("expected a cycle, got {other:?}"),
    }
}

#[test]
fn cycle_is_detected_before_the_repeated_address_is_resolved_again() {
    let a = "spec://org.vibevm.core/vibevm/a#x";
    let b = "spec://org.vibevm.core/vibevm/b#y";
    let mut resolved = Vec::new();
    let mut resolve = |address: &SpecAddress| {
        let key = address.without_pin();
        resolved.push(key.clone());
        match key.as_str() {
            key if key == a => Ok(format!("#embed {b}")),
            key if key == b => Ok(format!("#embed {a}")),
            _ => unreachable!("unexpected embed target"),
        }
    };
    let mut edge = |_from: &str, _ordinal: usize, _to: &SpecAddress| {};

    let error = expand_with(&format!("#embed {a}\n"), "", &mut resolve, &mut edge).unwrap_err();

    assert_eq!(error, EmbedError::Cycle(vec![a.into(), b.into(), a.into()]));
    assert_eq!(resolved, vec![a, b]);
}

#[test]
fn reports_an_unresolved_embed() {
    let src = MockSource::new(&[]);
    let err = expand_embeds("#embed spec://org.vibevm.core/vibevm/missing#x\n", &src).unwrap_err();
    assert!(matches!(err, EmbedError::Unresolved { .. }));
}

#[test]
fn markers_wrap_the_splice() {
    let src = MockSource::new(&[("spec://org.vibevm.core/vibevm/a#x", "BODY")]);
    let out = expand_embeds("#embed spec://org.vibevm.core/vibevm/a#x\n", &src).unwrap();
    assert!(out.contains("<!-- embed: spec://org.vibevm.core/vibevm/a#x -->"));
    assert!(out.contains("<!-- /embed: spec://org.vibevm.core/vibevm/a#x -->"));
}

#[test]
fn revision_pin_is_omitted_from_the_exact_embed_marker_key() {
    let key = "spec://org.vibevm.core/vibevm/a#x";
    let src = MockSource::new(&[(key, "BODY")]);

    let out = expand_embeds(&format!("#embed {key}~r7\n"), &src).unwrap();

    assert_eq!(
        out,
        format!("<!-- embed: {key} -->\nBODY\n<!-- /embed: {key} -->\n")
    );
}

/// A [`SectionSource`] backed by a real [`DocTree`]: it resolves an address's
/// anchor to a node — heading **or** fact leaf — and returns that node's
/// span, proving fact resolution + splice end to end (PROP-035 §5/§7.1).
struct DocSource(DocTree);

impl SectionSource for DocSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let node = self
            .0
            .resolve_path(&addr.anchor)
            .ok_or_else(|| "anchor not found".to_string())?;
        Ok(self.0.text(node))
    }
}

#[test]
fn embeds_a_fact_leafs_exact_span() {
    // `#embed` of a fact address splices exactly that fact's unit — not its
    // sibling, and not the whole section.
    let doc = DocTree::parse("# Doc {#root}\n- ##fact-a the exact unit\n- ##fact-b other\n");
    let src = DocSource(doc);
    let out = expand_embeds(
        "before\n#embed spec://org.vibevm.core/vibevm/d#fact-a\nafter\n",
        &src,
    )
    .unwrap();
    assert!(out.contains("##fact-a the exact unit"), "{out}");
    assert!(!out.contains("##fact-b"), "spliced too much:\n{out}");
    assert!(out.contains("before") && out.contains("after"));
    assert!(!out.contains("#embed"));
}

// ----- XML dependencies (PROP-045 ##PROJECTION-READ) ---------------------

/// The XML twin of `DEP.md`'s section — the canonical projection of this
/// document is byte-exact the `.md` fixture above it, which is what makes
/// the two workspaces' compiles comparable.
const DEP_XML: &str = concat!(
    "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
    "  <title id=\"d\">Dep</title>\n",
    "  <section id=\"laws\" title=\"The laws\">\n",
    "    <p>`req r1`</p>\n",
    "    <p><fact id=\"FACT-ONE\" status=\"impl/done\">the fact body</fact></p>\n",
    "  </section>\n",
    "</spec>\n"
);

const DEP_MD_TWIN: &str = concat!(
    "# Dep {#d}\n\n",
    "## The laws {#laws}\n\n",
    "`req r1`\n\n",
    "@fact:FACT-ONE the fact body @status:impl/done\n\n"
);

/// A workspace whose authored specs root holds the dependency in `form`
/// (the scaffold routes through the resolver's root probe — a fresh
/// tempdir falls back to the legacy name, PROP-052).
fn dep_ws(form: &str, text: &str) -> tempfile::TempDir {
    let ws = tempfile::tempdir().unwrap();
    let dir = crate::resolver::specs_root_under(ws.path()).join("common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("DEP.{form}")), text).unwrap();
    ws
}

fn dep_source(ws: &tempfile::TempDir) -> FsSectionSource {
    let resolver = crate::resolver::FileResolver::new(
        ws.path(),
        crate::resolver::SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into()),
    );
    FsSectionSource::new(resolver)
}

#[test]
fn a_section_of_an_xml_dependency_reads_as_its_canonical_twin() {
    // The same address resolves over either form and yields the SAME
    // section text: the XML file arrives as its canonical Markdown
    // projection, and the twin `.md` is that projection byte for byte.
    // (A FACT-leaf address into an XML dependency does NOT resolve yet —
    // the projection spells facts `@fact:`, and the tree's fact-leaf
    // grammar reads only `##`; recorded as the slice's leftover.)
    let md_ws = dep_ws("md", DEP_MD_TWIN);
    let xml_ws = dep_ws("xml", DEP_XML);
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/DEP#laws").unwrap();
    let md_text = dep_source(&md_ws).section_text(&addr).unwrap();
    let xml_text = dep_source(&xml_ws).section_text(&addr).unwrap();
    assert_eq!(md_text, xml_text);
    assert!(md_text.contains("## The laws {#laws}"), "{md_text}");
    assert!(md_text.contains("@fact:FACT-ONE"), "{md_text}");
}

#[test]
fn a_dialect_violation_in_a_dependency_is_an_error_naming_the_file() {
    let ws = dep_ws("xml", "<spec><p>x</p></spec>");
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/DEP#laws").unwrap();
    let err = dep_source(&ws).section_text(&addr).unwrap_err();
    assert!(err.contains("DEP.xml"), "the file must ride: {err}");
    assert!(
        err.contains("xmlns"),
        "the dialect message must ride: {err}"
    );
}
