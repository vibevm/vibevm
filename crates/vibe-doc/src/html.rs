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

use vibe_specdoc::doc::{Block, BlockNode, Cond, Fact, Section, SpecDoc, StatusEl, Unit};

use crate::content::Content;
use inline::{escape, escape_attr, render};

/// One attribute, already escaped.
type Attrs = Vec<(&'static str, String)>;

/// Render a page as an island.
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
    let mut out = String::new();
    let mut attrs: Attrs = vec![("class", "doc-page".to_owned())];
    push_status(&mut attrs, doc.status.as_ref());
    open(&mut out, 0, "article", &attrs);
    if let Some(title) = &doc.title {
        let mut attrs: Attrs = Vec::new();
        if let Some(id) = &title.id {
            attrs.push(("id", id.clone()));
        }
        line(&mut out, 1, "h1", &attrs, &render(&title.text));
    }
    for node in &doc.preamble {
        block(&mut out, 1, node, content);
    }
    for section in &doc.sections {
        section_html(&mut out, 1, section, 2, content);
    }
    close(&mut out, 0, "article");
    out
}

fn section_html(out: &mut String, depth: usize, s: &Section, level: usize, content: &Content) {
    let mut attrs: Attrs = Vec::new();
    if let Some(id) = &s.id {
        attrs.push(("id", id.clone()));
    }
    push_status(&mut attrs, s.status.as_ref());
    push_when(&mut attrs, s.when.as_ref());
    open(out, depth, "section", &attrs);
    // The dialect nests six levels deep at most, and so does HTML.
    let heading = format!("h{}", level.min(6));
    line(out, depth + 1, &heading, &[], &render(&s.title));
    for node in &s.blocks {
        block(out, depth + 1, node, content);
    }
    for sub in &s.sections {
        section_html(out, depth + 1, sub, level + 1, content);
    }
    close(out, depth, "section");
}

