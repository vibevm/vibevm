//! TAP (node's `--test-reporter=tap`) into the language-free test-gate
//! model — the TS twin of `testgate::parse_nextest_output`, feeding the
//! SAME `testgate::evaluate`, so the two languages cannot disagree on
//! what a test result is.
//!
//! node emits TAP 13 with nested subtests indented; the leaf lines are
//! `ok N - name` / `not ok N - name`, with `# SKIP`/`# TODO` directives
//! on skipped tests. Parent-suite summary lines repeat the child names
//! at lower indentation — the LAST occurrence of a name wins, which is
//! the outermost (whole-suite) verdict.

use std::collections::BTreeMap;

use specmap_core::testgate::RunStatus;

/// Parse a TAP stream into `test name → status`.
pub fn parse_tap_output(output: &str) -> BTreeMap<String, RunStatus> {
    let mut results = BTreeMap::new();
    for raw in output.lines() {
        let line = raw.trim_start();
        let (ok, rest) = if let Some(rest) = line.strip_prefix("not ok ") {
            (false, rest)
        } else if let Some(rest) = line.strip_prefix("ok ") {
            (true, rest)
        } else {
            continue;
        };
        // `N - name # directive` — the name is between the first ` - `
        // and the directive hash (if any).
        let Some((_, name_part)) = rest.split_once(" - ") else {
            continue;
        };
        let (name, directive) = match name_part.split_once(" # ") {
            Some((n, d)) => (n.trim(), Some(d.trim().to_ascii_uppercase())),
            None => (name_part.trim(), None),
        };
        if name.is_empty() {
            continue;
        }
        let status = match directive.as_deref() {
            Some(d) if d.starts_with("SKIP") || d.starts_with("TODO") => RunStatus::Skip,
            _ if ok => RunStatus::Pass,
            _ => RunStatus::Fail,
        };
        results.insert(name.to_string(), status);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLAY: &str = "TAP version 13\n\
        # Subtest: adds\n\
        ok 1 - adds\n\
        # Subtest: parses\n\
            ok 1 - inner step\n\
        not ok 2 - parses\n\
        ok 3 - flaky-thing # SKIP reason\n\
        1..3\n";

    #[test]
    fn leaf_and_suite_lines_parse_with_directives() {
        let results = parse_tap_output(REPLAY);
        assert_eq!(results.get("adds"), Some(&RunStatus::Pass));
        assert_eq!(results.get("parses"), Some(&RunStatus::Fail));
        assert_eq!(results.get("flaky-thing"), Some(&RunStatus::Skip));
        assert_eq!(results.get("inner step"), Some(&RunStatus::Pass));
    }
}
