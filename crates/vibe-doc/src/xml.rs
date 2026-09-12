//! The `.xml` projection — the page's structure, carrying its block
//! numbers (PROP-057 `##SITE-TRAILING-SLASH`,
//! `##READER-NUMBERED-BLOCKS`).
//!
//! This is the machine-facing sibling of the island and the Markdown: the
//! same page, the same numbers, in the dialect an agent already knows how
//! to walk. The number rides as the attribute `p="7"`, so a tool that
//! reads a page as a tree can name a block without counting its own way
//! to it.
//!
//! ## Two things it deliberately is not
//!
//! **It is not a source.** A page's `p` attribute is added by the build,
//! and the dialect's attribute vocabulary is closed — a reader would
//! refuse it. That is correct: a projection is one-way (PROP-045
//! `##DOC-VOCAB-MD-ONE-WAY` states the same law for Markdown), and the
//! form an author edits is the page in the package, not the file the site
//! publishes beside it.
//!
//! **It does not inline the fetched text.** A `rule` keeps its address
//! and a `derived` keeps its reference, unlike the island and the
//! Markdown, which print the text a reader came for. An agent reading XML
//! wants the address: it can resolve the citation itself, and a
//! substituted copy would be the one thing this whole pipeline exists to
//! avoid — a quotation that no longer says where it came from.
//!
//! The writer emits the GENERIC spellings of the dialect — `<section
//! id=…>`, `<fact id=…>` — rather than the elementable-name forms an
//! author may write. A projection has no author's bytes to preserve, and
//! one spelling is one fewer law to keep in step.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS");

use vibe_specdoc::doc::{Block, BlockNode, Cond, Section, SpecDoc, StatusEl, Unit};

use crate::numbering::{BlockPath, Numbering};

/// The dialect's namespace.
const NS: &str = "https://vibevm.org/spec/1";

type Attrs = Vec<(&'static str, String)>;

/// Render a page as dialect XML, without block numbers.
pub fn to_xml(doc: &SpecDoc) -> String {
    to_xml_numbered(doc, &Numbering::none())
}

/// Render a page as dialect XML carrying its block numbers.
///
/// ```
/// use vibe_doc::{numbering::number_blocks, xml};
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">Hello</title>\n<p>one</p>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
///
/// let text = xml::to_xml_numbered(&doc, &number_blocks(&doc));
/// assert!(text.contains("<p p=\"1\">one</p>"));
/// ```
pub fn to_xml_numbered(doc: &SpecDoc, numbering: &Numbering) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    open(&mut out, 0, "spec", &[("xmlns", NS.to_owned())]);
    if let Some(title) = &doc.title {
        let mut attrs: Attrs = Vec::new();
        if let Some(id) = &title.id {
            attrs.push(("id", id.clone()));
        }
        line(&mut out, 1, "title", &attrs, &escape_text(&title.text));
    }
    if let Some(status) = &doc.status {
        void(&mut out, 1, "status", &status_attrs(status));
    }
    blocks_xml(&mut out, 1, &doc.preamble, &[], numbering);
    for (i, section) in doc.sections.iter().enumerate() {
        section_xml(&mut out, 1, section, &[i as u16], numbering);
    }
    close(&mut out, 0, "spec");
    out
}

fn section_xml(out: &mut String, depth: usize, s: &Section, path: &[u16], numbering: &Numbering) {
    let mut attrs: Attrs = Vec::new();
    if let Some(id) = &s.id {
        attrs.push(("id", id.clone()));
    }
    attrs.push(("title", s.title.clone()));
    push_when(&mut attrs, s.when.as_ref());
    open(out, depth, "section", &attrs);
    if let Some(status) = &s.status {
        void(out, depth + 1, "status", &status_attrs(status));
    }
    blocks_xml(out, depth + 1, &s.blocks, path, numbering);
    for (i, sub) in s.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        section_xml(out, depth + 1, sub, &child, numbering);
    }
    close(out, depth, "section");
}

