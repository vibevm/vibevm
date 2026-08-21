//! `from_xml` — the closed-dialect XML frontend (quick-xml Reader).
//!
//! The vocabulary is closed (PROP-045 ##XML-DIALECT-IS-THE-MD-SUBSET):
//! `spec`, `title`, `status`, `section`, `p`, `fact`, `list`, `item`,
//! `table`, `tr`, `td`, `fence`, `quote` — thirteen elements, nothing
//! else. A foreign element or attribute, a DTD, a processing instruction,
//! or an entity that is not one of XML's five built-ins (or a character
//! reference) is a LOUD error naming the construct and its line/column —
//! never a silent skip. That is what makes the owner's degradation law
//! hold by construction: the dialect cannot express what Markdown cannot.
//!
//! One shared id namespace (the DuplicateId law, progress-core's message
//! verbatim): the title anchor, section ids and fact ids all mint into it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_support::{
    decode_attrs, kind, last_unit_fact_id, name_of, only_attrs, pos_of, push_text,
};
use crate::doc::{Block, Section, SpecDoc, Title};
use crate::{Error, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// One collected event: elements carry their decoded attributes, text and
/// CDATA are separate (CDATA is legal only inside `<fence>`), adjacent
/// text runs are pre-merged.
pub(super) enum Ev {
    Start(String, Vec<(String, String)>),
    End(String),
    Empty(String, Vec<(String, String)>),
    Text(String),
    CData(String),
}

/// One element opening as the descent sees it: `(name, attrs, position,
/// was_empty)` — name and decoded attributes, 1-based line/column, and
/// whether the element spelled itself `<e/>`.
pub(super) type StartEl = (String, Vec<(String, String)>, (usize, usize), bool);

/// Parse dialect XML into the pivot IR.
pub fn from_xml(xml: &str) -> Result<SpecDoc> {
    let mut p = Parser::collect(xml)?;
    p.skip_ws_text()?;
    let (name, attrs, at, was_empty) = p.take_start()?;
    if name != "spec" {
        return Err(p.err(
            at,
            format!(
                "the document root must be <spec>, found <{name}> — the dialect's vocabulary is closed"
            ),
        ));
    }
    p.check_root_attrs(&attrs, at)?;
    let doc = if was_empty {
        SpecDoc::default()
    } else {
        p.spec_children()?
    };
    p.skip_ws_text()?;
    if p.i < p.evs.len() {
        let at = p.poss[p.i];
        return Err(p.err(at, "content after </spec> — one document, one root".into()));
    }
    p.check_ids()?;
    Ok(doc)
}

pub(super) struct Parser<'a> {
    pub(super) evs: Vec<Ev>,
    /// (1-based line, byte column) per event, parallel to `evs`.
    pub(super) poss: Vec<(usize, usize)>,
    pub(super) i: usize,
    /// Every minted id in document order, with its position.
    ids: Vec<(String, (usize, usize))>,
    _src: &'a str,
}

