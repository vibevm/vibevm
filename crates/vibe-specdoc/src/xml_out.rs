//! `to_xml` — the deterministic dialect backend (quick-xml Writer).
//!
//! Determinism contract: 2-space indents, `\n` newlines, one fixed
//! attribute order per element, inline leaf content (text is never
//! re-indented, so unit text round-trips verbatim). With the reader this
//! gives the idempotence law: `from_xml(to_xml(d)) == d`, hence
//! XML→IR→XML is byte-in-byte.

use crate::doc::{Block, Fact, Section, SpecDoc, StatusEl, Unit};
use quick_xml::Writer;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;

/// The dialect's namespace — the root attribute that self-identifies a
/// spec file (PROP-045 ##DIALECT-SKETCH).
pub(crate) const NS: &str = "https://vibevm.org/spec/1";

type W = Writer<Vec<u8>>;
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
        block(&mut w, 1, b);
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
    if let Some(id) = &s.id {
        attrs.push(("id", id.clone()));
    }
    attrs.push(("title", s.title.clone()));
    start(w, depth, "section", &attrs);
    if let Some(st) = &s.status {
        empty(w, depth + 1, "status", status_attrs(st).as_slice());
    }
    for b in &s.blocks {
        block(w, depth + 1, b);
    }
    for sub in &s.sections {
        section(w, depth + 1, sub);
    }
    end(w, depth, "section");
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

/// The `<fact>` attribute set: id, then the compact `status="stage/state"`
/// pair, then the same extras in the same canonical order.
fn fact_attrs(f: &Fact) -> Attrs<'_> {
    let mut out: Attrs = Vec::new();
    if let Some(id) = &f.id {
        out.push(("id", id.clone()));
    }
    if let Some(st) = &f.status {
        out.push(("status", format!("{}/{}", st.stage, st.state)));
        push_status_extras(&mut out, st);
    }
    out
}

fn block(w: &mut W, depth: usize, b: &Block) {
    match b {
        Block::Paragraph(u) => unit(w, depth, "p", u),
        Block::Quote(u) => unit(w, depth, "quote", u),
        Block::Fence { lang, fact, text } => {
            let mut attrs: Attrs = Vec::new();
            if let Some(l) = lang {
                attrs.push(("lang", l.clone()));
            }
            if let Some(f) = fact {
                attrs.push(("fact", f.clone()));
            }
            if text.is_empty() {
                empty(w, depth, "fence", &attrs);
            } else {
                inline(w, depth, "fence", &attrs, text);
            }
        }
        Block::List { ordered, items } => {
            let ord = if *ordered { "true" } else { "false" };
            start(w, depth, "list", &[("ordered", ord.to_string())]);
            for item in items {
                unit(w, depth + 1, "item", item);
            }
            end(w, depth, "list");
        }
        Block::Table { rows } => {
            start(w, depth, "table", &[]);
            for row in rows {
                start(w, depth + 1, "tr", &[]);
                for cell in row {
                    unit(w, depth + 2, "td", cell);
                }
                end(w, depth + 1, "tr");
            }
            end(w, depth, "table");
        }
    }
}

/// One unit-bearing leaf (`p`, `item`, `quote`, `td`): either bare text or
/// one wrapping `<fact>` element. Empty text and no fact collapses to an
/// empty element (the empty table cell).
fn unit(w: &mut W, depth: usize, tag: &str, u: &Unit) {
    let Some(f) = u.fact.as_ref().filter(|f| f.is_meaningful()) else {
        if u.text.is_empty() {
            empty(w, depth, tag, &[]);
        } else {
            inline(w, depth, tag, &[], &u.text);
        }
        return;
    };
    indent(w, depth);
    let _ = w.write_event(Event::Start(BytesStart::new(tag)));
    let _ = w.write_event(Event::Start(bytes_start("fact", fact_attrs(f).as_slice())));
    if !u.text.is_empty() {
        let _ = w.write_event(Event::Text(BytesText::from_escaped(esc_text(&u.text))));
    }
    let _ = w.write_event(Event::End(BytesEnd::new("fact")));
    let _ = w.write_event(Event::End(BytesEnd::new(tag)));
}

fn bytes_start<'a>(name: &'a str, attrs: &[(&str, String)]) -> BytesStart<'a> {
    let mut s = BytesStart::new(name);
    for (k, v) in attrs {
        // Pre-escaped by `esc_attr`; `push_attribute` stores it verbatim.
        s.push_attribute(Attribute {
            key: QName(k.as_bytes()),
            value: std::borrow::Cow::Owned(esc_attr(v).into_owned().into_bytes()),
        });
    }
    s
}

/// Text-node escaping: the XML specials plus `\r` (so a CR can never be
/// normalised away by a downstream parser). `\n` stays literal — text
/// content is never line-normalised, and fences stay readable.
fn esc_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
    out
}

/// Attribute-value escaping: everything `esc_text` does, plus the quotes
/// and the whitespace characters an XML parser may normalise inside
/// attribute values (`\n`, `\t`).
fn esc_attr(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains(['&', '<', '>', '"', '\'', '\r', '\n', '\t']) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\r' => out.push_str("&#xD;"),
            '\n' => out.push_str("&#xA;"),
            '\t' => out.push_str("&#x9;"),
            _ => out.push(c),
        }
    }
    std::borrow::Cow::Owned(out)
}

fn start(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)]) {
    indent(w, depth);
    let _ = w.write_event(Event::Start(bytes_start(name, attrs)));
}

fn end(w: &mut W, depth: usize, name: &str) {
    indent(w, depth);
    let _ = w.write_event(Event::End(BytesEnd::new(name)));
}

fn empty(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)]) {
    indent(w, depth);
    let _ = w.write_event(Event::Empty(bytes_start(name, attrs)));
}

/// An inline leaf: one line, text untouched between the tags.
fn inline(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)], text: &str) {
    indent(w, depth);
    let _ = w.write_event(Event::Start(bytes_start(name, attrs)));
    let _ = w.write_event(Event::Text(BytesText::from_escaped(esc_text(text))));
    let _ = w.write_event(Event::End(BytesEnd::new(name)));
}

fn indent(w: &mut W, depth: usize) {
    let mut s = String::with_capacity(1 + 2 * depth);
    s.push('\n');
    for _ in 0..depth {
        s.push_str("  ");
    }
    let _ = w.write_event(Event::Text(BytesText::from_escaped(s)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_markdown;

    /// The shape test: a small document emits exactly the dialect's
    /// canonical form — pinned byte-for-byte, because this is the format
    /// contract every golden file rests on.
    #[test]
    fn canonical_form_is_pinned() {
        let d = from_markdown(
            "# T {#t}\n\n<status stage=\"spec\" state=\"work\"/>\n\n\
             @fact:A One. @status:impl/done\n\nplain paragraph\n",
        )
        .expect("parses");
        let xml = to_xml(&d);
        assert_eq!(
            xml,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
             <title id=\"t\">T</title>\n  \
             <status stage=\"spec\" state=\"work\"/>\n  \
             <p><fact id=\"A\" status=\"impl/done\">One.</fact></p>\n  \
             <p>plain paragraph</p>\n\
             </spec>\n"
        );
    }

    #[test]
    fn specials_are_escaped_and_round_trip() {
        let d = from_markdown("# T {#t}\n\n@fact:A `a < b` & `c > d`. @impl/done\n").unwrap();
        let xml = to_xml(&d);
        assert!(xml.contains("&lt;"), "{xml}");
        let back = crate::from_xml(&xml).unwrap();
        assert_eq!(back, d);
    }
}