fn block(out: &mut String, depth: usize, node: &BlockNode, content: &Content) {
    let mut attrs: Attrs = Vec::new();
    push_when(&mut attrs, node.when.as_ref());
    match &node.block {
        Block::Paragraph(u) => {
            push_unit(&mut attrs, u);
            line(out, depth, "p", &attrs, &render(&u.text));
        }
        Block::Quote(u) => {
            push_unit(&mut attrs, u);
            open(out, depth, "blockquote", &attrs);
            line(out, depth + 1, "p", &[], &render(&u.text));
            close(out, depth, "blockquote");
        }
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            open(out, depth, tag, &attrs);
            for item in items {
                let mut item_attrs: Attrs = Vec::new();
                push_unit(&mut item_attrs, item);
                line(out, depth + 1, "li", &item_attrs, &render(&item.text));
            }
            close(out, depth, tag);
        }
        Block::Table { rows } => table(out, depth, rows, &attrs),
        Block::Fence { lang, text, .. } => {
            pre_code(out, depth, &attrs, lang.as_deref(), text);
        }
        Block::Rule { uri, .. } => rule(out, depth, uri, &attrs, content),
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
            example_body(
                out,
                depth,
                &attrs,
                lang.as_deref(),
                run,
                expect,
                stderr.as_deref(),
            );
        }
        Block::ExampleRef { id } => match content.examples.get(id) {
            Some(body) => {
                attrs.push(("class", "example".to_owned()));
                attrs.push(("data-example", id.clone()));
                attrs.push(("data-example-ref", id.clone()));
                if body.exit.unwrap_or(0) != 0 {
                    attrs.push(("data-exit", body.exit.unwrap_or(0).to_string()));
                }
                example_body(
                    out,
                    depth,
                    &attrs,
                    body.lang.as_deref(),
                    &body.run,
                    &body.expect,
                    body.stderr.as_deref(),
                );
            }
            None => {
                attrs.push(("class", "example".to_owned()));
                attrs.push(("data-example-ref", id.clone()));
                attrs.push(("data-unresolved", "true".to_owned()));
                open(out, depth, "div", &attrs);
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
            pre_code(out, depth, &attrs, Some("text"), text.unwrap_or(""));
        }
        Block::Note { kind, body } => {
            attrs.push(("class", "note".to_owned()));
            attrs.push(("data-note", kind.as_str().to_owned()));
            push_unit(&mut attrs, body);
            open(out, depth, "aside", &attrs);
            line(out, depth + 1, "p", &[], &render(&body.text));
            close(out, depth, "aside");
        }
        Block::Figure { src, alt, caption } => {
            push_unit(&mut attrs, caption);
            open(out, depth, "figure", &attrs);
            let img: Attrs = vec![("src", src.clone()), ("alt", alt.clone())];
            void(out, depth + 1, "img", &img);
            line(out, depth + 1, "figcaption", &[], &render(&caption.text));
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
            pre_code(
                out,
                depth + 1,
                &[("class", "prompt-text".to_owned())],
                Some("prompt"),
                text,
            );
            if let Some(needs) = needs {
                line(
                    out,
                    depth + 1,
                    "p",
                    &[("class", "prompt-needs".to_owned())],
                    &render(needs),
                );
            }
            if let Some(outcome) = outcome {
                line(
                    out,
                    depth + 1,
                    "p",
                    &[("class", "prompt-outcome".to_owned())],
                    &render(outcome),
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
fn rule(out: &mut String, depth: usize, uri: &str, outer: &Attrs, content: &Content) {
    let mut attrs = outer.clone();
    attrs.push(("class", "rule".to_owned()));
    let found = content.rules.get(uri);
    if found.is_none() {
        attrs.push(("data-unresolved", "true".to_owned()));
    }
    open(out, depth, "blockquote", &attrs);

    let mut link_attrs: Attrs = vec![("class", "rule".to_owned())];
    if let Some(href) = content.link(uri) {
        link_attrs.push(("href", href));
    }
    link_attrs.push(("data-uri", uri.to_owned()));
    if let Some(found) = found {
        link_attrs.push(("lang", found.lang.clone()));
    }
    // The address itself is the honest body when the text is not in
    // hand: a reader can still follow it, and a blank quotation would
    // read as a rule that says nothing.
    let body = found
        .map(|f| render(&f.text))
        .unwrap_or_else(|| escape(uri));
    line(out, depth + 1, "a", &link_attrs, &body);
    close(out, depth, "blockquote");
}

/// An example's two (or three) verbatim blocks.
fn example_body(
    out: &mut String,
    depth: usize,
    attrs: &Attrs,
    lang: Option<&str>,
    run: &str,
    expect: &str,
    stderr: Option<&str>,
) {
    open(out, depth, "div", attrs);
    pre_code(
        out,
        depth + 1,
        &[("class", "example-run".to_owned())],
        Some(lang.unwrap_or("sh")),
        run,
    );
    pre_code(
        out,
        depth + 1,
        &[("class", "example-output".to_owned())],
        None,
        expect,
    );
    if let Some(err) = stderr {
        pre_code(
            out,
            depth + 1,
            &[("class", "example-stderr".to_owned())],
            None,
            err,
        );
    }
    close(out, depth, "div");
}

/// A table. The first row is the header when the source had one — the
/// pivot records that by carrying it as `rows[0]`.
fn table(out: &mut String, depth: usize, rows: &[Vec<Unit>], attrs: &Attrs) {
    open(out, depth, "table", attrs);
    let mut rows = rows.iter();
    if let Some(header) = rows.next() {
        open(out, depth + 1, "thead", &[]);
        open(out, depth + 2, "tr", &[]);
        for cell in header {
            let mut cell_attrs: Attrs = Vec::new();
            push_unit(&mut cell_attrs, cell);
            line(out, depth + 3, "th", &cell_attrs, &render(&cell.text));
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
            line(out, depth + 3, "td", &cell_attrs, &render(&cell.text));
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

// --- the emitters -------------------------------------------------------
//
// Indentation is structural and stable, so two builds of one page are
// one file: a diff of two islands then shows what MOVED, not how the
// writer felt about whitespace.

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

/// An element whose whole body fits on its own line.
fn line(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)], body: &str) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}>{body}</{tag}>\n", attr_text(attrs)));
}

fn void(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)]) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{} />\n", attr_text(attrs)));
}

/// A preformatted block, opening tag to closing tag on ONE line.
///
/// `<pre>` preserves whitespace, so the pretty-printing that makes the
/// rest of the island readable would become content here: an indented
/// `<code>` inside an indented `<pre>` shows a reader six spaces and two
/// blank lines that no page ever wrote. Every byte between the fences is
/// the author's; the indentation stops at the opening tag.
fn pre_code(
    out: &mut String,
    depth: usize,
    pre_attrs: &[(&'static str, String)],
    lang: Option<&str>,
    text: &str,
) {
    let mut code_attrs: Attrs = Vec::new();
    if let Some(lang) = lang {
        code_attrs.push(("class", format!("language-{lang}")));
    }
    indent(out, depth);
    out.push_str(&format!(
        "<pre{}><code{}>{}</code></pre>\n",
        attr_text(pre_attrs),
        attr_text(&code_attrs),
        escape(text)
    ));
}

#[cfg(test)]
mod tests;