fn blocks_xml(
    out: &mut String,
    depth: usize,
    nodes: &[BlockNode],
    path: &[u16],
    numbering: &Numbering,
) {
    for (i, node) in nodes.iter().enumerate() {
        let num = numbering.get(&BlockPath::new(path.to_vec(), i as u16));
        block_xml(out, depth, node, num);
    }
}

fn block_xml(out: &mut String, depth: usize, node: &BlockNode, num: Option<u32>) {
    let mut attrs: Attrs = Vec::new();
    push_when(&mut attrs, node.when.as_ref());
    push_p(&mut attrs, num);
    match &node.block {
        Block::Paragraph(u) => unit_el(out, depth, "p", attrs, u),
        Block::Quote(u) => unit_el(out, depth, "quote", attrs, u),
        Block::List { ordered, items } => {
            attrs.insert(0, ("ordered", ordered.to_string()));
            open(out, depth, "list", &attrs);
            for item in items {
                unit_el(out, depth + 1, "item", Vec::new(), item);
            }
            close(out, depth, "list");
        }
        Block::Table { rows } => {
            open(out, depth, "table", &attrs);
            for row in rows {
                open(out, depth + 1, "tr", &[]);
                for cell in row {
                    unit_el(out, depth + 2, "td", Vec::new(), cell);
                }
                close(out, depth + 1, "tr");
            }
            close(out, depth, "table");
        }
        Block::Fence { lang, fact, text } => {
            let mut fence_attrs: Attrs = Vec::new();
            if let Some(lang) = lang {
                fence_attrs.push(("lang", lang.clone()));
            }
            if let Some(fact) = fact {
                fence_attrs.push(("fact", fact.clone()));
            }
            fence_attrs.extend(attrs);
            line(out, depth, "fence", &fence_attrs, &escape_text(text));
        }
        Block::Rule { uri, .. } => {
            // The address, never a pin: `rev` is recorded by the reader so
            // an author's bytes survive a round trip and is never honoured
            // anywhere (PROP-045 `##DOC-VOCAB-RULE-ADDRESS`).
            attrs.insert(0, ("ref", uri.clone()));
            void(out, depth, "rule", &attrs);
        }
        Block::Example {
            id,
            fixture,
            lang,
            exit,
            run,
            expect,
            stderr,
        } => {
            let mut head: Attrs = vec![("id", id.clone()), ("fixture", fixture.clone())];
            if let Some(lang) = lang {
                head.push(("lang", lang.clone()));
            }
            if let Some(code) = exit {
                head.push(("exit", code.to_string()));
            }
            head.extend(attrs);
            open(out, depth, "example", &head);
            // The start/end pair always: `<expect></expect>` is how the
            // dialect says «this command prints nothing», and an assertion
            // is not an absence.
            line(out, depth + 1, "run", &[], &escape_text(run));
            line(out, depth + 1, "expect", &[], &escape_text(expect));
            if let Some(err) = stderr {
                line(out, depth + 1, "stderr", &[], &escape_text(err));
            }
            close(out, depth, "example");
        }
        Block::ExampleRef { id } => {
            attrs.insert(0, ("ref", id.clone()));
            void(out, depth, "example", &attrs);
        }
        Block::Derived { kind, reference } => {
            attrs.insert(0, ("kind", kind.as_str().to_owned()));
            attrs.insert(1, ("ref", reference.clone()));
            void(out, depth, "derived", &attrs);
        }
        Block::Note { kind, body } => {
            attrs.insert(0, ("kind", kind.as_str().to_owned()));
            unit_el(out, depth, "note", attrs, body);
        }
        Block::Figure { src, alt, caption } => {
            let mut head: Attrs = vec![("src", src.clone()), ("alt", alt.clone())];
            head.extend(attrs);
            open(out, depth, "figure", &head);
            unit_el(out, depth + 1, "caption", Vec::new(), caption);
            close(out, depth, "figure");
        }
        Block::Prompt {
            id,
            text,
            needs,
            outcome,
            asserts,
        } => {
            let mut head: Attrs = vec![("id", id.clone())];
            if asserts.is_empty() {
                head.push(("assert", "none".to_owned()));
            }
            head.extend(attrs);
            open(out, depth, "prompt", &head);
            indent(out, depth + 1);
            out.push_str(&escape_text(text));
            out.push('\n');
            if let Some(needs) = needs {
                line(out, depth + 1, "needs", &[], &escape_text(needs));
            }
            if let Some(outcome) = outcome {
                line(out, depth + 1, "outcome", &[], &escape_text(outcome));
            }
            for a in asserts {
                line(out, depth + 1, "assert", &[], &escape_text(a));
            }
            close(out, depth, "prompt");
        }
    }
}

