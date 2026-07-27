//! Run-matched delimiters for `batch-review`: what a backtick or tilde
//! *run* opens, and what closes it.
//!
//! This mirrors `progress-core`'s own `parse::delimiters` cell without
//! importing it — the tool is a second opinion, and a cross-check that
//! shares the instrument's code is not a cross-check. What it must share is
//! the *rule*: **a delimiter is a run, and only a run of at least the same
//! width closes it.** The two disagreed on that once, in the same direction,
//! which is why neither caught it.

/// The fence run a line opens with — its character and how many of it.
///
/// The **run length** is what lets a block nest. A document quoting fenced
/// markdown opens with four backticks and holds three-backtick blocks as
/// content; matching the `` ``` `` prefix alone reads that inner opener as
/// the outer closer and inverts everything after it.
pub(super) fn fence_run(trimmed: &str) -> Option<(char, usize)> {
    let ch = trimmed.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == ch).count();
    (len >= 3).then_some((ch, len))
}

/// Whether `trimmed` closes a fence opened by `open` of character `ch`:
/// the same character, a run at least as long, and nothing else on the
/// line (an info string makes it an opener, never a closer).
fn closes_fence(trimmed: &str, ch: char, open: usize) -> bool {
    fence_run(trimmed).is_some_and(|(c, n)| c == ch && n >= open)
        && trimmed.trim_end().chars().all(|c| c == ch)
}

/// Whether the line is a fence line at all, of either character and any
/// width — used to end a lazy-continuation candidate, where only the fact
/// that a fence starts here matters, never which one it closes.
pub(super) fn is_fence_line(line: &str) -> bool {
    fence_run(line.trim_start()).is_some()
}

/// Blank inline code spans on lines that are OUTSIDE a fenced block.
///
/// A fence is itself a backtick run, so blanking code spans over the whole
/// document swallows every fenced block — which made the fence check vacuous
/// in the first implementation: it compared "markers with fences eaten" against
/// "markers with fences blanked" and the two were equal by construction, so it
/// could never fire. The port's own control caught it; the Python original had
/// shipped with a check that could not fail.
pub(super) fn blank_code_spans_outside_fences(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.split('\n') {
        let t = line.trim_start();
        if let Some((ch, open)) = fence {
            out.push(line.to_string());
            if closes_fence(t, ch, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_run(t) {
            fence = Some(open);
            out.push(line.to_string());
            continue;
        }
        out.push(blank_code_spans(line));
    }
    out.join("\n")
}

/// Blank inline `` `code` ``, preserving length so offsets survive.
///
/// APPROXIMATION: a code span is a run of N backticks closed by a run of
/// exactly N. An unterminated run is left alone rather than swallowing the
/// rest of the document — which is precisely the failure F-084 was.
pub(super) fn blank_code_spans(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<char> = chars.clone();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '`' {
            i += 1;
            continue;
        }
        let open = run_len(&chars, i);
        let mut j = i + open;
        let close = loop {
            if j >= chars.len() {
                break None;
            }
            if chars[j] == '`' {
                let n = run_len(&chars, j);
                if n == open {
                    break Some(j);
                }
                j += n;
                continue;
            }
            j += 1;
        };
        match close {
            Some(end) => {
                for slot in out.iter_mut().take(end + open).skip(i) {
                    *slot = ' ';
                }
                i = end + open;
            }
            None => i += open,
        }
    }
    out.into_iter().collect()
}

pub(super) fn run_len(chars: &[char], at: usize) -> usize {
    let mut n = 0;
    while at + n < chars.len() && chars[at + n] == '`' {
        n += 1;
    }
    n
}

/// Replace fenced-code lines with empty ones, keeping line numbering.
pub(super) fn blank_fences(text: &str) -> String {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.split('\n') {
        let t = line.trim_start();
        if let Some((ch, open)) = fence {
            out.push(String::new());
            if closes_fence(t, ch, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_run(t) {
            fence = Some(open);
            out.push(String::new());
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fifth bug of the same species: a delimiter matched by prefix
    /// instead of by run. A four-backtick block quoting three-backtick ones
    /// was closed by the first inner opener, so the tool blanked the wrong
    /// half of it — the quoted commands stayed in the word stream and the
    /// prose after them dropped out. `progress-core` had the identical
    /// defect, which is why the two agreed and neither caught it.
    #[test]
    fn a_shorter_run_inside_a_longer_fence_is_content() {
        let body = "````markdown\n```\nacme init\n```\n\n**Expected.** It exits 0.\n````\n";
        assert_eq!(
            blank_fences(body).trim(),
            "",
            "the whole quoted block is code"
        );
    }

    /// The converse, so the fix cannot strand a block open and swallow the
    /// rest of a file: a longer run still closes a shorter opener.
    #[test]
    fn a_longer_run_closes_a_shorter_fence() {
        let body = "```\ncode\n````\n\nprose after the block\n";
        assert_eq!(blank_fences(body).trim(), "prose after the block");
    }

    /// A tilde fence is not closed by backticks, whatever their width.
    #[test]
    fn a_fence_is_never_closed_by_the_other_character() {
        let body = "~~~\n```\ncode\n```\n~~~\n\nprose after the block\n";
        assert_eq!(blank_fences(body).trim(), "prose after the block");
    }

    /// Inline spans keep their own rule, and it differs from
    /// `progress-core`'s deliberately: this side blanks the delimiters
    /// **with** the contents, because a word stream has no use for a
    /// backtick. What both sides owe is length preservation — offsets must
    /// survive — and inertness on an unpaired run (F-084).
    #[test]
    fn a_closed_span_is_blanked_whole_and_an_open_one_is_inert() {
        for s in ["a `bc` d", "a ``b`c`` d", "x ```y``` z"] {
            let out = blank_code_spans(s);
            assert_eq!(out.chars().count(), s.chars().count(), "{s:?} kept length");
            assert!(!out.contains('`'), "{s:?} -> {out:?} still has a delimiter");
        }
        // No closer of the same width: literal text, untouched.
        assert_eq!(blank_code_spans("a ` b c"), "a ` b c");
    }

    /// A span inside a fenced block is left alone: the fence wins, so a
    /// backtick run in code cannot be mistaken for a span delimiter.
    #[test]
    fn spans_inside_a_fence_are_left_alone() {
        let body = "```\na `span` in code\n```\n\nprose with a `span` in it\n";
        let out = blank_code_spans_outside_fences(body);
        assert!(out.contains("a `span` in code"), "fenced span untouched");
        let prose = out.lines().last().unwrap_or_default();
        assert!(
            prose.starts_with("prose with a ") && !prose.contains('`'),
            "prose span blanked: {prose:?}"
        );
    }
}
