//! The route template: where the island goes, what the policy has to
//! name, and the one edit a served head may take (PROP-057
//! `##LOCAL-CSP`, deferral X-035).
//!
//! ## The head is corrected in place, never added to
//!
//! Two things in a prerendered route name a PAGE rather than the shell —
//! the tab's title and the addresses of the page's projections — and the
//! server corrects both. It corrects them by rewriting the text and the
//! attributes of elements the template ALREADY DECLARES, and it inserts
//! nothing, removes nothing and moves nothing.
//!
//! That is a measured law. A resumable document carries a serialised
//! description of itself, and the description and the head agree on which
//! elements are there and in what order. Three `<link rel="alternate">`
//! inserted after `</title>` broke that agreement on this shell: the
//! framework stopped resuming, and with it every behaviour of the page —
//! the contents, the anchors, the reading settings, the rule panel, the
//! reading of the manifest — silently never started. No error reached a
//! gate; the page simply became text.
//!
//! So a template that declares no `<title>` and no `<link>` gets neither.
//! The shell writes the head it wants (it is the only party that knows
//! what the page is for), and this module repairs the two values in it
//! that a prerender could not have known.
//!
//! The shell ships pages with inline scripts that cannot become files.
//! The theme has to reach the root element before the first stylesheet is
//! fetched or a reader watches their own setting fail; the router's
//! scroll restoration has to be undone before the router's own bootstrap
//! runs; the framework writes its resumability state into the document.
//! A policy that answered this with `'unsafe-inline'` would admit every
//! script anybody ever injects, which is the one thing a policy is for.
//!
//! So each is named by the hash of its bytes, and the hashes are computed
//! from the SHELL THIS BINARY IS CARRYING, at start-up — never written
//! into a constant that goes stale the next time the shell is rebuilt
//! (X-035 says exactly this). The scanner below decides what an inline
//! script is the way a browser decides it, and it is the same decision
//! the web package's own build makes when it writes `csp.txt` for the
//! public site: an element with no `src`, whose `type` is absent, empty,
//! `module` or a JavaScript media type. A `<script type="qwik/state">`
//! block is data — the browser never executes it, a policy never blocks
//! it, and hashing it would grow the header by a hash nothing looks for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-CSP");

use base64::Engine as _;
use sha2::{Digest, Sha256};

/// The `type` values a browser treats as executable script.
const EXECUTABLE: [&str; 7] = [
    "",
    "module",
    "text/javascript",
    "application/javascript",
    "application/ecmascript",
    "text/ecmascript",
    "application/x-javascript",
];

/// Every inline script of one page, in document order — the exact text
/// between the tags, which is what a hash is taken over.
///
/// ```
/// use vibe_doc_shell::template;
///
/// let html = "<script src=\"/a.js\"></script><script>let a = 1;</script>\
///             <script type=\"qwik/state\">{}</script>";
/// assert_eq!(template::inline_scripts(html), vec!["let a = 1;"]);
/// ```
pub fn inline_scripts(html: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(open) = html[at..].find("<script").map(|i| at + i) {
        let after = open + "<script".len();
        let Some(close) = tag_end(html, after) else {
            return found;
        };
        let Some(end) = html[close..].find("</script>").map(|i| close + i) else {
            return found;
        };
        let attributes = &html[after..close];
        let body = &html[close + 1..end];
        at = end + "</script>".len();

        if has_src(attributes) {
            continue;
        }
        if !EXECUTABLE.contains(&type_of(attributes).as_str()) {
            continue;
        }
        found.push(body);
    }
    found
}

/// The CSP source expression naming one script by its bytes.
///
/// ```
/// use vibe_doc_shell::template;
/// assert!(template::hash_of("let a = 1;").starts_with("'sha256-"));
/// ```
pub fn hash_of(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
    format!("'sha256-{encoded}'")
}

/// Every script hash one page needs, sorted and without repetition.
pub fn hashes(html: &str) -> Vec<String> {
    let mut all: Vec<String> = inline_scripts(html).into_iter().map(hash_of).collect();
    all.sort();
    all.dedup();
    all
}

