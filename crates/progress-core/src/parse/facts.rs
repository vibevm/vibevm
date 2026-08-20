//! Phase 3 — fact segmentation (paragraphs, lead lines, list items, table cells).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{BlockKind, Fact, FactKind, Issue, IssueCode, ParsedDoc, Severity};
use specmark::spec;

/// Byte offset of a list item's content when the line opens one
/// (`- ` / `* ` / `+ ` / `N. ` / `N) `), else None. A GFM task-list
/// checkbox counts as part of the opener — see [`task_box_len`].
fn list_item_content(line: &str) -> Option<usize> {
    let off = list_marker_len(line)?;
    Some(off + task_box_len(&line[off..]))
}

/// Byte offset just past the list marker itself (`- ` / `* ` / `+ ` /
/// `N. ` / `N) `) when the line opens an item, else None.
fn list_marker_len(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    for pre in ["- ", "* ", "+ "] {
        if rest.starts_with(pre) {
            return Some(indent + pre.len());
        }
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if after.starts_with(". ") || after.starts_with(") ") {
            return Some(indent + digits + 2);
        }
    }
    None
}

/// Byte length of the GFM task-list checkbox opening `t` — `[ ]`, `[x]` or
/// `[X]` with its trailing spacing — else 0.
///
/// The checkbox is **structure**, exactly like the `-` bullet and the `1.`
/// ordinal it follows: a task item is a list item, and `##COUNTABLE-UNITS`
/// makes every list item countable with no task-list carve-out. GFM leaves
/// no room *before* the box — writing `##ID` there stops the line being a
/// task list at all — so the item's first token in the sense of
/// `##FACT-ANCHOR-SYNTAX` is the token after the box, and the box is
/// skipped with the marker rather than read as the item's content.
///
/// The spacing is eaten greedily, like the blockquote prefix, so the boxed
/// form is no pickier about alignment than the plain one.
///
/// (The two facts are cited by their sections — `##COUNTABLE-UNITS` lives
/// under §3.9 `#granularity`, `##FACT-ANCHOR-SYNTAX` under §3.8
/// `#placement` — because the specmark address grammar takes kebab-case
/// heading anchors only, not a fact anchor's `##<ID>`.)
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#granularity")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#placement")]
fn task_box_len(t: &str) -> usize {
    let b = t.as_bytes();
    if b.len() < 3 || b[0] != b'[' || b[2] != b']' || !matches!(b[1], b' ' | b'x' | b'X') {
        return 0;
    }
    // The box either ends the line or is followed by spacing; `[ ]glued`
    // is prose that merely looks like a box.
    let mut i = 3;
    if i < b.len() && b[i] != b' ' && b[i] != b'\t' {
        return 0;
    }
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    i
}

/// True when every cell of the table row is a `---`/`:--:`-style rule.
fn is_delimiter_row(cells: &[(usize, usize)], text: &str) -> bool {
    !cells.is_empty()
        && cells.iter().all(|&(s, e)| {
            let t = text[s..e].trim();
            !t.is_empty() && t.chars().all(|c| c == '-' || c == ':') && t.chars().any(|c| c == '-')
        })
}

/// Byte spans of the cells of one `|`-delimited row (segments between
/// bars, plus a leading/trailing bar-less segment when non-empty).
fn row_cells(text: &str, s: usize, e: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let bars: Vec<usize> = (s..e).filter(|&i| bytes[i] == b'|').collect();
    let mut out = Vec::new();
    if let Some(&first) = bars.first() {
        if !text[s..first].trim().is_empty() {
            out.push((s, first));
        }
        for w in bars.windows(2) {
            out.push((w[0] + 1, w[1]));
        }
        if let Some(&last) = bars.last()
            && !text[last + 1..e].trim().is_empty()
        {
            out.push((last + 1, e));
        }
    }
    out
}

