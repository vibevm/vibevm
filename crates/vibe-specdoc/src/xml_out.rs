//! `to_xml` — the deterministic dialect backend (quick-xml Writer).
//!
//! Determinism contract: 2-space indents, `\n` newlines, one fixed
//! attribute order per element, inline leaf content (text is never
//! re-indented, so unit text round-trips verbatim). With the reader this
//! gives the idempotence law: `from_xml(to_xml(d)) == d`, hence
//! XML→IR→XML is byte-in-byte.
//!
//! The writer is VOCABULARY-BLIND (##DOC-VOCAB-DISCRIMINATOR): one IR
//! serialises to the same bytes whichever vocabulary read it, the
//! elementability blacklist does not grow for the documentation genre, and
//! a documentation block is told from a named section by the same
//! discriminator the reader uses — a block never carries `title=`. The
//! slot condition `when` is emitted last on the element it guards, and
//! only ever appears on an IR that a `Doc` reader produced.
//!
//! This cell decides WHAT to write. The primitives that decide how the
//! bytes look — one indented element, and the two escapes its text and
//! its attributes go through — are [`crate::xml_out_emit`], and the
//! writer's own tests are [`crate::xml_out_tests`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use super::xml_out_emit::{bytes_start, empty, end, esc_text, indent, inline, start};
use crate::doc::{Block, BlockNode, Cond, Fact, Section, SpecDoc, StatusEl, Unit};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesText, Event};

/// The dialect's namespace — the root attribute that self-identifies a
/// spec file (PROP-045 ##DIALECT-SKETCH).
pub(crate) const NS: &str = "https://vibevm.org/spec/1";

/// Whether a section anchor or fact id can carry its identity as an XML
/// element name.
///
/// The named-section form is reserved for ASCII XML names outside the
/// dialect's structural vocabulary. XML reserves every case-insensitive
/// `xml` prefix, so those anchors stay in the generic `<section id=...>`
/// fallback too.
pub(crate) fn anchor_is_elementable(anchor: &str) -> bool {
    let mut chars = anchor.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return false;
    }
    if anchor
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("xml"))
    {
        return false;
    }
    !matches!(
        anchor,
        "spec"
            | "title"
            | "status"
            | "section"
            | "p"
            | "fact"
            | "facts"
            | "list"
            | "item"
            | "table"
            | "tr"
            | "td"
            | "fence"
            | "quote"
    )
}

pub(crate) type W = Writer<Vec<u8>>;
/// Owned attribute pairs (`&str` keys, pre-`to_string` values) — one
/// uniform shape every emitter below speaks.
type Attrs<'a> = Vec<(&'a str, String)>;

/// Emit a document as dialect XML.
pub fn to_xml(doc: &SpecDoc) -> String {
    let mut w = Writer::new(Vec::new());
    // `let _ =` by contract: the sink is a Vec, which cannot fail.
    let _ = w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)));
    let xmlns: Attrs = vec![("xmlns", NS.to_string())];
    start(&mut w, 0, "spec", &xmlns);
    if let Some(t) = &doc.title {
        let mut attrs: Attrs = Vec::new();
        if let Some(id) = &t.id {
            attrs.push(("id", id.clone()));
        }
        inline(&mut w, 1, "title", &attrs, &t.text);
    }
    if let Some(s) = &doc.status {
        empty(&mut w, 1, "status", status_attrs(s).as_slice());
    }
    for b in &doc.preamble {
        block_node(&mut w, 1, b);
    }
    for s in &doc.sections {
        section(&mut w, 1, s);
    }
    end(&mut w, 0, "spec");
    let _ = w.write_event(Event::Text(BytesText::from_escaped("\n")));
    String::from_utf8(w.into_inner()).expect("writer output is UTF-8")
}

fn section(w: &mut W, depth: usize, s: &Section) {
    let mut attrs: Attrs = Vec::new();
    let tag =
        s.id.as_deref()
            .filter(|id| anchor_is_elementable(id))
            .unwrap_or("section");
    if tag == "section"
        && let Some(id) = &s.id
    {
        attrs.push(("id", id.clone()));
    }
    attrs.push(("title", s.title.clone()));
    push_when(&mut attrs, s.when.as_ref());
    start(w, depth, tag, &attrs);
    if let Some(st) = &s.status {
        empty(w, depth + 1, "status", status_attrs(st).as_slice());
    }
    for b in &s.blocks {
        block_node(w, depth + 1, b);
    }
    for sub in &s.sections {
        section(w, depth + 1, sub);
    }
    end(w, depth, tag);
}

/// The `<status>` attribute set, one fixed order: stage, state, action,
/// actionstage, audience, comment, ref — the progress-core vocabulary.
fn status_attrs(s: &StatusEl) -> Attrs<'static> {
    let mut out: Attrs = vec![
        ("stage", s.stage.to_string()),
        ("state", s.state.to_string()),
    ];
    push_status_extras(&mut out, s);
    out
}

