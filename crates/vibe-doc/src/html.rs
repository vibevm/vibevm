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
//!   unpinned (`##OBS-RULE-EDGE-UNPINNED`, D-27);
//! * an `example` is its command and the output it must produce, both
//!   verbatim;
//! * a `derived` block is the fence its generator built at this build.
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

use vibe_specdoc::doc::{Block, BlockNode, Cond, Fact, Section, SpecDoc, StatusEl, Unit};

use crate::content::{Content, ExampleBody};
use crate::numbering::{BlockPath, Numbering};
use emit::{anchor, anchor_line, close, line, open, pre_code, push_p, void};
use inline::{escape, render_linked};
use links::Links;

/// One attribute, already escaped.
type Attrs = Vec<(&'static str, String)>;

/// Render a page as an island, without block numbers.
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
    to_html_numbered(doc, content, &Numbering::none())
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
/// let island = html::to_html_numbered(&doc, &Content::new(), &number_blocks(&doc));
/// assert!(island.contains("<p data-p=\"1\"><a class=\"p-anchor\" id=\"p01\" href=\"#p01\">01</a>one</p>"));
/// ```
pub fn to_html_numbered(doc: &SpecDoc, content: &Content, numbering: &Numbering) -> String {
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
            &render_linked(&title.text, &Links::page(&content.base)),
        );
    }
    blocks_html(&mut out, 1, &doc.preamble, &[], content, numbering);
    for (i, section) in doc.sections.iter().enumerate() {
        section_html(&mut out, 1, section, 2, &[i as u16], content, numbering);
    }
    close(&mut out, 0, "article");
    out
}

fn section_html(
    out: &mut String,
    depth: usize,
    s: &Section,
    level: usize,
    path: &[u16],
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
        &render_linked(&s.title, &Links::page(&content.base)),
    );
    blocks_html(out, depth + 1, &s.blocks, path, content, numbering);
    for (i, sub) in s.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        section_html(out, depth + 1, sub, level + 1, &child, content, numbering);
    }
    close(out, depth, "section");
}

fn blocks_html(
    out: &mut String,
    depth: usize,
    nodes: &[BlockNode],
    path: &[u16],
    content: &Content,
    numbering: &Numbering,
) {
    for (i, node) in nodes.iter().enumerate() {
        let num = numbering.get(&BlockPath::new(path.to_vec(), i as u16));
        block(out, depth, node, content, num);
    }
}

