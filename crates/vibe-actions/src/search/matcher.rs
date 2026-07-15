//! The single tiered scorer behind Search Everywhere (PROP-039 §10.3).
//!
//! One matcher produces **both** a score and the byte highlight ranges (fixes
//! the incumbents' two-engine mismatch, DO7) so every provider ranks on **one
//! commensurable scale**. The tier ladder, highest first:
//!
//! - **exact** (`equalIgnoreCase`) — base [`EXACT`];
//! - **prefix** — base [`PREFIX`] plus a short-haystack bonus;
//! - **word-boundary prefix** (a word start after a `._-/ ` separator or a
//!   camelHump) — base [`WORD_PREFIX`];
//! - **substring** — base [`SUBSTRING`];
//! - **subsequence** (all query chars in order) — base [`SUBSEQUENCE`] plus a
//!   consecutive-run / word-boundary bonus.
//!
//! Matching is case-insensitive; ranges are **byte** offsets into the original
//! `haystack`, so a surface can highlight the matched span directly. An empty
//! query matches everything at score `0` with no ranges (the empty-pattern
//! lane the engine drains in provider order).
//!
//! Spec: [PROP-039 §10.3](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#se-ranking).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#search-everywhere");

/// Tier base for an exact (case-insensitive) match.
pub(crate) const EXACT: i64 = 1000;
/// Tier base for a prefix match (before the short-haystack bonus).
pub(crate) const PREFIX: i64 = 800;
/// Tier base for a word-boundary prefix match.
pub(crate) const WORD_PREFIX: i64 = 700;
/// Tier base for an interior substring match.
pub(crate) const SUBSTRING: i64 = 600;
/// Tier base for a scattered subsequence match (before the run/boundary bonus).
pub(crate) const SUBSEQUENCE: i64 = 400;

/// Characters that begin a new word for word-boundary matching (§10.3).
const SEPARATORS: [char; 5] = ['.', '_', '-', '/', ' '];

/// The largest bonus a subsequence match may earn, kept strictly below the gap
/// to [`SUBSTRING`] so the tier ladder never inverts.
const MAX_SUBSEQ_BONUS: i64 = SUBSTRING - SUBSEQUENCE - 1;

/// Score `query` against `haystack`, returning the tier score and the byte
/// highlight ranges — or `None` when nothing matches. Case-insensitive; an
/// empty query is a zero-score match with no ranges.
///
/// The ranges are non-overlapping byte spans into `haystack`, in order.
pub(crate) fn score(query: &str, haystack: &str) -> Option<(i64, Vec<(usize, usize)>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let q: Vec<char> = query.chars().collect();
    let h: Vec<(usize, char)> = haystack.char_indices().collect();
    let hlen = haystack.len();

    if is_exact(&h, &q) {
        return Some((EXACT, vec![(0, hlen)]));
    }
    if let Some(end) = prefix_end(&h, &q, 0, hlen) {
        // Shorter haystacks score higher: a full-length prefix is nearly exact.
        let bonus = (q.len() as i64 * 100) / (h.len().max(1) as i64);
        return Some((PREFIX + bonus, vec![(0, end)]));
    }
    if let Some(span) = word_boundary_prefix(&h, &q, hlen) {
        return Some((WORD_PREFIX, vec![span]));
    }
    if let Some(span) = substring(&h, &q, hlen) {
        return Some((SUBSTRING, vec![span]));
    }
    if let Some((bonus, ranges)) = subsequence(&h, &q, hlen) {
        return Some((SUBSEQUENCE + bonus, ranges));
    }
    None
}

/// Case-insensitive character equality (ASCII fast path, then full Unicode
/// lowercase fold).
fn ci_eq(a: char, b: char) -> bool {
    a == b || a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
}

/// Whether `haystack` equals `query` ignoring case (same char count, all pairs
/// equal).
fn is_exact(h: &[(usize, char)], q: &[char]) -> bool {
    h.len() == q.len() && h.iter().zip(q).all(|((_, hc), qc)| ci_eq(*hc, *qc))
}

/// If `h[start..]` begins with `q` (case-insensitively), return the byte offset
/// just past the matched run; otherwise `None`.
fn prefix_end(h: &[(usize, char)], q: &[char], start: usize, hlen: usize) -> Option<usize> {
    if start + q.len() > h.len() {
        return None;
    }
    for (k, qc) in q.iter().enumerate() {
        if !ci_eq(h[start + k].1, *qc) {
            return None;
        }
    }
    let after = start + q.len();
    Some(if after < h.len() { h[after].0 } else { hlen })
}

/// Whether char index `i` begins a word — the string start, a char after a
/// [`SEPARATORS`] char, or a camelHump (a non-uppercase followed by uppercase).
fn is_boundary(h: &[(usize, char)], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = h[i - 1].1;
    let cur = h[i].1;
    SEPARATORS.contains(&prev) || (cur.is_uppercase() && !prev.is_uppercase())
}

/// The leftmost word-boundary at which `q` is a prefix (byte span of the match).
fn word_boundary_prefix(h: &[(usize, char)], q: &[char], hlen: usize) -> Option<(usize, usize)> {
    for i in 1..h.len() {
        if is_boundary(h, i)
            && let Some(end) = prefix_end(h, q, i, hlen)
        {
            return Some((h[i].0, end));
        }
    }
    None
}

