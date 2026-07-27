//! Scanning primitives for `batch-review` — pure text in, pure data out.
//!
//! No I/O, no git, no `progress-core`. The independence is the point (see the
//! module doc of the parent): a cross-check that shares the instrument's bugs
//! is not a cross-check, so every rule here is read off PROP-043 and
//! re-implemented rather than imported.
//!
//! Each approximation is named at its function, and the rule they all obey is:
//! **an approximation may only ever ADMIT a candidate for checking, never
//! silently suppress one.**
//!
//! The run-matched delimiters this module scans over — what a backtick or
//! tilde run opens and closes — live in [`super::fences`].

use super::fences::{blank_fences, is_fence_line};

// ---------------------------------------------------------------- vocabulary
// spec://vibevm/modules/vibe-progress/PROP-043#stages / #states / #actions
pub(super) const STAGES: &[&str] = &["idea", "spec", "impl", "test", "doc", "freeze", "unknown"];
pub(super) const STATES: &[&str] = &["plan", "work", "done", "hold", "void"];
pub(super) const ACTIONS: &[&str] = &["continue", "drift", "rework", "remove"];
pub(super) const AUDIENCES: &[&str] = &["user", "author", "dev"];

/// Characters a deconstruction may introduce as a list marker.
pub(super) const BULLET_TOKENS: &[&str] = &["-", "+", "*", "\u{2022}"];

pub(super) fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The delimiter set around a shorthand token: a `@word` touching any of these
/// is part of something else (`@ts-ignore`, `a@b`, `spec://…/@x`).
pub(super) fn is_shorthand_delim(c: char) -> bool {
    is_ident(c) || c == '/' || c == '-'
}

// ---------------------------------------------------------------- scanning
/// One `<status …>` or `</status>` element found in the text.
pub(super) struct StatusEl {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) body: String,
}

pub(super) fn find_status_elements(text: &str) -> Vec<StatusEl> {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let opens_self = starts_with_at(&bytes, i, "<status")
            && bytes
                .get(i + 7)
                .is_some_and(|c| c.is_whitespace() || *c == '/' || *c == '>');
        let opens_close = starts_with_at(&bytes, i, "</status");
        if (opens_self || opens_close)
            && let Some(end) = (i..bytes.len()).find(|&k| bytes[k] == '>')
        {
            out.push(StatusEl {
                start: i,
                end: end + 1,
                body: bytes[i..=end].iter().collect(),
            });
            i = end + 1;
            continue;
        }
        i += 1;
    }
    out
}

pub(super) fn starts_with_at(chars: &[char], at: usize, pat: &str) -> bool {
    pat.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

/// `key="value"` pairs inside a status element body.
pub(super) fn attributes(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let ks = i;
        while i < chars.len() && is_ident(chars[i]) {
            i += 1;
        }
        let key: String = chars[ks..i].iter().collect();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if chars.get(i) != Some(&'=') {
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if chars.get(i) != Some(&'"') {
            continue;
        }
        i += 1;
        let vs = i;
        while i < chars.len() && chars[i] != '"' {
            i += 1;
        }
        out.push((key, chars[vs..i].iter().collect()));
        i += 1;
    }
    out
}

/// A `@stage[/state]` shorthand: byte range, stage, optional state.
pub(super) struct Shorthand {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) stage: String,
    pub(super) state: Option<String>,
}

