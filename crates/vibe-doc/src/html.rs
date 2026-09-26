//! `to_html` — the island (PROP-057 `##PIPE-LIBRARY`,
//! `##PIPE-SHELL-PARSES-NOTHING`).
//!
//! An island is a page's CONTENT as finished HTML, and nothing else. The
//! Qwik shell on the public site and the local reader on `127.0.0.1` both
//! receive exactly these bytes, which is the whole point: if the site
//! parsed the sources again, anchors and facts would lose their identity
//! and the two renders would drift apart on a schedule nobody set.
//!
//! ## What an island never contains
//!
//! **No `<script>` and no `<style>`.** Not as a precaution — as a
//! contract. Launch parameters reach the shell as a non-executable
//! `<script type="application/json">` block that the SHELL emits, so the
//! content security policy needs no `'unsafe-inline'`
//! (`##PIPE-SHELL-PARSES-NOTHING`, `##LOCAL-CSP`). An island that could
//! carry a script would make that policy a wish.
//!
//! It also carries no page furniture: no `<html>`, no `<head>`, no
//! navigation, no meta block, no theme. Those are the shell's, and the
//! shell is a different crate for the same reason.
//!
//! ## The shape
//!
//! Structure comes from the pivot and rides on `data-` attributes, so the
//! shell can style and script around content it never parses:
//!
//! * a section is `<section id="…">` with its heading at its depth;
//! * a fact is its block plus `data-fact` and `data-status`;
//! * a slot's condition is `data-when` — the island keeps every variant
//!   and the shell (or the reader's platform) decides what to show, so
//!   one build serves every platform and every agent;
//! * a `rule` is a quotation carrying the fact's CURRENT text, with the
//!   address in `data-uri` and the language of the SPECIFICATION in
//!   `lang` — a translated page quotes an English rule in English and
//!   says so, and there is no `data-rev`, because a citation is live and
//!   unpinned (`##OBS-RULE-EDGE-UNPINNED`, D-27) — inside a disclosure
//!   that starts closed, described in one line ([`rule`],
//!   `##READER-RULE-FOLDED`);
//! * an `example` is its command and the output it must produce, both
//!   verbatim;
//! * a `derived` block is the fence its generator built at this build.
//!
//! A page whose documentation declared a glossary carries two things more:
//! every link to an entry of it takes `data-gloss` and `aria-describedby`
//! ([`inline`]), and the island ends with one hidden `aside` holding the
//! definitions of the entries THIS page names, resolved at build time so
//! that nothing is fetched or parsed in the browser
//! (`##READER-GLOSSARY-CARD`, `##PIPE-SHELL-PARSES-NOTHING`). The block
//! takes no number: it is apparatus and not flow, and the numbers name the
//! text a reader reads (`##READER-NUMBERED-BLOCKS`).
//!
//! A block whose text this build could not fetch is marked
//! `data-unresolved` and shows its address. Refusing to render the page
//! would tell an author about one defect per run; the checks
//! (`vibe doc check --citations`, `--derived`) are where a missing text
//! is a failure.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

pub mod inline;
pub mod links;

mod emit;
mod example;
mod gloss;
mod rule;

use vibe_specdoc::doc::{Block, BlockNode, Cond, Fact, Section, SpecDoc, StatusEl, Unit};

use crate::content::{Content, ExampleBody};
use crate::numbering::{BlockPath, Numbering};
use emit::{anchor, anchor_line, close, line, open, pre_code, push_p, void};
use inline::{escape, render_linked};
use links::Links;

pub use gloss::GLOSS_DEF_ID;

