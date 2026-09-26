//! Inline Markdown to HTML — the one place the island interprets what
//! rides inside a unit's text.
//!
//! The pivot models no inline grammar on purpose: emphasis, inline code,
//! links and `spec://` addresses ride inside `Unit::text` as literal
//! Markdown in both directions, and that is what keeps a round trip
//! byte-stable at the text level (PROP-045 `##INLINE-STAYS-MARKDOWN`).
//! A backend that prints a unit verbatim into HTML therefore shows a
//! reader `**this**` and `` `that` ``, which is not «the same content
//! path for web and local» — it is the same content path for neither.
//!
//! So this module converts, and its vocabulary is CLOSED and short:
//!
//! | written | rendered |
//! | --- | --- |
//! | `` `code` `` | `<code>` |
//! | `[text](href)` | `<a href>` |
//! | `<https://example.org>` | `<a href>` |
//! | `**strong**` | `<strong>` |
//! | `*em*`, `_em_` | `<em>` |
//!
//! Everything else is text and is escaped. That is a deliberate floor,
//! not an unfinished parser: a manual's prose uses these five and the
//! style law forbids the rest (no images inside prose — a figure is a
//! block; no raw HTML — the island is generated, never authored).
//!
//! Code spans bind FIRST and their content is never re-examined, so a
//! `` `**` `` inside one stays two asterisks. That is the rule every
//! Markdown implementation agrees on, and it is the one a manual full of
//! quoted markup depends on.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#INLINE-STAYS-MARKDOWN");

use super::links::Links;

/// Render one unit's inline Markdown as HTML, with every address exactly
/// as it was written.
///
/// This is the vocabulary alone, which is what makes it the shape to
/// read the table above against. An island uses [`render_linked`]: what a
/// relative address MEANS is a property of who wrote it, and that is not
/// a question about Markdown (PROP-057 `##SITE-MOUNT`).
///
/// ```
/// use vibe_doc::html::inline::render;
///
/// assert_eq!(render("a **bold** word"), "a <strong>bold</strong> word");
/// assert_eq!(render("call `vibe list`"), "call <code>vibe list</code>");
/// assert_eq!(render("see [the page](/doc/x/)"), "see <a href=\"/doc/x/\">the page</a>");
/// // A code span binds first: what is inside it is never markup.
/// assert_eq!(render("`**not bold**`"), "<code>**not bold**</code>");
/// // Anything outside the five conventions is text, and text is escaped.
/// assert_eq!(render("a < b & c"), "a &lt; b &amp; c");
/// ```
pub fn render(text: &str) -> String {
    render_linked(text, &Links::verbatim())
}

/// The same, with every address written as the site addresses it.
///
/// An address this build cannot place keeps its spelling in
/// `data-address` and becomes no link: an island states an address or
/// states none, and a link that 404s is the one answer that helps
/// nobody. The attribute is not called `data-href` on purpose — a
/// scanner looking for `href` on a word boundary finds one inside that
/// name and reads a link where there is none.
///
/// ```
/// use vibe_doc::html::{inline::render_linked, links::Links};
///
/// let links = Links::page("/doc/");
/// assert_eq!(
///     render_linked("see [the glossary](../glossary/index.xml#term)", &links),
///     "see <a href=\"../../glossary/index/#term\">the glossary</a>"
/// );
/// ```
pub fn render_linked(text: &str, links: &Links) -> String {
    let mut found: Vec<String> = Vec::new();
    scan(text, links, &mut found)
}