/// Every delimited `@stage[/state]` token in the text.
///
/// A shorthand is a standalone token: `@ts-ignore` and `@typescript-eslint` are
/// not shorthands, and neither is anything glued to a word or a path. The first
/// implementation matched bare `@word` anywhere and produced 45 false positives
/// on one batch.
pub(super) fn shorthands(text: &str) -> Vec<Shorthand> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '@' {
            i += 1;
            continue;
        }
        if i > 0 && is_shorthand_delim(chars[i - 1]) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let ss = j;
        while j < chars.len() && chars[j].is_ascii_lowercase() {
            j += 1;
        }
        if j == ss {
            i += 1;
            continue;
        }
        let stage: String = chars[ss..j].iter().collect();
        let mut state = None;
        if chars.get(j) == Some(&'/') {
            let vs = j + 1;
            let mut k = vs;
            while k < chars.len() && chars[k].is_ascii_lowercase() {
                k += 1;
            }
            if k == vs {
                // `@spec/` with no state: the whole token is rejected, never
                // silently downgraded to a bare `@spec`.
                i += 1;
                continue;
            }
            state = Some(chars[vs..k].iter().collect());
            j = k;
        }
        if chars.get(j).is_some_and(|c| is_shorthand_delim(*c)) {
            i += 1;
            continue;
        }
        out.push(Shorthand {
            start: i,
            end: j,
            stage,
            state,
        });
        i = j;
    }
    out
}

/// Shorthands in MARKER position: standalone, at a line's start or end.
///
/// APPROXIMATION, and it only ever admits: `##SHORTHAND-FORMS` allows a
/// shorthand as the first or last token of a *unit*; line edges are a looser
/// proxy for unit edges, so this can offer a candidate that is not a marker —
/// it can never hide one that is.
pub(super) fn marker_shorthands(text: &str) -> Vec<(usize, String, Option<String>)> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        for sh in shorthands(line) {
            let head: String = chars[..sh.start].iter().collect();
            let tail: String = chars[sh.end..].iter().collect();
            if head.trim().is_empty() || tail.trim().is_empty() {
                out.push((sh.start, sh.stage.clone(), sh.state.clone()));
            }
        }
    }
    out
}