/// One attribute, already escaped.
type Attrs = Vec<(&'static str, String)>;

/// The links of the page's OWN prose, able to recognise a term of the
/// declared glossary.
///
/// One function rather than a `Links::page(&content.base)` at every block,
/// because the glossary lens has to be on every one of them or on none: a
/// card that appeared in a paragraph and not in a list item would be a
/// reader wondering which words have definitions.
fn prose<'a>(content: &'a Content, page: Option<&'a str>) -> Links<'a> {
    let links = Links::page(&content.base);
    match page {
        Some(page) => links.glossing(page, content.glossary.as_ref()),
        None => links,
    }
}

/// Render a page as an island, without block numbers.
///
/// The document renders with NO address in a package, so a translation's
/// borrowed example renders as the marked gap it is: an id alone does not
/// name an example, and a caller that has a page address passes it to
/// [`to_html_numbered`] instead.
///
/// ```
/// use vibe_doc::{content::Content, html};
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">Hello</title>\n\
///        <p>A **bold** word.</p>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
///
/// let island = html::to_html(&doc, &Content::new());
/// assert!(island.contains("<h1 id=\"root\">Hello</h1>"));
/// assert!(island.contains("<p>A <strong>bold</strong> word.</p>"));
/// assert!(!island.contains("<script"));
/// ```
pub fn to_html(doc: &SpecDoc, content: &Content) -> String {
    island(doc, None, content, &Numbering::none())
}

/// Render a page as an island carrying its block numbers.
///
/// Each numbered block takes `data-p` and opens with its margin anchor —
/// `<a class="p-anchor" id="p07" href="#p07">07</a>` — so `#p07` lands on
/// the block and a reader can copy the link with one click. A section
/// heading keeps its own named anchor and takes no number: it has an
/// address already, and that address is immutable while `pNN` lives by
/// the current text (PROP-057 `##READER-NUMBERED-BLOCKS`).
///
/// `page` is the document's address in its own package — a
/// [`crate::pages::Page::rel`], e.g. `start/what-vibevm-is.xml`. It is
/// what lets a translation's `example ref` find the body it borrows:
/// example ids are unique per page, so the bundle is keyed by page and
/// then by id, and a renderer that did not know which page it was
/// rendering could only guess (`##LOC-EXAMPLE-REF`).
///
/// ```
/// use vibe_doc::{content::Content, html, numbering::number_blocks};
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
/// let island = html::to_html_numbered(
///     &doc,
///     "start/hello.xml",
///     &Content::new(),
///     &number_blocks(&doc),
/// );
/// assert!(island.contains("<p data-p=\"1\"><a class=\"p-anchor\" id=\"p01\" href=\"#p01\">01</a>one</p>"));
/// ```
pub fn to_html_numbered(
    doc: &SpecDoc,
    page: &str,
    content: &Content,
    numbering: &Numbering,
) -> String {
    island(doc, Some(page), content, numbering)
}

/// The one renderer both entry points call.
///
/// `page` is `None` for a document with no address in a package, which is
/// the only difference between them: an absent address resolves no
/// borrowed example, and the frame says so rather than showing a command
/// from some other page.
fn island(doc: &SpecDoc, page: Option<&str>, content: &Content, numbering: &Numbering) -> String {
    let mut out = String::new();
    let mut attrs: Attrs = vec![("class", "doc-page".to_owned())];
    push_status(&mut attrs, doc.status.as_ref());
    open(&mut out, 0, "article", &attrs);
    if let Some(title) = &doc.title {
        let mut attrs: Attrs = Vec::new();
        if let Some(id) = &title.id {
            attrs.push(("id", id.clone()));
        }
        line(
            &mut out,
            1,
            "h1",
            &attrs,
            &render_linked(&title.text, &prose(content, page)),
        );
    }
    blocks_html(&mut out, 1, &doc.preamble, &[], page, content, numbering);
    for (i, section) in doc.sections.iter().enumerate() {
        section_html(
            &mut out,
            1,
            section,
            2,
            &[i as u16],
            page,
            content,
            numbering,
        );
    }
    // Last inside the article, hidden, numbered nowhere: the definitions of
    // the glossary terms this page links, so a card and a screen reader both
    // have them without a request (`##READER-GLOSSARY-CARD`).
    gloss::defs(&mut out, 1, doc, page, content);
    close(&mut out, 0, "article");
    out
}

// A recursive descent carries the whole render down: where it is in the
// tree, which page it is rendering, and what was fetched for it. Bundling
// the tail into a struct would hide the recursion's own state among it.
#[allow(clippy::too_many_arguments)]
fn section_html(
    out: &mut String,
    depth: usize,
    s: &Section,
    level: usize,
    path: &[u16],
    page: Option<&str>,
    content: &Content,
    numbering: &Numbering,
) {
    let mut attrs: Attrs = Vec::new();
    if let Some(id) = &s.id {
        attrs.push(("id", id.clone()));
    }
    push_status(&mut attrs, s.status.as_ref());
    push_when(&mut attrs, s.when.as_ref());
    open(out, depth, "section", &attrs);
    // The dialect nests six levels deep at most, and so does HTML.
    let heading = format!("h{}", level.min(6));
    line(
        out,
        depth + 1,
        &heading,
        &[],
        &render_linked(&s.title, &prose(content, page)),
    );
    blocks_html(out, depth + 1, &s.blocks, path, page, content, numbering);
    for (i, sub) in s.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        section_html(
            out,
            depth + 1,
            sub,
            level + 1,
            &child,
            page,
            content,
            numbering,
        );
    }
    close(out, depth, "section");
}