/// Every address one unit's text points at, in the order it was written
/// — the same five conventions read by the same scanner, with the HTML
/// thrown away.
///
/// It is the one way to ask «what does this prose link to», and it exists
/// so that nothing has to ask with a second parser: a regular expression
/// over `](…)` would disagree with the renderer the first time a code
/// span held a bracket, and then a measurement over the corpus would
/// count links no reader can click. Both a `[text](href)` and an
/// `<https://…>` autolink are addresses; what rides inside a code span is
/// not, for the reason it is not markup either.
///
/// ```
/// use vibe_doc::html::inline::hrefs;
///
/// assert_eq!(
///     hrefs("see [the glossary](../glossary/index.xml#term) and [p07](#p07)"),
///     vec!["../glossary/index.xml#term".to_owned(), "#p07".to_owned()]
/// );
/// // A code span is text, in this reading exactly as in the renderer's.
/// assert!(hrefs("write `[label](target)` to link").is_empty());
/// // Markup nests, so a link inside emphasis is still a link.
/// assert_eq!(hrefs("*see [it](a.xml)*"), vec!["a.xml".to_owned()]);
/// ```
pub fn hrefs(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    scan(text, &Links::verbatim(), &mut found);
    found
}

/// The one walk over a unit's inline Markdown: it renders, and it
/// records every address it passes.
///
/// Both callers above are this function with one of its two results
/// dropped, which is what keeps «what the reader sees» and «what the
/// prose points at» two answers from ONE grammar. Splitting them would
/// give the corpus two opinions about where a link is.
fn scan(text: &str, links: &Links, found: &mut Vec<String>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < chars.len() {
        if let Some(next) = code_span(&chars, i, &mut out) {
            i = next;
            continue;
        }
        if let Some(next) = link(&chars, i, &mut out, links, found) {
            i = next;
            continue;
        }
        if let Some(next) = autolink(&chars, i, &mut out, links, found) {
            i = next;
            continue;
        }
        if let Some(next) = emphasis(&chars, i, &mut out, links, found) {
            i = next;
            continue;
        }
        push_escaped(chars[i], &mut out);
        i += 1;
    }
    out
}

/// The opening tag of one inline link: an address when this build can
/// place the target, and the spelling it was given when it cannot.
///
/// A link to an entry of the documentation's declared glossary takes two
/// attributes more and nothing else changes about it: `data-gloss` names the
/// entry, and `aria-describedby` points at the hidden definition the island
/// carries at its end, so a screen reader hears the definition as the link's
/// description on every device and the reader can show a card where a
/// pointer can hover (PROP-057 `##READER-GLOSSARY-CARD`). Selecting the
/// link still opens the glossary at the entry — the `href` is untouched.
fn open_anchor(href: &str, links: &Links) -> String {
    let mut out = String::from("<a");
    match links.href(href) {
        Some(at) => out.push_str(&format!(" href=\"{}\"", escape_attr(&at))),
        None => out.push_str(&format!(" data-address=\"{}\"", escape_attr(href))),
    }
    if let Some(entry) = links.gloss_of(href) {
        out.push_str(&format!(
            " data-gloss=\"{id}\" aria-describedby=\"{}{id}\"",
            super::GLOSS_DEF_ID,
            id = escape_attr(entry)
        ));
    }
    out.push('>');
    out
}

/// Escape one character for HTML text content.
fn push_escaped(c: char, out: &mut String) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        _ => out.push(c),
    }
}

/// Escape a whole string for HTML text content.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        push_escaped(c, &mut out);
    }
    out
}

/// Escape a string for an attribute value. Quotes matter here and
/// nowhere else, so the two escapes are two functions rather than one
/// that over-escapes both ways.
pub fn escape_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// `` `code` `` — a run of N backticks closed by a run of exactly N.
fn code_span(chars: &[char], at: usize, out: &mut String) -> Option<usize> {
    if chars[at] != '`' {
        return None;
    }
    let open = run_of('`', chars, at);
    let mut i = at + open;
    while i < chars.len() {
        if chars[i] == '`' {
            let close = run_of('`', chars, i);
            if close == open {
                let body: String = chars[at + open..i].iter().collect();
                out.push_str("<code>");
                out.push_str(&escape(&body));
                out.push_str("</code>");
                return Some(i + close);
            }
            i += close;
            continue;
        }
        i += 1;
    }
    None
}

