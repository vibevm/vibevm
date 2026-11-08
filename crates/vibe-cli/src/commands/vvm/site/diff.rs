//! A small line diff for `vibe self apply --diff`: profiles are a few
//! hundred lines, so a longest-common-subsequence table is plenty.

/// Above this many line pairs the diff is not computed, only announced.
const MAX_CELLS: usize = 4_000_000;
/// Unchanged lines shown around each change.
const CONTEXT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Keep,
    Remove,
    Add,
}

/// A unified-style diff of `old` → `new` (`-` removed, `+` added, two
/// lines of context, `@@` between distant hunks); empty when equal.
///
/// The edit script comes from an LCS table filled from the end, so the walk
/// forward always knows whether keeping, removing or adding is optimal.
pub(super) fn line_diff(old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if (a.len() + 1).saturating_mul(b.len() + 1) > MAX_CELLS {
        return format!(
            "(the files differ: {} lines → {} lines, too long to diff here)\n",
            a.len(),
            b.len()
        );
    }
    let width = b.len() + 1;
    let mut table = vec![0usize; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let mut script: Vec<(Op, &str)> = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            script.push((Op::Keep, a[i]));
            i += 1;
            j += 1;
        } else if i < a.len()
            && (j == b.len() || table[(i + 1) * width + j] >= table[i * width + j + 1])
        {
            script.push((Op::Remove, a[i]));
            i += 1;
        } else {
            script.push((Op::Add, b[j]));
            j += 1;
        }
    }
    render(&script)
}

fn render(script: &[(Op, &str)]) -> String {
    let changed: Vec<usize> = script
        .iter()
        .enumerate()
        .filter(|(_, (op, _))| *op != Op::Keep)
        .map(|(index, _)| index)
        .collect();
    let near = |index: usize| {
        changed
            .iter()
            .any(|&change| index + CONTEXT >= change && index <= change + CONTEXT)
    };
    let mut out = String::new();
    let mut last_shown: Option<usize> = None;
    for (index, (op, line)) in script.iter().enumerate() {
        if *op == Op::Keep && !near(index) {
            continue;
        }
        if last_shown.is_some_and(|last| index > last + 1) {
            out.push_str("@@\n");
        }
        let mark = match op {
            Op::Keep => ' ',
            Op::Remove => '-',
            Op::Add => '+',
        };
        out.push(mark);
        out.push_str(line);
        out.push('\n');
        last_shown = Some(index);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::line_diff;

    #[test]
    fn equal_texts_have_no_diff_and_a_change_shows_its_context() {
        assert_eq!(line_diff("a\nb\n", "a\nb\n"), "");
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
        let new = "1\n2\n3\nfour\n5\n6\n7\n8\n9\n10\neleven\n";
        assert_eq!(
            line_diff(old, new),
            " 2\n 3\n-4\n+four\n 5\n 6\n@@\n 9\n 10\n+eleven\n"
        );
        assert_eq!(line_diff("", "x\n"), "+x\n");
        assert_eq!(line_diff("x\n", ""), "-x\n");
    }
}