fn blocks_html(
    out: &mut String,
    depth: usize,
    nodes: &[BlockNode],
    path: &[u16],
    page: Option<&str>,
    content: &Content,
    numbering: &Numbering,
) {
    for (i, node) in nodes.iter().enumerate() {
        let num = numbering.get(&BlockPath::new(path.to_vec(), i as u16));
        block(out, depth, node, page, content, num);
    }
}

fn block(
    out: &mut String,
    depth: usize,
    node: &BlockNode,
    page: Option<&str>,
    content: &Content,
    num: Option<u32>,
) {
    let mut attrs: Attrs = Vec::new();
    push_when(&mut attrs, node.when.as_ref());
    push_p(&mut attrs, num);
    match &node.block {
        Block::Paragraph(u) => {
            push_unit(&mut attrs, u);
            line(
                out,
                depth,
                "p",
                &attrs,
                &format!(
                    "{}{}",
                    anchor(num),
                    render_linked(&u.text, &prose(content, page))
                ),
            );
        }
        Block::Quote(u) => {
            push_unit(&mut attrs, u);
            open(out, depth, "blockquote", &attrs);
            anchor_line(out, depth + 1, num);
            line(
                out,
                depth + 1,
                "p",
                &[],
                &render_linked(&u.text, &prose(content, page)),
            );
            close(out, depth, "blockquote");
        }
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            open(out, depth, tag, &attrs);
            for (i, item) in items.iter().enumerate() {
                let mut item_attrs: Attrs = Vec::new();
                push_unit(&mut item_attrs, item);
                // A list may hold nothing but `<li>`, so the block's
                // anchor rides at the head of the first item rather than
                // as a sibling HTML would hoist out of the list.
                let head = if i == 0 { anchor(num) } else { String::new() };
                line(
                    out,
                    depth + 1,
                    "li",
                    &item_attrs,
                    &format!("{head}{}", render_linked(&item.text, &prose(content, page))),
                );
            }
            close(out, depth, tag);
        }
        Block::Table { rows } => table(out, depth, rows, &attrs, page, content, num),
        Block::Fence { lang, text, .. } => {
            pre_code(out, depth, &attrs, lang.as_deref(), text, num);
        }
        Block::Rule { uri, .. } => rule::fold(out, depth, uri, &attrs, content, num),
        // Through the one conversion, so an authored example and the same
        // example borrowed by a translation are the same bytes.
        Block::Example { id, fixture, .. } => example::authored(
            out,
            depth,
            id,
            fixture,
            &ExampleBody::of(&node.block).unwrap_or_default(),
            attrs,
            num,
        ),
        // A borrowed example is resolved against the page being rendered,
        // never against the id alone: ids are unique per page, and a
        // document with no address in a package borrows nothing.
        Block::ExampleRef { id } => example::borrowed(
            out,
            depth,
            id,
            page.and_then(|page| content.example(page, id)),
            attrs,
            num,
        ),
        Block::Derived { kind, reference } => {
            attrs.push(("class", "derived".to_owned()));
            attrs.push(("data-derived", kind.as_str().to_owned()));
            attrs.push(("data-ref", reference.clone()));
            let text = content.derived_text(*kind, reference);
            if text.is_none() {
                attrs.push(("data-unresolved", "true".to_owned()));
            }
            pre_code(out, depth, &attrs, Some("text"), text.unwrap_or(""), num);
        }
        Block::Note { kind, body } => {
            attrs.push(("class", "note".to_owned()));
            attrs.push(("data-note", kind.as_str().to_owned()));
            push_unit(&mut attrs, body);
            open(out, depth, "aside", &attrs);
            anchor_line(out, depth + 1, num);
            line(
                out,
                depth + 1,
                "p",
                &[],
                &render_linked(&body.text, &prose(content, page)),
            );
            close(out, depth, "aside");
        }
        Block::Figure { src, alt, caption } => {
            push_unit(&mut attrs, caption);
            open(out, depth, "figure", &attrs);
            anchor_line(out, depth + 1, num);
            let links = prose(content, page);
            // A picture must have a source, so a target this build cannot
            // place keeps the spelling the page gave it — an `img` with
            // no `src` is not an honest gap, it is a hole.
            let at = links.href(src).unwrap_or_else(|| src.clone());
            let img: Attrs = vec![("src", at), ("alt", alt.clone())];
            void(out, depth + 1, "img", &img);
            line(
                out,
                depth + 1,
                "figcaption",
                &[],
                &render_linked(&caption.text, &links),
            );
            close(out, depth, "figure");
        }
        Block::Prompt {
            id,
            text,
            needs,
            outcome,
            asserts,
        } => {
            attrs.push(("class", "prompt".to_owned()));
            attrs.push(("data-prompt", id.clone()));
            if asserts.is_empty() {
                // The page spelled `assert="none"`: an illustrative
                // prompt, and saying so is different from saying nothing.
                attrs.push(("data-assert", "none".to_owned()));
            }
            open(out, depth, "div", &attrs);
            anchor_line(out, depth + 1, num);
            pre_code(
                out,
                depth + 1,
                &[("class", "prompt-text".to_owned())],
                Some("prompt"),
                text,
                None,
            );
            if let Some(needs) = needs {
                line(
                    out,
                    depth + 1,
                    "p",
                    &[("class", "prompt-needs".to_owned())],
                    &render_linked(needs, &prose(content, page)),
                );
            }
            if let Some(outcome) = outcome {
                line(
                    out,
                    depth + 1,
                    "p",
                    &[("class", "prompt-outcome".to_owned())],
                    &render_linked(outcome, &prose(content, page)),
                );
            }
            if !asserts.is_empty() {
                open(
                    out,
                    depth + 1,
                    "ul",
                    &[("class", "prompt-asserts".to_owned())],
                );
                for a in asserts {
                    line(
                        out,
                        depth + 2,
                        "li",
                        &[],
                        &format!("<code>{}</code>", escape(a)),
                    );
                }
                close(out, depth + 1, "ul");
            }
            close(out, depth, "div");
        }
    }
}