impl<'a> Parser<'a> {
    fn collect(xml: &'a str) -> Result<Parser<'a>> {
        let mut reader = Reader::from_str(xml);
        let mut evs: Vec<Ev> = Vec::new();
        let mut poss: Vec<(usize, usize)> = Vec::new();
        loop {
            let ev = reader.read_event().map_err(|e| {
                let at = pos_of(xml, reader.error_position() as usize);
                Error::at(at.0, format!("ill-formed XML: {e}"))
            })?;
            let at = pos_of(xml, reader.buffer_position() as usize);
            match ev {
                Event::Decl(_) | Event::Comment(_) => continue,
                Event::DocType(_) => {
                    return Err(Error::at(
                        at.0,
                        "the dialect forbids DTD (<!DOCTYPE>) — the vocabulary is closed",
                    ));
                }
                Event::PI(_) => {
                    return Err(Error::at(
                        at.0,
                        "the dialect forbids processing instructions — the vocabulary is closed",
                    ));
                }
                Event::Eof => break,
                Event::Start(e) => {
                    let attrs = decode_attrs(&e, at)?;
                    evs.push(Ev::Start(name_of(&e), attrs));
                    poss.push(at);
                }
                Event::End(e) => {
                    evs.push(Ev::End(
                        String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ));
                    poss.push(at);
                }
                Event::Empty(e) => {
                    let attrs = decode_attrs(&e, at)?;
                    evs.push(Ev::Empty(name_of(&e), attrs));
                    poss.push(at);
                }
                Event::Text(t) => {
                    let s = t
                        .decode()
                        .map_err(|e| Error::at(at.0, format!("undecodable text content: {e}")))?;
                    push_text(&mut evs, s.into_owned(), &mut poss, at);
                }
                Event::CData(c) => {
                    let s = c
                        .decode()
                        .map_err(|e| Error::at(at.0, format!("undecodable CDATA content: {e}")))?;
                    evs.push(Ev::CData(s.into_owned()));
                    poss.push(at);
                }
                Event::GeneralRef(r) => {
                    if let Ok(Some(ch)) = r.resolve_char_ref() {
                        push_text(&mut evs, ch.to_string(), &mut poss, at);
                        continue;
                    }
                    let name = r.decode().unwrap_or_default().into_owned();
                    let resolved = match name.as_str() {
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "amp" => Some('&'),
                        "apos" => Some('\''),
                        "quot" => Some('"'),
                        _ => None,
                    };
                    match resolved {
                        Some(ch) => push_text(&mut evs, ch.to_string(), &mut poss, at),
                        None => {
                            return Err(Error::at(
                                at.0,
                                format!(
                                    "the dialect forbids entities: &{name}; — only XML's five \
                                     built-ins and character references are allowed"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        Ok(Parser {
            evs,
            poss,
            i: 0,
            ids: Vec::new(),
            _src: xml,
        })
    }

    pub(super) fn err(&self, at: (usize, usize), message: String) -> Error {
        Error::at(at.0, format!("{message} (line {}, column {})", at.0, at.1))
    }

    /// Skip whitespace-only text events; non-ws text outside a leaf is an
    /// error (the writer never indents content).
    pub(super) fn skip_ws_text(&mut self) -> Result<()> {
        while let Some(Ev::Text(t)) = self.evs.get(self.i) {
            if !t.trim().is_empty() {
                let at = self.poss[self.i];
                return Err(self.err(
                    at,
                    format!("unexpected text content `{t}` — text lives inside the leaf elements"),
                ));
            }
            self.i += 1;
        }
        Ok(())
    }

    /// The next element opening (Start or Empty), as
    /// `(name, attrs, position, was_empty)`.
    pub(super) fn take_start(&mut self) -> Result<StartEl> {
        match self.evs.get(self.i) {
            Some(Ev::Start(n, a)) => {
                let out = (n.clone(), a.clone(), self.poss[self.i], false);
                self.i += 1;
                Ok(out)
            }
            Some(Ev::Empty(n, a)) => {
                let out = (n.clone(), a.clone(), self.poss[self.i], true);
                self.i += 1;
                Ok(out)
            }
            Some(Ev::End(n)) => {
                let at = self.poss[self.i];
                Err(self.err(at, format!("unexpected </{n}> — an element was expected")))
            }
            Some(other) => {
                let at = self.poss[self.i];
                Err(self.err(
                    at,
                    format!("an element was expected, found {}", kind(other)),
                ))
            }
            None => Err(Error::at(0, "unexpected end of input")),
        }
    }

    fn check_root_attrs(&mut self, attrs: &[(String, String)], at: (usize, usize)) -> Result<()> {
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

    fn spec_children(&mut self) -> Result<SpecDoc> {
        let mut doc = SpecDoc::default();
        let mut blocks: Vec<(Block, (usize, usize))> = Vec::new();
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
                        self.mint(id.clone(), at)?;
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
                    doc.sections.push(self.section(&attrs, at, was_empty, 2)?);
                }
                "p" | "list" | "table" | "fence" | "quote" => {
                    if have_section {
                        return Err(self.err(
                            at,
                            format!(
                                "<{name}> cannot follow a top-level <section> — Markdown preamble blocks come before sections"
                            ),
                        ));
                    }
                    have_content = true;
                    let b = self.block(&name, &attrs, at, was_empty)?;
                    blocks.push((b, at));
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
        only_attrs(attrs, &["id", "title"], "section", at, self)?;
        let id = attrs
            .iter()
            .find(|(k, _)| k == "id")
            .map(|(_, v)| v.clone());
        let Some((_, title)) = attrs.iter().find(|(k, _)| k == "title") else {
            return Err(self.err(
                at,
                "the <section> element requires a `title` attribute".into(),
            ));
        };
        let mut s = Section {
            id: id.clone(),
            title: title.clone(),
            status: None,
            blocks: Vec::new(),
            sections: Vec::new(),
        };
        if let Some(id) = &id {
            self.mint(id.clone(), at)?;
        }
        if was_empty {
            return Ok(s);
        }
        let mut blocks: Vec<(Block, (usize, usize))> = Vec::new();
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
                Some(Ev::End(n)) if n == "section" => {
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
                        .push(self.section(&attrs, at, was_empty, level + 1)?);
                }
                "p" | "list" | "table" | "fence" | "quote" => {
                    if have_subsection {
                        return Err(self.err(
                            at,
                            format!(
                                "<{name}> cannot follow a nested <section> — Markdown parent blocks come before child sections"
                            ),
                        ));
                    }
                    let b = self.block(&name, &attrs, at, was_empty)?;
                    blocks.push((b, at));
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

    pub(super) fn mint(&mut self, id: String, at: (usize, usize)) -> Result<()> {
        let spelling = format!("@fact:{id} body");
        let (parsed, _) = progress_core::parse::take_fact_id(&spelling, 0, spelling.len());
        if parsed.as_deref() != Some(id.as_str()) {
            return Err(self.err(
                at,
                format!(
                    "id `{id}` is not a Markdown-expressible fact/anchor id — use the shared progress-core anchor grammar"
                ),
            ));
        }
        self.ids.push((id, at));
        Ok(())
    }

    fn check_ids(&mut self) -> Result<()> {
        let mut seen: Vec<&(String, (usize, usize))> = Vec::new();
        for def in &self.ids {
            if let Some((_, first)) = seen.iter().find(|(s, _)| s.as_str() == def.0.as_str()) {
                return Err(Error::at(
                    def.1.0,
                    format!(
                        "fact id `@fact:{}` is defined twice in this file: lines {} and {}",
                        def.0, first.0, def.1.0
                    ),
                ));
            }
            seen.push(def);
        }
        Ok(())
    }

    /// `@fact/code` adjacency (the markup contract's binding law): the
    /// fence's `fact=` names the fact carried by the LAST unit of the
    /// immediately preceding block.
    fn validate_fence_bindings(&mut self, blocks: &[(Block, (usize, usize))]) -> Result<()> {
        for (i, (b, at)) in blocks.iter().enumerate() {
            let Block::Fence { fact: Some(id), .. } = b else {
                continue;
            };
            let bound = blocks
                .get(i.wrapping_sub(1))
                .and_then(|(p, _)| last_unit_fact_id(p));
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