/// The extra status attributes, appended in canonical order after the
/// stage/state pair (shared by the element form and the fact form).
fn push_status_extras(out: &mut Attrs, s: &StatusEl) {
    if let Some(a) = s.action {
        out.push(("action", a.to_string()));
    }
    if let Some(a) = s.actionstage {
        out.push(("actionstage", a.to_string()));
    }
    if !s.audience.is_empty() {
        out.push((
            "audience",
            s.audience
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
    if let Some(c) = &s.comment {
        out.push(("comment", c.clone()));
    }
    if let Some(r) = &s.r#ref {
        out.push(("ref", r.clone()));
    }
}

/// A fact element's attribute set. The named form starts with its
/// discriminator, then the compact `status="stage/state"` pair and extras;
/// the generic fallback starts with `id`, then carries the same status tail.
fn fact_attrs(f: &Fact, named: bool) -> Attrs<'_> {
    let mut out: Attrs = Vec::new();
    if named {
        out.push(("fact", "true".to_string()));
    } else if let Some(id) = &f.id {
        out.push(("id", id.clone()));
    }
    if let Some(st) = &f.status {
        out.push(("status", format!("{}/{}", st.stage, st.state)));
        if let Some(requirements) = &f.requirements {
            out.push(("requires", requirements.to_csv()));
        }
        push_status_extras(&mut out, st);
    }
    out
}

/// The slot's condition, appended last so every element's own attribute
/// order stays exactly as it was before the genre existed.
fn push_when(attrs: &mut Attrs<'static>, when: Option<&Cond>) {
    if let Some(c) = when {
        attrs.push(("when", c.to_string()));
    }
}

fn block_node(w: &mut W, depth: usize, node: &BlockNode) {
    block(w, depth, &node.block, node.when.as_ref());
}

fn block(w: &mut W, depth: usize, b: &Block, when: Option<&Cond>) {
    match b {
        Block::Paragraph(u) => unit_el(w, depth, "p", Vec::new(), u, when),
        Block::Quote(u) => unit_el(w, depth, "quote", Vec::new(), u, when),
        Block::Fence { lang, fact, text } => {
            let mut attrs: Attrs = Vec::new();
            if let Some(l) = lang {
                attrs.push(("lang", l.clone()));
            }
            if let Some(f) = fact {
                attrs.push(("fact", f.clone()));
            }
            push_when(&mut attrs, when);
            if text.is_empty() {
                empty(w, depth, "fence", &attrs);
            } else {
                inline(w, depth, "fence", &attrs, text);
            }
        }
        Block::List { ordered, items } => {
            let ord = if *ordered { "true" } else { "false" };
            let all_facts = !items.is_empty()
                && items
                    .iter()
                    .all(|u| u.fact.as_ref().is_some_and(|f| f.is_meaningful()));
            let mut attrs: Attrs = vec![("ordered", ord.to_string())];
            push_when(&mut attrs, when);
            if all_facts {
                start(w, depth, "facts", &attrs);
                for item in items {
                    if let Some(f) = &item.fact {
                        indent(w, depth + 1);
                        fact_element(w, f, &item.text);
                    }
                }
                end(w, depth, "facts");
            } else {
                start(w, depth, "list", &attrs);
                for item in items {
                    unit(w, depth + 1, "item", item);
                }
                end(w, depth, "list");
            }
        }
        Block::Table { rows } => {
            let mut attrs: Attrs = Vec::new();
            push_when(&mut attrs, when);
            start(w, depth, "table", &attrs);
            for row in rows {
                start(w, depth + 1, "tr", &[]);
                for cell in row {
                    unit(w, depth + 2, "td", cell);
                }
                end(w, depth + 1, "tr");
            }
            end(w, depth, "table");
        }
        // --- the documentation genre (PROP-045 §7) ----------------------
        Block::Example {
            id,
            fixture,
            lang,
            exit,
            run,
            expect,
            stderr,
        } => {
            let mut attrs: Attrs = vec![("id", id.clone()), ("fixture", fixture.clone())];
            if let Some(l) = lang {
                attrs.push(("lang", l.clone()));
            }
            if let Some(code) = exit {
                attrs.push(("exit", code.to_string()));
            }
            push_when(&mut attrs, when);
            start(w, depth, "example", &attrs);
            // The verbatim children always take the start/end pair, never
            // the self-closing form: `<expect></expect>` is how the dialect
            // spells «this command prints nothing», and an assertion is not
            // an absence.
            verbatim(w, depth + 1, "run", run);
            verbatim(w, depth + 1, "expect", expect);
            if let Some(err) = stderr {
                verbatim(w, depth + 1, "stderr", err);
            }
            end(w, depth, "example");
        }
        Block::ExampleRef { id } => {
            let mut attrs: Attrs = vec![("ref", id.clone())];
            push_when(&mut attrs, when);
            empty(w, depth, "example", &attrs);
        }
        Block::Rule { uri, rev } => {
            // The recorded `~rN` rides back out so the author's bytes
            // survive; the citation itself is `uri`, unpinned
            // (##DOC-VOCAB-RULE-ADDRESS).
            let address = match rev {
                Some(n) => format!("{uri}~r{n}"),
                None => uri.clone(),
            };
            let mut attrs: Attrs = vec![("ref", address)];
            push_when(&mut attrs, when);
            empty(w, depth, "rule", &attrs);
        }
        Block::Derived { kind, reference } => {
            let mut attrs: Attrs = vec![("kind", kind.to_string()), ("ref", reference.clone())];
            push_when(&mut attrs, when);
            empty(w, depth, "derived", &attrs);
        }
        Block::Note { kind, body } => {
            unit_el(
                w,
                depth,
                "note",
                vec![("kind", kind.to_string())],
                body,
                when,
            );
        }
        Block::Figure { src, alt, caption } => {
            let mut attrs: Attrs = vec![("src", src.clone()), ("alt", alt.clone())];
            push_when(&mut attrs, when);
            start(w, depth, "figure", &attrs);
            unit(w, depth + 1, "caption", caption);
            end(w, depth, "figure");
        }
        Block::Prompt {
            id,
            text,
            needs,
            outcome,
            asserts,
        } => {
            let mut attrs: Attrs = vec![("id", id.clone())];
            if asserts.is_empty() {
                attrs.push(("assert", "none".to_string()));
            }
            push_when(&mut attrs, when);
            indent(w, depth);
            let _ = w.write_event(Event::Start(bytes_start("prompt", &attrs)));
            // The body sits inline against its own start tag, so the
            // indentation before the first child is the writer's alone —
            // which is why the reader trims the body's outer whitespace.
            let _ = w.write_event(Event::Text(BytesText::from_escaped(esc_text(text))));
            if let Some(n) = needs {
                inline(w, depth + 1, "needs", &[], n);
            }
            if let Some(o) = outcome {
                inline(w, depth + 1, "outcome", &[], o);
            }
            for a in asserts {
                verbatim(w, depth + 1, "assert", a);
            }
            if needs.is_none() && outcome.is_none() && asserts.is_empty() {
                // A childless prompt is a leaf, and a leaf closes on its
                // own line exactly like `<p>` — the indent belongs to the
                // children, so with none there is nothing to indent from.
                let _ = w.write_event(Event::End(BytesEnd::new("prompt")));
            } else {
                end(w, depth, "prompt");
            }
        }
    }
}

/// One unit-bearing leaf (`p`, `item`, `quote`, `td`, `caption`): either
/// bare text or one wrapping `<fact>` element. Empty text and no fact
/// collapses to an empty element (the empty table cell).
fn unit(w: &mut W, depth: usize, tag: &str, u: &Unit) {
    unit_el(w, depth, tag, Vec::new(), u, None);
}

/// [`unit`] for a carrier that has attributes of its own (`<note kind>`) or
/// a slot condition.
fn unit_el(
    w: &mut W,
    depth: usize,
    tag: &str,
    mut attrs: Attrs<'static>,
    u: &Unit,
    when: Option<&Cond>,
) {
    push_when(&mut attrs, when);
    let Some(f) = u.fact.as_ref().filter(|f| f.is_meaningful()) else {
        if u.text.is_empty() {
            empty(w, depth, tag, &attrs);
        } else {
            inline(w, depth, tag, &attrs, &u.text);
        }
        return;
    };
    indent(w, depth);
    let _ = w.write_event(Event::Start(bytes_start(tag, &attrs)));
    fact_element(w, f, &u.text);
    let _ = w.write_event(Event::End(BytesEnd::new(tag)));
}

/// A verbatim child of the documentation genre: the start/end PAIR always
/// — never the fence's empty-collapse, because `<expect></expect>` is the
/// assertion «this command prints nothing» and must survive as one — with
/// the text exactly as the IR holds it (##DOC-VOCAB-VERBATIM-TEXTS).
fn verbatim(w: &mut W, depth: usize, tag: &str, text: &str) {
    inline(w, depth, tag, &[], text);
}

/// One generic or named fact element, without a carrier wrapper or indent.
fn fact_element(w: &mut W, f: &Fact, text: &str) {
    let fact_tag =
        f.id.as_deref()
            .filter(|id| anchor_is_elementable(id))
            .unwrap_or("fact");
    let named = fact_tag != "fact";
    let _ = w.write_event(Event::Start(bytes_start(
        fact_tag,
        fact_attrs(f, named).as_slice(),
    )));
    if !text.is_empty() {
        let _ = w.write_event(Event::Text(BytesText::from_escaped(esc_text(text))));
    }
    let _ = w.write_event(Event::End(BytesEnd::new(fact_tag)));
}
