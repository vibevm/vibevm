//! `#embed` expansion (PROP-035 §7.1) — the macro splice.
//!
//! `#embed <spec://…>` is replaced, textually, by the section (or whole
//! document) the address names. Expansion is **recursive to a fixed point**: an
//! embedded section may itself contain `#embed`, and those are expanded too, so
//! no `#embed` survives the output. A cycle guard (PROP-035 §9) keys on the
//! address currently being expanded and rejects a repeat with the offending
//! path (`a → b → a`), the same diagnostic C's include guards give.
//!
//! The section text an address resolves to is supplied by a [`SectionSource`],
//! so the expander is testable without a filesystem. [`FsSectionSource`] is the
//! real one — it composes the whole crate: [`FileResolver`] to find the file,
//! then [`DocTree`] to resolve the anchor to a node and take its text.
//!
//! Spliced text is wrapped in open/close markers (PROP-035 §11) so the result
//! stays reversible.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::address::SpecAddress;
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::resolver::FileResolver;

/// Supplies the text a `spec://` address resolves to. Abstract so `#embed`
/// expansion can be driven from a filesystem, an in-memory map, or a test mock.
///
/// The fold and the use-graph traverser both reach a document's `#source` edges
/// through this trait, so the one fact they share — a `#source` address may name
/// a *set* (a glob), not a file — lives here as [`SectionSource::expand_pattern`].
pub trait SectionSource {
    /// The text of the section (or whole document) `addr` names, or a reason it
    /// could not be produced.
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String>;

    /// Expand `addr` into the concrete addresses it denotes — a pattern (a `*`
    /// in the package name) into its sorted member set, a point address into
    /// exactly itself. The default returns the address unchanged, so a source
    /// with no notion of an installed set (every test mock, every in-memory map)
    /// degrades to point behaviour rather than breaking, and no existing
    /// [`SectionSource`] needs touching when this lands. [`FsSectionSource`]
    /// overrides it to delegate to the resolver's total oracle.
    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        Ok(vec![addr.clone()])
    }
}

/// Why `#embed` expansion failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbedError {
    #[error("embed cycle: {}", .0.join(" -> "))]
    Cycle(Vec<String>),
    #[error("cannot resolve embed {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
}

/// Expand every `#embed` in `text` to a fixed point.
pub fn expand_embeds(text: &str, source: &impl SectionSource) -> Result<String, EmbedError> {
    let mut resolve = |address: &SpecAddress| source.section_text(address);
    let mut edge = |_from: &str, _ordinal: usize, _to: &SpecAddress| {};
    expand_with(text, "", &mut resolve, &mut edge)
}

/// Shared byte engine used by the public helper and the named compiler pass.
/// `edge` receives the pinless source context and the embed directive's ordinal
/// among embeds in that context, a stable authored-occurrence identity even
/// when pre-embed normalization removes preceding use/source lines.
pub(crate) fn expand_with(
    text: &str,
    root_context: &str,
    resolve: &mut impl FnMut(&SpecAddress) -> Result<String, String>,
    edge: &mut impl FnMut(&str, usize, &SpecAddress),
) -> Result<String, EmbedError> {
    let mut stack = Vec::new();
    expand_rec(text, root_context, resolve, edge, &mut stack)
}

fn expand_rec(
    text: &str,
    context: &str,
    resolve: &mut impl FnMut(&SpecAddress) -> Result<String, String>,
    edge: &mut impl FnMut(&str, usize, &SpecAddress),
    stack: &mut Vec<String>,
) -> Result<String, EmbedError> {
    let directives = Directives::parse(text);
    let embeds: HashMap<usize, (usize, &SpecAddress)> = directives
        .directives
        .iter()
        .filter(|d| d.kind == DirectiveKind::Embed)
        .enumerate()
        .map(|(ordinal, d)| (d.line, (ordinal, &d.address)))
        .collect();

    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let Some((ordinal, addr)) = embeds.get(&i) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };

        let key = addr.without_pin();
        if stack.contains(&key) {
            let mut path = stack.clone();
            path.push(key);
            return Err(EmbedError::Cycle(path));
        }

        let section = resolve(addr).map_err(|reason| EmbedError::Unresolved {
            addr: addr.to_string(),
            reason,
        })?;

        edge(context, *ordinal, addr);
        stack.push(key.clone());
        let expanded = expand_rec(&section, &key, resolve, edge, stack)?;
        stack.pop();

        writeln!(out, "<!-- embed: {key} -->").unwrap();
        out.push_str(&expanded);
        if !expanded.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "<!-- /embed: {key} -->").unwrap();
    }
    Ok(out)
}

/// The real [`SectionSource`]: resolve the address to a file (either
/// serialisation), read it through the spec-source dispatch, and take the
/// addressed node's text — the crate's layers composed end to end.
///
/// A `.xml` source never reaches the tree raw: `load_spec_text` delivers its
/// canonical Markdown projection (PROP-045 ##PROJECTION-READ), so the fold,
/// the embed expansion and the anchor resolution all work unchanged over
/// either form — the recorded degradation being that a diagnostic naming a
/// line inside an XML dependency cites the projection's line, not the
/// XML source's.
pub struct FsSectionSource {
    resolver: FileResolver,
}

impl FsSectionSource {
    pub fn new(resolver: FileResolver) -> Self {
        Self { resolver }
    }
}

impl SectionSource for FsSectionSource {
    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        // Delegate to the resolver's total oracle: a non-pattern address
        // denotes exactly itself (it returns before any scan), a pattern
        // denotes its sorted members, and a pattern matching nothing is the
        // empty set — so the trait's "what does this address denote" question
        // has one answer, whatever calls it.
        self.resolver
            .expand_pattern(addr)
            .map_err(|e| e.to_string())
    }

    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let file = self
            .resolver
            .resolve_file(addr)
            .map_err(|e| e.to_string())?;
        let (src, _kind) = vibe_specdoc::load_spec_text(&file).map_err(|e| e.to_string())?;
        let tree = DocTree::parse(&src);
        let node = match tree.resolve_path(&addr.anchor) {
            Some(node) => node,
            None => {
                // B-011 §6.1 layer 3: a missed short anchor answers with its
                // qualified heirs, never emptiness. The flat segment being
                // resolved is the lookup's short name; the tree's
                // `<origin-slug>--<short>` tails are the rename's heirs.
                let candidates = addr
                    .anchor
                    .first()
                    .map(|short| tree.qualified_candidates(short.as_str()))
                    .filter(|c| !c.is_empty())
                    .map(|c| {
                        format!(
                            " (qualified candidates for `{}`: {})",
                            addr.anchor.first().map(String::as_str).unwrap_or(""),
                            c.join(", ")
                        )
                    })
                    .unwrap_or_default();
                return Err(format!(
                    "anchor not found in {}{candidates}",
                    file.display()
                ));
            }
        };
        Ok(tree.text(node))
    }
}

#[cfg(test)]
mod tests {
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
        let err =
            expand_embeds("#embed spec://org.vibevm.core/vibevm/missing#x\n", &src).unwrap_err();
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
}
