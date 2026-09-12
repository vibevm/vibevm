//! The island's emitters — tags, attributes, indentation, and the margin
//! anchor a numbered block opens with.
//!
//! Indentation is structural and stable, so two builds of one page are
//! one file: a diff of two islands then shows what MOVED, not how the
//! writer felt about whitespace. The one element that takes none is
//! `<pre>`, where whitespace is content.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use super::Attrs;
use super::inline::{escape, escape_attr};
use crate::numbering::Numbering;

pub(super) fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

pub(super) fn attr_text(attrs: &[(&'static str, String)]) -> String {
    attrs
        .iter()
        .map(|(k, v)| format!(" {k}=\"{}\"", escape_attr(v)))
        .collect()
}

pub(super) fn open(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)]) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}>\n", attr_text(attrs)));
}

pub(super) fn close(out: &mut String, depth: usize, tag: &str) {
    indent(out, depth);
    out.push_str(&format!("</{tag}>\n"));
}

/// An element whose whole body fits on its own line.
pub(super) fn line(
    out: &mut String,
    depth: usize,
    tag: &str,
    attrs: &[(&'static str, String)],
    body: &str,
) {
    indent(out, depth);
    out.push_str(&format!("<{tag}{}>{body}</{tag}>\n", attr_text(attrs)));
}

pub(super) fn void(out: &mut String, depth: usize, tag: &str, attrs: &[(&'static str, String)]) {
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
pub(super) fn pre_code(
    out: &mut String,
    depth: usize,
    pre_attrs: &[(&'static str, String)],
    lang: Option<&str>,
    text: &str,
    num: Option<u32>,
) {
    let mut code_attrs: Attrs = Vec::new();
    if let Some(lang) = lang {
        code_attrs.push(("class", format!("language-{lang}")));
    }
    indent(out, depth);
    out.push_str(&format!(
        "<pre{}>{}<code{}>{}</code></pre>\n",
        attr_text(pre_attrs),
        anchor(num),
        attr_text(&code_attrs),
        escape(text)
    ));
}

/// The block's margin anchor, as the markup that opens the block.
///
/// `id="p07"` is what `#p07` lands on, and the `href` back to itself is
/// what a click copies. The visible text is the bare digits: the `p` is
/// for the address, not for the margin.
pub(super) fn anchor(num: Option<u32>) -> String {
    let Some(n) = num else {
        return String::new();
    };
    let label = Numbering::spell(n);
    format!(
        "<a class=\"p-anchor\" id=\"{label}\" href=\"#{label}\">{}</a>",
        Numbering::digits(n)
    )
}

/// The same anchor as the first CHILD LINE of a container block.
pub(super) fn anchor_line(out: &mut String, depth: usize, num: Option<u32>) {
    if num.is_none() {
        return;
    }
    indent(out, depth);
    out.push_str(&anchor(num));
    out.push('\n');
}

/// The block's number as data, for a shell that wants it without parsing
/// the anchor.
pub(super) fn push_p(attrs: &mut Attrs, num: Option<u32>) {
    if let Some(n) = num {
        attrs.push(("data-p", n.to_string()));
    }
}
