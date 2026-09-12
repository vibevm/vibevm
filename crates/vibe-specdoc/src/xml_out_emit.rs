//! The writer's emitter primitives: one indented element, and the two
//! escapes its text and its attributes go through.
//!
//! Everything above this cell decides WHAT to write; this decides how the
//! bytes look. Both escapes are deliberately narrower than quick-xml's
//! defaults — the dialect's byte-idempotence law is a statement about
//! exactly these two functions, so they are kept where a reader can see
//! them together (PROP-045 ##XML-IS-BYTE-IDEMPOTENT).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use super::xml_out::W;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;

pub(crate) fn bytes_start<'a>(name: &'a str, attrs: &[(&str, String)]) -> BytesStart<'a> {
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
pub(crate) fn esc_text(s: &str) -> String {
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
pub(crate) fn esc_attr(s: &str) -> std::borrow::Cow<'_, str> {
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

pub(crate) fn start(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)]) {
    indent(w, depth);
    let _ = w.write_event(Event::Start(bytes_start(name, attrs)));
}

pub(crate) fn end(w: &mut W, depth: usize, name: &str) {
    indent(w, depth);
    let _ = w.write_event(Event::End(BytesEnd::new(name)));
}

pub(crate) fn empty(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)]) {
    indent(w, depth);
    let _ = w.write_event(Event::Empty(bytes_start(name, attrs)));
}

/// An inline leaf: one line, text untouched between the tags.
pub(crate) fn inline(w: &mut W, depth: usize, name: &str, attrs: &[(&str, String)], text: &str) {
    indent(w, depth);
    let _ = w.write_event(Event::Start(bytes_start(name, attrs)));
    let _ = w.write_event(Event::Text(BytesText::from_escaped(esc_text(text))));
    let _ = w.write_event(Event::End(BytesEnd::new(name)));
}

pub(crate) fn indent(w: &mut W, depth: usize) {
    let mut s = String::with_capacity(1 + 2 * depth);
    s.push('\n');
    for _ in 0..depth {
        s.push_str("  ");
    }
    let _ = w.write_event(Event::Text(BytesText::from_escaped(s)));
}