/// Where one `<script …>` tag ends, quote-aware.
///
/// A scan to the first `>` is wrong here and measurably so: the framework
/// writes a head script's source into an attribute of the same element,
/// and an attribute value containing a `>` would end the tag early and
/// take half the body with it.
fn tag_end(html: &str, from: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut quote: Option<u8> = None;
    for (offset, byte) in bytes.iter().enumerate().skip(from) {
        match quote {
            Some(open) if *byte == open => quote = None,
            Some(_) => {}
            None if *byte == b'"' || *byte == b'\'' => quote = Some(*byte),
            None if *byte == b'>' => return Some(offset),
            None => {}
        }
    }
    None
}

/// Does this tag carry a `src` attribute — that is, is the script a file
/// the policy covers with `'self'` rather than with a hash?
fn has_src(attributes: &str) -> bool {
    attribute(attributes, "src").is_some()
}

/// The tag's `type`, lowercased; the empty string when it carries none.
fn type_of(attributes: &str) -> String {
    attribute(attributes, "type")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// One attribute's value, by name, whatever quoting it uses.
///
/// Written by hand rather than by regular expression because the one
/// subtlety is not the syntax: `src` must not match the `data-src` the
/// framework writes on an inlined stylesheet, so the character before the
/// name has to be a boundary.
fn attribute(attributes: &str, name: &str) -> Option<String> {
    let bytes = attributes.as_bytes();
    let mut at = 0usize;
    while let Some(found) = attributes[at..].find(name).map(|i| at + i) {
        at = found + name.len();
        let before_ok = found == 0
            || bytes
                .get(found - 1)
                .is_some_and(|b| b.is_ascii_whitespace() || *b == b'/');
        if !before_ok {
            continue;
        }
        let rest = attributes[at..].trim_start();
        let Some(value) = rest.strip_prefix('=') else {
            // A bare attribute (`async`, `defer`) — present, valueless.
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return Some(String::new());
            }
            continue;
        };
        let value = value.trim_start();
        let quoted = |mark: char| {
            value
                .strip_prefix(mark)
                .and_then(|tail| tail.split_once(mark))
                .map(|(inside, _)| inside.to_string())
        };
        return quoted('"')
            .or_else(|| quoted('\''))
            .or_else(|| Some(value.split_whitespace().next().unwrap_or("").to_string()));
    }
    None
}

/// Put one rendered island where the template's marker is.
///
/// The first marker and only the first: a template carries exactly one
/// island, and replacing every occurrence would turn a marker that leaked
/// into the page's own text into a second copy of the documentation.
pub fn glue(template: &str, marker: &str, island: &str) -> String {
    match template.split_once(marker) {
        Some((before, after)) => format!("{before}{island}{after}"),
        None => template.to_string(),
    }
}

/// Rewrite the title element's text, leaving its attributes alone.
///
/// The template is a prerendered route, so its `<title>` holds whatever
/// page the build rendered. The reader knows which page it is actually
/// serving, and a browser tab is the one place that difference is visible
/// before anything else on the page.
///
/// A template that declares no title gets none written in — for the
/// reason in this module's header, and because a blank tab is worse than
/// a tab the shell fills in for itself.
///
/// ```
/// use vibe_doc_shell::template::retitle;
///
/// assert_eq!(retitle("<head><title>A</title></head>", "B"), "<head><title>B</title></head>");
/// assert_eq!(retitle("<head></head>", "B"), "<head></head>");
/// ```
pub fn retitle(html: &str, title: &str) -> String {
    let Some(open) = html.find("<title") else {
        return html.to_string();
    };
    let Some(close) = tag_end(html, open + "<title".len()) else {
        return html.to_string();
    };
    let Some(end) = html[close..].find("</title>").map(|i| close + i) else {
        return html.to_string();
    };
    format!("{}{}{}", &html[..close + 1], escape(title), &html[end..])
}

/// One machine projection of the page being served: the media type that
/// identifies its `<link rel="alternate">`, the address that element
/// should carry, and the words a browser shows for it in a menu of a
/// document's alternate versions.
pub struct Alternate<'a> {
    pub media_type: &'a str,
    pub href: &'a str,
    pub title: &'a str,
}