/// A table. The first row is the header when the source had one — the
/// pivot records that by carrying it as `rows[0]`.
fn table(
    out: &mut String,
    depth: usize,
    rows: &[Vec<Unit>],
    attrs: &Attrs,
    page: Option<&str>,
    content: &Content,
    num: Option<u32>,
) {
    let links = prose(content, page);
    open(out, depth, "table", attrs);
    // A table may hold nothing but a caption, column groups and rows, so
    // the block's anchor rides in the caption — the one place HTML puts
    // arbitrary content at the head of a table.
    if num.is_some() {
        line(out, depth + 1, "caption", &[], &anchor(num));
    }
    let mut rows = rows.iter();
    if let Some(header) = rows.next() {
        open(out, depth + 1, "thead", &[]);
        open(out, depth + 2, "tr", &[]);
        for cell in header {
            let mut cell_attrs: Attrs = Vec::new();
            push_unit(&mut cell_attrs, cell);
            line(
                out,
                depth + 3,
                "th",
                &cell_attrs,
                &render_linked(&cell.text, &links),
            );
        }
        close(out, depth + 2, "tr");
        close(out, depth + 1, "thead");
    }
    open(out, depth + 1, "tbody", &[]);
    for row in rows {
        open(out, depth + 2, "tr", &[]);
        for cell in row {
            let mut cell_attrs: Attrs = Vec::new();
            push_unit(&mut cell_attrs, cell);
            line(
                out,
                depth + 3,
                "td",
                &cell_attrs,
                &render_linked(&cell.text, &links),
            );
        }
        close(out, depth + 2, "tr");
    }
    close(out, depth + 1, "tbody");
    close(out, depth, "table");
}

/// The fact a unit carries, as attributes on the block that holds it.
fn push_unit(attrs: &mut Attrs, unit: &Unit) {
    let Some(fact) = unit.fact.as_ref().filter(|f| f.is_meaningful()) else {
        return;
    };
    push_fact(attrs, fact);
}

fn push_fact(attrs: &mut Attrs, fact: &Fact) {
    if let Some(id) = &fact.id {
        attrs.push(("data-fact", id.clone()));
    }
    push_status(attrs, fact.status.as_ref());
}

/// A status as the reader needs it: the stage/state pair, and the
/// audiences — which is what the page's meta block shows
/// (`##READER-META-AND-PRINT`). The authoring extras (`action`,
/// `comment`, `ref`) stay out of the island: they are the kitchen, and
/// nothing of the kitchen reaches a reader (`##OBS-NOTHING-LEAKS`).
fn push_status(attrs: &mut Attrs, status: Option<&StatusEl>) {
    let Some(status) = status else {
        return;
    };
    attrs.push(("data-status", format!("{}/{}", status.stage, status.state)));
    if !status.audience.is_empty() {
        attrs.push((
            "data-audience",
            status
                .audience
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
}

fn push_when(attrs: &mut Attrs, when: Option<&Cond>) {
    if let Some(cond) = when {
        attrs.push(("data-when", cond.to_string()));
    }
}

#[cfg(test)]
mod tests;