/// Byte length of the blockquote prefix opening `t`: a run of `>`
/// characters, each followed by any spacing (`> `, `>> `, `> > `, `>`,
/// `>   `). Zero when `t` does not open with `>` — a `>` that sits
/// mid-line, or inside inline code, is ordinary text and is never
/// stripped.
///
/// The spacing is eaten greedily rather than one space at a time so the
/// quoted form is no pickier than the plain one: `   ##ID` already mints
/// an anchor through the caller's leading trim, and `>   ##ID` must too.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#granularity")]
fn blockquote_prefix_len(t: &str) -> usize {
    let b = t.as_bytes();
    let mut i = 0usize;
    while i < b.len() && b[i] == b'>' {
        i += 1;
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
            i += 1;
        }
    }
    i
}

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
pub(super) fn take_fact_id(text: &str, s: usize, e: usize) -> (Option<String>, usize) {
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
struct Anchor<'a> {
    form: AnchorForm,
    ty: Option<&'a str>,
    id: &'a str,
    /// Byte offset into the ORIGINAL text, just past the anchor.
    content_start: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnchorForm {
    Qualified,
    Legacy,
}

struct AnchorToken<'a> {
    form: AnchorForm,
    ty: Option<&'a str>,
    id: &'a str,
    len: usize,
}

fn parse_anchor(text: &str, s: usize, e: usize) -> Option<Anchor<'_>> {
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
fn parse_anchor_token(t: &str) -> Option<AnchorToken<'_>> {
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

/// The object types an anchor may name. One entry, and that is not an
/// oversight: measured over this corpus, fenced blocks are the ONLY block
/// kind that falls outside every fact body — there are no images at all, 891
/// of 908 table rows and 84 of 96 block quotes already sit inside one.
/// A 1-based inclusive line range.
type LineRange = (usize, usize);

const KNOWN_TYPES: [(&str, BlockKind); 1] = [("code", BlockKind::Code)];