/// Repoint the template's `<link rel="alternate">` elements at the page
/// actually being served.
///
/// One attribute of elements that are already standing, matched by
/// `type`, and **nothing else**: no element is inserted, none is removed,
/// none is moved. That is not tidiness — it is the whole contract. A
/// resumable document and the state serialised beside it agree on what
/// the head CONTAINS and in what ORDER; an element inserted into the
/// middle of it breaks that agreement, the framework stops resuming, and
/// every behaviour of the page — its contents, its anchors, its settings,
/// its reading of the manifest — silently never starts. Measured on this
/// shell: three `<link rel="alternate">` inserted after `</title>` took a
/// working reader down to static text with no error a gate could see.
///
/// So a template that declares no such element gets no such element. The
/// shell writes the head it wants, and the server corrects the addresses
/// in it.
///
/// ```
/// use vibe_doc_shell::template::{Alternate, relink};
///
/// let head = "<head><title>A</title>\
///             <link rel=\"alternate\" type=\"text/markdown\" href=\"/fixture.md\" \
///             title=\"a fixture\"></head>";
/// let markdown = Alternate {
///     media_type: "text/markdown",
///     href: "/real.md",
///     title: "This page as Markdown",
/// };
/// let out = relink(head, &[markdown]);
/// assert!(out.contains("href=\"/real.md\""));
/// // The element is the one that was already there, with its values
/// // corrected: no second element joined it and nothing moved.
/// assert_eq!(out.matches("<link").count(), 1);
/// assert!(out.starts_with("<head><title>A</title><link rel=\"alternate\""));
/// // A head that declares nothing is left exactly as it was.
/// let markdown = Alternate {
///     media_type: "text/markdown",
///     href: "/real.md",
///     title: "This page as Markdown",
/// };
/// assert_eq!(relink("<head></head>", &[markdown]), "<head></head>");
/// ```
pub fn relink(html: &str, alternates: &[Alternate<'_>]) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find("<link") {
        let Some(close) = tag_end(rest, at + "<link".len()) else {
            break;
        };
        let tag = &rest[at..=close];
        out.push_str(&rest[..at]);
        out.push_str(&repointed(tag, alternates));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// One `<link>` tag with the values of the page it is serving, or the tag
/// exactly as it stands.
///
/// An attribute the tag does not declare is not written in: the length of
/// the head is the head's own business, and only the VALUES in it are the
/// server's.
fn repointed(tag: &str, alternates: &[Alternate<'_>]) -> String {
    let attributes = tag.trim_start_matches("<link").trim_end_matches('>');
    if attribute(attributes, "rel").as_deref() != Some("alternate") {
        return tag.to_string();
    }
    let Some(media_type) = attribute(attributes, "type") else {
        return tag.to_string();
    };
    let Some(wanted) = alternates
        .iter()
        .find(|one| one.media_type == media_type.trim())
    else {
        return tag.to_string();
    };
    let out = set_attribute(tag, "href", wanted.href).unwrap_or_else(|| tag.to_string());
    set_attribute(&out, "title", wanted.title).unwrap_or(out)
}

/// Rewrite one attribute's value in place, keeping every other byte of
/// the tag — or `None` when the tag does not declare that attribute,
/// because writing one in would be adding to the head rather than
/// correcting it.
fn set_attribute(tag: &str, name: &str, value: &str) -> Option<String> {
    let mut at = 0usize;
    let bytes = tag.as_bytes();
    while let Some(found) = tag[at..].find(name).map(|i| at + i) {
        at = found + name.len();
        let before_ok = bytes
            .get(found.checked_sub(1)?)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'/');
        if !before_ok {
            continue;
        }
        let rest = &tag[at..];
        let trimmed = rest.trim_start();
        let Some(after_equals) = trimmed.strip_prefix('=') else {
            continue;
        };
        let after_equals = after_equals.trim_start();
        let mark = after_equals.chars().next()?;
        if mark != '"' && mark != '\'' {
            continue;
        }
        let inside = after_equals.get(1..)?;
        let end = inside.find(mark)?;
        let opens = tag.len() - after_equals.len() + 1;
        return Some(format!(
            "{}{}{}",
            &tag[..opens],
            escape(value),
            &tag[opens + end..]
        ));
    }
    None
}

/// The four characters that cannot stand as themselves in element text or
/// an attribute value.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Repoint a template built for `from` at the base a reader mounted at.
///
/// The shell is built with a configurable base and relative addresses
/// (`##SHELL-RELATIVE`), and `/doc/` is the one it was built with. A
/// reader mounted elsewhere rewrites that prefix — which it can do
/// safely because every address in a template is the build's own, and the
/// one thing a template does not contain is documentation text.
pub fn rebase(html: &str, from: &str, to: &str) -> String {
    if from == to {
        return html.to_string();
    }
    html.replace(from, to)
}