fn block(out: &mut String, depth: usize, node: &BlockNode, content: &Content, num: Option<u32>) {
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
                    render_linked(&u.text, &Links::page(&content.base))
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
                &render_linked(&u.text, &Links::page(&content.base)),
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
                    &format!(
                        "{head}{}",
                        render_linked(&item.text, &Links::page(&content.base))
                    ),
                );
            }
            close(out, depth, tag);
        }
        Block::Table { rows } => table(out, depth, rows, &attrs, content, num),
        Block::Fence { lang, text, .. } => {
            pre_code(out, depth, &attrs, lang.as_deref(), text, num);
        }
        Block::Rule { uri, .. } => rule(out, depth, uri, &attrs, content, num),
        Block::Example {
            id,
            fixture,
            lang,
            exit,
            run,
            expect,
            stderr,
        } => {
            attrs.push(("class", "example".to_owned()));
            attrs.push(("data-example", id.clone()));
            attrs.push(("data-fixture", fixture.clone()));
            // A zero is the default and says nothing; a non-zero code is
            // the page promising that the command FAILS, which a reader
            // needs to see before typing it.
            if exit.unwrap_or(0) != 0 {
                attrs.push(("data-exit", exit.unwrap_or(0).to_string()));
            }
            let body = ExampleBody {
                lang: lang.clone(),
                exit: *exit,
                run: run.clone(),
                expect: expect.clone(),
                stderr: stderr.clone(),
            };
            example_body(out, depth, &attrs, &body, num);
        }
        Block::ExampleRef { id } => match content.examples.get(id) {
            Some(body) => {
                attrs.push(("class", "example".to_owned()));
                attrs.push(("data-example", id.clone()));
                attrs.push(("data-example-ref", id.clone()));
                if body.exit.unwrap_or(0) != 0 {
                    attrs.push(("data-exit", body.exit.unwrap_or(0).to_string()));
                }
                example_body(out, depth, &attrs, body, num);
            }
            None => {
                attrs.push(("class", "example".to_owned()));
                attrs.push(("data-example-ref", id.clone()));
                attrs.push(("data-unresolved", "true".to_owned()));
                open(out, depth, "div", &attrs);
                anchor_line(out, depth + 1, num);
                close(out, depth, "div");
            }
        },
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
                &render_linked(&body.text, &Links::page(&content.base)),
            );
            close(out, depth, "aside");
        }
        Block::Figure { src, alt, caption } => {
            push_unit(&mut attrs, caption);
            open(out, depth, "figure", &attrs);
            anchor_line(out, depth + 1, num);
            let links = Links::page(&content.base);
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
                    &render_linked(needs, &Links::page(&content.base)),
                );
            }
            if let Some(outcome) = outcome {
                line(
                    out,
                    depth + 1,
                    "p",
                    &[("class", "prompt-outcome".to_owned())],
                    &render_linked(outcome, &Links::page(&content.base)),
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

/// A cited rule: the fact's current text, its address, and the language
/// the specification is written in.
fn rule(
    out: &mut String,
    depth: usize,
    uri: &str,
    outer: &Attrs,
    content: &Content,
    num: Option<u32>,
) {
    let mut attrs = outer.clone();
    attrs.push(("class", "rule".to_owned()));
    let found = content.rules.get(uri);
    if found.is_none() {
        attrs.push(("data-unresolved", "true".to_owned()));
    }
    open(out, depth, "blockquote", &attrs);
    anchor_line(out, depth + 1, num);

    // The citation goes to the resolver rather than straight down the
    // address map: the pipeline cannot know what the mount in front of it
    // carries, and the resolver is the one address that can
    // (`##SEO-MANIFEST-AND-RESOLVER`).
    let links = Links::quoting(&content.base, uri);
    let mut link_attrs: Attrs = vec![("class", "rule".to_owned())];
    if let Some(href) = links.citation(uri) {
        link_attrs.push(("href", href));
    }
    link_attrs.push(("data-uri", uri.to_owned()));
    if let Some(found) = found {
        link_attrs.push(("lang", found.lang.clone()));
    }
    // The address itself is the honest body when the text is not in
    // hand: a reader can still follow it, and a blank quotation would
    // read as a rule that says nothing. The links INSIDE the text belong
    // to the document quoted, not to this page, so they are read against
    // its address.
    let body = found
        .map(|f| render_linked(&f.text, &links))
        .unwrap_or_else(|| escape(uri));
    line(out, depth + 1, "a", &link_attrs, &body);
    close(out, depth, "blockquote");
}

/// An example's two (or three) verbatim blocks.
fn example_body(
    out: &mut String,
    depth: usize,
    attrs: &Attrs,
    body: &ExampleBody,
    num: Option<u32>,
) {
    open(out, depth, "div", attrs);
    anchor_line(out, depth + 1, num);
    pre_code(
        out,
        depth + 1,
        &[("class", "example-run".to_owned())],
        Some(body.lang.as_deref().unwrap_or("sh")),
        &body.run,
        None,
    );
    pre_code(
        out,
        depth + 1,
        &[("class", "example-output".to_owned())],
        None,
        &body.expect,
        None,
    );
    if let Some(err) = &body.stderr {
        pre_code(
            out,
            depth + 1,
            &[("class", "example-stderr".to_owned())],
            None,
            err,
            None,
        );
    }
    close(out, depth, "div");
}

/// A table. The first row is the header when the source had one — the
/// pivot records that by carrying it as `rows[0]`.
fn table(
    out: &mut String,
    depth: usize,
    rows: &[Vec<Unit>],
    attrs: &Attrs,
    content: &Content,
    num: Option<u32>,
) {
    let links = Links::page(&content.base);
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
