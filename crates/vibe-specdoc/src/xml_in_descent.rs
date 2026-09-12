//! The structural descent: the root's attributes, the children of
//! `<spec>`, one section, and one block in its slot.
//!
//! This is where the closed vocabulary is actually enforced — every
//! foreign name reaching a position is a loud error naming the construct
//! and its line/column, never a silent skip — and where the two
//! genre-blind discriminators are consulted: a named section always
//! carries `title=`, a documentation block never does
//! (##DOC-VOCAB-DISCRIMINATOR).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser, is_doc_block};
use super::xml_support::{last_unit_fact_id, only_attrs, only_attrs_slot, take_when};
use crate::doc::{Block, BlockNode, Section, SpecDoc, Title, Vocabulary};
use crate::{Error, Result};

impl<'a> Parser<'a> {
    pub(super) fn check_root_attrs(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
    ) -> Result<()> {
        if attrs.is_empty() {
            return Err(self.err(
                at,
                format!(
                    "the <spec> element requires xmlns=\"{}\"",
                    super::xml_out::NS
                ),
            ));
        }
        for (k, v) in attrs {
            if k != "xmlns" {
                return Err(self.err(
                    at,
                    format!(
                        "the <spec> element has no `{k}` attribute — the dialect's vocabulary is closed"
                    ),
                ));
            }
            if v != super::xml_out::NS {
                return Err(self.err(
                    at,
                    format!(
                        "the <spec> namespace is xmlns=\"{}\", found `{v}`",
                        super::xml_out::NS
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn spec_children(&mut self) -> Result<SpecDoc> {
        let mut doc = SpecDoc::default();
        let mut blocks: Vec<(BlockNode, (usize, usize))> = Vec::new();
        let mut have_content = false;
        let mut have_section = false;
        loop {
            self.skip_ws_text()?;
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        "unexpected end of input — <spec> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "spec" => {
                    self.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (name, attrs, at, was_empty) = self.take_start()?;
            match name.as_str() {
                "title" => {
                    if doc.title.is_some() {
                        return Err(self.err(at, "one <title> per document".into()));
                    }
                    if have_content {
                        return Err(self.err(
                            at,
                            "the dialect puts <title> before any block or section".into(),
                        ));
                    }
                    only_attrs(&attrs, &["id"], "title", at, self)?;
                    let mut title = Title {
                        text: String::new(),
                        id: None,
                    };
                    if let Some((_, v)) = attrs.iter().find(|(k, _)| k == "id") {
                        title.id = Some(v.clone());
                    }
                    title.text = self.leaf_text("title", was_empty)?;
                    if let Some(id) = &title.id {
                        self.mint_heading(id.clone(), at)?;
                    }
                    doc.title = Some(title);
                }
                "status" => {
                    if doc.status.is_some() {
                        return Err(self.err(at, "one document <status> per document".into()));
                    }
                    if have_content {
                        return Err(self.err(
                            at,
                            "the document <status> comes before any block or section".into(),
                        ));
                    }
                    doc.status = Some(self.status_element(&attrs, at, was_empty)?);
                }
                "section" => {
                    have_content = true;
                    have_section = true;
                    doc.sections
                        .push(self.section("section", &attrs, at, was_empty, 2)?);
                }
                "p" | "list" | "facts" | "table" | "fence" | "quote" => {
                    if have_section {
                        return Err(self.err(
                            at,
                            format!(
                                "<{name}> cannot follow a top-level <section> — Markdown preamble blocks come before sections"
                            ),
                        ));
                    }
                    have_content = true;
                    let node = self.block_node(&name, &attrs, at, was_empty)?;
                    blocks.push((node, at));
                }
                // The documentation genre, before the named-section arm:
                // the discriminator (no `title=`) already told them apart.
                other if is_doc_block(other, &attrs) => {
                    if self.vocab != Vocabulary::Doc {
                        return Err(self.doc_genre_closed(other, at));
                    }
                    if have_section {
                        return Err(self.err(
                            at,
                            format!(
                                "<{other}> cannot follow a top-level <section> — Markdown preamble blocks come before sections"
                            ),
                        ));
                    }
                    have_content = true;
                    let node = self.block_node(other, &attrs, at, was_empty)?;
                    blocks.push((node, at));
                }
                other
                    if super::xml_out::anchor_is_elementable(other)
                        && attrs.iter().any(|(name, _)| name == "title") =>
                {
                    have_content = true;
                    have_section = true;
                    doc.sections
                        .push(self.section(other, &attrs, at, was_empty, 2)?);
                }
                other => {
                    return Err(self.err(
                        at,
                        format!(
                            "the dialect has no <{other}> element (inside <spec>) — the vocabulary is closed"
                        ),
                    ));
                }
            }
        }
        self.validate_fence_bindings(&blocks)?;
        doc.preamble = blocks.into_iter().map(|(b, _)| b).collect();
        Ok(doc)
    }

    fn section(
        &mut self,
        element_name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
        level: usize,
    ) -> Result<Section> {
        if level > 6 {
            return Err(self.err(
                at,
                "section nesting deeper than five levels is not Markdown-expressible (ATX headings stop at H6)".into(),
            ));
        }
        let id = if element_name == "section" {
            only_attrs_slot(attrs, &["id", "title"], "section", at, self)?;
            attrs
                .iter()
                .find(|(k, _)| k == "id")
                .map(|(_, v)| v.clone())
        } else {
            only_attrs_slot(attrs, &["title"], element_name, at, self)?;
            Some(element_name.to_string())
        };
        let when = take_when(attrs, at, self)?;
        let Some((_, title)) = attrs.iter().find(|(k, _)| k == "title") else {
            return Err(self.err(
                at,
                format!("the <{element_name}> element requires a `title` attribute"),
            ));
        };
        let mut s = Section {
            id: id.clone(),
            title: title.clone(),
            status: None,
            when,
            blocks: Vec::new(),
            sections: Vec::new(),
        };
        if let Some(id) = &id {
            self.mint_heading(id.clone(), at)?;
        }
        if was_empty {
            return Ok(s);
        }
        let mut blocks: Vec<(BlockNode, (usize, usize))> = Vec::new();
        let mut first = true;
        let mut have_subsection = false;
        loop {
            self.skip_ws_text()?;
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        format!("unexpected end of input — section {title:?} never closed"),
                    ));
                }
                Some(Ev::End(n)) if n == element_name => {
                    self.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (name, attrs, at, was_empty) = self.take_start()?;
            match name.as_str() {
                "status" if first => {
                    s.status = Some(self.status_element(&attrs, at, was_empty)?);
                }
                "status" => {
                    return Err(self.err(
                        at,
                        "a section <status> must be the section's first child — that is where the Markdown form can place it".into(),
                    ));
                }
                "section" => {
                    have_subsection = true;
                    s.sections
                        .push(self.section("section", &attrs, at, was_empty, level + 1)?);
                }
                "p" | "list" | "facts" | "table" | "fence" | "quote" => {
                    if have_subsection {
                        return Err(self.err(
                            at,
                            format!(
                                "<{name}> cannot follow a nested <section> — Markdown parent blocks come before child sections"
                            ),
                        ));
                    }
                    let node = self.block_node(&name, &attrs, at, was_empty)?;
                    blocks.push((node, at));
                }
                other if is_doc_block(other, &attrs) => {
                    if self.vocab != Vocabulary::Doc {
                        return Err(self.doc_genre_closed(other, at));
                    }
                    if have_subsection {
                        return Err(self.err(
                            at,
                            format!(
                                "<{other}> cannot follow a nested <section> — Markdown parent blocks come before child sections"
                            ),
                        ));
                    }
                    let node = self.block_node(other, &attrs, at, was_empty)?;
                    blocks.push((node, at));
                }
                other
                    if super::xml_out::anchor_is_elementable(other)
                        && attrs.iter().any(|(name, _)| name == "title") =>
                {
                    have_subsection = true;
                    s.sections
                        .push(self.section(other, &attrs, at, was_empty, level + 1)?);
                }
                other => {
                    return Err(self.err(
                        at,
                        format!(
                            "the dialect has no <{other}> element (inside <section>) — the vocabulary is closed"
                        ),
                    ));
                }
            }
            first = false;
        }
        self.validate_fence_bindings(&blocks)?;
        s.blocks = blocks.into_iter().map(|(b, _)| b).collect();
        Ok(s)
    }

    /// One block in its slot: the block itself plus the slot's `when`
    /// (##DOC-VOCAB-WHEN-SLOT). The attribute is validated inside
    /// `block` (which knows the element's own attribute set) and read
    /// back out here, so the condition never becomes a per-variant field.
    fn block_node(
        &mut self,
        name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<BlockNode> {
        let block = self.block(name, attrs, at, was_empty)?;
        let when = take_when(attrs, at, self)?;
        Ok(BlockNode { when, block })
    }

    /// `@fact/code` adjacency (the markup contract's binding law): the
    /// fence's `fact=` names the fact carried by the LAST unit of the
    /// immediately preceding block.
    fn validate_fence_bindings(&mut self, blocks: &[(BlockNode, (usize, usize))]) -> Result<()> {
        for (i, (node, at)) in blocks.iter().enumerate() {
            let Block::Fence { fact: Some(id), .. } = &node.block else {
                continue;
            };
            let bound = blocks
                .get(i.wrapping_sub(1))
                .and_then(|(p, _)| last_unit_fact_id(&p.block));
            if bound != Some(id.as_str()) {
                return Err(self.err(
                    *at,
                    format!(
                        "`fact=\"{id}\"` must name the fact of the immediately preceding \
                         <p>/<list>/<quote> unit — the @fact/code binding is adjacent by law"
                    ),
                ));
            }
        }
        Ok(())
    }
}