/// Bind each typed anchor to the block it covers, or record why it cannot.
///
/// Three refusals, all errors rather than silent skips:
/// an unknown type; a typed anchor that is not its block's last fact (the
/// block it would claim is not adjacent to it); and a typed anchor with no
/// matching block after it.
pub(super) fn bind_covered_blocks(doc: &mut ParsedDoc) {
    // (block index, fact index, the covered block's line range)
    let mut binds: Vec<(usize, usize, LineRange)> = Vec::new();
    let mut issues: Vec<Issue> = Vec::new();

    for (bi, b) in doc.blocks.iter().enumerate() {
        for (fi, f) in b.facts.iter().enumerate() {
            let Some(ty) = take_fact_type(&b.scan_text, f.span.0, f.span.1) else {
                continue;
            };
            let Some((_, want)) = KNOWN_TYPES.iter().find(|(n, _)| *n == ty) else {
                issues.push(Issue {
                    severity: Severity::Error,
                    line: f.line,
                    code: IssueCode::FenceBinding,
                    message: format!(
                        "`@fact/{ty}:` names an object type this markup does not implement \
                         (known: {})",
                        KNOWN_TYPES
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
                continue;
            };
            if fi + 1 != b.facts.len() {
                issues.push(Issue {
                    severity: Severity::Error,
                    line: f.line,
                    code: IssueCode::FenceBinding,
                    message: "a typed anchor must be the last fact of its block — \
                              another fact stands between it and the block it would cover"
                        .into(),
                });
                continue;
            }
            match doc.blocks.get(bi + 1) {
                Some(next) if next.kind == *want => {
                    binds.push((bi, fi, (next.line_start, next.line_end)));
                }
                _ => issues.push(Issue {
                    severity: Severity::Error,
                    line: f.line,
                    code: IssueCode::FenceBinding,
                    message: format!(
                        "`@fact/{ty}:` is not followed by a {ty} block — the anchor names \
                         a body it does not have"
                    ),
                }),
            }
        }
    }

    for (bi, fi, covers) in binds {
        doc.blocks[bi].facts[fi].covers = Some(covers);
    }
    doc.issues.extend(issues);
}

/// Segment every Text block into countable facts (PROP-043 §8):
/// a plain paragraph, the lead lines before the first list item, each
/// list item (with its continuation lines), each non-empty table body
/// cell. Header + delimiter rows of a table are structure, not facts.
pub(super) fn segment_facts(doc: &mut ParsedDoc) {
    for b in &mut doc.blocks {
        if b.kind != BlockKind::Text {
            continue;
        }
        let text = b.scan_text.clone();
        // Line starts (byte offsets) inside the block text.
        let mut offs: Vec<usize> = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                offs.push(i + 1);
            }
        }
        let n = offs.len();
        let span_of = |li: usize| -> (usize, usize) {
            let s = offs[li];
            let e = if li + 1 < n {
                offs[li + 1] - 1
            } else {
                text.len()
            };
            (s, e)
        };

        #[derive(Clone, Copy, PartialEq)]
        enum L {
            Item(usize),
            Table,
            Plain,
        }
        let classes: Vec<L> = (0..n)
            .map(|li| {
                let (s, e) = span_of(li);
                let line = &text[s..e];
                if line.trim_start().starts_with('|') {
                    L::Table
                } else if let Some(off) = list_item_content(line) {
                    L::Item(s + off)
                } else {
                    L::Plain
                }
            })
            .collect();

        let has_structure = classes.iter().any(|c| !matches!(c, L::Plain));
        if !has_structure {
            b.facts
                .push(mk_fact(FactKind::Para, &text, 0, text.len(), b.line_start));
            continue;
        }

        // Table delimiter/header rows to skip (structure, not facts).
        let mut structural: Vec<bool> = vec![false; n];
        for li in 0..n {
            if classes[li] == L::Table {
                let (s, e) = span_of(li);
                let cells = row_cells(&text, s, e);
                if is_delimiter_row(&cells, &text) {
                    structural[li] = true;
                    if li > 0 && classes[li - 1] == L::Table {
                        structural[li - 1] = true; // the header row
                    }
                }
            }
        }

        let mut li = 0usize;
        // Lead: plain lines before the first item/table line.
        if classes[0] == L::Plain {
            let mut end_li = 0;
            while end_li + 1 < n && classes[end_li + 1] == L::Plain {
                end_li += 1;
            }
            let (s, _) = span_of(0);
            let (_, e) = span_of(end_li);
            b.facts
                .push(mk_fact(FactKind::Lead, &text, s, e, b.line_start));
            li = end_li + 1;
        }
        while li < n {
            match classes[li] {
                L::Item(content) => {
                    // The item runs until the next item/table line.
                    let mut end_li = li;
                    while end_li + 1 < n && classes[end_li + 1] == L::Plain {
                        end_li += 1;
                    }
                    let (_, e) = span_of(end_li);
                    b.facts.push(mk_fact(
                        FactKind::Item,
                        &text,
                        content,
                        e,
                        b.line_start + li,
                    ));
                    li = end_li + 1;
                }
                L::Table => {
                    if !structural[li] {
                        let (s, e) = span_of(li);
                        for (cs, ce) in row_cells(&text, s, e) {
                            if !text[cs..ce].trim().is_empty() {
                                b.facts.push(mk_fact(
                                    FactKind::Cell,
                                    &text,
                                    cs,
                                    ce,
                                    b.line_start + li,
                                ));
                            }
                        }
                    }
                    li += 1;
                }
                L::Plain => {
                    // Plain lines after a table with no items: their own unit.
                    let start = li;
                    while li + 1 < n && classes[li + 1] == L::Plain {
                        li += 1;
                    }
                    let (s, _) = span_of(start);
                    let (_, e) = span_of(li);
                    b.facts
                        .push(mk_fact(FactKind::Para, &text, s, e, b.line_start + start));
                    li += 1;
                }
            }
        }
    }
}

fn mk_fact(kind: FactKind, text: &str, s: usize, e: usize, line: usize) -> Fact {
    // Cells may carry an id too (first cell of a row ⇒ the row's address,
    // any other cell ⇒ that cell's — §3.8 table addressing); only the
    // anchored-when-marked obligation exempts cells.
    let (id, _) = take_fact_id(text, s, e);
    Fact {
        kind,
        id,
        line,
        span: (s, e),
        marked: false,
        covers: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn blockquote_fact_anchor_is_taken() {
        let text = "> ##MY-FACT text @spec/done";
        let (id, at) = take_fact_id(text, 0, text.len());
        assert_eq!(id.as_deref(), Some("MY-FACT"));
        assert_eq!(&text[..at], "> ##MY-FACT");
        assert_eq!(&text[at..], " text @spec/done");
    }

    #[test]
    fn blockquote_anchor_offset_is_into_the_original_text() {
        // The span starts mid-`text`; the returned offset must still index
        // `text`, or every downstream span shifts.
        let text = "lead line\n> ##MY-FACT text @spec/done";
        let s = "lead line\n".len();
        let (id, at) = take_fact_id(text, s, text.len());
        assert_eq!(id.as_deref(), Some("MY-FACT"));
        assert_eq!(&text[at..], " text @spec/done");
    }

    #[test]
    fn nested_blockquote_anchor_is_taken() {
        for text in ["> > ##MY-FACT", ">> ##MY-FACT"] {
            let (id, at) = take_fact_id(text, 0, text.len());
            assert_eq!(id.as_deref(), Some("MY-FACT"), "text: {text}");
            assert_eq!(at, text.len(), "text: {text}");
        }
    }

    #[test]
    fn no_space_blockquote_anchor_is_taken() {
        let text = ">##MY-FACT rest";
        let (id, at) = take_fact_id(text, 0, text.len());
        assert_eq!(id.as_deref(), Some("MY-FACT"));
        assert_eq!(&text[at..], " rest");
    }

    /// The quoted form is no pickier than the plain one: whitespace is
    /// trimmed on both sides of the `>` prefix, so an author who lines a
    /// quote up with the text above it still gets an anchor.
    #[test]
    fn padded_blockquote_anchor_is_taken() {
        for text in [">   ##MY-FACT rest", ">  >  ##MY-FACT rest"] {
            let (id, at) = take_fact_id(text, 0, text.len());
            assert_eq!(id.as_deref(), Some("MY-FACT"), "text: {text}");
            assert_eq!(&text[at..], " rest", "text: {text}");
        }
    }

    #[test]
    fn gt_inside_text_is_not_stripped() {
        // Only a prefix counts: a mid-line `>` is prose.
        assert_eq!(take_fact_id("a > b ##NOT-AN-ANCHOR", 0, 21), (None, 0));
        // A quoted heading keeps the id grammar: `## ` is not `##<ID>`.
        let text = "> ## Quoted heading";
        assert_eq!(take_fact_id(text, 0, text.len()), (None, 0));
        // One `#` is not an anchor either.
        let text = "> > #hash";
        assert_eq!(take_fact_id(text, 0, text.len()), (None, 0));
    }

    #[test]
    fn blockquote_in_fence_is_not_an_anchor() {
        let text = "# H {#h}\n\n```md\n> ##QUOTED inside a fence @impl/done\n```\n";
        let doc = parse_document("x.md", text);
        assert!(
            doc.blocks.iter().all(|b| b.facts.is_empty()),
            "blocks: {:#?}",
            doc.blocks
        );
        assert!(doc.markers.is_empty(), "markers: {:#?}", doc.markers);
    }

    #[test]
    fn blockquote_paragraph_is_an_anchored_marked_fact() {
        let text = "# H {#h}\n\n> ##QUOTE-1 A quoted normative statement. @spec/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.fact_count, 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.unmarked_facts.len(), 0);
        let ids: Vec<&str> = doc
            .blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
            .collect();
        assert_eq!(ids, ["QUOTE-1"]);
    }

    /// The checkbox is structure: a task item's first token is the one
    /// after the box, so `- [ ] ##ID …` is anchored and stays legal GFM.
    #[test]
    fn task_list_checkbox_is_structure_and_the_anchor_follows_it() {
        let text = "# H {#h}\n\n- [ ] ##TASK-1 An unticked task. @impl/done\n\
                    - [x] ##TASK-2 A ticked one. @impl/done\n\
                      - [X] ##TASK-3 A nested one, capital X. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.fact_count, 3, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        let ids: Vec<&str> = doc
            .blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
            .collect();
        assert_eq!(ids, ["TASK-1", "TASK-2", "TASK-3"]);
    }

    /// A bare task item is a countable unit that nobody marked — exactly
    /// what an unmarked plain item is. Nothing about the box changes that.
    #[test]
    fn bare_task_list_item_stays_an_unmarked_unit() {
        let text = "# H {#h}\n\n- [ ] Just a box and some prose.\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.fact_count, 1);
        assert_eq!(doc.unmarked_facts.len(), 1);
    }

    /// Only a real box is structure: brackets that hold anything else, or
    /// that are glued to the following word, are the item's own text.
    #[test]
    fn bracket_lookalikes_are_not_task_boxes() {
        for t in ["[ ] rest", "[x] rest", "[X]\trest", "[ ]"] {
            assert!(task_box_len(t) > 0, "should be a box: {t:?}");
        }
        for t in [
            "[y] rest",
            "[ ]glued",
            "[  ] rest",
            "[]",
            "[ x] rest",
            "no box",
        ] {
            assert_eq!(task_box_len(t), 0, "should not be a box: {t:?}");
        }
    }
}
