//! A cited rule in the island: the line a reader meets, and the
//! quotation it opens (PROP-057 `##READER-RULE-FOLDED`).
//!
//! A manual cites far more than it writes. The official one quotes 802
//! rules over 49 pages — 38 words each at the median and up to 429 — so
//! unfolded they outweigh the prose somebody came to read, while an agent
//! needs every word of them. The fold answers both at once: the island
//! still carries the whole text, inside a disclosure that starts closed,
//! and the `.md` and `.xml` projections, the `llms` files and the MCP
//! surfaces are not touched at all. What is spared is a person's eye, and
//! nothing else about a citation changes.
//!
//! ## Why the line describes the rule in the rule's own words
//!
//! A fold that said only «spec:» would hide which rule it hid, and a
//! reader would open all of them to find one. The two other ways to get a
//! description were weighed and refused: a hand-written `summary` on
//! every citation is 1,604 lines across two editions and a new attribute
//! in the vocabulary, before anybody knows the rule's own words are not
//! enough; and the unit's label is an identifier rather than a phrase —
//! `FAM-CORE` describes nothing (`##RULE-FOLDED-DECISION` and the record
//! beside it).
//!
//! So the description is a pure function of the text. It reads that text
//! through [`super::inline`], which is the one place this crate decides
//! what a code span, an emphasis and a link ARE: a second reading of the
//! same Markdown would disagree with the quotation two lines below it on
//! the same page, and the first rule to quote a backtick would show it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#READER-RULE-FOLDED");

use crate::content::Content;

use super::Attrs;
use super::emit::{anchor_line, close, line, open};
use super::inline::{escape, render_linked};
use super::links::Links;

/// The mark that turns under the cursor. It is decoration and says
/// nothing a screen reader needs: the disclosure's own state is what
/// announces open or closed.
const MARK: &str = "▶";

/// What closes a description that had to stop early.
const ELLIPSIS: char = '…';

/// How many of the rule's first words a description keeps.
const FIRST_WORDS: usize = 6;

/// How many words follow a one-word lead such as «Decision», which has
/// already spent the head of the line.
const AFTER_LEAD: usize = 5;

/// A bold lead of at most this many words IS the description.
const LEAD_WORDS: usize = 8;

/// A longer lead is a sentence, and a sentence is cut to this.
const LEAD_CUT: usize = 6;

/// A rule of at most this many words is its own summary: a line
/// describing it would hide as much as it said.
const SHORT_RULE: usize = 10;

/// The three escapes [`super::inline::escape`] writes into text, and the
/// only ones a description has to undo: a quote is escaped in attribute
/// values alone, and attribute values live inside tags.
const ENTITIES: [(&str, char); 3] = [("&amp;", '&'), ("&lt;", '<'), ("&gt;", '>')];

/// The words a description may not end on. A truncated line that stops on
/// «of» or «the» reads as a sentence somebody dropped, and the word
/// carries no meaning to pay for the space.
const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "of", "to", "and", "or", "in", "on", "at", "by", "for", "with", "from", "as",
    "is", "are", "be", "that", "this", "which", "into", "its", "their", "your", "our",
];

/// Emit one cited rule: the disclosure, its line, and the quotation.
///
/// Everything the block itself carries — its number, its `when`, and the
/// mark that says this build could not fetch the text — rides on the
/// `details`, because that is the element a reader sees while the rule is
/// closed and the element a stylesheet and a shell address as «the
/// block». The `blockquote` inside it keeps the one class that says what
/// it is.
pub(super) fn fold(
    out: &mut String,
    depth: usize,
    uri: &str,
    outer: &Attrs,
    content: &Content,
    num: Option<u32>,
) {
    let mut attrs = outer.clone();
    attrs.push(("class", "rule-fold".to_owned()));
    let found = content.rules.get(uri);
    if found.is_none() {
        attrs.push(("data-unresolved", "true".to_owned()));
    }
    open(out, depth, "details", &attrs);
    open(
        out,
        depth + 1,
        "summary",
        &[("class", "rule-fold__line".to_owned())],
    );
    anchor_line(out, depth + 2, num);
    line(
        out,
        depth + 2,
        "span",
        &[
            ("class", "rule-fold__mark".to_owned()),
            ("aria-hidden", "true".to_owned()),
        ],
        MARK,
    );
    match found.and_then(|f| gist(&f.text).map(|g| (g, f.lang.clone()))) {
        // The description is the rule's, so it is announced as a
        // quotation of the specification and marked with the
        // specification's language — the same one the quotation below
        // carries, and not the edition's.
        Some((gist, lang)) => {
            line(
                out,
                depth + 2,
                "span",
                &[("class", "rule-fold__kind".to_owned())],
                "spec:",
            );
            line(
                out,
                depth + 2,
                "span",
                &[("class", "rule-fold__gist".to_owned()), ("lang", lang)],
                &escape(&gist),
            );
        }
        // Nothing of the rule's own is worth a line, so the line says
        // what it is in the EDITION's words: this one is the reader's
        // sentence and not the specification's (`##LOC-LANGUAGE-FIELD`).
        // The `spec:` label goes with it — it introduced a quotation, and
        // there is none to introduce.
        None => line(
            out,
            depth + 2,
            "span",
            &[
                (
                    "class",
                    "rule-fold__gist rule-fold__gist--generic".to_owned(),
                ),
                ("lang", content.edition_lang().to_owned()),
            ],
            generic(content.edition_lang()),
        ),
    }
    close(out, depth + 1, "summary");

    open(
        out,
        depth + 1,
        "blockquote",
        &[("class", "rule".to_owned())],
    );
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
    line(out, depth + 2, "a", &link_attrs, &body);
    close(out, depth + 1, "blockquote");
    close(out, depth, "details");
}