/// `##FACT-ID` anchors.
pub(super) fn fact_anchors(text: &str) -> Vec<(usize, usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < chars.len() {
        if chars[i] == '#' && chars[i + 1] == '#' && chars[i + 2].is_ascii_alphabetic() {
            let mut j = i + 2;
            while j < chars.len()
                && (chars[j].is_ascii_alphanumeric() || matches!(chars[j], '_' | '-'))
            {
                j += 1;
            }
            out.push((i, j, chars[i + 2..j].iter().collect()));
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// `{#heading-anchor}` ids.
pub(super) fn heading_anchors(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '#' && chars[i + 2].is_ascii_alphabetic() {
            let mut j = i + 2;
            while j < chars.len()
                && (chars[j].is_ascii_alphanumeric() || matches!(chars[j], '_' | '-'))
            {
                j += 1;
            }
            if chars.get(j) == Some(&'}') {
                out.push(chars[i + 2..j].iter().collect());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub(super) fn is_bullet_line(line: &str) -> bool {
    let t = line.trim_start();
    let mut cs = t.chars();
    match cs.next() {
        Some('-') | Some('*') | Some('+') => cs.next().is_some_and(|c| c == ' ' || c == '\t'),
        Some(d) if d.is_ascii_digit() => {
            let rest: String = cs.collect();
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let after = &rest[digits.len()..];
            after.starts_with(". ") || after.starts_with(".\t")
        }
        _ => false,
    }
}

/// A heading has hashes then a SPACE. `##ANCHOR` does not — excluding every
/// line beginning with `#` made the lazy-continuation check report clean on
/// the one tree known to contain two real cases.
pub(super) fn is_heading_line(line: &str) -> bool {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes)
        && line
            .chars()
            .nth(hashes)
            .is_some_and(|c| c == ' ' || c == '\t')
}

/// The author's words, with everything a markup pass may add removed.
///
/// BLIND SPOT 1: emphasis asterisks are stripped, so an emphasis change is
/// invisible here. Ruling 12 licenses re-applying `*` when an italic paragraph
/// is split, which is why; the asterisk *delta* is reported separately, because
/// ruling 12 permits an increase and never a decrease.
///
/// BLIND SPOT 2: bullet characters are dropped as standalone tokens anywhere,
/// never by line position. A position-sensitive rule is unsafe because reflow
/// is legal: one batch's prose reads "the four queries + lifecycle", and in the
/// pre-batch revision that `+` began a wrapped line. The cost is that a
/// standalone `-`, `+` or `*` added or removed in prose is invisible.
pub(super) fn word_stream(text: &str) -> Vec<String> {
    let mut t: Vec<char> = text.chars().collect();

    let blank = |t: &mut Vec<char>, from: usize, to: usize| {
        for slot in t.iter_mut().take(to).skip(from) {
            *slot = ' ';
        }
    };

    let s: String = t.iter().collect();
    for el in find_status_elements(&s) {
        blank(&mut t, el.start, el.end);
    }
    let s: String = t.iter().collect();
    for sh in shorthands(&s) {
        blank(&mut t, sh.start, sh.end);
    }
    let s: String = t.iter().collect();
    for (a, b, _) in fact_anchors(&s) {
        blank(&mut t, a, b);
    }

    let s: String = t.iter().collect();
    let mut out = String::new();
    for line in s.split('\n') {
        // Line-start ordinals are list markers a deconstruction may add.
        let trimmed = line.trim_start();
        let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        let rest = &trimmed[digits.len()..];
        if !digits.is_empty() && (rest.starts_with(". ") || rest.starts_with(".\t")) {
            out.push_str(&rest[2..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    // `{#anchor}` and emphasis.
    let mut cleaned = String::new();
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{'
            && chars.get(i + 1) == Some(&'#')
            && let Some(close) = (i..chars.len()).find(|&k| chars[k] == '}')
        {
            let inner: String = chars[i + 2..close].iter().collect();
            if !inner.is_empty()
                && inner
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && inner
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            {
                cleaned.push(' ');
                i = close + 1;
                continue;
            }
        }
        cleaned.push(if chars[i] == '*' { ' ' } else { chars[i] });
        i += 1;
    }

    cleaned
        .split_whitespace()
        .filter(|tok| !BULLET_TOKENS.contains(tok))
        .map(str::to_string)
        .collect()
}

/// The same stream with ruling-47 hyphen joins collapsed: a token ending in
/// `-` is glued to the token after it.
///
/// Ruling 47 licenses moving one word across a newline when an author's wrap
/// left a hyphen at a line end, because `word-\nrest` renders as `word- rest`.
/// No text byte changes, but the whitespace-split stream does — so a legal
/// repair reads as a reworded sentence to [`word_stream`].
///
/// This does NOT relax C3. It is only ever used to *classify* an already-
/// detected divergence: if the raw streams differ and the joined streams do
/// not, the difference is exactly a hyphen join and belongs in a judgement
/// queue rather than a failure. Anything else still fails. The tool's standing
/// law is that an approximation may admit a candidate for checking and never
/// suppress one, and reporting a licensed repair as an unexplained rewording
/// is the same defect pointing the other way: it teaches the reviewer to
/// discount C3.
pub(super) fn hyphen_joined(words: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in words {
        match out.last_mut() {
            Some(prev) if prev.ends_with('-') => prev.push_str(w),
            _ => out.push(w.clone()),
        }
    }
    out
}

/// Paragraphs sitting directly after a list item: `(line-no, first words)`.
///
/// Matched on the result and then diffed against the base — a paragraph that
/// already sat after a list in the source is the author's own layout, not a
/// ruling-30 repair. Without the diff this reported 19 candidates for one
/// batch's 2 real cases; with it, 2.
pub(super) fn lazy_signature(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<String> = blank_fences(text).split('\n').map(str::to_string).collect();
    let mut out = Vec::new();
    for i in 2..lines.len() {
        let (cur, blank, prev) = (&lines[i], &lines[i - 1], &lines[i - 2]);
        if !blank.trim().is_empty() || !is_bullet_line(prev) {
            continue;
        }
        let first = cur.chars().next();
        if first.is_none_or(|c| c == ' ' || c == '\t' || c == '>')
            || is_heading_line(cur)
            || is_bullet_line(cur)
            || is_fence_line(cur)
        {
            continue;
        }
        let key = word_stream(cur)
            .into_iter()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");
        out.push((i + 1, key));
    }
    out
}

// ------------------------------------------------------------- controls
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# T {#root}

<status stage=\"spec\" state=\"done\"/>

##FACT-ONE The *quick* brown fox jumps. @impl/done

##fact-two It landed cleanly. @spec/done
";

    #[test]
    fn a_reworded_sentence_is_caught() {
        let a = word_stream(SAMPLE);
        let b = word_stream(&SAMPLE.replace("jumps", "leaps"));
        assert_ne!(a, b, "a reworded sentence must move the word stream");
    }

    #[test]
    fn a_dropped_word_is_caught() {
        let a = word_stream(SAMPLE);
        let b = word_stream(&SAMPLE.replace("brown ", ""));
        assert_ne!(a, b);
    }

    #[test]
    fn markup_alone_leaves_the_word_stream_alone() {
        let bare = "The quick brown fox jumps.\n";
        let marked = "##FACT-ONE The quick brown fox jumps. @impl/done\n";
        assert_eq!(word_stream(bare), word_stream(marked));
    }

    #[test]
    fn a_paragraph_split_into_bullets_leaves_the_word_stream_alone() {
        let before = "one thing; two thing\n";
        let after = "- ##A one thing; @impl/done\n- ##B two thing @impl/done\n";
        assert_eq!(word_stream(before), word_stream(after));
    }

    /// The bug the first implementation shipped: a `+` beginning a wrapped
    /// line was eaten as a bullet, so a legal reflow read as a word change.

    #[test]
    fn a_plus_in_prose_survives_a_reflow() {
        let wrapped = "the four queries\n+ lifecycle, full stop\n";
        let inline = "the four queries + lifecycle, full stop\n";
        assert_eq!(word_stream(wrapped), word_stream(inline));
    }

    #[test]
    fn the_lazy_continuation_shape_is_surfaced() {
        let body = "# T {#root}\n\n- ##ITEM-ONE first item @impl/done\n\
                    - ##ITEM-TWO second item @impl/done\n\n\
                    ##RULE-SOMETHING *Rule:* the law over the list. @impl/done\n";
        let sig = lazy_signature(body);
        assert_eq!(sig.len(), 1, "expected exactly one candidate, got {sig:?}");
    }

    #[test]
    fn a_heading_after_a_list_is_not_a_lazy_continuation() {
        let body = "# T {#root}\n\n- ##ITEM-ONE first @impl/done\n\n## Next section {#next}\n";
        assert!(lazy_signature(body).is_empty());
    }

    /// The first ruling-47 repair ever made (B14) failed C3, because moving a
    /// word across a newline changes the whitespace-split stream while
    /// changing no text byte. Joined on both sides, the two agree.
    #[test]
    fn a_wrapped_hyphen_repair_is_word_identical_once_joined() {
        let wrapped = "followed by clause-by-\nclause commentary.\n";
        let repaired = "followed by clause-by-clause\ncommentary.\n";
        assert_ne!(
            word_stream(wrapped),
            word_stream(repaired),
            "the raw streams must still differ, or C3 would never see it"
        );
        assert_eq!(
            hyphen_joined(&word_stream(wrapped)),
            hyphen_joined(&word_stream(repaired))
        );
    }

    /// NEGATIVE CONTROL: joining does not make a real rewording disappear.
    #[test]
    fn a_reworded_sentence_survives_hyphen_joining() {
        let a = word_stream("followed by clause-by-\nclause commentary.\n");
        let b = word_stream("followed by clause-by-clause\nnotes.\n");
        assert_ne!(hyphen_joined(&a), hyphen_joined(&b));
    }

    /// NEGATIVE CONTROL: a dropped word next to a hyphen is not absorbed.
    #[test]
    fn a_dropped_word_beside_a_hyphen_survives_joining() {
        let a = word_stream("the pre- and post-conditions hold\n");
        let b = word_stream("the pre- post-conditions hold\n");
        assert_ne!(hyphen_joined(&a), hyphen_joined(&b));
    }
}
