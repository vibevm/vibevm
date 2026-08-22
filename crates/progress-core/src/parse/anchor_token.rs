//! The anchor-token grammar — the ONE reader of `@fact:` / `@fact/<type>:` /
//! legacy `##` openers. Extracted from `facts.rs` at the B-068/B-074 landing
//! (the file crossed the 600-line budget): every consumer — segmentation,
//! the swallowed-anchor checker, the duplicate law, the marker scanner —
//! parses the opener/type/id through these fns, so there is no second lexer
//! to drift.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#parsing");

use super::facts::blockquote_prefix_len;

/// The fact anchor at the start of a span: `(id, content_start)` where
/// `content_start` is the byte just past the id (for the marker position law).
/// No id ⇒ content_start == span start.
///
/// Two spellings are accepted. The **qualified** form `@fact:<ID>` names its
/// key, so it cannot be confused with a heading, with a foreign `@`
/// annotation, or with an address. The **legacy** form `##<ID>` is the
/// original spelling and is still read, so a document written before the
/// qualified form keeps parsing.
///
/// A blockquote paragraph is a countable unit like any other, so its `>`
/// prefix is consumed before the anchor is looked for — a quoted normative
/// statement is addressable, and anchored-when-marked reaches it.
pub fn take_fact_id(text: &str, s: usize, e: usize) -> (Option<String>, usize) {
    match parse_anchor(text, s, e) {
        Some(a) => (Some(a.id.to_string()), a.content_start),
        None => (None, s),
    }
}

/// The object type an anchor names, if it names one: `@fact/code:<ID>` ⇒
/// `Some("code")`. A plain `@fact:<ID>` or `##<ID>` covers only its own
/// paragraph and yields `None`.
pub(super) fn take_fact_type(text: &str, s: usize, e: usize) -> Option<String> {
    parse_anchor(text, s, e).and_then(|a| a.ty.map(str::to_string))
}

/// Whether the fact anchor at the start of a span uses the definition form
/// (`@fact:<ID>` / `@fact/<type>:<ID>`) rather than the legacy `##<ID>` form.
///
/// Duplicate checking uses this to avoid counting a parsed definition twice:
/// once from the segmented fact and once from the raw definition-token scan.
pub(super) fn fact_anchor_is_qualified(text: &str, s: usize, e: usize) -> bool {
    parse_anchor(text, s, e).is_some_and(|a| a.form == AnchorForm::Qualified)
}

/// Every qualified definition-form token in `text[s..e]`.
///
/// This is the shared token reader for the two placement diagnostics. The
/// caller chooses the lexical surface: `Block::scan_text` suppresses inline
/// code for the swallowed-anchor law, while raw text lets the duplicate law
/// catch a definition-form token mistakenly used as a code-formatted
/// citation. Fenced blocks never reach either caller.
pub(super) fn qualified_fact_tokens(text: &str, s: usize, e: usize) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut cursor = s;
    while cursor < e {
        let Some(rel) = text[cursor..e].find("@fact") else {
            break;
        };
        let at = cursor + rel;
        let boundary_ok = at == s
            || text[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphanumeric());
        if boundary_ok
            && let Some(token) = parse_anchor_token(&text[at..e])
            && token.form == AnchorForm::Qualified
        {
            let end = at + token.len;
            out.push((token.id.to_string(), at, end));
            cursor = end;
            continue;
        }
        cursor = at + "@fact".len();
    }
    out
}

/// The one reader of the anchor grammar. Both public entry points go through
/// it, so a type and an id can never be parsed by two slightly different
/// rules — the failure mode this whole markup has now paid for three times.
pub(super) struct Anchor<'a> {
    form: AnchorForm,
    ty: Option<&'a str>,
    id: &'a str,
    /// Byte offset into the ORIGINAL text, just past the anchor.
    content_start: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AnchorForm {
    Qualified,
    Legacy,
}

pub(super) struct AnchorToken<'a> {
    form: AnchorForm,
    ty: Option<&'a str>,
    id: &'a str,
    len: usize,
}

pub(super) fn parse_anchor(text: &str, s: usize, e: usize) -> Option<Anchor<'_>> {
    let seg = &text[s..e];
    let lead_ws = seg.len() - seg.trim_start().len();
    let lead = lead_ws + blockquote_prefix_len(&seg[lead_ws..]);
    let t = &seg[lead..];

    let token = parse_anchor_token(t)?;
    if !t[token.len..]
        .chars()
        .next()
        .is_none_or(|c| c.is_whitespace())
    {
        return None;
    }
    Some(Anchor {
        form: token.form,
        ty: token.ty,
        id: token.id,
        content_start: s + lead + token.len,
    })
}

/// Parse one anchor token without deciding what may follow it. The ordinary
/// fact parser adds the whitespace/end boundary; the definition-token checks
/// deliberately also accept Markdown punctuation such as the closing
/// backtick around a cited token. The opener, type, and id grammar therefore
/// still have exactly one reader.
pub(super) fn parse_anchor_token(t: &str) -> Option<AnchorToken<'_>> {
    // Qualified openers first — `@fact/<type>:` is longer than `@fact:` and
    // must be tried before it, or the type would be read as part of no id and
    // the anchor silently downgraded to an untyped one.
    let (form, ty, rest, opener_len) = if let Some(after) = t.strip_prefix("@fact/") {
        let ty_len = after
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .count();
        let after_ty = after.get(ty_len..)?;
        let body = after_ty.strip_prefix(':')?;
        if ty_len == 0 {
            return None;
        }
        (
            AnchorForm::Qualified,
            Some(&after[..ty_len]),
            body,
            "@fact/".len() + ty_len + 1,
        )
    } else if let Some(after) = t.strip_prefix("@fact:") {
        (AnchorForm::Qualified, None, after, "@fact:".len())
    } else if let Some(after) = t.strip_prefix("##") {
        (AnchorForm::Legacy, None, after, 2)
    } else {
        return None;
    };

    let id_len = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .count();
    if id_len == 0 || !rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(AnchorToken {
        form,
        ty,
        id: &rest[..id_len],
        len: opener_len + id_len,
    })
}
