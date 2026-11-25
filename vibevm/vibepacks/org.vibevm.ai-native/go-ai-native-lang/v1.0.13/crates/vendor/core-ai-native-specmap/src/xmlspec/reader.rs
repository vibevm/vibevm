//! Event collection for the XML dialect reader — the quick-xml spine of
//! `xmlspec`. The dialect's structural laws live in the sibling descent
//! modules (`doc`, `blocks`); this one owns the event stream: collection,
//! positions, attribute decoding, and the parser primitives the descent
//! composes.
//!
//! A violation is a [`Violation`] carrying the 1-based native source line —
//! the caller degrades it to the engine's loud `xml-dialect` warning and
//! drops the document (the scanner never hard-fails; the crate's own design
//! rule). Positions come from the quick-xml buffer offset, translated to a
//! line by counting the newlines before it.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::{XBlock, XDoc};

/// The dialect's root namespace (PROP-045 §2 `DIALECT-SKETCH`).
pub(super) const NS: &str = "https://vibevm.org/spec/1";

/// One dialect violation: the native line it sits on and what is wrong.
pub(super) struct Violation {
    pub line: u32,
    pub message: String,
}

impl Violation {
    pub(super) fn at(line: usize, message: impl Into<String>) -> Violation {
        Violation {
            line: line as u32,
            message: message.into(),
        }
    }
}

/// One collected event; adjacent text runs are pre-merged.
pub(super) enum Ev {
    Start(String, Vec<(String, String)>),
    End(String),
    Empty(String, Vec<(String, String)>),
    Text(String),
    CData(String),
}

impl Ev {
    /// The `<shape>` name of an event, for messages.
    pub(super) fn what(&self) -> String {
        match self {
            Ev::Start(n, _) => format!("<{n}>"),
            Ev::End(n) => format!("</{n}>"),
            Ev::Empty(n, _) => format!("<{n}/>"),
            Ev::Text(t) => format!("text `{t}`"),
            Ev::CData(_) => "CDATA".to_string(),
        }
    }
}

/// The collected event stream with a 1-based native line per event.
pub(super) struct Parser {
    pub(super) evs: Vec<Ev>,
    pub(super) poss: Vec<usize>,
    pub(super) i: usize,
}

/// Read one document: the full closed-vocabulary walk. `Ok(XDoc)` or the
/// first `Violation` (the caller degrades it to a warning).
pub(super) fn read_document(xml: &str) -> Result<XDoc, Violation> {
    let mut p = collect(xml)?;
    let doc = super::doc::document(&mut p)?;
    // Trailing whitespace after </spec> is layout, not content.
    p.skip_ws_text()?;
    if p.i < p.evs.len() {
        return Err(Violation::at(
            p.poss[p.i],
            "content after </spec> — one document, one root",
        ));
    }
    Ok(doc)
}

/// 1-based line of a byte offset in the source.
fn line_of(xml: &str, off: usize) -> usize {
    xml[..off.min(xml.len())].matches('\n').count() + 1
}

fn collect(xml: &str) -> Result<Parser, Violation> {
    let mut reader = Reader::from_str(xml);
    let mut evs: Vec<Ev> = Vec::new();
    let mut poss: Vec<usize> = Vec::new();
    loop {
        let ev = match reader.read_event() {
            Ok(ev) => ev,
            Err(e) => {
                let at = line_of(xml, reader.error_position() as usize);
                return Err(Violation::at(at, format!("ill-formed XML: {e}")));
            }
        };
        let at = line_of(xml, reader.buffer_position() as usize);
        match ev {
            Event::Decl(_) | Event::Comment(_) => continue,
            Event::DocType(_) => {
                return Err(Violation::at(
                    at,
                    "the dialect forbids DTD (<!DOCTYPE>) — the vocabulary is closed",
                ));
            }
            Event::PI(_) => {
                return Err(Violation::at(
                    at,
                    "the dialect forbids processing instructions — the vocabulary is closed",
                ));
            }
            Event::Eof => break,
            Event::Start(e) => {
                let attrs = decode_attrs(&e, at)?;
                push_ev(&mut evs, Ev::Start(name_of(&e), attrs), &mut poss, at);
            }
            Event::End(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                push_ev(&mut evs, Ev::End(name), &mut poss, at);
            }
            Event::Empty(e) => {
                let attrs = decode_attrs(&e, at)?;
                push_ev(&mut evs, Ev::Empty(name_of(&e), attrs), &mut poss, at);
            }
            Event::Text(t) => match t.decode() {
                Ok(s) => push_ev(&mut evs, Ev::Text(s.into_owned()), &mut poss, at),
                Err(_) => return Err(Violation::at(at, "undecodable text content")),
            },
            Event::CData(c) => match c.decode() {
                Ok(s) => push_ev(&mut evs, Ev::CData(s.into_owned()), &mut poss, at),
                Err(_) => return Err(Violation::at(at, "undecodable CDATA content")),
            },
            Event::GeneralRef(r) => match general_ref(&r) {
                Ok(text) => push_ev(&mut evs, Ev::Text(text), &mut poss, at),
                Err(name) => {
                    return Err(Violation::at(
                        at,
                        format!(
                            "the dialect forbids entities: &{name}; — only XML's five \
                             built-ins and character references are allowed"
                        ),
                    ));
                }
            },
        }
    }
    Ok(Parser { evs, poss, i: 0 })
}