/// The leftmost contiguous (case-insensitive) occurrence of `q` in `h`.
fn substring(h: &[(usize, char)], q: &[char], hlen: usize) -> Option<(usize, usize)> {
    if q.is_empty() || q.len() > h.len() {
        return None;
    }
    for start in 0..=(h.len() - q.len()) {
        if let Some(end) = prefix_end(h, q, start, hlen) {
            return Some((h[start].0, end));
        }
    }
    None
}

/// Greedy left-to-right subsequence match: every `q` char in order, earliest
/// available. Returns the run/boundary bonus and the merged byte ranges, or
/// `None` if some query char never appears after its predecessor.
fn subsequence(h: &[(usize, char)], q: &[char], hlen: usize) -> Option<(i64, Vec<(usize, usize)>)> {
    let mut matched: Vec<usize> = Vec::with_capacity(q.len());
    let mut hi = 0usize;
    for &qc in q {
        let mut found = None;
        while hi < h.len() {
            let here = hi;
            hi += 1;
            if ci_eq(h[here].1, qc) {
                found = Some(here);
                break;
            }
        }
        matched.push(found?);
    }

    let mut bonus = 0i64;
    let mut run = 0i64;
    for pair in matched.windows(2) {
        if pair[1] == pair[0] + 1 {
            run += 1;
            bonus += run * 10;
        } else {
            run = 0;
        }
    }
    for &idx in &matched {
        if is_boundary(h, idx) {
            bonus += 15;
        }
    }
    Some((bonus.min(MAX_SUBSEQ_BONUS), merge_ranges(h, &matched, hlen)))
}

/// Turn matched char indices into merged byte ranges (adjacent chars coalesce
/// into one span).
fn merge_ranges(h: &[(usize, char)], idxs: &[usize], hlen: usize) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &idx in idxs {
        let start = h[idx].0;
        let end = if idx + 1 < h.len() {
            h[idx + 1].0
        } else {
            hlen
        };
        if let Some(last) = ranges.last_mut()
            && last.1 == start
        {
            last.1 = end;
            continue;
        }
        ranges.push((start, end));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(query: &str, haystack: &str) -> i64 {
        score(query, haystack).expect("expected a match").0
    }

    #[test]
    fn empty_query_is_a_zero_score_match() {
        let (sc, ranges) = score("", "anything").unwrap();
        assert_eq!(sc, 0);
        assert!(ranges.is_empty());
    }

    #[test]
    fn exact_tops_the_ladder() {
        assert_eq!(s("copy", "copy"), EXACT);
        // case-insensitive exact still exact.
        assert_eq!(s("COPY", "copy"), EXACT);
    }

    #[test]
    fn prefix_beats_word_boundary_beats_substring_beats_subsequence() {
        let prefix = s("cop", "copyfile"); // starts with
        let word = s("cop", "do.copy"); // word start after `.`
        let substr = s("cop", "scope"); // interior, not a word start
        let subseq = s("cop", "c_o_x_p"); // scattered
        assert!(prefix > word, "prefix {prefix} !> word {word}");
        assert!(word > substr, "word {word} !> substr {substr}");
        assert!(substr > subseq, "substr {substr} !> subseq {subseq}");
        assert!(prefix < EXACT);
    }

    #[test]
    fn prefix_bonus_favours_shorter_haystacks() {
        let short = s("cop", "cop9"); // query nearly the whole haystack
        let long = s("cop", "copytheverylongname");
        assert!(short > long, "short {short} !> long {long}");
    }

    #[test]
    fn word_boundary_after_camel_hump() {
        // "Parser" starts at a camelHump inside "xmlParser".
        assert_eq!(s("par", "xmlParser"), WORD_PREFIX);
    }

    #[test]
    fn substring_span_is_the_found_bytes() {
        let (sc, ranges) = score("cop", "scope").unwrap();
        assert_eq!(sc, SUBSTRING);
        assert_eq!(ranges, vec![(1, 4)]); // "cop" at bytes 1..4 of "scope"
    }

    #[test]
    fn subsequence_matches_in_order_and_merges_runs() {
        // "cop": c at 0, then o,p consecutive → merged into one run span.
        let (sc, ranges) = score("cop", "c_op").unwrap();
        assert!(sc >= SUBSEQUENCE);
        assert!(sc < SUBSTRING, "subseq {sc} must stay below substring");
        // c at byte 0 (one span), "op" at bytes 2..4 (merged).
        assert_eq!(ranges, vec![(0, 1), (2, 4)]);
    }

    #[test]
    fn no_match_returns_none() {
        assert!(score("zzz", "copy").is_none());
        // query longer than haystack cannot match.
        assert!(score("copies", "cop").is_none());
    }

    #[test]
    fn ranges_are_byte_offsets_over_multibyte_haystack() {
        // "é" is two bytes; a match after it must report byte, not char, offsets.
        let (_, ranges) = score("ab", "éab").unwrap();
        assert_eq!(ranges, vec![(2, 4)]); // "ab" lives at bytes 2..4
    }

    #[test]
    fn case_insensitive_across_tiers() {
        assert_eq!(s("COP", "Copyfile"), s("cop", "copyfile"));
    }
}