/// A unit-carrying element. A unit with a meaningful fact wraps its text
/// in the generic `<fact>` element, which is the form the dialect accepts
/// for any anchor.
fn unit_el(out: &mut String, depth: usize, tag: &str, attrs: Attrs, u: &Unit) {
    let Some(fact) = u.fact.as_ref().filter(|f| f.is_meaningful()) else {
        line(out, depth, tag, &attrs, &escape_text(&u.text));
        return;
    };
    let mut fact_attrs: Attrs = Vec::new();
    if let Some(id) = &fact.id {
        fact_attrs.push(("id", id.clone()));
    }
    if let Some(status) = &fact.status {
        fact_attrs.push(("status", format!("{}/{}", status.stage, status.state)));
        push_status_extras(&mut fact_attrs, status);
    }
    let body = format!(
        "<fact{}>{}</fact>",
        attr_text(&fact_attrs),
        escape_text(&u.text)
    );
    line(out, depth, tag, &attrs, &body);
}

fn status_attrs(status: &StatusEl) -> Attrs {
    let mut out: Attrs = vec![
        ("stage", status.stage.to_string()),
        ("state", status.state.to_string()),
    ];
    push_status_extras(&mut out, status);
    out
}

fn push_status_extras(out: &mut Attrs, status: &StatusEl) {
    if let Some(action) = status.action {
        out.push(("action", action.to_string()));
    }
    if let Some(stage) = status.actionstage {
        out.push(("actionstage", stage.to_string()));
    }
    if !status.audience.is_empty() {
        out.push((
            "audience",
            status
                .audience
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
    if let Some(comment) = &status.comment {
        out.push(("comment", comment.clone()));
    }
    if let Some(r) = &status.r#ref {
        out.push(("ref", r.clone()));
    }
}

fn push_when(attrs: &mut Attrs, when: Option<&Cond>) {
    if let Some(cond) = when {
        attrs.push(("when", cond.to_string()));
    }
}

/// The block's number. The plain ordinal, not the padded label: the
/// padding is for a margin a reader looks at, and an attribute is read by
/// a machine.
fn push_p(attrs: &mut Attrs, num: Option<u32>) {
    if let Some(n) = num {
        attrs.push(("p", n.to_string()));
    }
}

// --- the emitters -------------------------------------------------------

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn attr_text(attrs: &[(&'static str, String)]) -> String {
    attrs
        .iter()
        .map(|(k, v)| format!(" {k}=\"{}\"", escape_attr(v)))
        .collect()
}

fn open(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)]) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}>\n", attr_text(attrs)));
}

fn close(out: &mut String, depth: usize, tag: &str) {
    indent(out, depth);
    out.push_str(&format!("</{tag}>\n"));
}

fn line(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)], body: &str) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}>{body}</{tag}>\n", attr_text(attrs)));
}

fn void(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)]) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}/>\n", attr_text(attrs)));
}

/// Text-node escaping: the XML specials plus `\r`, so a carriage return
/// cannot be normalised away by a downstream parser.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