/// Resolve a general entity reference to text, or name the refused entity.
fn general_ref(r: &quick_xml::events::BytesRef<'_>) -> Result<String, String> {
    if let Ok(Some(ch)) = r.resolve_char_ref() {
        return Ok(ch.to_string());
    }
    let name = r.decode().unwrap_or_default().into_owned();
    let ch = match name.as_str() {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "apos" => '\'',
        "quot" => '"',
        _ => return Err(name),
    };
    Ok(ch.to_string())
}

/// Push an event, merging adjacent text runs.
fn push_ev(evs: &mut Vec<Ev>, ev: Ev, poss: &mut Vec<usize>, at: usize) {
    if let Ev::Text(t) = &ev
        && let Some(Ev::Text(prev)) = evs.last_mut()
    {
        prev.push_str(t);
        return;
    }
    poss.push(at);
    evs.push(ev);
}

fn name_of(e: &quick_xml::events::BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

fn decode_attrs(
    e: &quick_xml::events::BytesStart<'_>,
    at: usize,
) -> Result<Vec<(String, String)>, Violation> {
    let mut out = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|_| Violation::at(at, "ill-formed attribute"))?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let raw = String::from_utf8_lossy(&a.value).into_owned();
        let value = unescape(&raw)
            .map_err(|_| Violation::at(at, format!("bad escape in attribute `{key}`")))?
            .into_owned();
        out.push((key, value));
    }
    Ok(out)
}

impl Parser {
    /// The current position's native line (0 at end of input).
    pub(super) fn here(&self) -> usize {
        self.poss.get(self.i).copied().unwrap_or(0)
    }

    /// Skip whitespace-only text events; non-ws text outside a leaf is a
    /// violation (the canonical writer never indents content).
    pub(super) fn skip_ws_text(&mut self) -> Result<(), Violation> {
        while let Some(Ev::Text(t)) = self.evs.get(self.i) {
            if !t.trim().is_empty() {
                return Err(Violation::at(
                    self.here(),
                    format!("unexpected text content `{t}` — text lives inside the leaf elements"),
                ));
            }
            self.i += 1;
        }
        Ok(())
    }

    /// The next element opening (Start or Empty) as
    /// `(name, attrs, native line, was_empty)`.
    pub(super) fn take_start(&mut self) -> Result<StartEl, Violation> {
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
            Some(Ev::End(n)) => Err(Violation::at(
                self.poss[self.i],
                format!("unexpected </{n}> — an element was expected"),
            )),
            Some(other) => Err(Violation::at(
                self.poss[self.i],
                format!("an element was expected, found {}", other.what()),
            )),
            None => Err(Violation::at(0, "unexpected end of input")),
        }
    }
}

/// An element opening as the descent sees it:
/// `(name, attrs, native line, was_empty)`.
pub(super) type StartEl = (String, Vec<(String, String)>, usize, bool);

/// Refuse every attribute not in `allowed`.
pub(super) fn only_attrs(
    attrs: &[(String, String)],
    allowed: &[&str],
    el: &str,
    at: usize,
) -> Result<(), Violation> {
    for (k, _) in attrs {
        if !allowed.contains(&k.as_str()) {
            return Err(Violation::at(
                at,
                format!(
                    "the <{el}> element has no `{k}` attribute — the dialect's vocabulary is closed"
                ),
            ));
        }
    }
    Ok(())
}

/// Look one attribute up by key.
pub(super) fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// The `@fact/code` adjacency pre-pass over one block list (the markup
/// contract's binding law): the fence's `fact=` names the fact carried by
/// the LAST unit of the immediately preceding block.
pub(super) fn validate_fence_bindings(blocks: &[XBlock]) -> Result<(), Violation> {
    for (i, b) in blocks.iter().enumerate() {
        let XBlock::Fence {
            fact: Some(id),
            line,
            ..
        } = b
        else {
            continue;
        };
        let bound = blocks.get(i.wrapping_sub(1)).and_then(last_unit_fact_id);
        if bound != Some(id.as_str()) {
            return Err(Violation::at(
                *line as usize,
                format!(
                    "`fact=\"{id}\"` must name the fact of the immediately preceding \
                     <p>/<list>/<quote> unit — the @fact/code binding is adjacent by law"
                ),
            ));
        }
    }
    Ok(())
}

fn last_unit_fact_id(b: &XBlock) -> Option<&str> {
    let unit = match b {
        XBlock::Para { unit, .. } | XBlock::Quote(unit) => Some(unit),
        XBlock::List { items, .. } => items.last(),
        _ => None,
    };
    unit?.fact.as_ref()?.id.as_deref()
}