/// `[text](href)` — the text may carry its own inline markup, the href
/// may not.
fn link(
    chars: &[char],
    at: usize,
    out: &mut String,
    links: &Links,
    found: &mut Vec<String>,
) -> Option<usize> {
    if chars[at] != '[' {
        return None;
    }
    let close = find(chars, at + 1, ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = find(chars, close + 2, ')')?;
    let label: String = chars[at + 1..close].iter().collect();
    let href: String = chars[close + 2..end].iter().collect();
    out.push_str(&open_anchor(&href, links));
    found.push(href);
    out.push_str(&scan(&label, links, found));
    out.push_str("</a>");
    Some(end + 1)
}

/// `<https://example.org>` — the address is its own label.
fn autolink(
    chars: &[char],
    at: usize,
    out: &mut String,
    links: &Links,
    found: &mut Vec<String>,
) -> Option<usize> {
    if chars[at] != '<' {
        return None;
    }
    let end = find(chars, at + 1, '>')?;
    let body: String = chars[at + 1..end].iter().collect();
    let is_url = ["https://", "http://", "spec://", "mailto:"]
        .iter()
        .any(|scheme| body.starts_with(scheme));
    if !is_url || body.chars().any(char::is_whitespace) {
        return None;
    }
    out.push_str(&open_anchor(&body, links));
    out.push_str(&escape(&body));
    found.push(body);
    out.push_str("</a>");
    Some(end + 1)
}

/// `**strong**` before `*em*`, because the longer marker wins; `_em_`
/// only at a word boundary, so `snake_case_names` stay whole.
fn emphasis(
    chars: &[char],
    at: usize,
    out: &mut String,
    links: &Links,
    found: &mut Vec<String>,
) -> Option<usize> {
    if chars[at] == '*' && chars.get(at + 1) == Some(&'*') {
        if let Some(end) = find_marker(chars, at + 2, "**") {
            let body: String = chars[at + 2..end].iter().collect();
            out.push_str("<strong>");
            out.push_str(&scan(&body, links, found));
            out.push_str("</strong>");
            return Some(end + 2);
        }
        return None;
    }
    if chars[at] == '*' {
        let end = find(chars, at + 1, '*')?;
        if end == at + 1 {
            return None;
        }
        let body: String = chars[at + 1..end].iter().collect();
        out.push_str("<em>");
        out.push_str(&scan(&body, links, found));
        out.push_str("</em>");
        return Some(end + 1);
    }
    if chars[at] == '_' && at_word_edge(chars, at) {
        let end = find(chars, at + 1, '_')?;
        if end == at + 1 || !at_word_edge(chars, end) {
            return None;
        }
        let body: String = chars[at + 1..end].iter().collect();
        out.push_str("<em>");
        out.push_str(&scan(&body, links, found));
        out.push_str("</em>");
        return Some(end + 1);
    }
    None
}

/// An underscore opens or closes emphasis only when it is not inside a
/// word: `a_b_c` is an identifier, not emphasis.
fn at_word_edge(chars: &[char], at: usize) -> bool {
    let before = at.checked_sub(1).and_then(|i| chars.get(i));
    let after = chars.get(at + 1);
    let alnum = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric());
    !(alnum(before) && alnum(after))
}

fn run_of(c: char, chars: &[char], at: usize) -> usize {
    chars[at..].iter().take_while(|&&x| x == c).count()
}

fn find(chars: &[char], from: usize, c: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == c)
}

fn find_marker(chars: &[char], from: usize, marker: &str) -> Option<usize> {
    let marker: Vec<char> = marker.chars().collect();
    (from..chars.len().saturating_sub(marker.len() - 1))
        .find(|&i| chars[i..i + marker.len()] == marker[..])
}

#[cfg(test)]
mod tests;
