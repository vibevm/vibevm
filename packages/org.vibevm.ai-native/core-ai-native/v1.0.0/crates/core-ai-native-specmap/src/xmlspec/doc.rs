//! Document-level descent for the closed XML dialect: `<spec>` and its
//! children, `<section>` nesting, and the `<status>` element. The block/
//! leaf descent lives in the sibling `blocks` module; this one owns the
//! tree's spine.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use super::blocks::{block, status_element};
use super::reader::{Ev, NS, Parser, Violation, attr, only_attrs, validate_fence_bindings};
use super::{XBlock, XDoc, XSection, XTitle};

/// The `<spec>` root and everything under it.
pub(super) fn document(p: &mut Parser) -> Result<XDoc, Violation> {
    p.skip_ws_text()?;
    let (name, attrs, at, was_empty) = p.take_start()?;
    if name != "spec" {
        return Err(Violation::at(
            at,
            format!(
                "the document root must be <spec>, found <{name}> — the dialect's vocabulary is closed"
            ),
        ));
    }
    check_root_attrs(&attrs, at)?;
    if was_empty {
        return Ok(XDoc::default());
    }
    spec_children(p)
}

fn check_root_attrs(attrs: &[(String, String)], at: usize) -> Result<(), Violation> {
    if attrs.is_empty() {
        return Err(Violation::at(
            at,
            format!("the <spec> element requires xmlns=\"{NS}\""),
        ));
    }
    for (k, v) in attrs {
        if k != "xmlns" {
            return Err(Violation::at(
                at,
                format!(
                    "the <spec> element has no `{k}` attribute — the dialect's vocabulary is closed"
                ),
            ));
        }
        if v != NS {
            return Err(Violation::at(
                at,
                format!("the <spec> namespace is xmlns=\"{NS}\", found `{v}`"),
            ));
        }
    }
    Ok(())
}

fn spec_children(p: &mut Parser) -> Result<XDoc, Violation> {
    let mut doc = XDoc::default();
    let mut blocks: Vec<XBlock> = Vec::new();
    let mut have_content = false;
    let mut have_section = false;
    loop {
        p.skip_ws_text()?;
        match p.evs.get(p.i) {
            None => {
                return Err(Violation::at(
                    0,
                    "unexpected end of input — <spec> never closed",
                ));
            }
            Some(Ev::End(n)) if n == "spec" => {
                doc.end_line = p.poss[p.i] as u32;
                p.i += 1;
                break;
            }
            Some(_) => {}
        }
        let (name, attrs, at, was_empty) = p.take_start()?;
        match name.as_str() {
            "title" => {
                if doc.title.is_some() {
                    return Err(Violation::at(at, "one <title> per document"));
                }
                if have_content {
                    return Err(Violation::at(
                        at,
                        "the dialect puts <title> before any block or section",
                    ));
                }
                only_attrs(&attrs, &["id"], "title", at)?;
                let id = attr(&attrs, "id").map(str::to_string);
                let text = super::blocks::leaf_text(p, "title", was_empty)?;
                doc.title = Some(XTitle {
                    text,
                    id,
                    line: at as u32,
                });
            }
            "status" => {
                if doc.status.is_some() {
                    return Err(Violation::at(at, "one document <status> per document"));
                }
                if have_content {
                    return Err(Violation::at(
                        at,
                        "the document <status> comes before any block or section",
                    ));
                }
                doc.status = Some(status_element(p, &attrs, at, was_empty)?);
            }
            "section" => {
                have_content = true;
                have_section = true;
                doc.sections.push(section(p, &attrs, at, was_empty, 2)?);
            }
            "p" | "list" | "table" | "fence" | "quote" => {
                if have_section {
                    return Err(Violation::at(
                        at,
                        format!(
                            "<{name}> cannot follow a top-level <section> — Markdown preamble blocks come before sections"
                        ),
                    ));
                }
                have_content = true;
                blocks.push(block(p, &name, &attrs, at, was_empty)?);
            }
            other => {
                return Err(Violation::at(
                    at,
                    format!(
                        "the dialect has no <{other}> element (inside <spec>) — the vocabulary is closed"
                    ),
                ));
            }
        }
    }
    validate_fence_bindings(&blocks)?;
    doc.preamble = blocks;
    Ok(doc)
}

fn section(
    p: &mut Parser,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
    level: usize,
) -> Result<XSection, Violation> {
    if level > 6 {
        return Err(Violation::at(
            at,
            "section nesting deeper than five levels is not Markdown-expressible (ATX headings stop at H6)",
        ));
    }
    only_attrs(attrs, &["id", "title"], "section", at)?;
    let id = attr(attrs, "id").map(str::to_string);
    let Some(title) = attr(attrs, "title") else {
        return Err(Violation::at(
            at,
            "the <section> element requires a `title` attribute",
        ));
    };
    let title = title.to_string();
    if was_empty {
        return Ok(XSection {
            id,
            title,
            line: at as u32,
            end_line: at as u32,
            status: None,
            blocks: Vec::new(),
            sections: Vec::new(),
        });
    }
    let mut s = XSection {
        id,
        title,
        line: at as u32,
        end_line: at as u32,
        status: None,
        blocks: Vec::new(),
        sections: Vec::new(),
    };
    let mut first = true;
    let mut have_subsection = false;
    let mut blocks: Vec<XBlock> = Vec::new();
    loop {
        p.skip_ws_text()?;
        match p.evs.get(p.i) {
            None => {
                return Err(Violation::at(
                    0,
                    format!(
                        "unexpected end of input — section {:?} never closed",
                        s.title
                    ),
                ));
            }
            Some(Ev::End(n)) if n == "section" => {
                s.end_line = p.poss[p.i] as u32;
                p.i += 1;
                break;
            }
            Some(_) => {}
        }
        let (name, attrs, at, was_empty) = p.take_start()?;
        match name.as_str() {
            "status" if first => {
                s.status = Some(status_element(p, &attrs, at, was_empty)?);
            }
            "status" => {
                return Err(Violation::at(
                    at,
                    "a section <status> must be the section's first child — that is where the Markdown form can place it",
                ));
            }
            "section" => {
                have_subsection = true;
                s.sections
                    .push(section(p, &attrs, at, was_empty, level + 1)?);
            }
            "p" | "list" | "table" | "fence" | "quote" => {
                if have_subsection {
                    return Err(Violation::at(
                        at,
                        format!(
                            "<{name}> cannot follow a nested <section> — Markdown parent blocks come before child sections"
                        ),
                    ));
                }
                blocks.push(block(p, &name, &attrs, at, was_empty)?);
            }
            other => {
                return Err(Violation::at(
                    at,
                    format!(
                        "the dialect has no <{other}> element (inside <section>) — the vocabulary is closed"
                    ),
                ));
            }
        }
        first = false;
    }
    validate_fence_bindings(&blocks)?;
    s.blocks = blocks;
    Ok(s)
}