/// The words the line shows when the rule's own cannot describe it.
///
/// The EDITION's language and not the specification's: a Russian manual
/// quoting an English rule still says this much in Russian, because this
/// sentence is the reader's and not the rule's (`##READER-RULE-FOLDED`,
/// `##LOC-LANGUAGE-FIELD`).
fn generic(lang: &str) -> &'static str {
    match lang {
        "ru" => "цитата из спецификации",
        _ => "quote from the specification",
    }
}

/// The rule described in one line, or nothing when its own words cannot
/// do it.
///
/// Three shapes of rule, and one answer each. A rule that opens with a
/// bold lead has already been summarised by its author, so the lead is
/// the line. A lead of one word — «Decision», «Why» — names a kind rather
/// than a subject, so it keeps the words that follow it. Anything else
/// gives up its first words.
///
/// Nothing is returned for a rule of ten words or fewer: the line would
/// be as long as the rule and would hide it for no gain.
pub(super) fn gist(text: &str) -> Option<String> {
    let html = super::inline::render(text);
    let whole = plain(&html);
    if words(&whole) <= SHORT_RULE {
        return None;
    }
    let described = match lead(&html) {
        Some((lead, rest)) => from_lead(&lead, &rest),
        None => first_words(&whole, FIRST_WORDS),
    };
    (!described.is_empty()).then_some(described)
}

/// The description a bold lead gives.
fn from_lead(lead: &str, rest: &str) -> String {
    let lead = lead.trim().trim_end_matches(['.', ':']).trim_end();
    match words(lead) {
        // A bold run that held nothing but punctuation is not a lead, and
        // the rule's first words answer instead.
        0 => first_words(rest, FIRST_WORDS),
        1 => match first_words(rest, AFTER_LEAD) {
            after if after.is_empty() => lead.to_owned(),
            after => format!("{lead}: {after}"),
        },
        2..=LEAD_WORDS => lead.to_owned(),
        _ => first_words(lead, LEAD_CUT),
    }
}

/// The rule's bold lead and the text after it, when the rule opens with
/// one.
///
/// «Opens with» is exactly what it says: a bold run in the middle of a
/// sentence emphasises a word, it does not label a rule, and the island's
/// own rendering is what decides where the run begins.
fn lead(html: &str) -> Option<(String, String)> {
    let inside = html.strip_prefix("<strong>")?;
    let (lead, rest) = inside.split_once("</strong>")?;
    Some((plain(lead), plain(rest)))
}

/// A run of the rule's first words, closed by an ellipsis when there was
/// more.
///
/// The count is a ceiling and not a target: a run that would end on a
/// function word gives that word up, because «computed against the…»
/// spends a word of the line on nothing.
fn first_words(text: &str, most: usize) -> String {
    let all: Vec<&str> = text.split_whitespace().collect();
    if all.len() <= most {
        return all.join(" ");
    }
    let mut kept = most;
    while kept > 0 && is_function_word(all[kept - 1]) {
        kept -= 1;
    }
    if kept == 0 {
        return String::new();
    }
    let mut out = all[..kept].join(" ");
    // The comma the sentence went on after is not punctuation of this
    // line: an ellipsis is already saying that something followed.
    while out.ends_with([',', ';', ':']) {
        out.pop();
    }
    out.push(ELLIPSIS);
    out
}

/// Whether a word is one of the closed list, read without the
/// punctuation it may be wearing.
fn is_function_word(word: &str) -> bool {
    let bare = word
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    FUNCTION_WORDS.contains(&bare.as_str())
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// The rule as it would be read aloud: the island's own inline grammar,
/// with the tags taken back off.
///
/// The detour through HTML is the point. Emphasis, code spans and links
/// are decided once, in [`super::inline`], and a description that read
/// the Markdown itself would be a second opinion about the same
/// characters — visibly so on the first rule that quotes a backtick.
/// Coming back is short and closed: the island's vocabulary is five
/// elements, and the only escapes inside text are the three
/// [`super::inline::escape`] writes.
fn plain(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix('<') {
            // An attribute value carries no raw `>` — it is escaped on
            // the way in — so the first one closes the tag.
            rest = tail.split_once('>').map_or("", |(_, after)| after);
            continue;
        }
        if let Some((entity, c)) = ENTITIES.iter().find(|(e, _)| rest.starts_with(e)) {
            out.push(*c);
            rest = &rest[entity.len()..];
            continue;
        }
        let mut chars = rest.chars();
        if let Some(c) = chars.next() {
            out.push(c);
        }
        rest = chars.as_str();
    }
    out
}

#[cfg(test)]
mod tests;
